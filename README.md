# isonim-freya

Nim bindings for [Freya](https://github.com/marc2332/freya), a native GUI
framework built on Dioxus and Skia (via WGPU). Implements the IsoNim
`RendererBackend` interface so that IsoNim's reactive core and DSL can drive
a Freya-based native GUI.

## Architecture

```
Nim (IsoNim DSL / reactive core)
  |
  v
Nim C bindings  (src/isonim_freya/bindings.nim)
  |
  v  (extern "C" FFI)
Rust shim       (rust/freya-nim-shim/)
  |
  v
Freya / Dioxus / Skia
```

## Development

```bash
nix develop          # enter dev shell with Rust + Nim + Skia deps
just rust-check      # verify Rust crate compiles
just nim-check       # verify Nim code compiles
```
