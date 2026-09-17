## isonim_freya/reactive_root.nim — NH-M1 reactive root entry point for Freya.
##
## `renderFreya` is Freya's half of the native mount seam described in
## `codetracer-specs/Front-Ends/IsoNim/Hot-Module-Reload-Native.milestones.org`
## (NH-M1). It delegates to `isonim/renderers/native`'s `renderNative`, which
## opens a `createRoot` scope and re-runs the accessor inside a
## `createRenderEffect`, so NH-M2's hot-component proxy can replace the root
## element without disposing the reactive root.
##
## Unlike GPUI, the Freya shim owns an explicit root slot
## (`freya_set_root_element`), so the windowed entry point can publish the
## current root to the shim directly. The headless launchers keep their own
## handle, which is what the callback overload is for.
##
## SHIM DEPENDENCY. Every proc in `bindings.nim` is `dynlib:
## libfreya_nim_shim.so`; a binary that reaches the shim needs the Rust cdylib
## present at run time. `renderFreyaIntoShimRoot` calls
## `freya_set_root_element` and therefore does; the plain accessor/mount
## overload does not.

import ./renderer
import ./bindings

import isonim/renderers/native as native_root
export native_root.NativeRootAccessor, native_root.NativeRootMount,
       native_root.NativeRootHandle, native_root.renderNative,
       native_root.staticNativeRoot, native_root.dispose,
       native_root.isDisposed

proc renderFreya*(accessor: NativeRootAccessor[FreyaElement];
                  mount: NativeRootMount[FreyaElement]
                 ): NativeRootHandle[FreyaElement] =
  ## Mount `accessor`'s Freya element tree through a reactive root. `mount`
  ## publishes the current root wherever the host needs it (a frame source, the
  ## shim's root slot, a test's capture slot).
  renderNative(accessor, mount)

proc renderFreya*(r: FreyaRenderer; host: FreyaElement;
                  accessor: NativeRootAccessor[FreyaElement]
                 ): NativeRootHandle[FreyaElement] =
  ## Overload for the case where the root is inserted under an existing shim
  ## element: the reactive insert goes through `FreyaRenderer`'s own
  ## `appendChild` / `removeChild`.
  renderNative(r, host, accessor)

proc renderFreyaIntoShimRoot*(accessor: NativeRootAccessor[FreyaElement]
                             ): NativeRootHandle[FreyaElement] =
  ## Windowed entry point: each render effect run publishes the current root to
  ## the shim via `freya_set_root_element`. Requires `libfreya_nim_shim`.
  renderFreya(accessor, proc(node: FreyaElement) =
    freya_set_root_element(node))
