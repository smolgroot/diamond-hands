#!/usr/bin/env python3
"""Builds the wassie's 32x32 dialogue portrait in game/gfx/ from the player sprite sheet.

The portrait is the one in the sheet's top-left corner (.claude/skills/diamond-hands-gba/
sprite-sheet.png). It is scaled to 32 pixels tall by sampling the centre of each of its
art pixels, which keeps the edges crisp, trimmed to 32 wide, and reduced to 15 colours so
it fits one sprite palette.

- wassie-portrait.idx: 1024 colour indices (1-15), left to right then top to bottom
- wassie-portrait.pal: the 16-colour palette, little-endian BGR555; colour 0 unused
- wassie-portrait.png: the result, for review

    python3 scripts/make-portrait.py   # needs Pillow
"""
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SHEET = ROOT / ".claude/skills/diamond-hands-gba/sprite-sheet.png"
# The portrait's box on the sheet.
BOX = (39, 21, 236, 182)
SIZE = 32

sheet = Image.open(SHEET).convert("RGB")
art = sheet.crop(BOX)
width = round(art.width * SIZE / art.height)
art = art.resize((width, SIZE), Image.Resampling.NEAREST)
left = (width - SIZE) // 2
art = art.crop((left, 0, left + SIZE, SIZE))
quant = art.quantize(15, method=Image.Quantize.MEDIANCUT, kmeans=8, dither=Image.Dither.NONE)

flat = quant.getpalette()[: 3 * 15]
colours = [tuple(flat[i : i + 3]) for i in range(0, len(flat), 3)]
colours += [(0, 0, 0)] * (15 - len(colours))
pal = bytearray()
for r, g, b in [(0, 0, 0)] + colours:
    pal += (r >> 3 | (g >> 3) << 5 | (b >> 3) << 10).to_bytes(2, "little")

gfx = ROOT / "game/gfx"
(gfx / "wassie-portrait.idx").write_bytes(bytes(1 + i for i in quant.tobytes()))
(gfx / "wassie-portrait.pal").write_bytes(pal)
quant.convert("RGB").save(gfx / "wassie-portrait.png")
print("wrote game/gfx/wassie-portrait.* from the sprite sheet")


# Talking frames: the lower bill drops (1 then 2 pixels) off the closed portrait, with
# the gap filled by the dark mouth, so the bill visibly flaps while the words type out.
# wassie-portrait-talk.idx holds the two frames back to back (2 x 1024 indices).
JAW_X = 19  # the jaw starts here and runs to the bill tip
JAW_TOP, JAW_BOTTOM = 17, 22  # the rows that travel with the jaw (pink lip to its outline)
MOUTH = 12  # the dark inside of the mouth


def open_mouth(closed: bytes, drop: int) -> bytes:
    px = bytearray(closed)
    for x in range(JAW_X, 32):
        col = [closed[y * 32 + x] for y in range(JAW_TOP, JAW_BOTTOM)]
        for i in range(drop):
            px[(JAW_TOP + i) * 32 + x] = MOUTH
        for i, c in enumerate(col):
            y = JAW_TOP + drop + i
            if y < 32:
                px[y * 32 + x] = c
    return bytes(px)


closed = (gfx / "wassie-portrait.idx").read_bytes()
frames = [open_mouth(closed, 1), open_mouth(closed, 2)]
(gfx / "wassie-portrait-talk.idx").write_bytes(b"".join(frames))
print("wrote game/gfx/wassie-portrait-talk.idx (2 frames)")

# A review strip: closed, half, open.
strip = Image.new("RGB", (96, 32))
rgb = []
for i in range(16):
    v = pal[2 * i] | pal[2 * i + 1] << 8
    rgb.append(((v & 31) << 3, (v >> 5 & 31) << 3, (v >> 10 & 31) << 3))
for n, f in enumerate([closed, *frames]):
    if n == 3:
        break
    for i, c in enumerate(f):
        strip.putpixel((n * 32 + i % 32, i // 32), rgb[c] if c else (144, 80, 200))
strip.resize((384, 128), Image.Resampling.NEAREST).save(gfx / "wassie-portrait-talk.png")
