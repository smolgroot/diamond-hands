#!/usr/bin/env python3
"""Builds the sheet-drawn bots' sprites in game/gfx/: the Badlands' Sentry and the
casino's Guard (the ninja bot).

Every pose is cut from the bot's sprite sheet in .claude/skills/diamond-hands-gba/ and
brought to gameplay size: each gameplay pixel takes the commonest colour of its block
of sheet pixels, so thin outlines and the glowing eyes survive where sampling one point
could miss them. Standing, each bot is `TALL` pixels: a little taller than the wassie's
hitbox. The plum backdrop is flood-filled away from the crop's edges, and specks (the
sheet's pain marks and smoke, which the game draws itself) are dropped. Each bot's
poses share one 15-colour palette.

Each pose is laid out in a 32x32 frame, facing right as on the sheet, feet on row 31
and its legs centred on column `anchor`, so a raised gun has room on the right.
Saved as the GBA's 4-bit tiles, 512 bytes a pose:

- <bot>.tiles: the poses in `poses` order (the game indexes them)
- <bot>.pal: the palette, little-endian BGR555; colour 0 transparent
- <bot>.png: the results, for review

A pose can keep only its muzzle flame (`flame`) or everything but it (`noflame`), so
a flame too long for the frame can be drawn as its own sprite at the muzzle; the
script prints where it sits.

    python3 scripts/make-bots.py   # needs Pillow
"""
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SHEETS = ROOT / ".claude/skills/diamond-hands-gba"
GFX = ROOT / "game/gfx"
FRAME = 32
TALL = 28

def backdrop(rgb):
    """The sheets' plum backdrop; the bots' near-black outlines are kept."""
    r, g, b = rgb
    return abs(r - 22) + abs(g - 5) + abs(b - 32) < 16


def smoke(rgb):
    r, g, b = rgb
    return b > 110 and r > 90 and g > 70 and abs(r - b) < 70


def green_eye(rgb):
    r, g, b = rgb
    return g > 150 and r < 140 and b < 140


def red_visor(rgb):
    r, g, b = rgb
    return r > 200 and g < 90 and b > 50


def flame(rgb):
    """Muzzle flame and the sheet's orange alert marks, glow and all."""
    r, g, b = rgb
    return r > 150 and g > 60 and b < 140 and r > g


def shrink(art, scale, eye):
    """Each gameplay pixel takes the commonest colour of its block of sheet pixels
    (the backdrop only if it fills most of the block); an eye wins with a sixth."""
    w, h = round(art.width * scale), round(art.height * scale)
    src = art.load()
    out = Image.new("RGB", (w, h))
    for y in range(h):
        for x in range(w):
            x0, x1 = int(x / scale), max(int(x / scale) + 1, int((x + 1) / scale))
            y0, y1 = int(y / scale), max(int(y / scale) + 1, int((y + 1) / scale))
            block = [src[i, j] for j in range(y0, min(y1, art.height)) for i in range(x0, min(x1, art.width))]
            # Bucket near-identical colours so anti-aliasing noise doesn't split votes.
            votes = {}
            for p in block:
                votes.setdefault((p[0] >> 4, p[1] >> 4, p[2] >> 4), []).append(p)
            if sum(eye(p) for p in block) * 6 >= len(block):
                pick = [p for p in block if eye(p)]
            else:
                solid = {k: v for k, v in votes.items() if not backdrop(v[0])}
                fill = sum(len(v) for v in solid.values())
                pool = solid if fill * 3 >= len(block) else votes
                pick = max(pool.values(), key=len)
            out.putpixel((x, y), tuple(sum(c) // len(pick) for c in zip(*pick)))
    return out


def despeck(clear, w, h):
    """Drops specks: islands of a few pixels (pain marks, stray smoke)."""
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
            if len(island) < 20:
                for a, b in island:
                    clear[b][a] = True


def cut(sheet, bot, name, box, part):
    small = shrink(sheet.crop(box), TALL / bot["standing"], bot["eye"])
    w, h = small.size
    px = small.load()
    clear = [[False] * w for _ in range(h)]
    todo = [(x, y) for x in range(w) for y in (0, h - 1)] + [(x, y) for y in range(h) for x in (0, w - 1)]
    while todo:
        x, y = todo.pop()
        if not (0 <= x < w and 0 <= y < h) or clear[y][x] or not backdrop(px[x, y]):
            continue
        clear[y][x] = True
        todo += [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]
    for y in range(h):
        for x in range(w):
            if name in bot["smoke"] and smoke(px[x, y]):
                clear[y][x] = True
    despeck(clear, w, h)
    # Where the body sits, before a flame is split off.
    body = [[clear[y][x] or (part is not None and flame(px[x, y])) for x in range(w)] for y in range(h)]
    if part is not None:
        # What's left of an alert mark once its flame-coloured part is gone.
        despeck(body, w, h)
    if part == "noflame":
        clear = body
    elif part == "flame":
        clear = [[clear[y][x] or not flame(px[x, y]) for x in range(w)] for y in range(h)]
    return small, clear, body


def frame_of(small, clear, body, anchor):
    """The pose in a 32x32 frame, placed by its body; also every pixel with where it
    landed (some may be off the frame, clipped)."""
    w, h = small.size
    cols = [x for x in range(w) if any(not body[y][x] for y in range(h))]
    rows = [y for y in range(h) if any(not body[y][x] for x in range(w))]
    left, top, bottom = cols[0], rows[0], rows[-1]
    # Centre on the legs: the opaque span a few rows above the feet.
    legs = bottom - max(3, (bottom - top) // 5)
    span = [x for x in range(w) if not body[legs][x]]
    mid = (span[0] + span[-1]) // 2
    ox = max(-left, anchor - mid)
    oy = FRAME - 1 - bottom
    out = Image.new("RGBA", (FRAME, FRAME), (0, 0, 0, 0))
    px = small.load()
    placed = []
    for y in range(h):
        for x in range(w):
            if not clear[y][x]:
                X, Y = x + ox, y + oy
                placed.append((X, Y, px[x, y]))
                if 0 <= X < FRAME and 0 <= Y < FRAME:
                    out.putpixel((X, Y), px[x, y] + (255,))
    return out, placed


def bgr555(colours):
    pal = bytearray()
    for r, g, b in [(0, 0, 0)] + colours + [(0, 0, 0)] * (15 - len(colours)):
        pal += (r >> 3 | (g >> 3) << 5 | (b >> 3) << 10).to_bytes(2, "little")
    return bytes(pal)


def distance(a, b):
    """Squared "redmean" colour distance: closer to the eye than plain RGB."""
    rm = (a[0] + b[0]) / 2
    dr, dg, db = a[0] - b[0], a[1] - b[1], a[2] - b[2]
    return (2 + rm / 256) * dr * dr + 4 * dg * dg + (2 + (255 - rm) / 256) * db * db


def quantise(images, keep):
    opaque = [p[:3] for im in images for p in im.get_flattened_data() if p[3]]
    sample = Image.new("RGB", (len(opaque), 1))
    sample.putdata(opaque)
    n = 15 - len(keep)
    q = sample.quantize(n, method=Image.Quantize.MEDIANCUT, kmeans=8, dither=Image.Dither.NONE)
    flat = q.getpalette()[: 3 * n]
    colours = [tuple(flat[i : i + 3]) for i in range(0, len(flat), 3)] + keep
    colours.sort(key=sum)
    cache = {}

    def nearest(p):
        if p not in cache:
            cache[p] = min(range(15), key=lambda i: distance(p, colours[i]))
        return cache[p]

    indices = [[1 + nearest(p[:3]) if p[3] else 0 for p in im.get_flattened_data()] for im in images]
    return colours, indices


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


BOTS = [
    {
        "name": "sentry",
        "sheet": "lvl1-bot-sprite-sheet.png",
        # Sheet pixels the standing bot is tall.
        "standing": 126,
        "anchor": 13,
        "eye": green_eye,
        # Colours the palette must keep however few pixels use them: the green eye and
        # its highlight, and the cannon's pink charge glow.
        "keep": [(40, 220, 60), (180, 255, 150), (255, 60, 190)],
        # The purple smoke over a beaten bot touches it: drop it by colour.
        "smoke": {"stagger", "kneel", "down"},
        "poses": [
            ("walk0", (192, 262, 324, 404)),
            ("walk1", (376, 262, 510, 404)),
            ("walk2", (566, 262, 692, 404)),
            ("walk3", (758, 262, 882, 404)),
            ("walk4", (941, 262, 1069, 404)),
            ("walk5", (1125, 262, 1250, 404)),
            ("aim", (569, 446, 715, 584)),
            ("charge", (757, 446, 913, 584)),
            ("hit", (564, 632, 693, 770)),
            ("stagger", (363, 832, 488, 962)),
            ("kneel", (524, 832, 645, 962)),
            ("down", (686, 880, 821, 962)),
        ],
    },
    {
        "name": "ninja",
        "sheet": "lvl2-bot-ninja-sprite-sheet.png",
        "standing": 88,
        "anchor": 12,
        "eye": red_visor,
        # The red visor glow and the orange muzzle flame.
        "keep": [(255, 40, 90), (255, 150, 30), (255, 235, 120)],
        "smoke": {"stagger", "kneel", "down"},
        "poses": [
            ("walk0", (214, 270, 308, 370)),
            ("walk1", (368, 270, 461, 370)),
            ("walk2", (520, 270, 614, 370)),
            ("walk3", (672, 270, 768, 370)),
            ("walk4", (826, 270, 920, 370)),
            ("walk5", (980, 270, 1074, 370)),
            # Hand up, signalling: the wind-up before a burst.
            ("signal", (366, 425, 466, 538), "noflame"),
            # Arm out, gun levelled; the flame on its own.
            ("point", (510, 437, 653, 538), "noflame"),
            ("flame", (510, 437, 653, 538), "flame"),
            ("hit", (547, 619, 641, 721)),
            ("stagger", (375, 790, 468, 913)),
            ("kneel", (523, 798, 632, 913)),
            ("down", (687, 853, 804, 913)),
        ],
    },
]


for bot in BOTS:
    sheet = Image.open(SHEETS / bot["sheet"]).convert("RGB")
    frames = []
    for name, box, *part in bot["poses"]:
        small, clear, body = cut(sheet, bot, name, box, part[0] if part else None)
        frame, placed = frame_of(small, clear, body, bot["anchor"])
        if part and part[0] == "flame":
            # The flame as its own frame, moved to the left edge: the game draws it
            # `dx` pixels right of the pose, which may be past the pose's frame.
            dx = min(x for x, _, _ in placed)
            frame = Image.new("RGBA", (FRAME, FRAME), (0, 0, 0, 0))
            for x, y, c in placed:
                frame.putpixel((x - dx, y), c + (255,))
            ys = [y for _, y, _ in placed]
            print(f"{bot['name']} {name}: {dx} px right of the pose, rows {min(ys)}..{max(ys)}")
        frames.append(frame)
    colours, indices = quantise(frames, bot["keep"])
    name = bot["name"]
    (GFX / f"{name}.tiles").write_bytes(b"".join(tiles(i, FRAME) for i in indices))
    (GFX / f"{name}.pal").write_bytes(bgr555(colours))
    review = Image.new("RGB", (FRAME * len(frames), FRAME), (40, 12, 40))
    for n, idx in enumerate(indices):
        for p, c in enumerate(idx):
            if c:
                review.putpixel((n * FRAME + p % FRAME, p // FRAME), colours[c - 1])
    review.save(GFX / f"{name}.png")
    print(f"wrote game/gfx/{name}.* ({len(frames)} poses) from {bot['sheet']}")
