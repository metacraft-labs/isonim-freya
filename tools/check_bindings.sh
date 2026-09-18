#!/usr/bin/env bash
# Validate that all Rust extern "C" functions have corresponding Nim bindings.
#
# Usage: ./tools/check_bindings.sh
#
# Exits 0 if all exports are covered, 1 if there are mismatches, 2 if the
# scan itself could not read the tree.
#
# REPAIRED 2026-09-18 (NH-M3). Three defects, the same three `isonim-gpui`'s
# copy carried before PLAT-19 repaired it there — and this copy had been RED
# at HEAD for as long as the headless path existed:
#
#   1. THE SUBJECT WAS ONE FILE. It scanned `src/lib.rs` only, while
#      `freya_render_to_pixels` / `freya_free_pixels` live in
#      `src/freya_headless.rs`. Both were reported as "EXTRA in Nim
#      bindings" on every run. The scan now covers `src/*.rs`.
#   2. `pub unsafe extern "C" fn` DID NOT MATCH, and `freya_free_pixels` is
#      spelled that way.
#   3. THERE WAS NO NON-VACUITY FLOOR. `wc -l` answers 1 for the empty
#      string and `comm` over two empty sets reports nothing, so a scan that
#      read nothing would print "All 1 Rust exports have matching Nim
#      bindings" and exit 0.
#
# NOTE ON WHAT THIS GATE STILL CANNOT SEE, because it is the reason
# `check_exported_symbols.sh` exists next to it: this regex is `cfg`-BLIND.
# It counts a declaration inside a module the default feature selection
# compiles out, so it cannot distinguish "declared" from "exported by the
# artifact a consumer dlopens". That distinction is what made the Freya
# launcher die with "could not import: freya_render_to_pixels" while this
# script was, by its own lights, satisfied.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUST_SRC_DIR="$REPO_ROOT/rust/freya-nim-shim/src"
NIM_BINDINGS="$REPO_ROOT/src/isonim_freya/bindings.nim"

# A floor, not a formality: well under the current figures (48 / 48 on
# 2026-09-18) so a scan that has stopped reading cannot report a clean
# sweep, and deliberately NOT the exact counts, which would turn every
# added export into a failure here. Exact agreement is what the `comm`s
# below assert.
MIN_EXPORTS=30
MIN_BINDINGS=30

if [ ! -d "$RUST_SRC_DIR" ]; then
	echo "REFUSING TO REPORT: Rust source directory not found: $RUST_SRC_DIR" >&2
	exit 2
fi
if [ ! -f "$NIM_BINDINGS" ]; then
	echo "REFUSING TO REPORT: Nim bindings not found: $NIM_BINDINGS" >&2
	exit 2
fi

# `|| true` is deliberate: a `grep` that matches nothing exits 1, and with
# `pipefail` + `errexit` that would kill the script HERE, before the floor
# below could say which scan came back empty — and with the same exit code
# a real mismatch uses. A floor behind an `errexit` is not a floor.
RUST_FUNCS=$(grep -rhoP 'pub (?:unsafe )?extern "C" fn \K\w+' "$RUST_SRC_DIR"/*.rs | sort -u || true)
RUST_COUNT=$(printf '%s\n' "$RUST_FUNCS" | grep -c . || true)

# Extract imported function names from Nim bindings
NIM_FUNCS=$(grep -oP '(?<=proc )\w+(?=\*)' "$NIM_BINDINGS" | sort -u || true)
NIM_COUNT=$(printf '%s\n' "$NIM_FUNCS" | grep -c . || true)

echo "Rust extern \"C\" exports: $RUST_COUNT"
echo "Nim binding imports:     $NIM_COUNT"
echo ""

if [ "$RUST_COUNT" -lt "$MIN_EXPORTS" ]; then
	echo "REFUSING TO REPORT: found $RUST_COUNT Rust exports, floor is $MIN_EXPORTS." >&2
	echo "The scan is not reading the crate; a clean sweep here would be vacuous." >&2
	exit 2
fi
if [ "$NIM_COUNT" -lt "$MIN_BINDINGS" ]; then
	echo "REFUSING TO REPORT: found $NIM_COUNT Nim bindings, floor is $MIN_BINDINGS." >&2
	exit 2
fi

MISSING=$(comm -23 <(echo "$RUST_FUNCS") <(echo "$NIM_FUNCS"))
EXTRA=$(comm -13 <(echo "$RUST_FUNCS") <(echo "$NIM_FUNCS"))

STATUS=0

if [ -n "$MISSING" ]; then
    echo "MISSING in Nim bindings (present in Rust but not in Nim):"
    echo "$MISSING" | sed 's/^/  - /'
    STATUS=1
fi

if [ -n "$EXTRA" ]; then
    echo "EXTRA in Nim bindings (present in Nim but not in Rust):"
    echo "$EXTRA" | sed 's/^/  - /'
    STATUS=1
fi

if [ "$STATUS" -eq 0 ]; then
    echo "All $RUST_COUNT Rust exports have matching Nim bindings."

    # Also check the generated bindings file if it exists.
    #
    # THIS BLOCK WAS DARK UNTIL 2026-09-18. It only runs when the
    # comparison above is clean, and that comparison had been failing at
    # HEAD ever since `freya_headless.rs` landed (the scan read one file
    # and this one was not it). So the staleness it reports is not new
    # drift — it is drift that nothing has been able to observe.
    #
    # It exits 3, not 1, and the distinction is the point: "the shipped
    # bindings disagree with the shim" and "a regenerable convenience
    # file is out of date" are different facts with different remedies,
    # and collapsing them onto one code is what makes a refusal
    # unreadable. `bindings_generated.nim` is imported by nothing in this
    # repo; `bindings.nim` is the one that ships.
    GENERATED="$REPO_ROOT/src/isonim_freya/bindings_generated.nim"
    if [ -f "$GENERATED" ]; then
        GEN_FUNCS=$(grep -oP 'importc: "\K\w+' "$GENERATED" | sort -u || true)
        GEN_MISSING=$(comm -23 <(printf '%s\n' "$RUST_FUNCS") <(printf '%s\n' "$GEN_FUNCS"))
        if [ -n "$GEN_MISSING" ]; then
            echo ""
            echo "STALE: bindings_generated.nim is missing $(printf '%s\n' "$GEN_MISSING" | grep -c .) of $RUST_COUNT exports:"
            printf '%s\n' "$GEN_MISSING" | sed 's/^/  - /'
            echo ""
            echo "Remedy: './tools/generate_bindings.sh' (via 'just generate-bindings')."
            echo "It needs 'nbindgen', which the dev shell does not currently ship"
            echo "(cargo install nbindgen). This file is consumed by nothing in this"
            echo "repo — 'src/isonim_freya/bindings.nim' is the one that ships, and it"
            echo "agrees with the shim, as the $RUST_COUNT/$NIM_COUNT line above says."
            STATUS=3
        else
            echo "Generated bindings also match ($NIM_COUNT/$RUST_COUNT)."
        fi
    fi
fi

echo ""
exit $STATUS
