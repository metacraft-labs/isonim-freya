# isonim-freya build commands

# Check that the Rust shim compiles
rust-check:
    cd rust && cargo check

# Build the Rust shim as a cdylib
rust-build:
    cd rust && cargo build

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
test:
    LD_LIBRARY_PATH=rust/target/debug:${LD_LIBRARY_PATH:-} nim c -r --nimcache:nimcache/test_basic tests/test_basic.nim
    LD_LIBRARY_PATH=rust/target/debug:${LD_LIBRARY_PATH:-} nim c -r --nimcache:nimcache/test_renderer tests/test_renderer.nim
    LD_LIBRARY_PATH=rust/target/debug:${LD_LIBRARY_PATH:-} nim c -r --nimcache:nimcache/test_window tests/test_window.nim

# Run cross-renderer tests (M5 — requires Rust shim and isonim)
test-cross:
    LD_LIBRARY_PATH=rust/target/debug:${LD_LIBRARY_PATH:-} nim c -r --path:../isonim/src --nimcache:nimcache/test_cross_renderer tests/test_cross_renderer.nim

# Run demo app tests (M6 — requires Rust shim and isonim)
test-demo:
    LD_LIBRARY_PATH=rust/target/debug:${LD_LIBRARY_PATH:-} nim c -r --path:../isonim/src --path:demos/task-manager/src --nimcache:nimcache/test_demo_app tests/test_demo_app.nim

# Build the demo app (headless mode)
demo-build:
    LD_LIBRARY_PATH=rust/target/debug:${LD_LIBRARY_PATH:-} nim c --path:../isonim/src --nimcache:nimcache/demo_task_manager demos/task-manager/src/main.nim

# Run the demo app (headless mode — prints interaction trace)
demo-run:
    LD_LIBRARY_PATH=rust/target/debug:${LD_LIBRARY_PATH:-} nim c -r --path:../isonim/src --nimcache:nimcache/demo_task_manager demos/task-manager/src/main.nim

# Run render integration tests (G3-F — requires Rust shim and isonim)
test-integration:
    LD_LIBRARY_PATH=rust/target/debug:${LD_LIBRARY_PATH:-} nim c -r --path:../isonim/src --nimcache:nimcache/test_render_integration tests/test_render_integration.nim

# Run all tests (Rust + Nim)
test-all: rust-test test test-cross test-demo test-integration

# Generate Nim bindings from Rust shim using nbindgen
generate-bindings:
    ./tools/generate_bindings.sh

# Validate that Nim bindings match Rust exports
check-bindings:
    ./tools/check_bindings.sh

# Check that the binding test compiles
nim-check-bindings:
    nim c --nimcache:nimcache/test_bindings tests/test_bindings.nim

# Clean build artifacts
clean:
    rm -rf target nimcache tests/test_basic tests/test_bindings tests/test_renderer tests/test_window
    cd rust && cargo clean
