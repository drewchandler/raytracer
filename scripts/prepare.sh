#!/usr/bin/env bash
# Reproduce the legacy build environment locally or on a clean GitHub runner.
set -euo pipefail
tools=${RAYTRACER_TOOLING:-/workspace/.raytracer-tools}
repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
mkdir -p "$tools"
tools=$(cd -- "$tools" && pwd)

export CARGO_HOME="$tools/cargo"
export RUSTUP_HOME="$tools/rustup"
export PATH="$CARGO_HOME/bin:$PATH"
if [[ ! -x "$CARGO_HOME/bin/rustup" ]]; then
    curl --fail --silent --show-error --location https://sh.rustup.rs -o "$tools/rustup-init.sh"
    sh "$tools/rustup-init.sh" -y --no-modify-path --profile minimal --default-toolchain 1.70.0
fi
for version in 1.70.0 1.20.0 1.8.0; do
    "$CARGO_HOME/bin/rustup" toolchain install "$version" --profile minimal
done

mkdir -p "$tools/build" "$tools/legacy-cargo"
if [[ -e "$tools/build/src" && ! -L "$tools/build/src" ]]; then
    echo "Refusing to replace a non-symlink source directory: $tools/build/src" >&2
    exit 1
fi
ln -sfn "$repo/src" "$tools/build/src"
cp "$repo/Cargo.toml" "$repo/Cargo.lock" "$tools/build/"
# Modern Cargo adds registry checksums to the helper lock, without changing versions.
cargo +1.70.0 vendor --manifest-path "$tools/build/Cargo.toml" "$tools/vendor" > "$tools/vendor-config.tmp"
mv "$tools/vendor-config.tmp" "$tools/legacy-cargo/config"
# Let the compatible Cargo migrate only its external copy of the original lock.
cp "$repo/Cargo.lock" "$tools/build/Cargo.lock"
export CARGO_HOME="$tools/legacy-cargo"
export RUSTC="$tools/rustup/toolchains/1.8.0-x86_64-unknown-linux-gnu/bin/rustc"
cargo +1.20.0 build --manifest-path "$tools/build/Cargo.toml" -j 2
