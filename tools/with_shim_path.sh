#!/usr/bin/env bash
# Run a command with the debug build of the Rust shim on the loader path.
#
# Usage: ./tools/with_shim_path.sh <command> [args...]
#   e.g. ./tools/with_shim_path.sh nim c -r tests/test_renderer.nim
#
# `src/isonim_freya/bindings.nim` imports the shim with a bare soname
# (`{.dynlib: "libfreya_nim_shim.so".}`), which the dynamic loader resolves
# when the test binary starts. So the directory cargo wrote the shim to has
# to be on `LD_LIBRARY_PATH`. That directory is `<cargo target dir>/debug`,
# and the target dir is NOT always `rust/target`: see
# `tools/cargo_target_dir.sh`, the one place that resolves it.
#
# The directory is PREPENDED, so a library path the dev shell already set
# is kept. No empty entry is added when `LD_LIBRARY_PATH` was unset: an
# empty entry means the current directory to the loader.
#
# If the target dir cannot be determined, this exits 2 before running the
# command, rather than running it with a guessed path.

set -euo pipefail

if [ "$#" -eq 0 ]; then
	echo "usage: $0 <command> [args...]" >&2
	exit 2
fi

target_dir="$("$(dirname "$0")/cargo_target_dir.sh")"
export LD_LIBRARY_PATH="$target_dir/debug${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec "$@"
