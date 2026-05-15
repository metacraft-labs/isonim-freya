//! RS-M14 Phase 1: headless RGBA rendering via freya-testing's Skia raster path.
//!
//! Used by isonim-render-serve's Freya adapter to deliver real Freya
//! pixels through the F/M/I bridge instead of the synthetic vertical-
//! stack stripes the pre-RS-M14 adapter produced. Runs the same
//! `freya_core::render::RenderPipeline` Freya uses on-screen, just
//! pointed at a `skia-safe` raster CPU surface — see
//! `freya-testing`'s `TestingHandler::create_snapshot` for the
//! reference implementation pattern.
//!
//! ## Production-path preservation
//!
//! This module is behind the `freya-headless` Cargo feature and does
//! not touch the existing windowed launch path in `freya_app.rs`.
//! Builds that only enable `freya-backend` (the windowed path) do not
//! pay for the headless surface; builds that enable both can use
//! either entry point.
//!
//! ## Snapshot encoding
//!
//! `TestingHandler::create_snapshot()` in freya-testing 0.3.4 returns
//! a PNG-encoded `skia_safe::Data` blob — the encoder is hard-wired to
//! `EncodedImageFormat::PNG` and the surface fields are `pub(crate)`,
//! so we cannot access the raw `surface.peek_pixels()` from outside
//! the crate. We pay one round-trip (encode → decode) through the
//! `image` crate's PNG decoder. The decoded buffer is RGBA8888
//! non-premultiplied in sRGB byte order, which is exactly what the
//! F-packet protocol expects, so no manual BGRA↔RGBA conversion is
//! needed.

use freya_testing::prelude::{launch_test_with_config, TestingConfig};
use tokio::runtime::Builder as TokioRuntimeBuilder;

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
/// row first — the canonical F-packet pixel format. PNG decoding
/// via the `image` crate normalizes whatever in-memory layout
/// Skia used (BGRA pre-multiplied on most platforms) into this
/// canonical shape, so callers receive bytes that can be fed
/// straight into a `canvas.putImageData` `ImageData(RGBA8888)`
/// without manual conversion.
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
    let result = std::panic::catch_unwind(|| {
        render_to_rgba(width, height, scale)
    });

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
    PngDecode = 4,
    SizeMismatch = 5,
    Panic = 6,
}

/// Drive freya-testing's render pipeline to produce raw RGBA bytes.
fn render_to_rgba(width: u32, height: u32, scale: f32) -> Result<Vec<u8>, ErrorCode> {
    // The layout canvas is sized in logical pixels; the snapshot is
    // emitted at the same logical resolution because freya-testing
    // 0.3.4 hard-codes SCALE_FACTOR = 1.0. The output buffer we
    // return is at *output* pixel dimensions (width × height).
    let logical_w = (width as f32 / scale).max(1.0);
    let logical_h = (height as f32 / scale).max(1.0);

    let config: TestingConfig<()> = TestingConfig::<()> {
        size: (logical_w, logical_h).into(),
        ..TestingConfig::default()
    };

    // freya-testing's pipeline uses tokio internally (signal
    // dispatch, async wait_for_update). Build a single-threaded
    // current-thread runtime to drive the snapshot synchronously.
    let runtime = TokioRuntimeBuilder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| ErrorCode::RuntimeBuild)?;

    let png_bytes: Vec<u8> = runtime.block_on(async move {
        let mut utils = launch_test_with_config(shadow_tree_app, config);
        // Apply the latest VDOM changes; the shadow tree has already
        // been populated by Nim before this entry point was called.
        utils.wait_for_update().await;
        let data = utils.create_snapshot();
        // skia_safe::Data derefs to &[u8]; copy out before dropping.
        data.as_bytes().to_vec()
    });

    if png_bytes.is_empty() {
        return Err(ErrorCode::SnapshotEmpty);
    }

    // PNG decode → canonical RGBA8888 sRGB. The `image` crate
    // normalises the channel order regardless of how Skia encoded
    // the surface internally (BGRA premultiplied on most platforms).
    let img = image::load_from_memory_with_format(
        &png_bytes,
        image::ImageFormat::Png,
    ).map_err(|_| ErrorCode::PngDecode)?;
    let rgba = img.to_rgba8();

    // freya-testing renders at the configured logical size; if the
    // caller asked for a different output size, fall back to a
    // size-mismatch error rather than returning a buffer whose
    // dimensions disagree with what was advertised. Most callers
    // pass scale = 1.0 and the sizes line up.
    let actual_w = rgba.width();
    let actual_h = rgba.height();
    let expected_w = width;
    let expected_h = height;
    if actual_w == expected_w && actual_h == expected_h {
        return Ok(rgba.into_raw());
    }

    // For now we accept the size as authoritative — return the
    // pixels at the size the renderer produced. The callers in this
    // milestone pass scale = 1.0 so this is exercised only as a
    // defensive resize. If scale != 1.0 we'd want a proper resampler
    // here; the spec deliberately scopes that out of Phase 1.
    let _ = (actual_w, actual_h, expected_w, expected_h);
    // Surface the mismatch as an error code; callers can fall back
    // to the synthetic-stripes path documented in the adapter.
    Err(ErrorCode::SizeMismatch)
}

