## Reprobuild project file for isonim-freya.
##
## **Typed-Cross-Project-Deps rollout — a Nim CONSUMER of the isonim
## ecosystem (SC-11 develop-mode from-source sibling consumption) with a
## Rust cdylib shim, built by a cargo edge, consumed at runtime via dynlib.**
## ``isonim-freya`` provides the Nim ``RendererBackend`` bindings for the
## Freya native GUI framework. Its ``src/isonim_freya/*`` module tree is a
## Nim binding layer over a Rust ``cdylib`` (``rust/freya-nim-shim``):
## ``src/isonim_freya/bindings.nim`` declares the FFI as
## ``{.push cdecl, dynlib: "libfreya_nim_shim.so".}`` — a bare-soname
## ``dlopen`` resolved at RUNTIME (NOT at link time), so the shim only has
## to be reachable by the dynamic loader when a test binary runs. The tests
## reach it via ``LD_LIBRARY_PATH=rust/target/debug`` in the ``Justfile``;
## this recipe instead bakes an ABSOLUTE ``-Wl,-rpath,<repo>/rust/target/debug``
## into every test binary (via ``extraPassL``), so the shim ``dlopen``s from
## its rpath with no runtime env at all.
##
## **The Rust shim is a cargo build edge** (``cargo.build`` from
## ``repro_dsl_stdlib/packages/cargo``, collected into ``default``, the shape
## ``isonim-gpui/repro.nim`` uses for its shim). It builds the DEFAULT
## (no-feature) profile → ``rust/target/debug/libfreya_nim_shim.so``, which
## provides the shadow-tree + render-plan API the whole headless corpus
## exercises; the Skia/GPU ``freya-backend`` / ``freya-headless`` features are
## only needed by the display-gated GUI arm (deferred, see below). Every test
## BUILD edge runs ``after`` it and every EXECUTE edge lists the library in
## ``requiredBinaries``, so the shim is built before a test runs and a changed
## shim re-runs the tests.
##
## This used to be a PREBUILT artifact built out-of-band (``just rust-build``)
## BEFORE ``repro build``/``repro test`` -- on the stated ground that
## reprobuild had no ``cargo`` tool. It has one, and nothing in
## ``.github/workflows/ci-reprobuild.yml`` builds the shim before ``repro
## test`` runs, so in a fresh checkout every execute edge died with
## ``could not load: libfreya_nim_shim.so``.
##
## One landed workspace Nim-library producer plus its transitive platform
## seam are consumed from source at build time:
##
##   * ``isonim`` — the isomorphic reactive UI framework. The render-plan
##     integration + structural-comparison tests
##     (``import isonim/core/{signals,computation,owner}``) drive the REAL
##     isonim reactive core against ``FreyaRenderer``. Producer:
##     ``isonim/repro.nim`` → ``library isonim`` (exported path ``src``).
##   * ``nim-everywhere`` — the cross-target platform seam isonim's reactive
##     core pulls in transitively (``isonim/core/platform.nim`` does
##     ``import nim_everywhere/platform``). Producer:
##     ``nim-everywhere/repro.nim`` → ``library nim_everywhere``.
##
## The repo's own ``Justfile`` ``test-integration`` / ``test-structural``
## recipes resolve these with a hand-maintained ``--path:../isonim/src``
## flag (and the flake overrides ``isonim`` as a sibling input, which drags
## its own ``nim-everywhere`` seam in). This recipe expresses those two
## sibling dependencies the reprobuild-native way instead: ``uses:
## "<sibling>"`` names each PRODUCER project by its workspace directory
## name; reprobuild builds each from source (its ``library`` edge) and
## threads its ``src/`` root onto this repo's ``nim c --path:`` via the
## SC-11 ``nimPathDirs`` aux channel (Cross-Repo-Source-Consumption.md
## §4.2a) — replacing the hardcoded ``../isonim/src`` literal. Editing a
## sibling's ``src/`` invalidates + rebuilds this repo's affected test
## compiles. Mirrors the ``ngx-isonim/repro.nim`` consumer precedent
## (``uses: "isonim"`` + ``uses: "nim-everywhere"``).
##
## Both siblings are in the rollout's AVAILABLE set (each ships a landed
## ``repro.nim`` with a ``library`` export), so this is proper SC-11
## develop-mode consumption — NOT a SKIP and NOT a hardcoded path.
##
## **Third-party deps (NOT ``uses:``).** The isonim-SSR-adjacent reactive
## core transitively pulls in two status-im workspace source trees —
## ``../nim-faststreams`` (the isonim nimble ``requires "faststreams"`` dep)
## and ``../nim-stew`` — exactly as isonim's own build resolves them. These
## are THIRD-PARTY upstreams EXCLUDED from the rollout (no ``repro.nim``
## ``library`` export), so they are NOT ``uses:`` sibling-from-source edges:
## they are threaded via the edge ``paths:`` slot the way the repo's own
## build treats them (matching ``ngx-isonim/repro.nim``). If/when they land
## a ``repro.nim`` with a ``library`` export they can be promoted to
## ``uses:`` edges.
##
## A Mode 1 / Mode 3 hybrid (per
## ``reprobuild-specs/Three-Mode-Convention-System.md``) modelled on the
## canonical Nim-consumer recipe ``ngx-isonim/repro.nim`` (``uses: "isonim"``
## + ``uses: "nim-everywhere"``) and the leaf ``nim-libvterm/repro.nim``:
##
## * Declares the toolchain floor via ``uses:`` (``nim`` + ``gcc``) plus the
##   two sibling ``uses:`` edges. Mirrors the nimble file's
##   ``requires "nim >= 2.0.0"`` + ``requires "isonim >= 0.1.0"``.
## * Declares ``library isonim_freya`` — the importable ``src/`` tree (so a
##   downstream repo — e.g. ``isonim-examples`` — could consume this
##   renderer via ``uses: "isonim-freya"``). The exported path is ``src``
##   (convention default). The importable umbrella modules are
##   ``src/isonim_freya/{renderer,bindings,window}`` (consumers
##   ``import isonim_freya/renderer`` etc.).
## * Emits, per HEADLESS-runnable test file under ``tests/``, a BUILD edge
##   (``buildNimUnittest.build``) that compiles ``build/test-bin/<stem>`` and
##   an EXECUTE edge (``edge.testBinary.run``) that runs it — the two-edge
##   test template from ``reprobuild-specs/Package-Model.md`` §"The test
##   template". BUILD halves collect into ``test-builds``; EXECUTE halves
##   into ``test`` so ``repro build test`` / ``repro test`` materialise the
##   runnable closure (each execute edge transitively depends on its build
##   edge). EVERY test binary is compiled with the absolute shim rpath so it
##   ``dlopen``s ``libfreya_nim_shim.so`` at run time.
##
## **Two path groups.**
##
##   * **Self-only group** — ``import isonim_freya/*`` only; no isonim
##     sibling. ``paths = @["src"]`` supplies ``--path:src`` (the repo's
##     ``nim.cfg`` bakes ``--path:src`` + ``--path:../isonim/src``, but the
##     engine build does not read ``nim.cfg``, so ``src`` is passed
##     explicitly and the isonim path is threaded only for the consumer
##     group). Five files:
##       - ``test_basic``      (compile-time signature conformance + smoke)
##       - ``test_bindings``   (19 FFI binding signatures, compile-time)
##       - ``test_renderer``   (tag/style/attr mapping + callback registry)
##       - ``test_window``     (FreyaWindow lifecycle / event loop)
##       - ``test_gui``        (render-plan smoke + launch-integration suites;
##         see the ``freyaBackend`` gate note — the DEFAULT no-feature build
##         runs fully HEADLESS, no display server; only the
##         ``when defined(freyaBackend)`` arm needs a display + the Skia
##         ``freya-backend`` shim, which is DEFERRED below).
##
##   * **isonim-consumer group** (SC-11) — ``import isonim/core/*`` drives
##     the real reactive core. The ``isonim/src`` + ``nim-everywhere/src``
##     roots are threaded automatically by the ``uses:`` ``nimPathDirs``
##     channel; the edge ``paths:`` adds only this repo's own ``src`` and the
##     THIRD-PARTY ``../nim-faststreams`` + ``../nim-stew`` trees. Three files:
##       - ``test_render_integration``   (Nim→FFI→Rust→render-plan pipeline)
##       - ``test_structural_comparison`` (task-manager render-plan structure)
##       - ``test_cross_renderer``        (FreyaRenderer vs MockRenderer vs
##         TerminalRenderer parity; ``import isonim/renderers/terminal_demo``.
##         Formerly BROKEN by a stale ``isonim/renderers/terminal`` import
##         after the isonim ``2b0567f`` rename; now FIXED + MODELLED — the
##         one-line import fix mirrors the sibling ``isonim-gpui`` commit
##         ``ff0b7c2``. ``terminal_demo`` still exports ``TerminalRenderer`` /
##         ``TerminalNode`` / ``textContent`` / ``fireEvent``, so every
##         ``terminal.<sym>`` qualifier is preserved via ``as terminal``.)
##
## **Per-test platform gating.** Every emitted test file compiles + runs to
## exit 0 under ``nim c`` on this Linux host — verified by a direct ``nim c
## -r`` sweep with the same paths + rpath the edges below use. The ONLY
## OS/feature conditional in the emitted corpus is ``test_gui.nim``'s
## ``when defined(freyaBackend):`` suite (a POSITIVE arm that is EXCLUDED on
## the default build — we do NOT pass ``-d:freyaBackend`` — not a gate that
## excludes the file). So there are no ``when defined(<os>)`` extraction
## gates: the emitted corpus is portable-and-runnable here and every edge is
## unconditionally in the graph.
##
## ==========================================================================
## DEFERRED / not-modelled test sets (documented, NOT deleted or weakened)
## ==========================================================================
##
## (B) **``test_gui``'s ``when defined(freyaBackend)`` GUI arm** — the
##     ``Justfile`` ``test-gui-x11`` / ``test-gui-wayland`` recipes compile
##     ``test_gui`` with ``-d:freyaBackend`` (which needs the shim rebuilt
##     ``--features freya-backend`` → Skia/GPU) and run it under a headless
##     display (Xvfb / Sway via ``scripts/{xvfb,wayland}-run-test.sh``). That
##     is a live-display + GPU-shim integration path, un-provisionable in
##     this headless sandbox, so it is DEFERRED: ``test_gui`` is emitted
##     WITHOUT ``-d:freyaBackend`` (its render-plan-smoke + launch-integration
##     suites run fully headless against the default shim), and the
##     display-gated ``freya_backend_feature_enabled`` suite is simply not
##     compiled in on this build. NOT weakened — the file's headless suites
##     all run to exit 0; only the display arm is off its environment.
##
## (C) **Rust ``cargo test`` corpus** (``rust/freya-nim-shim/tests/*.rs`` —
##     ``freya_rendering.rs``, ``test_headless_render.rs``) + the ``Justfile``
##     ``rust-test`` / ``test-gui-record`` recipes. These exercise the Rust
##     side directly; only the shim's BUILD is an edge here (see the
##     header), so the Rust unit corpus is NOT modelled as reprobuild
##     edges. Run via the repo's
##     own ``just rust-test`` in its Rust dev shell. Out of the Nim ``nim c``
##     test scope this recipe covers.
##
## **Tool provisioning.** ``defaultToolProvisioning "path"`` matches the
## canonical recipes: the nix dev shell puts ``nim`` + ``gcc`` on ``PATH``,
## so the weak-local PATH resolver is the right default. It is also required
## for the ``uses:`` declarations to resolve at all ("typed tool provisioning
## is required for uses declarations").

import std/os
import repro_project_dsl
import repro_dsl_stdlib/foreign_env

# ``ct_test_nim_unittest`` supplies the ``buildNimUnittest.build(...)``
# typed-tool used by every test BUILD edge and the ``edge.testBinary.run(...)``
# UFCS dispatch for the EXECUTE edges. It re-exports ``repro_project_dsl`` so
# the import order is unimportant. Like the other consumer sibling recipes
# this file does NOT import ``ct_test_runner_install`` (engine-coupled,
# reprobuild-internal): the execute edges route through the engine's default
# direct-binary runner (run the binary, key on exit status), which is exactly
# the exit-0 verification this corpus needs — Nim ``unittest`` prints per-suite
# results and exits non-zero on failure.
import ct_test_nim_unittest

# Absolute path to the Rust cdylib's output directory. ``currentSourcePath``
# is this ``repro.nim`` at the repo root, so ``parentDir`` is the repo root;
# joining ``rust/target/debug`` + ``absolutePath`` yields the directory that
# holds ``libfreya_nim_shim.so``. Baking this as an rpath onto every test
# binary lets the ``{.dynlib: "libfreya_nim_shim.so".}`` FFI ``dlopen`` the
# shim at run time with NO ``LD_LIBRARY_PATH`` (matching the ``Justfile``'s
# ``LD_LIBRARY_PATH=rust/target/debug`` intent, hermetically). The shim is
# built by the ``cargo.build`` edge below (see the header).
const repoRoot = currentSourcePath().parentDir()
let shimRpath = absolutePath(repoRoot / "rust" / "target" / "debug")

type
  FreyaTestSpec = object
    ## One entry per HEADLESS-runnable test file. ``source`` is the
    ## repo-relative ``.nim`` path; ``binary`` is the ``build/test-bin/<stem>``
    ## output (``<stem>.exe`` on Windows, see ``testBinaryExt``);
    ## ``consumesIsonim`` selects the isonim-consumer path group.
    source: string
    binary: string
    consumesIsonim: bool

# The test binary's file extension on the host the graph is built for.
# ``nim c --out:build/test-bin/<stem>`` writes ``<stem>.exe`` on Windows, and
# ``buildNimUnittest.build`` declares that ``.exe`` as the compile edge's
# output, but it hands the ``binary`` it was given UNCHANGED to the typed
# ``testBinary`` that ``edge.testBinary.run`` executes; its contract is that
# the caller names the runnable file. Naming ``<stem>`` here therefore made
# every Windows execute edge launch a file that does not exist
# (``CreateProcessW failed (err=2): …\build\test-bin\test_basic``). With the
# suffix in ``binary`` itself, the compile output, the typed binary and the
# executed path are the same file, and ``buildNimUnittest`` does not append a
# second ``.exe``. The nimcache directory and the action ids derive from
# ``splitFile(binary).name``, so they are identical on every host.
const testBinaryExt = when defined(windows): ".exe" else: ""

proc spec(stem: string; consumesIsonim = false): FreyaTestSpec =
  FreyaTestSpec(source: "tests/" & stem & ".nim",
    binary: "build/test-bin/" & stem & testBinaryExt,
    consumesIsonim: consumesIsonim)

# The HEADLESS native corpus. The four self-only tests + the headless
# ``test_gui`` need only ``--path:src``; the three isonim-consumer tests
# additionally get the SC-11 sibling ``src`` roots (threaded off the
# ``uses:`` edges) + the third-party faststreams/stew paths. Every entry
# compiles + runs to exit 0 under ``nim c`` on this Linux host. The
# ``-d:freyaBackend`` GUI arm and the Rust ``cargo`` corpus are DEFERRED
# per the module docstring (sets B / C); none is run off its environment
# and none is weakened. ``test_cross_renderer`` (formerly deferred as a
# stale-sibling-import bug) is now FIXED + modelled — see the consumer
# group note above (mirrors isonim-gpui ``ff0b7c2``).
const freyaTestSpecs: seq[FreyaTestSpec] = @[
  # --- Self-only group (import isonim_freya/* only) ---
  spec("test_basic"),
  spec("test_bindings"),
  spec("test_renderer"),
  spec("test_window"),
  spec("test_gui"),
  # --- isonim-consumer group (SC-11: import isonim/core/*) ---
  spec("test_render_integration", consumesIsonim = true),
  spec("test_structural_comparison", consumesIsonim = true),
  spec("test_cross_renderer", consumesIsonim = true),
]

package isonim_freya:
  # ``repro exec`` (and so CI's ``dev-exec just build`` / ``just test`` in
  # ci-reprobuild.yml) runs inside the flake's dev shell, which is where
  # ``just`` and the shim's native libraries come from. Without it the
  # environment had ``nim``, ``cargo`` and ``gcc`` from ``uses:`` but no
  # ``just``. Same activation as ``isonim-gpui/repro.nim``.
  devEnv:
    when not defined(windows):
      useFlakeDevShell()

  defaultToolProvisioning "path"

  uses:
    # Toolchain floor — the PATH-resolvable binaries the build needs. ``nim``
    # compiles every test binary (the ``buildNimUnittest.build`` edges below,
    # matching the nimble file's ``requires "nim >= 2.0.0"``); ``gcc`` is the
    # C back-end ``nim c`` shells out to. Sufficient for the path-mode
    # resolver under ``nix develop``.
    "nim >=2.0"
    "gcc >=12"
    # ``cargo`` (+ its bundled ``rustc``) builds the ``freya-nim-shim``
    # cdylib (the ``cargo.build`` edge below).
    "cargo >=1.85"

    # The landed sibling Nim-library producers the isonim-consumer tests
    # consume from source (SC-11 develop-mode). Naming the workspace project
    # here makes reprobuild build the sibling from source (its ``library``
    # edge) and thread its ``src/`` root onto this repo's ``nim c --path:``
    # via the ``nimPathDirs`` aux channel — replacing the ``Justfile``'s
    # hardcoded ``--path:../isonim/src``. ``nim-everywhere`` is isonim's
    # transitive platform seam (``isonim/core/platform.nim`` →
    # ``import nim_everywhere/platform``). Mirrors ``ngx-isonim/repro.nim``.
    "isonim"          # library isonim
    "nim-everywhere"  # library nim_everywhere

  # Library declaration — the ``src/`` tree is importable when this package
  # is consumed via ``uses: "isonim-freya"``. The umbrella modules are
  # ``src/isonim_freya/{renderer,bindings,window}``. The exported path is
  # ``src`` (convention default).
  library isonim_freya

  build:
    # ---- Native cargo build of the freya-nim-shim cdylib ---------------
    #
    # DEFAULT (feature-less) profile: no Skia, no GPU. The cargo workspace
    # root is ``rust/Cargo.toml`` (the repo root has none), so the manifest is
    # pinned; ``rust/Cargo.lock`` is tracked, so ``locked = true`` fails fast
    # rather than silently re-resolving. The cdylib lands in
    # ``rust/target/debug/`` and is declared as the edge's output.
    const dylibName =
      when defined(windows): "freya_nim_shim.dll"
      elif defined(macosx): "libfreya_nim_shim.dylib"
      else: "libfreya_nim_shim.so"
    const shimLib = "rust/target/debug/" & dylibName
    const cargoManifest = "rust/Cargo.toml"
    const cargoLockfile = "rust/Cargo.lock"

    let shimBuild = cargo.build(
      locked = true,
      manifestPath = cargoManifest,
      actionId = "isonim-freya.cargo-build",
      extraInputs = @[
        cargoManifest, cargoLockfile,
        "rust/freya-nim-shim/Cargo.toml",
        "rust/freya-nim-shim/src",
      ],
      extraOutputs = @[shimLib])
    discard collect("default", @[shimBuild])

    # Two-edge test template (Package-Model.md §"The test template"): one
    # compile BUILD edge + one EXECUTE edge per test file. BUILD halves
    # collect into ``test-builds`` (compile verification); EXECUTE halves
    # into ``test`` so ``repro test`` / ``repro build test`` materialise the
    # runnable closure (each execute edge transitively depends on its build
    # edge).
    #
    # ``rpathPassL`` bakes the absolute shim directory as an ``-rpath`` onto
    # every test binary so its ``{.dynlib: "libfreya_nim_shim.so".}`` FFI
    # ``dlopen``s the cargo-built cdylib at run time (no runtime
    # ``LD_LIBRARY_PATH``). ``$ORIGIN`` is not usable here (the binary lives
    # under ``build/test-bin/`` while the shim lives under ``rust/target/debug``),
    # so an absolute rpath is baked in.
    let rpathPassL = @["-Wl,-rpath," & shimRpath]

    # ``src`` roots for the two path groups. The isonim + nim-everywhere
    # ``src`` roots are NOT listed here — they are threaded by the SC-11
    # ``uses:`` ``nimPathDirs`` channel. The third-party status-im trees
    # (``../nim-faststreams`` + ``../nim-stew``) ARE listed for the consumer
    # group, exactly as isonim's own build resolves them.
    const selfPaths = @["src"]
    const isonimConsumerPaths = @["src", "../nim-faststreams", "../nim-stew"]

    var testBuildActions: seq[BuildActionDef] = @[]
    var testExecuteActions: seq[BuildActionDef] = @[]

    proc emitTest(source, binary: string; paths, extraPassL: seq[string];
                  buildActions, executeActions: var seq[BuildActionDef]) =
      let stem = splitFile(binary).name
      let edge = buildNimUnittest.build(
        source = source,
        binary = binary,
        paths = paths,
        extraPassL = extraPassL,
        after = @[shimBuild],
        actionId = "isonim-freya.test_build." & stem,
        # ``src`` + the nimble file are declared inputs so the monitor tracks
        # the transitively imported ``src/isonim_freya/**`` binding tree.
        extraInputs = @["src", "isonim_freya.nimble", shimLib])
      buildActions.add(edge.action)
      # ``registerImplicitName = false``: the BUILD edge already owns the
      # binary basename as the implicit target name; the explicit ``actionId``
      # is the execute edge's selector (two-edge shape).
      let executeEdge = edge.testBinary.run(
        actionId = "isonim-freya.test_execute." & stem,
        requiredBinaries = @[shimLib],
        registerImplicitName = false)
      executeActions.add(executeEdge)

    for s in freyaTestSpecs:
      let paths = if s.consumesIsonim: isonimConsumerPaths else: selfPaths
      emitTest(s.source, s.binary, paths, rpathPassL,
        testBuildActions, testExecuteActions)

    discard collect("test", testExecuteActions)
    discard collect("test-builds", testBuildActions)
