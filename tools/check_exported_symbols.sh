#!/usr/bin/env bash
# Validate that every symbol `bindings.nim` imports is actually EXPORTED BY
# THE BUILT SHIM — not merely written down somewhere in the Rust sources.
#
# Usage: ./tools/check_exported_symbols.sh
#
# Exits 0 when the image and the bindings agree, 1 on a mismatch,
# 2 when the check could not read what it needs (so "the scan broke" and
# "the bindings disagree" are never the same number).
#
# ## Why this exists alongside `check_bindings.sh`
#
# `check_bindings.sh` reads `extern "C" fn` declarations out of
# `rust/freya-nim-shim/src/*.rs` and compares the names to `bindings.nim`.
# That regex is `cfg`-BLIND. Two of those declarations —
# `freya_render_to_pixels` and `freya_free_pixels` — live in
# `freya_headless.rs`, whose whole module was
# `#[cfg(feature = "freya-headless")]` against a crate with
# `default = []`, so on 2026-09-17 the source gate reported a clean
# 48 == 48 while `nm -D rust/target/debug/libfreya_nim_shim.so` listed
# 46 — and `bindings.nim` imports all 48 under a single
# `{.push … dynlib.}`, which Nim resolves at process start:
#
#   $ ./build/backends/isonim-examples-freya --demo=tasks --port 39872
#   could not import: freya_render_to_pixels
#
# `test_freya_launcher_element_tree` and the multi-renderer gates in
# `isonim-examples` failed as "launcher … did not bind to
# 127.0.0.1:<port> within 4s", and the gate whose entire purpose is to
# catch binding drift could not see it, because the population it
# enumerated was the source text and the population that matters is the
# linked image.
#
# This is `codetracer-specs/Testing/Verification-Harness-Traps.md` §18 —
# *diagnose the class in the source; enumerate it in the linked image* —
# and the remedy there is the one used here: read `nm -D` of a built
# artifact. The identical gate lives in `isonim-gpui`, which had the same
# defect with six symbols instead of two.
#
# ## Why it builds the artifact itself
#
# §18's second corollary: *pin the image, or the manifest measures
# whichever build ran last.* `rust/target/debug/libfreya_nim_shim.so` is
# a path several recipes write to with different feature selections. A
# gate that merely read whatever was lying there would report a different
# answer depending on what ran before it. So it builds the DEFAULT,
# feature-less profile first — the profile every consumer of the
# prebuilt cdylib actually uses — and then reads that.
#
# A missing `cargo` is a REFUSAL with a remedy, never a skip.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE_DIR="$REPO_ROOT/rust"
NIM_BINDINGS="$REPO_ROOT/src/isonim_freya/bindings.nim"
SYMBOL_PREFIX="freya_"

case "$(uname -s)" in
Darwin) LIB_NAME="libfreya_nim_shim.dylib" ;;
*) LIB_NAME="libfreya_nim_shim.so" ;;
esac
LIB_PATH="$CRATE_DIR/target/debug/$LIB_NAME"

# A floor, not a formality, and deliberately well under the current
# figures (48 / 48 on 2026-09-18) rather than equal to them. Per §18's
# first corollary this number is a claim about the subject's size and
# must be re-derived if the surface ever shrinks below it.
MIN_SYMBOLS=30

if [ ! -f "$NIM_BINDINGS" ]; then
	echo "REFUSING TO REPORT: Nim bindings not found: $NIM_BINDINGS" >&2
	exit 2
fi
if [ ! -d "$CRATE_DIR" ]; then
	echo "REFUSING TO REPORT: Rust crate directory not found: $CRATE_DIR" >&2
	exit 2
fi

if ! command -v cargo >/dev/null 2>&1; then
	echo "REFUSING TO REPORT: no 'cargo' on PATH." >&2
	echo "" >&2
	echo "This gate compares the Nim bindings against the symbols a BUILT" >&2
	echo "shim exports, so it has to build one. Run it inside the repo dev" >&2
	echo "shell, which ships the pinned Rust toolchain:" >&2
	echo "    direnv exec $REPO_ROOT ./tools/check_exported_symbols.sh" >&2
	echo "" >&2
	echo "It is NOT skipped without cargo: a pass that inspected no image" >&2
	echo "is the exact defect this check was added to remove." >&2
	exit 2
fi

NM_BIN="nm"
if ! command -v "$NM_BIN" >/dev/null 2>&1; then
	echo "REFUSING TO REPORT: no 'nm' on PATH; cannot enumerate the image." >&2
	exit 2
fi

echo "[check-exported-symbols] building the DEFAULT (feature-less) profile"
(cd "$CRATE_DIR" && cargo build --quiet)

if [ ! -f "$LIB_PATH" ]; then
	echo "REFUSING TO REPORT: cargo build succeeded but $LIB_PATH is absent." >&2
	echo "Check [lib] crate-type in rust/freya-nim-shim/Cargo.toml." >&2
	exit 2
fi

# What the image actually exports. `T` is a defined text symbol; `W` a
# weak one. Filtered to the shim's own prefix so Rust runtime and libc
# symbols do not enter the comparison.
IMAGE_SYMS=$("$NM_BIN" -D --defined-only "$LIB_PATH" 2>/dev/null |
	awk '$2 == "T" || $2 == "W" { print $3 }' |
	grep "^$SYMBOL_PREFIX" | sort -u || true)
IMAGE_COUNT=$(printf '%s\n' "$IMAGE_SYMS" | grep -c . || true)

# What Nim will `dlsym` at process start. Read from the `importc:` pragma
# — the actual C name — rather than from the Nim proc name, so a binding
# that renames a symbol is compared on the name that matters.
NIM_SYMS=$(grep -oP 'importc:\s*"\K[^"]+' "$NIM_BINDINGS" | sort -u || true)
NIM_COUNT=$(printf '%s\n' "$NIM_SYMS" | grep -c . || true)

echo "Exported by $LIB_NAME (prefix '$SYMBOL_PREFIX'): $IMAGE_COUNT"
echo "Imported by $(basename "$NIM_BINDINGS"):                  $NIM_COUNT"
echo ""

if [ "$IMAGE_COUNT" -lt "$MIN_SYMBOLS" ]; then
	echo "REFUSING TO REPORT: the image exports $IMAGE_COUNT '$SYMBOL_PREFIX' symbols, floor is $MIN_SYMBOLS." >&2
	echo "The enumeration is not reading the artifact; a clean sweep would be vacuous." >&2
	exit 2
fi
if [ "$NIM_COUNT" -lt "$MIN_SYMBOLS" ]; then
	echo "REFUSING TO REPORT: the bindings import $NIM_COUNT symbols, floor is $MIN_SYMBOLS." >&2
	exit 2
fi

# Declared by Nim, absent from the image. THIS IS THE CRASH CLASS: Nim's
# `dynlib` resolves every referenced symbol before `main`, so one entry
# here is a process that dies at load with "could not import: <name>".
UNRESOLVED=$(comm -23 <(printf '%s\n' "$NIM_SYMS") <(printf '%s\n' "$IMAGE_SYMS"))
# Exported by the image, bound by nothing. Not a crash, but it means the
# two sides have drifted and one of them is wrong.
UNBOUND=$(comm -13 <(printf '%s\n' "$NIM_SYMS") <(printf '%s\n' "$IMAGE_SYMS"))

STATUS=0

if [ -n "$UNRESOLVED" ]; then
	echo "IMPORTED BY NIM, NOT EXPORTED BY THE BUILT SHIM:"
	mapfile -t unresolved_list <<<"$UNRESOLVED"
	printf '  - %s\n' "${unresolved_list[@]}"
	echo ""
	echo "Every one of these aborts the process at load time:"
	echo "    could not import: ${unresolved_list[0]}"
	echo ""
	echo "The usual cause is a Rust module behind a #[cfg(feature = ...)]"
	echo "that the default profile does not enable, while bindings.nim"
	echo "declares its exports unconditionally. The fix is to give the"
	echo "module a feature-less arm so the ABI does not vary with the"
	echo "feature selection (see src/freya_headless_unavailable.rs), NOT"
	echo "to delete the binding."
	STATUS=1
fi

if [ -n "$UNBOUND" ]; then
	echo "EXPORTED BY THE BUILT SHIM, IMPORTED BY NOTHING IN NIM:"
	mapfile -t unbound_list <<<"$UNBOUND"
	printf '  - %s\n' "${unbound_list[@]}"
	STATUS=1
fi

if [ "$STATUS" -eq 0 ]; then
	echo "All $NIM_COUNT imported symbols are exported by the built shim."
fi

exit "$STATUS"
