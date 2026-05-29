//! RS-M14 Phase 1 + EPP-M10: headless RGBA rendering via freya-testing's
//! Skia raster path.
//!
//! Used by isonim-render-serve's Freya adapter to deliver real Freya
//! pixels through the F/M/I bridge instead of the synthetic vertical-
//! stack stripes the pre-RS-M14 adapter produced. Runs the same
//! `freya_core::render::RenderPipeline` Freya uses on-screen, just
//! pointed at a `skia-safe` raster CPU surface.
//!
//! ## EPP-M10 perf rework
//!
//! The original RS-M14 implementation called `launch_test_with_config`
//! + `TestingHandler::create_snapshot` per frame. Profiling at the
//! Desktop viewport (1440x900) showed a ~148 ms median frame cost,
//! 3x over the EPP-M8 goal of 50 ms. Per-step breakdown:
//!
//! | Step                                  | Median   |
//! |---------------------------------------|----------|
//! | tokio runtime build                   | 0.03 ms  |
//! | launch_test_with_config (VDOM + fonts)| 62 ms    |
//! | wait_for_update (layout + events)     | 20 ms    |
//! | create_snapshot (Skia raster + PNG)   | 32 ms    |
//! | image-crate PNG decode                | 33 ms    |
//! | **total**                             | **148 ms** |
//!
//! Two interventions land here:
//!
//! 1. **Cache the `TestingHandler` across frames.** Keyed on
//!    `(width, height)`; invalidated on resize. Eliminates the 62 ms
//!    launch cost after the first frame.
//!
//! 2. **Skip the PNG round-trip entirely.** Replace
//!    `TestingHandler::create_snapshot` with a custom render that
//!    drives `freya_core::render::RenderPipeline` directly against
//!    a `raster_n32_premul` surface, then reads pixels via
//!    `surface.read_pixels` into a RGBA8888-non-premultiplied buffer.
//!    No PNG encode and no `image` crate decode. Saves ~50 ms.
//!
//! Combined target: under 50 ms median per frame. Acceptance gate at
//! `tests/test_freya_render_budget.nim`.
//!
//! ## Production-path preservation
//!
//! This module is behind the `freya-headless` Cargo feature and does
//! not touch the existing windowed launch path in `freya_app.rs`.
//! Builds that only enable `freya-backend` (the windowed path) do not
//! pay for the headless surface; builds that enable both can use
//! either entry point.

use std::cell::RefCell;

use freya_core::{
    render::{Compositor, RenderPipeline},
    style::default_fonts as core_default_fonts,
};
use freya_engine::prelude::{
    raster_n32_premul,
    Color,
    ColorType,
    FontCollection,
    FontMgr,
    ImageInfo,
    Surface,
};
use skia_safe::AlphaType;
use freya_testing::prelude::{launch_test_with_config, TestingConfig, TestingHandler};
use tokio::runtime::{Builder as TokioRuntimeBuilder, Runtime};
use torin::geometry::Area;

use crate::render_sync::freya_render::shadow_tree_app;

/// Render the current shadow tree to RGBA8888 pixels at the
/// specified size.
///
/// Returns 0 on success and writes the pixel-buffer pointer to
/// `*out_ptr` + byte count to `*out_len`. The buffer is allocated
/// by this function and MUST be released via
/// [`freya_free_pixels`]; the caller owns the buffer for the
/// duration between the two calls.
///
/// Returns a non-zero error code on failure; on error the function
/// writes a null pointer + zero length to the caller's out
/// parameters so naive callers that always free the buffer remain
/// safe.
///
/// # Color space + byte order
///
/// The output is RGBA8888 non-premultiplied sRGB, row-major, top
/// row first — the canonical F-packet pixel format. We request this
/// format directly from Skia via `read_pixels(..., RGBA_8888,
/// Unpremul, ...)` so no manual BGRA<->RGBA conversion is needed
/// regardless of the host Skia surface's native byte order.
///
/// # Scale semantics
///
/// `width` and `height` are the output pixel dimensions. `scale`
/// is the logical-to-physical ratio: the layout pass runs at
/// `(width / scale, height / scale)` logical pixels. Freya
/// testing's internal `SCALE_FACTOR` is hard-coded to 1.0 in the
/// 0.3.4 release, so we approximate by dividing the configured
/// canvas size — adequate for the 1:1 case the Freya adapter uses
/// today.
///
/// # Safety
///
/// `out_ptr` and `out_len` MUST be non-null and point to writable
/// storage. The caller MUST NOT read or free the output buffer if
/// the function returns a non-zero error code.
#[no_mangle]
pub extern "C" fn freya_render_to_pixels(
    width: u32,
    height: u32,
    scale: f32,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    // 1. Argument validation. Bail before allocating any resources.
    if out_ptr.is_null() || out_len.is_null() {
        return ErrorCode::InvalidArgs as i32;
    }
    // Initialize the out parameters to "null, 0" so the caller can
    // safely call `freya_free_pixels` even after an error.
    unsafe {
        *out_ptr = std::ptr::null_mut();
        *out_len = 0;
    }
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return ErrorCode::InvalidArgs as i32;
    }
    if !scale.is_finite() || scale <= 0.0 {
        return ErrorCode::InvalidArgs as i32;
    }

    // 2. Catch panics so we always return a clean error code across
    //    the FFI boundary — unwinding into Nim is UB.
    let result = std::panic::catch_unwind(|| render_to_rgba(width, height, scale));

    let rgba = match result {
        Ok(Ok(rgba)) => rgba,
        Ok(Err(code)) => return code as i32,
        Err(_) => return ErrorCode::Panic as i32,
    };

    // 3. Hand the buffer over to the caller. `Box::leak` keeps the
    //    allocation alive until `freya_free_pixels` reclaims it.
    let len = rgba.len();
    let boxed: Box<[u8]> = rgba.into_boxed_slice();
    let ptr = Box::leak(boxed).as_mut_ptr();
    unsafe {
        *out_ptr = ptr;
        *out_len = len;
    }
    0
}

/// Free a buffer previously returned by [`freya_render_to_pixels`].
///
/// # Safety
///
/// `ptr` MUST be a pointer returned by `freya_render_to_pixels`,
/// and `len` MUST be the byte count associated with that
/// allocation. Calling with `ptr == null` or `len == 0` is a
/// no-op. Calling twice on the same buffer is undefined behaviour
/// (double-free).
#[no_mangle]
pub unsafe extern "C" fn freya_free_pixels(ptr: *mut u8, len: usize) {
    if ptr.is_null() || len == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts_mut(ptr, len);
    drop(Box::from_raw(slice));
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Upper bound on a single output dimension. Defended against
/// (width * height * 4) overflow; 16384 * 16384 * 4 = 1 GiB which is
/// already absurdly large for an offscreen UI snapshot.
const MAX_DIMENSION: u32 = 16_384;

/// Error codes returned across the FFI boundary. Values are stable
/// and documented in the spec; callers can compare against the
/// numeric constants directly.
#[repr(i32)]
enum ErrorCode {
    InvalidArgs = 1,
    RuntimeBuild = 2,
    SnapshotEmpty = 3,
    #[allow(dead_code)]
    PngDecode = 4,
    SizeMismatch = 5,
    Panic = 6,
}

/// Cached per-thread render context. Reused across `render_to_rgba`
/// calls so VDOM init, font collection setup, surface allocation, and
/// compositor state survive across frames. Invalidated on dimension
/// change (the surfaces and FreyaDOM layout cache are size-dependent).
struct CachedRender {
    runtime: Runtime,
    handler: TestingHandler<()>,
    surface: Surface,
    dirty_surface: Surface,
    compositor: Compositor,
    font_collection: FontCollection,
    font_mgr: FontMgr,
    output: Vec<u8>,
    width: u32,
    height: u32,
}

thread_local! {
    /// Thread-local cache so the render path can be called from the
    /// streaming bridge's frame loop without paying the launch cost
    /// (62 ms median per call pre-cache, ~0 post-cache).
    ///
    /// Stored as a `*mut CachedRender` so the cache is intentionally
    /// leaked on thread exit. Dropping a `TestingHandler` runs the
    /// `VirtualDom`'s destructor, which accesses Dioxus's own
    /// thread-locals — but those may already be destroyed by the time
    /// our TL destructor runs, causing a "cannot access a Thread Local
    /// Storage value during or after destruction" panic. Leaking the
    /// handler sidesteps the ordering hazard; the process is about to
    /// exit anyway, and OS reclaim is sufficient.
    static CACHE: RefCell<*mut CachedRender> = const { RefCell::new(std::ptr::null_mut()) };
}

/// Drive freya-testing's render pipeline to produce raw RGBA bytes.
///
/// EPP-M10 path: caches the `TestingHandler` and surface across
/// frames, and reads pixels directly from the Skia surface to skip
/// the PNG encode/decode round-trip.
fn render_to_rgba(width: u32, height: u32, scale: f32) -> Result<Vec<u8>, ErrorCode> {
    CACHE.with(|cell| {
        let mut slot = cell.borrow_mut();

        // Drop the cache if dimensions changed — surfaces and the
        // freya-core layout cache key on size. We Box::leak the
        // previous cache so the VirtualDom destructor (which touches
        // thread-locals) doesn't run during our cleanup.
        if !slot.is_null() {
            // SAFETY: We allocated the box via Box::leak below and
            // never aliased the pointer.
            let cached_ref = unsafe { &*(*slot) };
            if cached_ref.width != width || cached_ref.height != height {
                // Leak the old cache deliberately — drop ordering is
                // fragile with VirtualDom thread-locals. The shim
                // is a long-running cdylib; size changes are rare.
                *slot = std::ptr::null_mut();
            }
        }

        if slot.is_null() {
            let cache = init_cache(width, height, scale)?;
            let leaked: &'static mut CachedRender = Box::leak(Box::new(cache));
            *slot = leaked as *mut CachedRender;
        }

        // SAFETY: Pointer is leaked-static; no other code reads it.
        let cached = unsafe { &mut **slot };
        render_one_frame(cached, scale)
    })
}

fn init_cache(width: u32, height: u32, scale: f32) -> Result<CachedRender, ErrorCode> {
    let logical_w = (width as f32 / scale).max(1.0);
    let logical_h = (height as f32 / scale).max(1.0);

    let config: TestingConfig<()> = TestingConfig::<()> {
        size: (logical_w, logical_h).into(),
        ..TestingConfig::default()
    };

    let runtime = TokioRuntimeBuilder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| ErrorCode::RuntimeBuild)?;

    // freya-testing's `launch_test_with_config` builds the VDOM,
    // SafeDOM, FontCollection, etc. and runs `init_doms` + `resize`.
    // We capture the handler and keep it alive for the lifetime of
    // this cache slot.
    let handler = launch_test_with_config(shadow_tree_app, config);

    // Surfaces for the EPP-M10 direct-readback path. We allocate a
    // pair to match the (main, dirty) shape `freya_core::RenderPipeline`
    // expects.
    let mut surface = raster_n32_premul((width as i32, height as i32))
        .ok_or(ErrorCode::SnapshotEmpty)?;
    let dirty_surface = surface
        .new_surface_with_dimensions((width as i32, height as i32))
        .ok_or(ErrorCode::SnapshotEmpty)?;

    // Font collection + manager for our render pipeline. The handler's
    // own font_collection is `pub(crate)` so we can't reuse it; we
    // build a parallel pair here. Layout positions come from the
    // handler's pass, which uses the handler's fonts — text glyph
    // rasterization in our render pipeline uses ours. For ASCII text
    // and default fallback, the two produce equivalent output.
    let mut font_collection = FontCollection::new();
    let font_mgr = FontMgr::default();
    font_collection.set_dynamic_font_manager(font_mgr.clone());
    font_collection.set_default_font_manager(font_mgr.clone(), None);

    let output = vec![0u8; (width as usize) * (height as usize) * 4];

    Ok(CachedRender {
        runtime,
        handler,
        surface,
        dirty_surface,
        compositor: Compositor::default(),
        font_collection,
        font_mgr,
        output,
        width,
        height,
    })
}

fn render_one_frame(cached: &mut CachedRender, _scale: f32) -> Result<Vec<u8>, ErrorCode> {
    let width = cached.width;
    let height = cached.height;

    // Drive the VDOM tick from inside the cached runtime so spawned
    // tasks (notably `shadow_tree_app`'s repaint poller) keep running
    // across frames on the same executor.
    cached.runtime.block_on(async {
        cached.handler.wait_for_update().await;
    });

    // Clear surfaces. We don't trust the compositor dirty tracking
    // across our caller's tree mutations (they happen entirely
    // outside the VDOM via the shadow tree FFI) — force a full
    // repaint each frame by clearing both surfaces and marking the
    // entire canvas as dirty.
    cached.surface.canvas().clear(Color::WHITE);
    cached.dirty_surface.canvas().clear(Color::WHITE);

    // Mark the full canvas as the compositor dirty area so the
    // pipeline re-rasterizes everything. This is conservative but
    // simple; future work could plumb the shadow-tree mutation
    // counter to the compositor's per-node dirty set.
    let canvas_area = Area::from_size((width as f32, height as f32).into());

    // Force a full re-render every frame. We can't trust dioxus-side
    // dirty tracking because the shadow tree is mutated entirely
    // outside the VDOM via the FFI tree ops. The cheap path is to
    // ask the compositor to do a full pass and mark the canvas area
    // dirty up-front; this matches what `TestingHandler::create_snapshot`
    // achieves by spinning a fresh `Compositor::default()` per call.
    cached.compositor.reset();

    {
        let fdom = cached.handler.sdom().get();
        {
            let mut dirty_area = fdom.compositor_dirty_area();
            dirty_area.unite_or_insert(&canvas_area);
        }

        // Hold all the mutex guards in locals so the pipeline can
        // borrow them. They are released when this inner block ends,
        // freeing the fdom borrow before we touch the surface again.
        let mut compositor_dirty_area = fdom.compositor_dirty_area();
        let mut compositor_dirty_nodes = fdom.compositor_dirty_nodes();
        let mut compositor_cache = fdom.compositor_cache();
        let mut layers = fdom.layers();
        let mut layout = fdom.layout();
        let mut images_cache = fdom.images_cache();

        let mut pipeline = RenderPipeline {
            canvas_area,
            rdom: fdom.rdom(),
            compositor_dirty_area: &mut compositor_dirty_area,
            compositor_dirty_nodes: &mut compositor_dirty_nodes,
            compositor_cache: &mut compositor_cache,
            layers: &mut layers,
            layout: &mut layout,
            background: Color::WHITE,
            surface: &mut cached.surface,
            dirty_surface: &mut cached.dirty_surface,
            compositor: &mut cached.compositor,
            scale_factor: 1.0,
            selected_node: None,
            font_collection: &mut cached.font_collection,
            font_manager: &cached.font_mgr,
            default_fonts: default_fonts(),
            images_cache: &mut images_cache,
        };
        pipeline.run();
    }

    // Read RGBA8888 non-premultiplied pixels directly from the
    // surface into our cached output buffer. This is the EPP-M10
    // PNG-skip win: no encode, no decode, just a memcpy through
    // Skia's pixel conversion.
    let row_bytes = (width as usize) * 4;
    let image_info = ImageInfo::new(
        (width as i32, height as i32),
        ColorType::RGBA8888,
        AlphaType::Unpremul,
        None,
    );
    let ok = cached
        .surface
        .read_pixels(&image_info, &mut cached.output, row_bytes, (0, 0));
    if !ok {
        return Err(ErrorCode::SnapshotEmpty);
    }

    if cached.output.len() != (width as usize) * (height as usize) * 4 {
        return Err(ErrorCode::SizeMismatch);
    }

    // Return a copy — callers free via `freya_free_pixels`, and we
    // want the cached buffer to stay alive for the next frame.
    Ok(cached.output.clone())
}

fn default_fonts<'a>() -> &'a [String] {
    use std::sync::OnceLock;
    static DEFAULT_FONTS: OnceLock<Vec<String>> = OnceLock::new();
    DEFAULT_FONTS.get_or_init(core_default_fonts)
}
