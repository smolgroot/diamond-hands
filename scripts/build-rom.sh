#!/usr/bin/env bash
# Builds the release ROM and writes a header-fixed dist/diamond-hands.gba.
# Needs the nightly toolchain from game/rust-toolchain.toml and `cargo install agb-gbafix`.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"

(cd "$root/game" && cargo build --release)
mkdir -p "$root/dist"
agb-gbafix "$root/game/target/thumbv4t-none-eabi/release/diamond-hands" \
    -o "$root/dist/diamond-hands.gba"
echo "wrote $root/dist/diamond-hands.gba"
