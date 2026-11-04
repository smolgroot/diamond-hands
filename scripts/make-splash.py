#!/usr/bin/env python3
"""Builds the 240x160 splash screen in game/gfx/ from cover.png.

Crops the artwork to the right of the cartridge's side label down to the GBA's 3:2
screen, framing the title and the wassie, and shrinks it with a box filter. agb gives
backgrounds 32 KB of tiles, too little for 600 tiles of 256 colours, so the art is
fitted to 16-colour tiles: every 8x8 tile is drawn from one of 16 palettes of 15
colours. Tiles are grouped by colour and each group gets its own palette, refined a
few rounds.

agb's `include_background_gfx!` can't pack palettes this full, so this writes the
hardware formats that game/src/splash.rs loads as they are:

- splash.tiles: the 600 tiles, left to right then top to bottom, 4 bits per pixel
- splash.map: each tile's palette, one byte per tile
- splash.pal: the 16 palettes of 16 colours, as little-endian BGR555; colour 0 unused
- splash.png: what the screen shows, for review

    python3 scripts/make-splash.py   # needs Pillow
"""
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
# Cover pixels: the artwork right of the side label, from just above the title down
# to the wassie's bandolier.
LEFT, TOP, RIGHT = 230, 36, 1254
WIDTH, HEIGHT = 240, 160
PALETTES, COLOURS = 16, 15
ROUNDS = 6


def gba(c):
    """Rounds a channel to the GBA's 5 bits, spread back over 0-255."""
    c = min(255, c + 4) >> 3
    return c << 3 | c >> 2


def dist(a, b):
    return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2


def nearest(colour, palette):
    return min(palette, key=lambda p: dist(colour, p))


def palette_for(pixels):
    strip = Image.new("RGB", (len(pixels), 1))
    strip.putdata(pixels)
    quant = strip.quantize(COLOURS, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE)
    flat = quant.getpalette()[: 3 * COLOURS]
    return sorted({tuple(gba(c) for c in flat[i : i + 3]) for i in range(0, len(flat), 3)})


def error(tile, palette):
    return sum(dist(p, nearest(p, palette)) for p in tile)


def mean(tile):
    return tuple(sum(p[i] for p in tile) // len(tile) for i in range(3))


cover = Image.open(ROOT / "cover.png").convert("RGB")
bottom = TOP + (RIGHT - LEFT) * HEIGHT // WIDTH
art = cover.crop((LEFT, TOP, RIGHT, bottom)).resize((WIDTH, HEIGHT), Image.Resampling.BOX)
px = art.load()
tiles = [
    [px[tx + x, ty + y] for y in range(8) for x in range(8)]
    for ty in range(0, HEIGHT, 8)
    for tx in range(0, WIDTH, 8)
]

# First grouping: k-means on each tile's mean colour, seeded far apart.
means = [mean(t) for t in tiles]
centres = [means[0]]
while len(centres) < PALETTES:
    centres.append(max(means, key=lambda m: min(dist(m, c) for c in centres)))
for _ in range(10):
    groups = [min(range(PALETTES), key=lambda g: dist(m, centres[g])) for m in means]
    for g in range(PALETTES):
        members = [means[i] for i in range(len(tiles)) if groups[i] == g]
        if members:
            centres[g] = mean(members)

# Then alternate: a palette per group, and each tile moves to the palette that draws it best.
palettes = []
for _ in range(ROUNDS):
    previous, palettes = palettes, []
    for g in range(PALETTES):
        pixels = [p for i, t in enumerate(tiles) if groups[i] == g for p in t]
        if not pixels:
            # An empty group takes over the tile last round drew worst.
            worst = max(range(len(tiles)), key=lambda i: error(tiles[i], previous[groups[i]]))
            pixels = tiles[worst]
        palettes.append(palette_for(pixels))
    groups = [min(range(PALETTES), key=lambda g: error(t, palettes[g])) for t in tiles]

gfx = ROOT / "game/gfx"
preview = Image.new("RGB", (WIDTH, HEIGHT))
out = preview.load()
tile_bytes, map_bytes = bytearray(), bytearray()
for i, tile in enumerate(tiles):
    palette = palettes[groups[i]]
    indices = [1 + palette.index(nearest(p, palette)) for p in tile]
    tx, ty = i % (WIDTH // 8) * 8, i // (WIDTH // 8) * 8
    for j, index in enumerate(indices):
        out[tx + j % 8, ty + j // 8] = palette[index - 1]
    tile_bytes += bytes(indices[j] | indices[j + 1] << 4 for j in range(0, 64, 2))
    map_bytes.append(groups[i])

pal_bytes = bytearray()
for palette in palettes:
    for r, g, b in [(0, 0, 0)] + palette + [(0, 0, 0)] * (COLOURS - len(palette)):
        pal_bytes += (r >> 3 | (g >> 3) << 5 | (b >> 3) << 10).to_bytes(2, "little")

(gfx / "splash.tiles").write_bytes(tile_bytes)
(gfx / "splash.map").write_bytes(map_bytes)
(gfx / "splash.pal").write_bytes(pal_bytes)
preview.save(gfx / "splash.png")
print(f"wrote game/gfx/splash.* from cover.png ({LEFT},{TOP})-({RIGHT},{bottom})")
