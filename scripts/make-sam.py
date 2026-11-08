#!/usr/bin/env python3
"""Builds SAM's sprites in game/gfx/ from his sprite sheet.

Every pose is cut from .claude/skills/diamond-hands-gba/sprite-sheet-SBF.png and brought
down to gameplay size by sampling the centre of each art pixel, so the sheet's own
pixel clusters survive and the edges stay crisp. Standing, he is about 45 pixels tall:
half again the wassie's height. Each row of the sheet is drawn at a slightly different
scale, so each gets its own factor. The black backdrop is flood-filled away from the
crop's edges; dark pixels inside his outline stay. Effects the game draws itself (the
dizzy stars, the pain marks) are dropped. All poses share one 15-colour palette.

Each pose is laid out in a 64x64 frame, facing right as on the sheet, feet on row 63
and his body centred on column 32, and saved as the GBA's 4-bit tiles so the game can
copy one pose to sprite VRAM when he changes pose.

- sam.tiles: the poses in `POSES` order, 2048 bytes each (64 tiles of 32 bytes)
- sam.pal: the palette, little-endian BGR555; colour 0 transparent
- sam-portrait.idx / .pal: the 32x32 dialogue portrait, from the sheet's top-left card
- sam.png / sam-portrait.png: the results, for review

    python3 scripts/make-sam.py   # needs Pillow
"""
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SHEET = ROOT / ".claude/skills/diamond-hands-gba/sprite-sheet-SBF.png"
GFX = ROOT / "game/gfx"
FRAME = 64
# Sheet pixels to gameplay pixels: the standing height on each row of the sheet.
TALL = 45
ROW_TALL = {"idle": 112, "attack": 127, "hit": 121, "defeat": 123}

# name: (sheet row, crop box, nudge right). Order matters: the game indexes it.
# Effects the game draws itself are dropped from the poses named in `DROP`.
POSES = [
    ("idle0", "idle", (198, 428, 280, 542), 0),
    ("idle1", "idle", (337, 428, 419, 542), 0),
    ("walk0", "idle", (832, 429, 912, 542), 0),
    ("walk1", "idle", (979, 429, 1059, 542), 0),
    ("walk2", "idle", (1125, 429, 1205, 542), 0),
    ("walk3", "idle", (1273, 429, 1350, 542), 0),
    ("hold", "attack", (71, 624, 157, 753), 0),
    ("raise", "attack", (229, 624, 347, 753), 0),
    ("throw", "attack", (399, 624, 530, 753), -6),
    ("charge", "attack", (895, 616, 1065, 758), 0),
    ("punch", "attack", (1096, 616, 1255, 758), 0),
    ("ouch", "hit", (60, 824, 187, 963), 0),
    ("dazed", "hit", (342, 824, 491, 963), 0),
    ("shock", "defeat", (731, 838, 822, 963), 0),
    ("kneel", "defeat", (893, 838, 1014, 964), 0),
    ("down", "defeat", (1276, 845, 1449, 964), 0),
]

DROP = {"ouch", "dazed"}
# Colours the palette must keep, however few pixels use them: his pink glasses and
# implant lamp, their highlight, and the cucumber's two greens.
KEEP = [(245, 20, 140), (255, 150, 215), (140, 222, 8), (24, 112, 6)]

PORTRAIT_BOX = (64, 40, 364, 340)


def dropped(rgb, x, y, w, h):
    """The sheet's own effects round his head: yellow stars and hot-pink pain marks."""
    r, g, b = rgb
    star = r > 160 and g > 130 and b < 100
    marks = r > 200 and g < 70 and x > w * 2 // 3 and y < h // 2
    return star or marks


def dark(rgb):
    return max(rgb) < 48


def cut(name, row, box):
    art = SHEET_IMG.crop(box)
    f = TALL / ROW_TALL[row]
    small = art.resize((round(art.width * f), round(art.height * f)), Image.Resampling.NEAREST)
    w, h = small.size
    px = small.load()
    clear = [[False] * w for _ in range(h)]
    # Flood the backdrop in from the edges through dark pixels.
    todo = [(x, y) for x in range(w) for y in (0, h - 1)] + [(x, y) for y in range(h) for x in (0, w - 1)]
    while todo:
        x, y = todo.pop()
        if not (0 <= x < w and 0 <= y < h) or clear[y][x] or not dark(px[x, y]):
            continue
        clear[y][x] = True
        todo += [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]
    for y in range(h):
        for x in range(w):
            if name in DROP and dropped(px[x, y], x, y, w, h):
                clear[y][x] = True
    # Drop specks and the dust kicked up round his feet: islands of a few pixels. The
    # steam over him when he's down is kept.
    least = 6 if name == "down" else 24
    seen = [[False] * w for _ in range(h)]
    for y in range(h):
        for x in range(w):
            if clear[y][x] or seen[y][x]:
                continue
            island, todo = [], [(x, y)]
            seen[y][x] = True
            while todo:
                a, b = todo.pop()
                island.append((a, b))
                for c, d in [(a + 1, b), (a - 1, b), (a, b + 1), (a, b - 1)]:
                    if 0 <= c < w and 0 <= d < h and not clear[d][c] and not seen[d][c]:
                        seen[d][c] = True
                        todo.append((c, d))
            if len(island) < least:
                for a, b in island:
                    clear[b][a] = True
    return small, clear


def frame_of(small, clear, nudge):
    w, h = small.size
    cols = [x for x in range(w) if any(not clear[y][x] for y in range(h))]
    rows = [y for y in range(h) if any(not clear[y][x] for x in range(w))]
    left, right, top, bottom = cols[0], cols[-1], rows[0], rows[-1]
    # Centre on his body: the middle of the opaque span across the belly, a third of
    # the way up, where neither a flung arm nor the hair widens him.
    belly = bottom - (bottom - top) // 3
    span = [x for x in range(w) if not clear[belly][x]]
    mid = (span[0] + span[-1]) // 2
    # Poses too wide to centre (lying flat) are pushed back inside the frame.
    ox = max(-left, min(FRAME - 1 - right, FRAME // 2 - mid + nudge))
    oy = FRAME - 1 - bottom
    out = Image.new("RGBA", (FRAME, FRAME), (0, 0, 0, 0))
    px = small.load()
    for y in range(top, bottom + 1):
        for x in range(left, right + 1):
            if not clear[y][x]:
                X, Y = x + ox, y + oy
                assert 0 <= X < FRAME and 0 <= Y < FRAME, "pose overflows its frame"
                out.putpixel((X, Y), px[x, y] + (255,))
    return out


def bgr555(colours):
    pal = bytearray()
    for r, g, b in [(0, 0, 0)] + colours + [(0, 0, 0)] * (15 - len(colours)):
        pal += (r >> 3 | (g >> 3) << 5 | (b >> 3) << 10).to_bytes(2, "little")
    return bytes(pal)


def quantise(images):
    """One 15-colour palette for all `images`; returns it and each image's indices."""
    opaque = []
    for i, im in enumerate(images):
        for y in range(FRAME):
            for x in range(FRAME):
                p = im.getpixel((x, y))
                if p[3]:
                    opaque.append(p[:3])
    # Quantise only the opaque pixels: a strip of them.
    sample = Image.new("RGB", (len(opaque), 1))
    sample.putdata(opaque)
    n = 15 - len(KEEP)
    q = sample.quantize(n, method=Image.Quantize.MEDIANCUT, kmeans=8, dither=Image.Dither.NONE)
    flat = q.getpalette()[: 3 * n]
    colours = [tuple(flat[i : i + 3]) for i in range(0, len(flat), 3)] + KEEP
    # Darkest first, so colour 1 is the outline.
    colours.sort(key=sum)
    cache = {}

    def nearest(p):
        if p not in cache:
            cache[p] = min(range(15), key=lambda i: distance(p, colours[i]))
        return cache[p]

    indices = []
    for im in images:
        idx = [0] * (FRAME * FRAME)
        for y in range(FRAME):
            for x in range(FRAME):
                p = im.getpixel((x, y))
                if p[3]:
                    idx[y * FRAME + x] = 1 + nearest(p[:3])
        indices.append(idx)
    return colours, indices


def distance(a, b):
    """Squared "redmean" colour distance: closer to the eye than plain RGB."""
    rm = (a[0] + b[0]) / 2
    dr, dg, db = a[0] - b[0], a[1] - b[1], a[2] - b[2]
    return (2 + rm / 256) * dr * dr + 4 * dg * dg + (2 + (255 - rm) / 256) * db * db


def tiles(idx, w):
    """Row-major 8x8 tiles, 4 bits per pixel, left pixel in the low nibble."""
    out = bytearray()
    h = len(idx) // w
    for ty in range(h // 8):
        for tx in range(w // 8):
            for y in range(8):
                for x in range(0, 8, 2):
                    a = idx[(ty * 8 + y) * w + tx * 8 + x]
                    b = idx[(ty * 8 + y) * w + tx * 8 + x + 1]
                    out.append(a | b << 4)
    return bytes(out)


SHEET_IMG = Image.open(SHEET).convert("RGB")

frames = [frame_of(*cut(name, row, box), nudge) for name, row, box, nudge in POSES]
colours, indices = quantise(frames)
(GFX / "sam.tiles").write_bytes(b"".join(tiles(i, FRAME) for i in indices))
(GFX / "sam.pal").write_bytes(bgr555(colours))

review = Image.new("RGB", (FRAME * len(frames), FRAME), (40, 12, 40))
for n, idx in enumerate(indices):
    for p, c in enumerate(idx):
        if c:
            review.putpixel((n * FRAME + p % FRAME, p // FRAME), colours[c - 1])
review.save(GFX / "sam.png")

# The portrait: the card's face, fitted to 32x32 on its own palette.
card = SHEET_IMG.crop(PORTRAIT_BOX).resize((32, 32), Image.Resampling.BOX)
q = card.quantize(15, method=Image.Quantize.MEDIANCUT, kmeans=8, dither=Image.Dither.NONE)
flat = q.getpalette()[: 3 * 15]
(GFX / "sam-portrait.idx").write_bytes(bytes(1 + i for i in q.tobytes()))
(GFX / "sam-portrait.pal").write_bytes(bgr555([tuple(flat[i : i + 3]) for i in range(0, len(flat), 3)]))
q.convert("RGB").save(GFX / "sam-portrait.png")
print(f"wrote game/gfx/sam.* ({len(POSES)} poses) and sam-portrait.* from the SBF sheet")
