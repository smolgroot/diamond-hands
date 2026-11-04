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
