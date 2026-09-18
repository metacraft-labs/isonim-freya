//! The feature-less arm of the two `freya_headless` exports.
//!
//! Same defect, same shape, same fix as
//! `isonim-gpui/rust/gpui-nim-shim/src/gpui_headless_unavailable.rs`;
//! read that module's docs for the full account. In short:
//!
//! `freya_headless` is gated as a whole (`#[cfg(feature =
//! "freya-headless")] pub mod freya_headless;` in `lib.rs`) against a
//! crate whose `default = []`. A plain `cargo build` — which is what
//! `just rust-build` and every consumer of
//! `rust/target/debug/libfreya_nim_shim.so` produce — therefore shipped
//! **46** exported `freya_*` symbols while `bindings.nim` declared
//! **48** under one `{.push dynlib.}`. Nim resolves every referenced
//! `dynlib` symbol at process start, so the Freya launcher aborted
//! before `main`:
//!
//! ```text
//! $ ./build/backends/isonim-examples-freya --demo=tasks --port 39872
//! could not import: freya_render_to_pixels
//! ```
//!
//! which surfaced in `isonim-examples` as
//! `test_freya_launcher_element_tree` (and the multi-renderer gates)
//! reporting "launcher … did not bind to 127.0.0.1:<port> within 4s".
//!
//! Turning the feature on is not the fix: `freya-headless` pulls
//! `freya`, `freya-testing`, `freya-core`, `freya-engine`, `torin`,
//! `skia-safe` and `image`, and the default profile is deliberately the
//! shadow-tree shim that the headless element-tree tests and the Linux
//! launchers are built against. Keeping the **ABI** constant and letting
//! the **behaviour** vary is what `freya_launch` already does one layer
//! up with its `#[cfg(feature = "freya-backend")]` pair.
//!
//! `freya_render_to_pixels` returns `ErrorCode::RuntimeBuild` (2) — "the
//! render runtime could not be constructed", which is precisely true
//! here — after writing `(null, 0)` to the out parameters, so a caller
//! that always frees stays safe. `freya_free_pixels` is a no-op because
//! this arm never allocates.

/// Mirror of `freya_headless::ErrorCode::InvalidArgs` / `RuntimeBuild`.
/// Spelled as literals because that enum lives inside the gated module;
/// the numeric values are the stable part of the FFI contract.
const ERROR_INVALID_ARGS: i32 = 1;
const ERROR_RUNTIME_BUILD: i32 = 2;

/// # Safety
///
/// `out_ptr` and `out_len` MUST be non-null and point to writable
/// storage, as in the real arm.
#[no_mangle]
pub extern "C" fn freya_render_to_pixels(
    _width: u32,
    _height: u32,
    _scale: f32,
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
) -> i32 {
    if out_ptr.is_null() || out_len.is_null() {
        return ERROR_INVALID_ARGS;
    }
    unsafe {
        *out_ptr = std::ptr::null_mut();
        *out_len = 0;
    }
    ERROR_RUNTIME_BUILD
}

/// # Safety
///
/// This arm never allocates, so the only correct argument is a null
/// pointer / zero length. The signature is kept identical to the real
/// arm so the ABI does not vary with features.
#[no_mangle]
pub unsafe extern "C" fn freya_free_pixels(_ptr: *mut u8, _len: usize) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_to_pixels_reports_runtime_build_and_nulls_the_out_params() {
        let mut ptr: *mut u8 = 0x1 as *mut u8;
        let mut len: usize = 99;
        let rc = freya_render_to_pixels(10, 10, 1.0, &mut ptr, &mut len);
        assert_eq!(rc, ERROR_RUNTIME_BUILD);
        assert!(ptr.is_null());
        assert_eq!(len, 0);
    }

    #[test]
    fn render_to_pixels_rejects_null_out_params() {
        let rc = freya_render_to_pixels(10, 10, 1.0, std::ptr::null_mut(), std::ptr::null_mut());
        assert_eq!(rc, ERROR_INVALID_ARGS);
    }
}
