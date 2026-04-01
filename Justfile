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

# Run Nim tests (requires Rust shim to be built: just rust-build)
test:
    LD_LIBRARY_PATH=rust/target/debug:${LD_LIBRARY_PATH:-} nim c -r --nimcache:nimcache/test_basic tests/test_basic.nim
    LD_LIBRARY_PATH=rust/target/debug:${LD_LIBRARY_PATH:-} nim c -r --nimcache:nimcache/test_renderer tests/test_renderer.nim

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
    rm -rf target nimcache tests/test_basic tests/test_bindings tests/test_renderer
    cd rust && cargo clean
