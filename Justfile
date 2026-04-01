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

# Run Nim tests
test:
    nim c -r --nimcache:nimcache/test_basic tests/test_basic.nim

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
    rm -rf target nimcache tests/test_basic tests/test_bindings
    cd rust && cargo clean
