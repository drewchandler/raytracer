#!/usr/bin/env bash
# Use the compatible toolchain and verified dependencies in the prepared cloud environment.
set -euo pipefail
tools=${RAYTRACER_TOOLING:-/workspace/.raytracer-tools}
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

if [[ $# -eq 0 ]]; then
    echo 'Usage: scripts/cargo.sh build|check|test|run [Cargo arguments]' >&2
    exit 2
fi
command_name=$1
shift
case "$command_name" in
    check)
        echo 'Rust 1.8 predates metadata-only cargo check; running a full build.' >&2
        command_name=build
        ;;
    build|test|run) ;;
    *) echo "Unsupported command: $command_name (use build, check, test, or run)" >&2; exit 2 ;;
esac

if [[ ! -x "$tools/cargo/bin/cargo" || ! -f "$tools/legacy-cargo/config" ||
      ! -x "$tools/rustup/toolchains/1.8.0-x86_64-unknown-linux-gnu/bin/rustc" ]]; then
    echo 'The prepared Rust environment is missing. See README.md for setup details.' >&2
    exit 1
fi
if [[ "$(readlink -f "$tools/build/src")" != "$(readlink -f "$repo/src")" ]]; then
    echo 'The helper manifest does not point at this checkout; refusing to build another source tree.' >&2
    exit 1
fi

export PATH="$tools/cargo/bin:$PATH"
export RUSTUP_HOME="$tools/rustup"
export CARGO_HOME="$tools/legacy-cargo"
export RUSTC="$tools/rustup/toolchains/1.8.0-x86_64-unknown-linux-gnu/bin/rustc"
export RUSTDOC="$tools/rustup/toolchains/1.8.0-x86_64-unknown-linux-gnu/bin/rustdoc"
exec "$tools/cargo/bin/cargo" +1.20.0 "$command_name" --manifest-path "$tools/build/Cargo.toml" "$@"
