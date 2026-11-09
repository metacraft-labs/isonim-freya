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
# On Windows the loader ignores `LD_LIBRARY_PATH`. It searches the
# executable's own directory and, later, the directories on `PATH`:
# https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-search-order
# `nim c -r` writes the test binary beside its source in `tests/`, not beside
# `freya_nim_shim.dll`, so under Git-for-Windows / MSYS2 / Cygwin `sh` the
# shim directory is prepended to `PATH` as well.
#
# It is prepended in POSIX form (`cygpath -u`), not Windows form. Inside
# these shells `PATH` is a colon-separated list of POSIX paths (the runtime
# converts it to that form when the shell starts,
# https://cygwin.com/cygwin-ug-net/setup-env.html), so a `C:\...` entry
# would be split at its drive colon. When the shell starts a native program
# such as `nim`, path lists in the environment are converted back to the
# Windows form (https://www.msys2.org/docs/filesystem-paths/), and the test
# binary `nim c -r` starts inherits that.
# `cargo metadata` from a native cargo reports the Windows form
# (`C:\...\rust\target`), which `cygpath -u` accepts.
#
# If the target dir cannot be determined, or `cygpath` is missing on
# Windows, this exits 2 before running the command, rather than running it
# with a guessed path.

set -euo pipefail

if [ "$#" -eq 0 ]; then
	echo "usage: $0 <command> [args...]" >&2
	exit 2
fi

target_dir="$("$(dirname "$0")/cargo_target_dir.sh")"
export LD_LIBRARY_PATH="$target_dir/debug${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

case "$(uname -s)" in
MINGW* | MSYS* | CYGWIN*)
	if ! command -v cygpath >/dev/null 2>&1; then
		echo "with_shim_path.sh: no 'cygpath' on PATH; cannot put the shim directory on the Windows DLL search path." >&2
		exit 2
	fi
	shim_dir="$(cygpath -u "$target_dir/debug")"
	export PATH="$shim_dir${PATH:+:$PATH}"
	;;
esac
exec "$@"
