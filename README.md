# isonim-freya

Nim bindings for [Freya](https://github.com/marc2332/freya), a native GUI
framework built on Dioxus and Skia (via WGPU). Implements the IsoNim
`RendererBackend` interface so that IsoNim's reactive core and DSL can drive
a Freya-based native GUI.

## Architecture

```
Nim (IsoNim DSL / reactive core)
  │
  v
Nim C bindings  (src/isonim_freya/bindings.nim)
  │
  v  extern "C" FFI
Rust shim       (rust/freya-nim-shim/)
  │
  v
Freya / Dioxus / Skia
```

The Rust shim maintains an in-memory **shadow tree** that mirrors the Nim-side
element tree. A **render plan** converts this tree into Freya-native elements.
The `freya-backend` Cargo feature gates actual Freya rendering; without it the
shim provides the shadow tree API only (for testing and CI).

## Prerequisites

- [Nix](https://nixos.org/) with flakes enabled
- direnv (recommended)
- The `isonim` core library checked out as a sibling: `../isonim/`

## Quick Start

```bash
# Enter dev shell (Rust + Nim + Skia/GPU deps)
direnv allow   # or: nix develop

# Build the Rust shim (stub mode, no display server needed)
just rust-build

# Run all Rust tests
just rust-test

# Run all Nim tests
just test-all
```

## Running the Demo App

The repo includes a **Task Manager** demo at `demos/task-manager/src/main.nim`.
It exercises signals, memos, reactive rendering, event dispatch, and tree
mutations — the same app that runs in the browser via isonim's web renderer.

### Headless mode (no display server)

Builds the UI tree and runs through all interactions programmatically,
printing the results to stdout:

```bash
just demo-run
```

### Window mode (requires display server)

First build the Rust shim with the Freya backend enabled, then compile with
`-d:freyaGui`:

```bash
just rust-build          # build the shim library
LD_LIBRARY_PATH=rust/target/debug:${LD_LIBRARY_PATH:-} \
  nim c -r -d:freyaGui --path:../isonim/src demos/task-manager/src/main.nim
```

> Window mode requires a running X11 or Wayland display. For headless CI
> environments, use the Xvfb wrapper (see Testing below).

## Testing

### Nim tests

```bash
just test              # core renderer tests
just test-cross        # cross-renderer compatibility with isonim
just test-demo         # task manager demo verification
just test-integration  # render plan integration tests
just test-structural   # structural comparison tests
just test-all          # all of the above + Rust tests
```

### Rust tests

```bash
just rust-test                                    # lib + integration tests (stub mode)
cd rust/freya-nim-shim && cargo test              # same, from crate dir
cd rust/freya-nim-shim && cargo test --features freya-backend  # with Freya backend
```

The integration test suite (`rust/freya-nim-shim/tests/freya_rendering.rs`)
uses the `freya-testing` crate to render through the actual Freya/Skia
pipeline headlessly — no display server needed.

### GUI tests under headless display

```bash
just test-gui-x11                # run GUI tests under Xvfb
just test-gui-wayland            # run GUI tests under headless Sway
just test-gui-record             # run under Xvfb and record video
just test-gui-x11 --stream       # run under Xvfb with live video stream
```

## Project Structure

```
isonim-freya/
├── flake.nix                       # Nix flake (Rust + Nim + Skia/GPU deps)
├── Justfile                        # Build/test commands
├── scripts/
│   ├── xvfb-run-test.sh           # X11 headless test runner
│   └── wayland-run-test.sh        # Wayland headless test runner
├── rust/
│   └── freya-nim-shim/
│       ├── src/
│       │   ├── lib.rs             # extern "C" FFI exports
│       │   ├── tree.rs            # Shadow element tree
│       │   ├── render_sync.rs     # Render plan builder
│       │   ├── freya_app.rs       # Freya app launcher
│       │   └── window.rs          # Window state machine
│       └── tests/
│           └── freya_rendering.rs # Integration tests (freya-testing)
├── src/isonim_freya/
│   ├── bindings.nim               # Raw C bindings to Rust shim
│   └── renderer.nim               # FreyaRenderer (RendererBackend impl)
├── tests/                         # Nim test suite
└── demos/task-manager/            # Task Manager demo app
```
