#!/usr/bin/env python3
"""First-pass pixel art for Ethereum City, drawn by script until the author redraws it.

Everything here is a stand-in the author asked for on 2026-11-25 ("script a first
pass"): it goes to project/generated/, never to project/assets/, and each file is listed
in the README as art to redraw. Same seed, same pixels.

- sprites/ape-boss.png: BORED KONG, the level's boss: a giant ape with a bored, half-lidded
  stare. 64x64 frames, facing right, feet on the bottom row, 8 a row:
    0-1 idle, 2-5 walk, 6-7 chest pound (the leap's tell), 8 crouch, 9 airborne,
    10 slam, 11-12 throw (wind-up, release), 13-14 charge, 15 stunned, 16 hit,
    17-19 death (slump, sit, flat)
- sprites/ape-boss-portrait.png: its face for the dialogue box, 32x32
- sprites/ape-rubble.png: the chunk of masonry it throws, 16x16, 2 frames
- sprites/emerald.png: the green emerald that recharges the plasma shield, 16x16, 4 frames
- sprites/plasma-shield.png: the bubble around the wassie, 64x64: 0-3 humming, 4-5 a hit
  soaked up
- sprites/shield-hud.png: the shield's charge in the HUD, 8x8: 0 charged, 1 spent
- backgrounds/eth-city-sky.png, backgrounds/eth-city-skyline.png: New York at dusk, 256x160

    python3 tools/eth-city/draw-art.py   # needs Pillow and numpy
"""
import math
import random
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
SPRITES = ROOT / "project/generated/sprites"
BACKGROUNDS = ROOT / "project/generated/backgrounds"


def hexrgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i : i + 2], 16) for i in (0, 2, 4))


# --------------------------------------------------------------------------------------
# The ape. Parts are drawn back to front into a palette-index canvas with a part map
# beside it; a cel-shading pass then lights each part's top-left edge and darkens its
# bottom-right one, and an outline closes the silhouette.

APE_PALETTE = [
    None,  # 0: transparent
    "#140b1a",  # 1 outline
    "#2e1c16",  # 2 fur dark
    "#4f3426",  # 3 fur
    "#77513a",  # 4 fur light
    "#8a5a3c",  # 5 skin dark
    "#c08a5e",  # 6 skin
    "#e6b886",  # 7 skin light
    "#5a4650",  # 8 chest
    "#7c6670",  # 9 chest light
    "#f2eee0",  # 10 white
    "#b8263e",  # 11 mouth
    "#8b8b9c",  # 12 rubble
    "#55566a",  # 13 rubble dark
    "#ffd84a",  # 14 gold (dizzy stars)
]
OUT, FUR_D, FUR, FUR_L, SKIN_D, SKIN, SKIN_L, CHEST, CHEST_L, WHITE, MOUTH, ROCK, ROCK_D, GOLD = range(1, 15)
# A part's material: (dark, base, light).
MATERIAL = {"fur": (FUR_D, FUR, FUR_L), "skin": (SKIN_D, SKIN, SKIN_L), "chest": (FUR_D, CHEST, CHEST_L), "rock": (ROCK_D, ROCK, ROCK)}


class Canvas:
    def __init__(self, w, h):
        self.w, self.h = w, h
        self.img = Image.new("L", (w, h), 0)
        self.parts = Image.new("L", (w, h), 0)
        self.material = {}
        self.next = 1

    def _part(self, material):
        pid = self.next
        self.next += 1
        self.material[pid] = material
        return pid

    def ellipse(self, cx, cy, rx, ry, material, outline=True):
        pid = self._part(material)
        box = [cx - rx, cy - ry, cx + rx, cy + ry]
        d, p = ImageDraw.Draw(self.img), ImageDraw.Draw(self.parts)
        base = MATERIAL[material][1]
        if outline:
            d.ellipse([box[0] - 1, box[1] - 1, box[2] + 1, box[3] + 1], fill=OUT)
        d.ellipse(box, fill=base)
        p.ellipse(box, fill=pid)

    def limb(self, points, width, material):
        """A thick bent line through `points`, rounded at the joints, outlined."""
        pid = self._part(material)
        d, p = ImageDraw.Draw(self.img), ImageDraw.Draw(self.parts)
        base = MATERIAL[material][1]
        for w, col, target in [(width + 2, OUT, None), (width, base, pid)]:
            d.line(points, fill=col, width=w, joint="curve")
            r = w / 2
            for x, y in points:
                d.ellipse([x - r + 0.5, y - r + 0.5, x + r - 0.5, y + r - 0.5], fill=col)
            if target:
                p.line(points, fill=target, width=w, joint="curve")
                for x, y in points:
                    p.ellipse([x - r + 0.5, y - r + 0.5, x + r - 0.5, y + r - 0.5], fill=target)

    def poly(self, points, material):
        pid = self._part(material)
        d, p = ImageDraw.Draw(self.img), ImageDraw.Draw(self.parts)
        d.polygon(points, fill=MATERIAL[material][1], outline=OUT)
        p.polygon(points, fill=pid)

    def px(self, x, y, col):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.img.putpixel((int(x), int(y)), col)

    def line(self, points, col, width=1):
        ImageDraw.Draw(self.img).line(points, fill=col, width=width)

    def shade(self):
        """Cel shading by part: a part's pixels facing up-left are lit, those facing
        down-right are in shadow. Detail drawn after this keeps its own colours."""
        img = np.array(self.img)
        parts = np.array(self.parts)
        out = img.copy()
        h, w = img.shape
        for y in range(h):
            for x in range(w):
                pid = parts[y, x]
                if pid == 0 or img[y, x] == OUT:
                    continue
                dark, base, light = MATERIAL[self.material[pid]]
                if img[y, x] != base:
                    continue

                def other(dx, dy):
                    xx, yy = x + dx, y + dy
                    return not (0 <= xx < w and 0 <= yy < h) or parts[yy, xx] != pid

                if other(-1, -1) or other(0, -2):
                    out[y, x] = light
                elif other(2, 2) or other(1, 3) or other(3, 1):
                    out[y, x] = dark
        self.img = Image.fromarray(out)
        self.parts = Image.fromarray(np.where(out > 0, parts, 0).astype(np.uint8))

    def outline(self):
        """A dark line around the whole silhouette."""
        img = np.array(self.img)
        solid = img > 0
        ring = np.zeros_like(solid)
        ring[1:, :] |= solid[:-1, :]
        ring[:-1, :] |= solid[1:, :]
        ring[:, 1:] |= solid[:, :-1]
        ring[:, :-1] |= solid[:, 1:]
        img[ring & ~solid] = OUT
        self.img = Image.fromarray(img)


def ape_head(c, hx, hy, s=1.0, mouth="flat", eyes="bored", tilt=0):
    """The head, facing right, its dome centred on (hx, hy). `s` scales it (the portrait)."""
    S = lambda v: v * s
    c.ellipse(hx, hy, S(10), S(9.5), "fur")  # skull
    c.ellipse(hx - S(8), hy + S(2), S(3), S(3.5), "skin")  # ear
    c.ellipse(hx + S(4), hy + S(3), S(7.5), S(7), "skin")  # face mask
    c.ellipse(hx + S(8), hy + S(7.5 + tilt), S(6.5), S(4.5), "skin")  # muzzle
    c.shade()
    # The brow: a heavy fur ridge over the eyes.
    brow_y = hy - S(1)
    c.line([(hx - S(1), brow_y), (hx + S(10), brow_y + S(0.5))], OUT, width=max(1, round(S(1.4))))
    # Eyes: wide whites under heavy lids, pupils low and to the side. Bored.
    for ex in (hx + S(2.5), hx + S(8)):
        ey = hy + S(2)
        ew = S(2.6)
        if eyes == "x":
            r = max(1, round(S(1.5)))
            c.line([(ex - r, ey - r), (ex + r, ey + r)], OUT)
            c.line([(ex - r, ey + r), (ex + r, ey - r)], OUT)
            continue
        ImageDraw.Draw(c.img).ellipse([ex - ew, ey - S(1.6), ex + ew, ey + S(1.6)], fill=WHITE, outline=OUT)
        # Pupil looking ahead (right), low in the eye.
        pr = max(1, round(S(0.9)))
        px, py = ex + S(0.8), ey + S(0.4)
        ImageDraw.Draw(c.img).rectangle([px - pr + 1, py - pr + 1, px + pr - 1, py + pr - 1], fill=OUT)
        if eyes == "bored":
            # The lid covers the top half of the eye, a hard flat line: unimpressed.
            ImageDraw.Draw(c.img).rectangle([ex - ew, ey - S(1.8), ex + ew, ey - S(0.1)], fill=SKIN_D)
            c.line([(ex - ew, ey), (ex + ew, ey)], OUT)
        elif eyes == "angry":
            ImageDraw.Draw(c.img).polygon([(ex - ew, ey - S(2)), (ex + ew, ey - S(2)), (ex + ew, ey + S(0.2)), (ex - ew, ey - S(1.2))], fill=SKIN_D)
            c.line([(ex - ew, ey - S(1.2)), (ex + ew, ey + S(0.2))], OUT)
    # Nostrils on the muzzle.
    nx, ny = hx + S(11), hy + S(5.5 + tilt)
    c.px(nx, ny, OUT)
    c.px(nx + S(2), ny, OUT)
    # Mouth: a long flat line, or open in a roar.
    my = hy + S(9 + tilt)
    if mouth == "flat":
        c.line([(hx + S(4), my), (hx + S(14), my - S(0.5))], OUT)
        c.px(hx + S(4), my - 1, OUT)
    else:
        box = [hx + S(4), my - S(2), hx + S(14), my + S(3)]
        ImageDraw.Draw(c.img).ellipse(box, fill=MOUTH, outline=OUT)
        # Fangs.
        for fx in (hx + S(6), hx + S(12)):
            c.px(fx, my - S(1), WHITE)
            c.px(fx, my - S(1) + 1, WHITE)


def ape(pose):
    """One 64x64 frame of the ape, from a pose: points for its limbs, torso and head."""
    c = Canvas(64, 64)
    p = pose
    # Back to front: far arm, far leg, torso and chest, near leg, head, near arm.
    c.limb(p["far_arm"], 9, "fur")
    if p.get("far_fist"):
        c.ellipse(*p["far_fist"], 5, 4.5, "fur")
    c.limb(p["far_leg"], 9, "fur")
    c.ellipse(*p["torso"], "fur")
    if p.get("chest"):
        c.ellipse(*p["chest"], "chest")
    c.limb(p["near_leg"], 10, "fur")
    c.ellipse(p["near_leg"][-1][0] + 2, p["near_leg"][-1][1], 5, 2.5, "skin")  # foot
    c.shade()
    hx, hy = p["head"]
    ape_head(c, hx, hy, mouth=p.get("mouth", "flat"), eyes=p.get("eyes", "bored"), tilt=p.get("tilt", 0))
    c.limb(p["near_arm"], 10, "fur")
    if p.get("near_fist"):
        c.ellipse(*p["near_fist"], 5.5, 5, "skin")
    if p.get("rock"):
        x, y = p["rock"]
        c.poly([(x - 6, y - 2), (x - 2, y - 6), (x + 5, y - 5), (x + 7, y + 1), (x + 3, y + 6), (x - 5, y + 5)], "rock")
    c.shade()
    c.outline()
    for x, y in p.get("stars", []):
        for dx, dy in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)]:
            c.px(x + dx, y + dy, GOLD)
    return c.img


def ape_frames():
    """The poses, by frame. Feet on row 62-63, body centred near column 30."""
    def standing(breath=0, arms=0, mouth="flat", eyes="bored", head=(41, 19)):
        b = breath
        return {
            "far_arm": [(24, 30 + b), (15 + arms, 44), (17 + arms, 57)],
            "far_fist": (17 + arms, 58, ),
            "far_leg": [(26, 47), (21, 55), (21, 61)],
            "torso": (30, 37 + b, 15, 14),
            "chest": (36, 39 + b, 8, 9),
            "near_leg": [(33, 48), (37, 55), (37, 61)],
            "head": (head[0], head[1] + b),
            "near_arm": [(37, 31 + b), (46 - arms, 45), (47 - arms, 56)],
            "near_fist": (47 - arms, 58),
            "mouth": mouth,
            "eyes": eyes,
        }

    frames = []
    frames.append(standing(0))  # 0 idle
    frames.append(standing(1))  # 1 idle, breathing out
    for swing in (-4, -1, 3, 1):  # 2-5 walk: the arms swing as it knuckles along
        f = standing(0, arms=swing)
        f["far_leg"] = [(26, 47), (21 - swing, 55), (21 - swing, 61)]
        f["near_leg"] = [(33, 48), (37 + swing, 55), (37 + swing, 61)]
        frames.append(f)
    for beat in (0, 1):  # 6-7 chest pound
        f = standing(0, mouth="open", eyes="angry", head=(41, 16))
        f["near_arm"] = [(37, 31), (47, 38 - 3 * beat), (40, 35 - 2 * beat)]
        f["near_fist"] = (40, 35 - 2 * beat)
        f["far_arm"] = [(24, 30), (34, 40 - 3 * (1 - beat)), (33, 36 - 2 * (1 - beat))]
        f["far_fist"] = (34, 37 - 2 * (1 - beat))
        frames.append(f)
    # 8 crouch: low, gathering itself for the leap.
    f = standing(0, eyes="angry")
    for k in ("torso", "chest"):
        x, y, rx, ry = f[k]
        f[k] = (x, y + 7, rx + 1, ry - 1)
    f["head"] = (42, 27)
    f["far_arm"] = [(24, 37), (14, 48), (13, 58)]
    f["far_fist"] = (13, 59)
    f["near_arm"] = [(37, 38), (48, 50), (50, 58)]
    f["near_fist"] = (50, 59)
    f["far_leg"] = [(25, 52), (17, 56), (20, 61)]
    f["near_leg"] = [(34, 53), (42, 56), (40, 61)]
    frames.append(f)
    # 9 airborne: arms over its head, fists together, legs tucked.
    f = standing(0, mouth="open", eyes="angry", head=(40, 22))
    f["torso"] = (30, 38, 14, 13)
    f["chest"] = (35, 40, 7, 8)
    f["far_arm"] = [(25, 30), (24, 14), (33, 5)]
    f["far_fist"] = (33, 6)
    f["near_arm"] = [(36, 30), (44, 14), (38, 5)]
    f["near_fist"] = (37, 6)
    f["far_leg"] = [(26, 48), (19, 53), (24, 58)]
    f["near_leg"] = [(33, 48), (40, 53), (36, 58)]
    frames.append(f)
    # 10 slam: fists into the ground either side, roaring.
    f = standing(0, mouth="open", eyes="angry", head=(40, 26))
    f["torso"] = (31, 43, 16, 12)
    f["chest"] = (36, 45, 8, 7)
    f["far_arm"] = [(22, 38), (11, 48), (7, 58)]
    f["far_fist"] = (7, 59)
    f["near_arm"] = [(40, 38), (52, 48), (56, 58)]
    f["near_fist"] = (56, 59)
    f["far_leg"] = [(26, 52), (19, 56), (20, 61)]
    f["near_leg"] = [(34, 53), (41, 56), (40, 61)]
    frames.append(f)
    # 11 throw, wind-up: a chunk of masonry held back over its shoulder.
    f = standing(0, mouth="open", eyes="angry")
    f["near_arm"] = [(36, 31), (27, 18), (20, 9)]
    f["near_fist"] = (20, 10)
    f["rock"] = (16, 8)
    frames.append(f)
    # 12 throw, release: the arm flung forward.
    f = standing(0, mouth="open", eyes="angry", head=(43, 20))
    f["near_arm"] = [(38, 31), (50, 27), (58, 23)]
    f["near_fist"] = (58, 23)
    frames.append(f)
    for stride in (0, 1):  # 13-14 charge: low, on its knuckles, head down
        s = 3 * stride
        f = {
            "far_arm": [(36, 40), (46 + s, 50), (54 + s, 58)],
            "far_fist": (54 + s, 59),
            "far_leg": [(18, 46), (12 - s, 54), (10 - s, 61)],
            "torso": (27, 44, 17, 10),
            "chest": (33, 48, 9, 5),
            "near_leg": [(22, 48), (17 + s, 55), (16 + s, 61)],
            "head": (45, 37),
            "near_arm": [(38, 42), (48 - s, 52), (53 - s, 59)],
            "near_fist": (53 - s, 59),
            "mouth": "open",
            "eyes": "angry",
        }
        frames.append(f)
    # 15 stunned: slumped, seeing stars.
    f = standing(1, eyes="x", head=(40, 22))
    f["tilt"] = 1
    f["stars"] = [(30, 6), (40, 3), (50, 7)]
    frames.append(f)
    # 16 hit: rocked back.
    f = standing(0, mouth="open", eyes="angry", head=(37, 19))
    f["torso"] = (28, 37, 15, 14)
    f["chest"] = (34, 39, 8, 9)
    frames.append(f)
    # 17-19 death: slumps to its knees, sits back, then lies flat.
    f = standing(0, eyes="x", head=(42, 30))
    for k in ("torso", "chest"):
        x, y, rx, ry = f[k]
        f[k] = (x, y + 9, rx, ry - 2)
    f["far_arm"] = [(25, 40), (19, 52), (21, 61)]
    f["far_fist"] = (21, 60)
    f["near_arm"] = [(37, 41), (44, 52), (45, 61)]
    f["near_fist"] = (45, 60)
    f["far_leg"] = [(25, 55), (18, 59), (14, 61)]
    f["near_leg"] = [(33, 56), (40, 59), (46, 61)]
    frames.append(f)
    f = standing(0, eyes="x", head=(30, 34))
    f["torso"] = (26, 50, 15, 10)
    f["chest"] = (31, 51, 8, 6)
    f["far_arm"] = [(20, 46), (12, 54), (8, 61)]
    f["far_fist"] = (8, 60)
    f["near_arm"] = [(32, 46), (40, 55), (46, 61)]
    f["near_fist"] = (46, 60)
    f["far_leg"] = [(28, 56), (40, 59), (50, 61)]
    f["near_leg"] = [(30, 57), (44, 59), (54, 61)]
    frames.append(f)
    f = {
        "far_arm": [(20, 54), (10, 57), (4, 61)],
        "far_fist": (5, 60),
        "far_leg": [(40, 55), (50, 58), (58, 61)],
        "torso": (30, 55, 18, 7),
        "chest": (32, 53, 9, 4),
        "near_leg": [(38, 57), (48, 60), (56, 61)],
        "head": (13, 49),
        "near_arm": [(28, 56), (36, 60), (44, 61)],
        "near_fist": (44, 60),
        "eyes": "x",
    }
    frames.append(f)
    assert len(frames) == 20
    return frames


def palette_image(index_img, palette, size=None):
    """An RGBA picture from a palette-index image; index 0 is transparent."""
    idx = np.array(index_img)
    out = np.zeros((*idx.shape, 4), np.uint8)
    for i, col in enumerate(palette):
        if col is None:
            continue
        out[idx == i] = (*hexrgb(col), 255)
    return Image.fromarray(out, "RGBA")


def sheet(frames, w, h, cols):
    rows = (len(frames) + cols - 1) // cols
    canvas = Image.new("RGBA", (cols * w, rows * h), (0, 0, 0, 0))
    for i, f in enumerate(frames):
        canvas.paste(f, ((i % cols) * w, (i // cols) * h))
    return canvas


def ape_portrait():
    c = Canvas(32, 32)
    # Shoulders along the bottom, the head big above them.
    c.ellipse(15, 34, 16, 8, "fur")
    c.shade()
    ape_head(c, 12, 13, s=1.35)
    c.shade()
    c.outline()
    return palette_image(c.img, APE_PALETTE)


def rubble():
    frames = []
    for turn in (0, 1):
        c = Canvas(16, 16)
        pts = [(2, 6), (6, 2), (12, 3), (14, 9), (10, 14), (3, 12)]
        if turn:
            pts = [(16 - y, x) for x, y in pts]
        c.poly(pts, "rock")
        c.shade()
        c.outline()
        # A brick's mortar lines.
        c.px(6, 7, ROCK_D)
        c.px(7, 7, ROCK_D)
        c.px(8, 7, ROCK_D)
        c.px(9, 9, ROCK_D)
        frames.append(palette_image(c.img, APE_PALETTE))
    return frames


# --------------------------------------------------------------------------------------
# The emerald and the plasma shield: one green, so the pickup reads as the shield's fuel.

GREEN = [None, "#0b2e22", "#147a48", "#2fcf78", "#8df5b9", "#f0fff6", "#1f4f9e", "#7fd8ff"]
G_OUT, G_DARK, G_BASE, G_LIGHT, G_WHITE, G_BLUE, G_CYAN = range(1, 8)


def emerald_frames():
    """An emerald, step cut, catching the light across its four frames."""
    frames = []
    shape = [(5, 1), (10, 1), (14, 5), (14, 10), (10, 14), (5, 14), (1, 10), (1, 5)]
    table = [(6, 4), (9, 4), (11, 6), (11, 9), (9, 11), (6, 11), (4, 9), (4, 6)]
    for f in range(4):
        img = Image.new("L", (16, 16), 0)
        d = ImageDraw.Draw(img)
        d.polygon(shape, fill=G_BASE, outline=G_OUT)
        # Facets: dark lower right, light upper left, the table in the middle.
        d.polygon([(14, 5), (14, 10), (10, 14), (11, 9), (11, 6)], fill=G_DARK)
        d.polygon([(10, 14), (5, 14), (6, 11), (9, 11)], fill=G_DARK)
        d.polygon([(5, 1), (10, 1), (9, 4), (6, 4)], fill=G_LIGHT)
        d.polygon([(1, 5), (5, 1), (6, 4), (4, 6)], fill=G_LIGHT)
        d.polygon(table, fill=G_BASE)
        d.line([(6, 4), (9, 4)], fill=G_LIGHT)
        d.polygon(shape, outline=G_OUT)
        # The glint slides across the table.
        gx = [5, 7, 9, 11][f]
        gy = [5, 6, 7, 8][f]
        for dx, dy in [(0, 0), (1, 0), (0, 1)]:
            img.putpixel((gx + dx - 1, gy + dy - 1), G_WHITE)
        if f == 0:
            img.putpixel((3, 3), G_WHITE)
        frames.append(palette_image(img, GREEN))
    return frames


def shield_frames():
    """A hexagon-lattice bubble, 22 px in radius, centred in a 64x64 frame: a thin ring
    whose bright arcs turn as it hums, and a thick flash for a hit it soaks up."""
    frames = []
    cx, cy, r = 31.5, 31.5, 21
    for f in range(6):
        img = Image.new("L", (64, 64), 0)
        hit = f >= 4
        for a in range(0, 360, 2):
            t = math.radians(a)
            x, y = cx + r * math.cos(t), cy + r * math.sin(t)
            # Bright arcs, three of them, turning a step each frame.
            arc = (a + f * 30) % 120
            col = G_WHITE if hit and f == 4 else G_LIGHT if arc < 40 or hit else G_BASE
            img.putpixel((round(x), round(y)), col)
            if hit or arc < 20:
                xi, yi = cx + (r - 1) * math.cos(t), cy + (r - 1) * math.sin(t)
                img.putpixel((round(xi), round(yi)), G_LIGHT if f != 4 else G_CYAN)
        # Lattice glints inside the ring, so it reads as a field and not a hoop.
        rng = random.Random(7 + (f % 4))
        for _ in range(5 if not hit else 10):
            t = rng.random() * 2 * math.pi
            d = r - 3 - rng.random() * 4
            img.putpixel((round(cx + d * math.cos(t)), round(cy + d * math.sin(t))), G_LIGHT if not hit else G_WHITE)
        frames.append(palette_image(img, GREEN))
    return frames


def shield_hud():
    frames = []
    for full in (True, False):
        img = Image.new("L", (8, 8), 0)
        d = ImageDraw.Draw(img)
        hexagon = [(2, 0), (5, 0), (7, 3), (7, 4), (5, 7), (2, 7), (0, 4), (0, 3)]
        d.polygon(hexagon, fill=G_BASE if full else G_OUT, outline=G_OUT)
        if full:
            d.line([(2, 1), (4, 1)], fill=G_WHITE)
            d.point([(1, 3), (2, 2)], fill=G_LIGHT)
            d.line([(3, 6), (5, 6)], fill=G_DARK)
        else:
            d.polygon(hexagon, outline=G_DARK)
        frames.append(palette_image(img, GREEN))
    return frames


# --------------------------------------------------------------------------------------
# New York at dusk: an indigo sky over the Ethereum-violet glow of the city, and a
# skyline of brownstones, water towers and art-deco spires in front of it.

SKY = ["#140c33", "#1f1550", "#2d1f6b", "#43308a", "#6446a8", "#8d63c0", "#c487c8", "#f4b7b0", "#fff3c8", "#e0dcff"]


def sky():
    w, h = 256, 160
    img = Image.new("RGB", (w, h))
    bands = [(0, 0), (24, 1), (46, 2), (66, 3), (84, 4), (100, 5), (114, 6), (126, 7), (136, 8)]
    px = img.load()
    for y in range(h):
        band = max(i for top, i in bands if y >= top)
        nxt = min(band + 1, 8)
        top = dict((i, t) for t, i in bands)[band]
        edge = dict((i, t) for t, i in bands).get(nxt, h)
        for x in range(w):
            # Ordered dither over the last rows of each band, to soften the steps.
            near = edge - y <= 3 and nxt != band
            bayer = [[0, 2], [3, 1]][y % 2][x % 2]
            use = nxt if near and bayer >= edge - y else band
            px[x, y] = hexrgb(SKY[use])
    # Stars in the top of the sky, and the moon.
    rng = random.Random(4)
    for _ in range(40):
        x, y = rng.randrange(w), rng.randrange(70)
        px[x, y] = hexrgb(SKY[9])
    # A crescent: the moon's disc less a disc offset up and right, the sky showing there.
    moon = Image.new("L", (w, h), 0)
    ImageDraw.Draw(moon).ellipse([196, 18, 213, 35], fill=255)
    ImageDraw.Draw(moon).ellipse([201, 15, 218, 32], fill=0)
    img.paste(Image.new("RGB", (w, h), hexrgb(SKY[8])), (0, 0), moon)
    return img


CITY = [None, "#0c0a24", "#1a1440", "#2a2160", "#3b2f7c", "#ffd86a", "#a78bfa", "#ff7ab6", "#5b4a9a"]


def skyline():
    """Buildings in two depths: a far row in muted violet, a near row almost black with
    lit windows, water towers and the spires of midtown. Transparent above them."""
    w, h = 256, 160
    img = Image.new("L", (w, h), 0)
    d = ImageDraw.Draw(img)
    rng = random.Random(12)
    # The far row.
    x = 0
    while x < w:
        bw = rng.randrange(14, 30)
        top = rng.randrange(70, 104)
        d.rectangle([x, top, x + bw - 1, h], fill=3)
        x += bw
    # Midtown's two great spires, far back: the stepped tower with its mast, and the
    # crowned one with its arches.
    def stepped(cx, base):
        for i, (half, top) in enumerate([(12, base), (9, base - 18), (6, base - 30), (3, base - 38)]):
            d.rectangle([cx - half, top, cx + half, h], fill=3 if i == 0 else 4)
        d.line([(cx, base - 54), (cx, base - 38)], fill=4)
        d.point((cx, base - 55), fill=7)

    def crowned(cx, base):
        d.rectangle([cx - 9, base, cx + 9, h], fill=4)
        for i in range(4):
            r = 9 - 2 * i
            d.pieslice([cx - r, base - 8 - 6 * i, cx + r, base + 6 - 6 * i], 180, 360, fill=4 if i % 2 == 0 else 8)
        d.polygon([(cx - 1, base - 26), (cx + 1, base - 26), (cx, base - 40)], fill=8)

    stepped(64, 66)
    crowned(178, 62)
    # The near row, with windows and water towers.
    x = -4
    while x < w + 4:
        bw = rng.randrange(18, 36)
        top = rng.randrange(98, 126)
        d.rectangle([x, top, x + bw - 1, h], fill=1)
        d.line([(x, top), (x + bw - 1, top)], fill=2)
        for wy in range(top + 4, h - 4, 6):
            for wx in range(x + 3, x + bw - 3, 5):
                roll = rng.random()
                if roll < 0.32:
                    d.rectangle([wx, wy, wx + 1, wy + 2], fill=5 if roll < 0.22 else 6)
        if rng.random() < 0.55 and bw > 20:
            # A water tower on stilts: barrel, cone roof.
            tx = x + rng.randrange(4, bw - 12)
            d.rectangle([tx, top - 14, tx + 8, top - 6], fill=2)
            d.polygon([(tx - 1, top - 14), (tx + 9, top - 14), (tx + 4, top - 19)], fill=2)
            d.line([(tx + 1, top - 6), (tx + 1, top)], fill=2)
            d.line([(tx + 7, top - 6), (tx + 7, top)], fill=2)
        x += bw
    # The street's glow along the bottom.
    d.rectangle([0, 154, w, h], fill=2)
    return palette_image(img, CITY)


def main():
    SPRITES.mkdir(parents=True, exist_ok=True)
    BACKGROUNDS.mkdir(parents=True, exist_ok=True)
    frames = [palette_image(ape(p), APE_PALETTE) for p in ape_frames()]
    sheet(frames, 64, 64, 8).save(SPRITES / "ape-boss.png")
    ape_portrait().save(SPRITES / "ape-boss-portrait.png")
    sheet(rubble(), 16, 16, 2).save(SPRITES / "ape-rubble.png")
    sheet(emerald_frames(), 16, 16, 4).save(SPRITES / "emerald.png")
    sheet(shield_frames(), 64, 64, 6).save(SPRITES / "plasma-shield.png")
    sheet(shield_hud(), 8, 8, 2).save(SPRITES / "shield-hud.png")
    sky().save(BACKGROUNDS / "eth-city-sky.png")
    skyline().save(BACKGROUNDS / "eth-city-skyline.png")
    print("wrote Ethereum City's first-pass art to", SPRITES, "and", BACKGROUNDS)


if __name__ == "__main__":
    main()
