# isonim-freya build commands

# Check that the Rust shim compiles
rust-check:
    cd rust && cargo check

# Build the Rust shim as a cdylib
rust-build:
    cd rust && cargo build

# Build the project: the default (feature-less) Rust shim, the one artefact
# this repository produces -- the Nim side is a source library. This is the
# standard recipe `.github/workflows/ci-reprobuild.yml` cross-checks with
# `just build`; without it that step failed with "Justfile does not contain
# recipe `build`".
build: rust-build

# Build the Rust shim in release mode
rust-build-release:
    cd rust && cargo build --release

# Check that the Nim code compiles
nim-check:
    nim c --nimcache:nimcache/test_basic tests/test_basic.nim

# Check that the renderer compiles
nim-check-renderer:
    nim check --nimcache:nimcache/check_renderer src/isonim_freya/renderer.nim

# Check that the window module compiles
nim-check-window:
    nim check --nimcache:nimcache/check_window src/isonim_freya/window.nim

# Run Rust tests
rust-test:
    cd rust && cargo test

# Run Nim tests (requires Rust shim to be built: just rust-build)
#
# `tools/with_shim_path.sh` puts the shim's directory on LD_LIBRARY_PATH.
# That directory is `<cargo target dir>/debug`, and the target dir is
# whatever cargo resolves (`CARGO_TARGET_DIR`, `.cargo/config.toml`), not
# necessarily `rust/target`; `tools/cargo_target_dir.sh` asks cargo for it.
test:
    nim c -r --hints:off tests/test_nimcache_is_worktree_local.nim
    ./tools/with_shim_path.sh nim c -r --nimcache:nimcache/test_basic tests/test_basic.nim
    ./tools/with_shim_path.sh nim c -r --nimcache:nimcache/test_renderer tests/test_renderer.nim
    ./tools/with_shim_path.sh nim c -r --nimcache:nimcache/test_window tests/test_window.nim

# Run cross-renderer tests (M5 — requires Rust shim and isonim)
test-cross:
    ./tools/with_shim_path.sh nim c -r --path:../isonim/src --nimcache:nimcache/test_cross_renderer tests/test_cross_renderer.nim

# Build the canonical task-app demo (lives in isonim-examples since
# EX-M4; this recipe just defers to that repo's composition root).
demo-build:
    ./tools/with_shim_path.sh nim c --path:../isonim/src --path:../isonim-examples --nimcache:nimcache/demo ../isonim-examples/task_app/main_freya.nim

# Run the canonical task-app demo (headless mode). Sources live in
# `isonim-examples/task_app/` per the EX-M4 migration; the Rust shim
# is still built in this repo.
demo-run:
    ./tools/with_shim_path.sh nim c -r --path:../isonim/src --path:../isonim-examples --nimcache:nimcache/demo ../isonim-examples/task_app/main_freya.nim

# Run render integration tests (G3-F — requires Rust shim and isonim)
test-integration:
    ./tools/with_shim_path.sh nim c -r --path:../isonim/src --nimcache:nimcache/test_render_integration tests/test_render_integration.nim

# Run structural comparison tests (G4 — requires Rust shim and isonim)
test-structural:
    ./tools/with_shim_path.sh nim c -r --path:../isonim/src --nimcache:nimcache/test_structural_comparison tests/test_structural_comparison.nim

# NH-M1: the reactive root seam (`renderFreya`). Requires the Rust shim —
# every `freya_*` binding is `dynlib`, so without `just rust-build` the binary
# does not start ("could not load: libfreya_nim_shim.so"). That hard failure
# is deliberate: there is no skip arm, because a suite that went green without
# the shim would be asserting nothing about Freya.
test-reactive-root:
    ./tools/with_shim_path.sh nim c -r --path:../isonim/src --nimcache:nimcache/test_render_native_freya_reactive tests/test_render_native_freya_reactive.nim

# Run all tests (Rust + Nim + cross-renderer + integration). The
# task-manager demo's tests live in `isonim-examples/tests/` since
# EX-M4 (`test_freya_leaves_end_to_end.nim`); run them via that repo's
# `just test` recipe.
# NH-M3: the per-renderer reconciler instance and its identity gate.
# Requires the Rust shim (`just rust-build`) for the same reason
# `test-reactive-root` does: every `freya_*` binding is `dynlib`, so
# without it the binary does not start. No skip arm — a suite that went
# green without the shim would be asserting nothing about Freya.
test-reconciler:
    ./tools/with_shim_path.sh nim c -r --path:../isonim/src -d:isonimHmr --nimcache:nimcache/test_freya_reconciler tests/test_freya_reconciler_identity.nim

test-all: check-exported-symbols rust-test test test-cross test-integration test-structural test-reactive-root test-reconciler test-pixel-lengths

# Run GUI tests under Xvfb (X11 headless)
test-gui-x11 *ARGS:
    cd rust && cargo build --features freya-backend
    ./scripts/xvfb-run-test.sh {{ARGS}} just _run-gui-tests

# Run GUI tests under Wayland headless (Sway)
test-gui-wayland *ARGS:
    cd rust && cargo build --features freya-backend
    ./scripts/wayland-run-test.sh {{ARGS}} just _run-gui-tests

# Run GUI tests with video recording (X11)
test-gui-record:
    just test-gui-x11 --record

# Internal: actual GUI test commands (run inside headless display)
_run-gui-tests:
    ./tools/with_shim_path.sh nim c -r --nimcache:nimcache/test_gui -d:freyaBackend --path:../isonim/src tests/test_gui.nim

# Generate Nim bindings from Rust shim using nbindgen
generate-bindings:
    ./tools/generate_bindings.sh

# Validate that Nim bindings match Rust exports.
#
# SOURCE-LEVEL. This reads `extern "C"` declarations out of the crate's
# .rs files and is `cfg`-blind by construction, so it cannot see a
# declaration that the default feature selection compiles out. Run
# `check-exported-symbols` for that; the two are complements, not
# alternatives, and only the second one can catch a "could not import:"
# at process load.
check-bindings:
    ./tools/check_bindings.sh

# Check that every symbol bindings.nim imports is EXPORTED BY A BUILT
# SHIM. Builds the default (feature-less) profile itself and reads
# `nm -D` off it — see the script header and
# `codetracer-specs/Testing/Verification-Harness-Traps.md` §18.
check-exported-symbols:
    ./tools/check_exported_symbols.sh

# Check that the binding test compiles
nim-check-bindings:
    nim c --nimcache:nimcache/test_bindings tests/test_bindings.nim

# Clean build artifacts
clean:
    rm -rf target nimcache tests/test_basic tests/test_bindings tests/test_renderer tests/test_window
    cd rust && cargo clean

# Actual public FFI payloads into checksum-bound original native parsers.
test-pixel-lengths: rust-build
    python3 tools/test_pixel_lengths.py
