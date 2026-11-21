#!/usr/bin/env python3
"""Derives the "sheen" sheets of Sol City's breakable blocks from their own art.

A sheen is a band of light sweeping diagonally across a block's face, as a skeleton
placeholder's wave does on the web: it tells the player the block is something to
shoot. Nothing new is drawn. Each frame is the block's own 16x16 picture with the
pixels in the band swapped for a lighter colour already in its sheet, so the palette
(at most 15 colours, one GBA sprite palette) and the dark outline stay as they are.

Output: project/generated/sprites/<name>-sheen.png, a row of 16x16 frames:
frame 0 is the block as it is, frames 1..FRAMES the band crossing it from the top-left
corner to the bottom-right one. Register a sheet with the studio's sprite_create
(16x16 frames). Like the studio's own tools, this never replaces a file that is
already there unless given --force, so a sheet the author has repainted stays.

    python3 tools/block-sheen.py [--force]
"""

import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent / "project"
# (sprite name, source sheet, block frame in that sheet of 16x16 frames)
BLOCKS = [
    ("sand-block", "generated/tilesets/beach-sand-blocks.png", 6),
    ("dock-crate", "generated/tilesets/dock-crates.png", 6),
    ("soil-block", "generated/tilesets/zoo-jungle.png", 6),
]
SIZE = 16
FRAMES = 8
# Half-widths of the band along the diagonal, in pixels: the bright core, and the
# softer edge around it.
CORE = 1.5
EDGE = 3.5
# How far each colour is pushed towards white: the core may add its shade to the sheet's
# palette, the edge only reuses a colour already in it that is close enough.
CORE_LIFT = 0.4
EDGE_LIFT = 0.2
# Squared RGB distance within which a colour in the sheet stands in for a shade.
CLOSE = 30 * 30
# Colours in one 4bpp GBA palette, besides transparency.
MAX_COLOURS = 15


def luma(c):
    return 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]


def lift(colour, amount):
    return tuple(round(v + (255 - v) * amount) for v in colour)


def distance(a, b):
    return sum((x - y) ** 2 for x, y in zip(a, b))


class Shades:
    """Lighter shades of a sheet's colours: one already in the sheet when it is close
    enough, else a new one, as long as the sheet stays within one GBA palette."""

    def __init__(self, palette):
        self.palette = list(palette)

    def shade(self, colour, amount, may_add):
        target = lift(colour, amount)
        brighter = [p for p in self.palette if luma(p) > luma(colour)]
        best = min(brighter, key=lambda p: distance(p, target), default=None)
        if best is not None and distance(best, target) <= CLOSE:
            return best
        if may_add and len(self.palette) < MAX_COLOURS:
            self.palette.append(target)
            return target
        return None


def sheen(name, source, frame, force):
    out = ROOT / "generated" / "sprites" / f"{name}-sheen.png"
    if out.exists() and not force:
        print(f"{out.relative_to(ROOT)}: already there, left as it is")
        return
    sheet = Image.open(ROOT / source).convert("RGBA")
    columns = sheet.width // SIZE
    x0, y0 = (frame % columns) * SIZE, (frame // columns) * SIZE
    block = sheet.crop((x0, y0, x0 + SIZE, y0 + SIZE))
    palette = sorted({p[:3] for p in sheet.get_flattened_data() if p[3] > 0})
    outline = min(palette, key=luma)
    shades = Shades(palette)

    strip = Image.new("RGBA", (SIZE * (FRAMES + 1), SIZE), (0, 0, 0, 0))
    strip.paste(block, (0, 0))
    span = 2 * (SIZE - 1)
    for f in range(FRAMES):
        # The band's centre along x + y, from just before the first corner to just
        # past the last.
        centre = -EDGE + (span + 2 * EDGE) * (f + 0.5) / FRAMES
        img = block.copy()
        px = img.load()
        for y in range(SIZE):
            for x in range(SIZE):
                r, g, b, a = px[x, y]
                if a == 0 or (r, g, b) == outline:
                    continue
                d = abs(x + y - centre)
                if d <= CORE:
                    shade = shades.shade((r, g, b), CORE_LIFT, may_add=True)
                elif d <= EDGE:
                    shade = shades.shade((r, g, b), EDGE_LIFT, may_add=False)
                else:
                    shade = None
                if shade is not None:
                    px[x, y] = (*shade, a)
        strip.paste(img, (SIZE * (f + 1), 0))
    out.parent.mkdir(parents=True, exist_ok=True)
    strip.save(out)
    print(f"{out.relative_to(ROOT)}: {FRAMES + 1} frames of {SIZE}x{SIZE}, {len(shades.palette)} colours")


def main():
    force = "--force" in sys.argv[1:]
    for name, source, frame in BLOCKS:
        sheen(name, source, frame, force)


if __name__ == "__main__":
    main()
