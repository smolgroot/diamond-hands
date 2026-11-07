#!/usr/bin/env bash
# Builds the release ROM and writes a header-fixed dist/diamond-hands.gba.
# Needs the nightly toolchain from game/rust-toolchain.toml and `cargo install agb-gbafix`.
#
#   ./scripts/build-rom.sh         # dist/diamond-hands.gba
#   ./scripts/build-rom.sh --dev   # dist/diamond-hands-dev.gba: every level open on the map
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"

features=()
out="$root/dist/diamond-hands.gba"
if [[ "${1:-}" == "--dev" ]]; then
    features=(--features dev)
    out="$root/dist/diamond-hands-dev.gba"
fi

(cd "$root/game" && cargo build --release "${features[@]}")
mkdir -p "$root/dist"
agb-gbafix "$root/game/target/thumbv4t-none-eabi/release/diamond-hands" -o "$out"
echo "wrote $out"
