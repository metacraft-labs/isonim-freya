#!/usr/bin/env bash
# Print the absolute directory cargo writes this repository's build
# artefacts to: the `target/` that `cd rust && cargo build` uses.
#
# Usage: ./tools/cargo_target_dir.sh
#
# Exits 0 and prints one line on success, 2 when the directory could not
# be determined (no cargo, or output this script cannot read). It never
# guesses: a guessed path is how the shim went missing in the first place.
#
# ## Why not `rust/target`
#
# `rust/target` is only cargo's default. A host or a user can move it with
# the `CARGO_TARGET_DIR` (or `CARGO_BUILD_TARGET_DIR`) environment
# variable or with `build.target-dir` in a `.cargo/config.toml`
# (https://doc.rust-lang.org/cargo/reference/config.html#buildtarget-dir).
# Some self-hosted CI runners set `CARGO_TARGET_DIR` to a directory outside
# the checkout, so `cargo build` there succeeded while every test that put
# `rust/target/debug` on the loader path died with
# "could not load: libfreya_nim_shim.so". This script does not override
# that choice; it reports it.
#
# ## How
#
# Cargo itself answers the question: `cargo metadata` reports the resolved
# `target_directory`
# (https://doc.rust-lang.org/cargo/commands/cargo-metadata.html#json-format).
# It runs from `rust/`, the directory every recipe runs `cargo build` from,
# because cargo resolves a relative `CARGO_TARGET_DIR` and discovers
# `.cargo/config.toml` relative to its working directory. `--no-deps` and
# `--offline` keep it from resolving or downloading the dependency graph.
#
# The JSON is read with `sed` rather than `jq` or Python so that the script
# needs nothing beyond cargo and a POSIX userland on every host, Windows
# (Git Bash) included. Only the two escapes a path can plausibly carry are
# decoded: `\\` (every Windows path separator) and `\"`. Any other escape
# is refused rather than mis-decoded.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE_DIR="$REPO_ROOT/rust"

if ! command -v cargo >/dev/null 2>&1; then
	echo "cargo_target_dir.sh: no 'cargo' on PATH; run it inside the repo dev shell." >&2
	exit 2
fi

metadata="$(cd "$CRATE_DIR" && cargo metadata --format-version 1 --no-deps --offline)"

# `cargo metadata` prints the whole document on one line, and the
# workspace-level `target_directory` key occurs once.
raw="$(printf '%s\n' "$metadata" |
	sed -n -E 's/.*"target_directory":"(([^"\\]|\\.)*)".*/\1/p')"

if [ -z "$raw" ]; then
	echo "cargo_target_dir.sh: 'cargo metadata' reported no target_directory." >&2
	exit 2
fi

# Strip the escapes we decode; any backslash left over is one we do not.
case "$(printf '%s' "$raw" | sed -E 's/\\[\\"]//g')" in
*\\*)
	echo "cargo_target_dir.sh: target_directory carries a JSON escape this script does not decode: $raw" >&2
	exit 2
	;;
esac

printf '%s\n' "$raw" | sed -E 's/\\([\\"])/\1/g'
