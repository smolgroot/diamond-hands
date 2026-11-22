#!/usr/bin/env bash
# Builds the release ROM and writes a header-fixed dist/diamond-hands.gba.
# Needs the nightly toolchain from game/rust-toolchain.toml and `cargo install agb-gbafix`.
#
#   ./scripts/build-rom.sh                 # the code-drawn ROM (game/) -> dist/diamond-hands.gba
#   ./scripts/build-rom.sh --dev           # ... with every level open on the map: dist/diamond-hands-dev.gba
#   ./scripts/build-rom.sh --studio        # the same game with its art and music from GBA-Studio
#                                          # (project/) -> dist/diamond-hands-studio.gba
#   ./scripts/build-rom.sh --studio --dev  # ... with every level open: dist/diamond-hands-studio-dev.gba
#
# The studio build fetches GBA-Studio's runtime and build helper from GitHub, at the commit
# pinned in project/Cargo.toml.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"

studio=0
features=()
suffix=""
for arg in "$@"; do
    case "$arg" in
        --studio) studio=1 ;;
        --dev) features=(--features dev); suffix="-dev" ;;
        *) echo "unknown option $arg" >&2; exit 2 ;;
    esac
done

if [[ $studio == 1 ]]; then
    crate="$root/project"
    bin="diamond-hands-studio"
    out="$root/dist/diamond-hands-studio$suffix.gba"
else
    crate="$root/game"
    bin="diamond-hands"
    out="$root/dist/diamond-hands$suffix.gba"
fi

(cd "$crate" && cargo build --release "${features[@]}")
mkdir -p "$root/dist"
agb-gbafix "$crate/target/thumbv4t-none-eabi/release/$bin" -o "$out"
echo "wrote $out"
