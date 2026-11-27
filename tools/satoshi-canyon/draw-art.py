#!/usr/bin/env python3
"""First-pass pixel art for Satoshi Canyon, drawn by script until the author redraws it.

The author asked for this on 2026-11-26 ("script a first pass"), as for Ethereum City:
everything goes to project/generated/, never to project/assets/, and each file is listed
in the README as art to redraw. A sheet redrawn at the same size and frame layout drops
straight in. Same script, same pixels.

Sprites (project/generated/sprites/), frames numbered row by row:

- inscriber.png: THE INSCRIBER, the level's boss: a laser-eyed Bitcoin cyborg king with
  a pickaxe, who inscribes every sat in the mine, after the author's Bitcoin Cyborg mockup
  (.claude/mockup/bitcoin-cyborg.gif: crown, cyborg mask with green eyes, long beard,
  red cape, gold pauldron, gunmetal armour, ₿ belt coin, gold pickaxe). 64x64 frames, facing right, feet on
  the bottom row, 8 a row:
    0-1 idle, 2-5 walk, 6-7 gaze (the laser eyes' tell), 8-9 laser, 10-11 inscribe
    (a glowing tablet raised), 12 pickaxe raised, 13 slam, 14 stuck (the pickaxe in the
    rock), 15-16 the Halving (glitching), 17 hit, 18-20 death (kneel, slump, flat)
- inscriber-portrait.png: its face for the dialogue box, 32x32
- inscriber-laser.png: the green laser raking along the floor, 32x8, 2 frames
- sat-glyph.png: an inscribed sat falling from the ceiling, 16x16, 4 frames (spinning)
- btc-puppet.png: the Bitcoin Puppet, after the author's reference: a brown monkey in a
  green sweater with a pencil and a bunny hand puppet, 32x32: 0-1 idle, 2-3 cheering
- btc-puppet-portrait.png: its face, 32x32
- stalactite.png: a stalactite trap, 16x32 (the rock in the top 24 rows): 0 hanging,
  1 cracking (the tell)
- mine-cart.png: a runaway mine cart full of gold, 32x16: 0-1 parked, 2-3 rolling
- brick-block.png, ore-block.png: the breakable blocks, 16x16 (tools/block-sheen.py
  makes their sheen sheets from these)

Backgrounds (project/generated/backgrounds/), 256x160:

- satoshi-sky.png: a desert sunset with a Bitcoin-orange sun
- satoshi-mesas.png: mesas, buttes, saguaros and a mine headframe, transparent above
- satoshi-mine-back.png: the mine's rock wall, timber frames and lanterns

    python3 tools/satoshi-canyon/draw-art.py   # needs Pillow
"""
import math
import random
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
SPRITES = ROOT / "project/generated/sprites"
BACKGROUNDS = ROOT / "project/generated/backgrounds"


def hexrgb(h):
    h = h.lstrip("#")
    return tuple(int(h[i : i + 2], 16) for i in (0, 2, 4))


def palette_image(index_img, palette):
    """An RGBA picture from a palette-index image; index 0 is transparent."""
    lut = [(0, 0, 0, 0) if col is None else (*hexrgb(col), 255) for col in palette]
    out = Image.new("RGBA", index_img.size)
    out.putdata([lut[i] if i < len(lut) else (255, 0, 255, 255) for i in index_img.tobytes()])
    return out


def sheet(frames, w, h, cols):
    rows = (len(frames) + cols - 1) // cols
    canvas = Image.new("RGBA", (cols * w, rows * h), (0, 0, 0, 0))
    for i, f in enumerate(frames):
        canvas.paste(f, ((i % cols) * w, (i // cols) * h))
    return canvas


class Canvas:
    """Parts drawn back to front into a palette-index canvas, with a part map beside it.
    `shade` lights each part's top-left edge and darkens its bottom-right one; `outline`
    closes the silhouette. `materials` maps a name to (dark, base, light) indices."""

    def __init__(self, w, h, materials, out):
        self.w, self.h = w, h
        self.materials = materials
        self.out = out
        self.img = Image.new("L", (w, h), 0)
        self.parts = Image.new("L", (w, h), 0)
        self.material = {}
        self.next = 1

    def _part(self, material):
        pid = self.next
        self.next += 1
        self.material[pid] = material
        return pid

    def ellipse(self, cx, cy, rx, ry, material):
        pid = self._part(material)
        box = [cx - rx, cy - ry, cx + rx, cy + ry]
        ImageDraw.Draw(self.img).ellipse([box[0] - 1, box[1] - 1, box[2] + 1, box[3] + 1], fill=self.out)
        ImageDraw.Draw(self.img).ellipse(box, fill=self.materials[material][1])
        ImageDraw.Draw(self.parts).ellipse(box, fill=pid)

    def rect(self, x0, y0, x1, y1, material):
        pid = self._part(material)
        ImageDraw.Draw(self.img).rectangle([x0 - 1, y0 - 1, x1 + 1, y1 + 1], fill=self.out)
        ImageDraw.Draw(self.img).rectangle([x0, y0, x1, y1], fill=self.materials[material][1])
        ImageDraw.Draw(self.parts).rectangle([x0, y0, x1, y1], fill=pid)

    def poly(self, points, material):
        pid = self._part(material)
        ImageDraw.Draw(self.img).polygon(points, fill=self.materials[material][1], outline=self.out)
        ImageDraw.Draw(self.parts).polygon(points, fill=pid)

    def limb(self, points, width, material):
        """A thick bent line through `points`, rounded at the joints, outlined."""
        pid = self._part(material)
        d, p = ImageDraw.Draw(self.img), ImageDraw.Draw(self.parts)
        base = self.materials[material][1]
        for w, col, target in [(width + 2, self.out, None), (width, base, pid)]:
            d.line(points, fill=col, width=w, joint="curve")
            r = w / 2
            for x, y in points:
                d.ellipse([x - r + 0.5, y - r + 0.5, x + r - 0.5, y + r - 0.5], fill=col)
            if target:
                p.line(points, fill=target, width=w, joint="curve")
                for x, y in points:
                    p.ellipse([x - r + 0.5, y - r + 0.5, x + r - 0.5, y + r - 0.5], fill=target)

    def px(self, x, y, col):
        x, y = int(round(x)), int(round(y))
        if 0 <= x < self.w and 0 <= y < self.h:
            self.img.putpixel((x, y), col)

    def line(self, points, col, width=1):
        ImageDraw.Draw(self.img).line(points, fill=col, width=width)

    def shade(self):
        w, h = self.w, self.h
        img, parts = self.img.load(), self.parts.load()
        out = self.img.copy()
        o = out.load()
        for y in range(h):
            for x in range(w):
                pid = parts[x, y]
                if pid == 0 or img[x, y] == self.out:
                    continue
                dark, base, light = self.materials[self.material[pid]]
                if img[x, y] != base:
                    continue

                def other(dx, dy):
                    xx, yy = x + dx, y + dy
                    return not (0 <= xx < w and 0 <= yy < h) or parts[xx, yy] != pid

                if other(-1, -1) or other(0, -2):
                    o[x, y] = light
                elif other(2, 2) or other(1, 3) or other(3, 1):
                    o[x, y] = dark
        self.img = out
        pp = self.parts.load()
        for y in range(h):
            for x in range(w):
                if o[x, y] == 0:
                    pp[x, y] = 0

    def outline(self):
        w, h = self.w, self.h
        src = self.img.copy().load()
        img = self.img.load()
        for y in range(h):
            for x in range(w):
                if src[x, y]:
                    continue
                near = any(0 <= x + dx < w and 0 <= y + dy < h and src[x + dx, y + dy] for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
                if near:
                    img[x, y] = self.out


# --------------------------------------------------------------------------------------
# The inscribed sats' palette: Bitcoin orange coins with an inscription-cyan glyph (the
# first-pass INSCRIBER's palette; the boss itself is now drawn in KING_PALETTE below).

INS_PALETTE = [
    None,
    "#120c18",  # 1 outline
    "#272b44",  # 2 steel dark
    "#4c5574",  # 3 steel
    "#8e99b8",  # 4 steel light
    "#e8ecf8",  # 5 chrome, white-hot
    "#8a4204",  # 6 orange dark
    "#f7931a",  # 7 Bitcoin orange
    "#ffc95c",  # 8 orange light
    "#ff2436",  # 9 laser red
    "#ffb0b8",  # 10 laser pink
    "#5a3418",  # 11 wood dark
    "#93602e",  # 12 wood
    "#4a1030",  # 13 cape dark
    "#7c2048",  # 14 cape
    "#78f2ff",  # 15 inscription cyan
]
I_OUT, ST_D, ST, ST_L, CHROME, OR_D, OR, OR_L, RED, PINK, WOOD_D, WOOD, CAPE_D, CAPE, CYAN = range(1, 16)
INS_MAT = {
    "steel": (ST_D, ST, ST_L),
    "orange": (OR_D, OR, OR_L),
    "wood": (WOOD_D, WOOD, WOOD),
    "cape": (CAPE_D, CAPE, CAPE),
    "visor": (I_OUT, ST_D, ST),
}


def bitcoin_b(c, x, y, col, scale=1):
    """A small ₿: 4x6 pixels with its two bars, top-left at (x, y)."""
    rows = ["1110", "1001", "1110", "1001", "1001", "1110"]
    for j, row in enumerate(rows):
        for i, ch in enumerate(row):
            if ch == "1":
                for sx in range(scale):
                    for sy in range(scale):
                        c.px(x + i * scale + sx, y + j * scale + sy, col)
    # The bars through the top and the bottom.
    for sx in range(scale):
        c.px(x + 1 * scale + sx, y - scale, col)
        c.px(x + 1 * scale + sx, y + 6 * scale, col)


# --------------------------------------------------------------------------------------
# THE INSCRIBER, after the author's Bitcoin Cyborg mockup (.claude/mockup/bitcoin-cyborg.gif):
# a bearded king in a gold crown with a red jewel, a dark cyborg mask with two angry
# green eyes, a long brown beard, a red cape, a gold pauldron with a round stud, dark
# gunmetal armour banded in lighter grey, a gold belt with a big ₿ coin, and a pickaxe
# with a gold handle and a gold head tipped in steel. Its laser eyes and the tablet it
# raises burn the same acid green.

KING_PALETTE = [
    None,
    "#140c14",  # 1 outline
    "#25232e",  # 2 steel dark
    "#45434f",  # 3 steel
    "#7d7b8a",  # 4 steel light
    "#93480a",  # 5 gold dark
    "#eb9a16",  # 6 gold
    "#ffd447",  # 7 gold light
    "#861422",  # 8 cape dark
    "#dc2f2a",  # 9 cape red, the crown's jewel
    "#3a2216",  # 10 beard dark
    "#6c4128",  # 11 beard
    "#b8714c",  # 12 skin dark
    "#eaa97c",  # 13 skin
    "#8cff2e",  # 14 eye green
    "#f2ffd6",  # 15 white-hot
]
K_OUT, K_ST_D, K_ST, K_ST_L, K_GO_D, K_GO, K_GO_L, K_CAPE_D, K_CAPE, K_BEARD_D, K_BEARD, K_SKIN_D, K_SKIN, K_GREEN, K_HOT = range(1, 16)
KING_MAT = {
    "steel": (K_ST_D, K_ST, K_ST_L),
    "gold": (K_GO_D, K_GO, K_GO_L),
    "cape": (K_CAPE_D, K_CAPE, K_CAPE),
    "beard": (K_BEARD_D, K_BEARD, K_BEARD),
    "skin": (K_SKIN_D, K_SKIN, K_SKIN),
    "mask": (K_OUT, K_ST_D, K_ST),
}


def king_pickaxe(c, grip, head):
    """A pickaxe: a gold handle from `grip` to `head`, and a curved gold head across it
    with steel tips."""
    gx, gy = grip
    hx, hy = head
    c.limb([(gx, gy), (hx, hy)], 3, "gold")
    dx, dy = hx - gx, hy - gy
    n = math.hypot(dx, dy) or 1
    ux, uy = dx / n, dy / n
    px_, py_ = -uy, ux
    a = (hx + px_ * 11 - ux * 4, hy + py_ * 11 - uy * 4)
    b = (hx - px_ * 11 - ux * 4, hy - py_ * 11 - uy * 4)
    a2 = (hx + px_ * 7 - ux * 1, hy + py_ * 7 - uy * 1)
    b2 = (hx - px_ * 7 - ux * 1, hy - py_ * 7 - uy * 1)
    c.limb([a2, (hx + ux * 2, hy + uy * 2), b2], 4, "gold")
    c.limb([a, a2], 3, "steel")
    c.limb([b, b2], 3, "steel")


def king_crown(c, cx, by, s=1.0):
    """The crown, its band's bottom-left at (cx, by): five points, a red jewel."""
    S = lambda v: v * s
    w, band, tall = S(16), S(3), S(6)
    pts = [(cx, by), (cx, by - band - tall)]
    for i in range(1, 5):
        x = cx + w * i / 4
        pts += [(x - w / 8, by - band), (x, by - band - tall)]
    pts += [(cx + w, by)]
    c.poly(pts, "gold")
    c.shade()
    c.line([(cx + 1, by - band), (cx + w - 1, by - band)], K_GO_D)
    for i in range(5):
        c.px(cx + w * i / 4, by - band - tall + 1, K_GO_L)
    jx, jy = cx + w / 2, by - band - S(1)
    ImageDraw.Draw(c.img).ellipse([jx - S(1.5), jy - S(1.5), jx + S(1.5), jy + S(1.5)], fill=K_CAPE, outline=K_CAPE_D)
    c.px(jx - S(0.5), jy - S(0.5), K_HOT)


def king_head(c, hx, hy, eyes="lit", s=1.0, crown=True):
    """Hair, face, mask, beard and crown, facing right, the face centred on (hx, hy)."""
    S = lambda v: v * s
    c.ellipse(hx - S(3), hy + S(1), S(8), S(9), "beard")  # hair, wild at the back
    c.ellipse(hx + S(2), hy, S(7), S(7), "skin")  # face
    # The long beard: from the cheeks, under the nose, down to a point on the chest.
    c.poly([(hx - S(7), hy - S(1)), (hx - S(1), hy + S(1)), (hx + S(3), hy + S(3)), (hx + S(9), hy + S(2)),
            (hx + S(9), hy + S(8)), (hx + S(6), hy + S(14)), (hx + S(2), hy + S(19)),
            (hx - S(2), hy + S(14)), (hx - S(7), hy + S(9))], "beard")
    c.rect(hx - S(3), hy - S(4), hx + S(9), hy - S(1), "mask")  # the cyborg mask
    c.shade()
    d = ImageDraw.Draw(c.img)
    # Strands in the beard.
    for k, x in enumerate((hx - S(4), hx - S(1), hx + S(2), hx + S(5))):
        y0 = hy + S(5 + k % 2)
        d.line([(x, y0), (x + S(0.5), y0 + S(5 + k % 3))], fill=K_BEARD_D)
    # A grim mouth under the moustache, its corners down.
    my = hy + S(5)
    d.line([(hx + S(4), my), (hx + S(7), my)], fill=K_OUT)
    c.px(hx + S(3), my + 1, K_OUT)
    c.px(hx + S(8), my + 1, K_OUT)
    # The nose over the moustache.
    c.px(hx + S(8), hy + S(1), K_SKIN_D)
    c.px(hx + S(9), hy, K_SKIN)
    # Angry eyes in the mask, 3x2, slanted down to the middle.
    ew = max(3, round(S(3)))
    for ex, inner_right in ((hx + S(0.5), True), (hx + S(5), False)):
        ex, ey = round(ex), round(hy - S(3))
        if eyes == "dead":
            for dx in range(ew):
                c.px(ex + dx, ey + 1, K_ST_L)
            continue
        hot = eyes in ("glow", "fire")
        for dx in range(ew):
            c.px(ex + dx, ey, K_GREEN)
            c.px(ex + dx, ey + 1, K_GREEN)
        # The slant: the outer top corner cut, the inner corner a pixel lower.
        c.px(ex if inner_right else ex + ew - 1, ey, K_ST_D)
        c.px(ex + ew - 1 if inner_right else ex, ey + 2, K_GREEN)
        c.px(ex + ew // 2, ey + 1, K_HOT if hot else K_GREEN)
        if eyes == "fire":
            c.px(ex + ew // 2, ey - 1, K_GREEN)
    if crown:
        king_crown(c, hx - S(6), hy - S(4.5), s)


def bands(c, points, width, col):
    """Dark rings across a limb at the middle of each of its segments."""
    for (x0, y0), (x1, y1) in zip(points, points[1:]):
        mx, my = (x0 + x1) / 2, (y0 + y1) / 2
        n = math.hypot(x1 - x0, y1 - y0) or 1
        px_, py_ = -(y1 - y0) / n, (x1 - x0) / n
        r = width / 2 - 0.5
        c.line([(mx - px_ * r, my - py_ * r), (mx + px_ * r, my + py_ * r)], col)


def inscriber(pose):
    c = Canvas(64, 64, KING_MAT, K_OUT)
    p = pose
    # Back to front: cape, far arm and leg, torso, near leg, head, pauldron, near arm
    # and pickaxe.
    if p.get("cape"):
        c.poly(p["cape"], "cape")
        c.shade()
        # Folds down the cape.
        (x0, y0), (x1, _), (x2, y2), (x3, y3) = p["cape"]
        for t in (0.35, 0.65):
            c.line([(x0 + (x1 - x0) * t, y0 + 3), (x3 + (x2 - x3) * t, y3 - 2)], K_CAPE_D)
    c.limb(p["far_arm"], 7, "steel")
    if p.get("tablet"):
        tx, ty = p["tablet"]
        c.rect(tx - 5, ty - 7, tx + 5, ty + 6, "steel")
    c.limb(p["far_leg"], 8, "steel")
    tx, ty, rx, ry = p["torso"]
    c.ellipse(tx, ty, rx, ry, "steel")
    c.limb(p["near_leg"], 8, "steel")
    fx, fy = p["near_leg"][-1]
    c.rect(fx - 4, fy - 1, fx + 6, fy + 1, "steel")  # boots
    bx, by = p["far_leg"][-1]
    c.rect(bx - 4, by - 1, bx + 5, by + 1, "steel")
    c.shade()
    for limb in ("far_arm", "far_leg", "near_leg"):
        bands(c, p[limb], 7, K_ST_D)
    # The gold belt and its ₿ coin.
    belt = ty + ry // 2
    c.rect(tx - rx + 2, belt - 1, tx + rx - 2, belt + 1, "gold")
    c.ellipse(tx + 3, belt, 5, 5, "gold")
    c.shade()
    bitcoin_b(c, tx + 2, belt - 3, K_GO_D)
    hx, hy = p["head"]
    king_head(c, hx, hy, eyes=p.get("eyes", "lit"), s=1.15, crown=p.get("crown", True))
    # The big gold pauldron on its back shoulder, a stud in the middle.
    sx, sy = p["shoulder"]
    c.poly([(sx - 8, sy - 2), (sx - 2, sy - 6), (sx + 5, sy - 4), (sx + 4, sy + 3), (sx - 5, sy + 5)], "gold")
    c.ellipse(sx, sy - 0.5, 2, 2, "gold")
    c.shade()
    if p.get("pick"):
        king_pickaxe(c, *p["pick"])
    c.limb(p["near_arm"], 7, "steel")
    c.shade()
    bands(c, p["near_arm"], 7, K_ST_D)
    nx, ny = p["near_arm"][-1]
    c.ellipse(nx, ny, 3.5, 3.5, "steel")  # fist
    c.shade()
    c.outline()
    if p.get("tablet"):
        # The tablet's face: a glyph burning green.
        tx, ty = p["tablet"]
        glow = p.get("glow", 0)
        for gx, gy in [(-2, -4), (0, -4), (2, -4), (-2, -2), (2, -2), (-2, 0), (0, 0), (-2, 2), (0, 2), (2, 2)]:
            c.px(tx + gx, ty + gy, K_GREEN)
        if glow:
            for ang in range(0, 360, 45):
                r = 9 + glow
                c.px(tx + r * math.cos(math.radians(ang)), ty + r * math.sin(math.radians(ang)), K_GREEN)
    if p.get("fallen_crown"):
        x, y = p["fallen_crown"]
        king_crown(c, x, y)
        c.outline()
    for x0, y0, x1, y1 in p.get("beams", []):
        # Laser eyes: green lines with a white-hot core, out to the floor.
        c.line([(x0, y0), (x1, y1)], K_GREEN, width=3)
        c.line([(x0, y0), (x1, y1)], K_HOT, width=1)
    for x, y in p.get("sparks", []):
        for dx, dy in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)]:
            c.px(x + dx, y + dy, K_GO_L if (dx + dy) % 2 else K_HOT)
    return c.img


def inscriber_frames():
    def standing(breath=0, eyes="lit", head=(40, 18), stride=0, cape=True):
        b = breath
        f = {
            "far_arm": [(25, 29 + b), (19, 38), (21, 46)],
            "far_leg": [(27, 46), (25 - stride, 54), (25 - stride, 61)],
            "torso": (31, 36 + b, 12, 11),
            "near_leg": [(34, 46), (35 + stride, 54), (35 + stride, 61)],
            "head": (head[0], head[1] + b),
            "shoulder": (25, 28 + b),
            "near_arm": [(37, 30 + b), (44, 37), (48, 42)],
            "pick": ((48, 42), (56, 57)),
            "eyes": eyes,
        }
        if cape:
            f["cape"] = [(20, 24 + b), (32, 24 + b), (24, 56), (9, 59)]
        return f

    frames = [standing(0), standing(1)]  # 0-1 idle
    for s in (-3, -1, 3, 1):  # 2-5 walk
        f = standing(0, stride=s)
        f["far_arm"] = [(25, 29), (19 + s, 38), (21 + s, 46)]
        f["cape"] = [(20, 24), (32, 24), (24, 56), (9 - abs(s), 58 + (s > 0))]
        frames.append(f)
    for glow in (0, 1):  # 6-7 gaze: head down a little, eyes building
        f = standing(0, eyes="glow", head=(41, 20))
        if glow:
            f["sparks"] = [(50, 14), (53, 21)]
        frames.append(f)
    for k in (0, 1):  # 8-9 laser: beams from the eyes to the floor ahead
        f = standing(0, eyes="fire", head=(42, 21))
        f["beams"] = [(44, 19, 63, 58 + k), (48, 19, 63, 61)]
        frames.append(f)
    for glow in (0, 2):  # 10-11 inscribe: the tablet raised high, glowing
        f = standing(0, eyes="glow")
        f["far_arm"] = [(25, 29), (21, 19), (21, 10)]
        f["tablet"] = (19, 10)
        f["glow"] = glow
        frames.append(f)
    # 12 pickaxe raised over its head, behind it.
    f = standing(0, eyes="glow", head=(39, 19))
    f["near_arm"] = [(37, 30), (36, 20), (31, 12)]
    f["pick"] = ((31, 13), (12, 12))
    frames.append(f)
    # 13 slam: the pickaxe swung down into the floor ahead.
    f = standing(0, eyes="fire", head=(43, 22))
    f["torso"] = (33, 38, 12, 10)
    f["near_arm"] = [(40, 32), (49, 39), (54, 45)]
    f["pick"] = ((53, 45), (58, 58))
    f["sparks"] = [(56, 61), (61, 59), (51, 60)]
    frames.append(f)
    # 14 stuck: bent over, both hands on the handle, the head buried.
    f = standing(0, eyes="lit", head=(43, 27))
    f["torso"] = (33, 40, 12, 9)
    f["far_arm"] = [(28, 34), (40, 41), (50, 46)]
    f["near_arm"] = [(40, 35), (48, 41), (52, 46)]
    f["pick"] = ((51, 46), (57, 60))
    f["shoulder"] = (27, 33)
    f["cape"] = [(21, 30), (33, 30), (22, 56), (8, 58)]
    f["sparks"] = [(58, 62), (54, 58)]
    frames.append(f)
    # 15-16 the Halving: glitching, its top half sliding off its bottom half.
    for k in (0, 1):
        f = standing(0, eyes="fire")
        f["sparks"] = [(20, 20), (46, 30)] if k else [(28, 10), (50, 24)]
        frames.append(("halving", f, k))
    # 17 hit: rocked back.
    f = standing(0, eyes="glow", head=(37, 18))
    f["torso"] = (29, 36, 12, 11)
    frames.append(f)
    # 18-20 death: to one knee, sparking; slumped, the crown slipping; flat, the crown
    # rolled off beside it.
    f = standing(0, eyes="lit", head=(40, 29))
    f["torso"] = (31, 44, 12, 10)
    f["shoulder"] = (25, 37)
    f["far_leg"] = [(27, 53), (20, 57), (18, 61)]
    f["near_leg"] = [(34, 54), (42, 57), (44, 61)]
    f["far_arm"] = [(25, 38), (21, 47), (23, 55)]
    f["near_arm"] = [(37, 39), (44, 46), (47, 52)]
    f["pick"] = ((47, 52), (58, 41))
    f["cape"] = [(20, 34), (32, 34), (22, 59), (9, 61)]
    f["sparks"] = [(30, 24), (49, 20)]
    frames.append(f)
    f = standing(0, eyes="dead", head=(45, 42))
    f["torso"] = (33, 51, 13, 8)
    f["shoulder"] = (27, 47)
    f["far_leg"] = [(25, 56), (16, 59), (10, 61)]
    f["near_leg"] = [(30, 57), (22, 60), (16, 61)]
    f["far_arm"] = [(28, 48), (24, 55), (26, 61)]
    f["near_arm"] = [(40, 49), (48, 55), (52, 60)]
    f["pick"] = ((52, 60), (60, 47))
    f["cape"] = [(20, 45), (34, 45), (14, 61), (4, 61)]
    frames.append(f)
    f = {
        "far_arm": [(22, 56), (14, 59), (8, 61)],
        "far_leg": [(40, 57), (50, 59), (57, 61)],
        "torso": (30, 57, 14, 5),
        "near_leg": [(38, 58), (48, 60), (55, 61)],
        "head": (14, 52),
        "shoulder": (24, 54),
        "near_arm": [(28, 58), (36, 60), (42, 61)],
        "pick": ((46, 61), (54, 50)),
        "eyes": "dead",
        "crown": False,
        "fallen_crown": (1, 62),
    }
    frames.append(f)
    assert len(frames) == 21, len(frames)
    images = []
    for f in frames:
        if isinstance(f, tuple):
            _, pose, k = f
            src = inscriber(pose)
            # The Halving: the top half slides one way, then the other, with a gap.
            shift = 3 if k == 0 else -3
            img = src.copy()
            img.paste(0, (0, 0, 64, 38))
            img.paste(src.crop((0, 1, 64, 38)), (shift, 0))
            # Scanlines through it.
            px = img.load()
            for y in range(4 + k * 2, 60, 6):
                for x in range(64):
                    if px[x, y]:
                        px[x, y] = K_GREEN if (y // 6) % 2 else K_ST_L
            images.append(img)
        else:
            images.append(inscriber(f))
    return images


def inscriber_portrait():
    c = Canvas(32, 32, KING_MAT, K_OUT)
    c.ellipse(8, 33, 10, 7, "cape")
    c.ellipse(16, 34, 13, 7, "steel")
    c.poly([(1, 27), (6, 23), (12, 25), (10, 31), (1, 32)], "gold")  # the pauldron
    c.shade()
    king_head(c, 13, 17, eyes="glow", s=1.2)
    c.shade()
    c.outline()
    return palette_image(c.img, KING_PALETTE)


def laser_frames():
    """The laser raking along the floor: a white-hot core in a green beam, sparks off its head."""
    frames = []
    for k in range(2):
        img = Image.new("L", (32, 8), 0)
        d = ImageDraw.Draw(img)
        d.rectangle([0, 2, 31, 5], fill=K_GREEN)
        d.rectangle([2, 3, 31, 4], fill=K_HOT)
        d.line([(8 + 4 * k, 3), (31, 3)], fill=K_HOT)
        # The head flares; the tail breaks up.
        d.rectangle([28, 1, 31, 6], fill=K_HOT)
        d.point([(30, 0), (31, 7), (29, 7) if k else (30, 7)], fill=K_GREEN)
        for x in range(0, 8, 2):
            img.putpixel((x + k, 2), 0)
            img.putpixel((x + 1 - k, 5), 0)
        frames.append(palette_image(img, KING_PALETTE))
    return frames


def sat_frames():
    """An inscribed sat: an orange coin with a cyan inscription, turning as it falls."""
    frames = []
    for k, half in enumerate([6, 4, 1, 4]):
        img = Image.new("L", (16, 16), 0)
        d = ImageDraw.Draw(img)
        d.ellipse([7.5 - half - 1, 1, 7.5 + half + 1, 14], fill=I_OUT)
        d.ellipse([7.5 - half, 2, 7.5 + half, 13], fill=OR)
        if half > 2:
            d.ellipse([7.5 - half + 1, 3, 7.5 + half - 1, 12], outline=OR_D)
            # The glyph: a ring and a stroke, the sign of an inscription.
            gx = 7 if k != 3 else 8
            for x, y in [(gx - 1, 6), (gx, 5), (gx + 1, 6), (gx - 1, 8), (gx + 1, 8), (gx, 9), (gx, 7)]:
                img.putpixel((x, y), CYAN)
            img.putpixel((5 if k != 3 else 9, 4), OR_L)
        else:
            d.line([(7, 2), (7, 13)], fill=OR_L)
        frames.append(palette_image(img, INS_PALETTE))
    return frames


# --------------------------------------------------------------------------------------
# The Bitcoin Puppet, after the reference the author shared on 2026-11-26: a brown monkey
# with a big round head and round ears, a tan face, two dot eyes and a wide black grin,
# a black ₿ on its face, a green sweater with an orange peace sign, a pencil in one hand
# and a little grey bunny hand puppet on the other. Drawn crude and friendly, as the
# collection is.

PUP_PALETTE = [
    None,
    "#1a1210",  # 1 outline, eyes and grin
    "#5a3418",  # 2 fur dark
    "#8a5a2c",  # 3 fur
    "#b07a44",  # 4 fur light
    "#c8986a",  # 5 face dark
    "#ecc89c",  # 6 face
    "#1e3a1a",  # 7 sweater dark
    "#2f5a28",  # 8 sweater
    "#4a7c3a",  # 9 sweater light
    "#f08a1a",  # 10 peace sign, pencil wood
    "#f4c84a",  # 11 pencil
    "#f0a0b4",  # 12 eraser, bunny's ears
    "#9c98a8",  # 13 bunny
    "#d8d4e2",  # 14 bunny light
]
P_OUT, FUR_D, FUR, FUR_L, FACE_D, FACE, SW_D, SW, SW_L, P_ORANGE, P_YELLOW, P_PINK, BUN, BUN_L = range(1, 15)
PUP_MAT = {"fur": (FUR_D, FUR, FUR_L), "face": (FACE_D, FACE, FACE), "sweater": (SW_D, SW, SW_L), "bunny": (BUN, BUN_L, BUN_L)}


def puppet_head(c, hx, hy, s=1.0, grin=1):
    """The head, facing out, centred on (hx, hy): ears, fur, the face, eyes and grin."""
    S = lambda v: v * s
    for side in (-1, 1):
        c.ellipse(hx + side * S(9), hy + S(1), S(3.5), S(3.5), "fur")
    c.ellipse(hx, hy, S(9), S(8.5), "fur")
    c.ellipse(hx + S(0.5), hy + S(2.5), S(6.5), S(5), "face")
    c.shade()
    for side in (-1, 1):
        c.px(hx + side * S(9.5), hy + S(1), FACE_D)
    # Two dot eyes, wide apart.
    for ex in (hx - S(3), hx + S(3.5)):
        r = max(0, round(S(0.6)))
        ImageDraw.Draw(c.img).rectangle([ex - r, hy + S(0.5) - r, ex + r, hy + S(1.5) + r], fill=P_OUT)
    # The grin: a long thick curve across the lower face.
    y = hy + S(4.5)
    left, right = hx - S(5.5), hx + S(6)
    width = max(1, round(S(1.2)))
    ImageDraw.Draw(c.img).arc([left, y - S(3) * grin, right, y + S(2)], 15, 165, fill=P_OUT, width=width)
    # The ₿ on its brow, above the left eye.
    bx, by = hx - S(5.5), hy - S(3.5)
    mark = ["1110", "1011", "1110", "1011", "1110"] if s < 1.1 else ["11110", "11011", "11110", "11011", "11011", "11110"]
    for j, row in enumerate(mark):
        for i, ch in enumerate(row):
            if ch == "1":
                c.px(bx + i, by + j, P_OUT)
    c.px(bx + 1, by - 1, P_OUT)
    c.px(bx + 1, by + len(mark), P_OUT)


def bunny(c, x, y):
    """A little grey bunny hand puppet, its head at (x, y)."""
    c.limb([(x - 2, y - 3), (x - 2, y - 8)], 2, "bunny")
    c.limb([(x + 1, y - 3), (x + 2, y - 8)], 2, "bunny")
    c.ellipse(x, y, 3, 2.5, "bunny")
    c.shade()
    c.px(x - 2, y - 6, P_PINK)
    c.px(x + 2, y - 6, P_PINK)
    c.px(x - 1, y, P_OUT)
    c.px(x + 1, y, P_OUT)


def pencil(c, x, y, up):
    """A pencil held in a fist at (x, y), its point up or forward."""
    tip = (x + 3, y - 8) if up else (x + 6, y - 4)
    c.line([(x, y + 2), tip], P_YELLOW, width=2)
    c.px(x, y + 3, P_PINK)
    c.px(x - 1, y + 3, P_PINK)
    c.px(tip[0], tip[1], P_ORANGE)
    c.px(tip[0], tip[1] - (1 if up else 0), P_OUT)


def puppet(cheer=False, bob=0, grin=1):
    c = Canvas(32, 32, PUP_MAT, P_OUT)
    # The sweater: a round-shouldered body, arms out to either side.
    c.ellipse(16, 26 + bob, 8.5, 7, "sweater")
    if cheer:
        left_hand, right_hand = (5, 12 + bob), (27, 12 + bob)
        c.limb([(10, 23 + bob), (6, 17 + bob), left_hand], 4, "sweater")
        c.limb([(22, 23 + bob), (26, 17 + bob), right_hand], 4, "sweater")
    else:
        left_hand, right_hand = (5, 24 + bob), (27, 24 + bob)
        c.limb([(10, 23 + bob), left_hand], 4, "sweater")
        c.limb([(22, 23 + bob), right_hand], 4, "sweater")
    c.ellipse(*right_hand, 2.5, 2.5, "fur")
    c.shade()
    # The peace sign on its chest.
    px_, py_ = 19, 26 + bob
    ImageDraw.Draw(c.img).ellipse([px_ - 2, py_ - 2, px_ + 2, py_ + 2], outline=P_ORANGE)
    c.px(px_, py_ - 1, P_ORANGE)
    c.px(px_, py_, P_ORANGE)
    c.px(px_ - 1, py_ + 1, P_ORANGE)
    c.px(px_ + 1, py_ + 1, P_ORANGE)
    puppet_head(c, 16, 11 + bob, grin=grin)
    bunny(c, left_hand[0], left_hand[1] - 1)
    c.outline()
    pencil(c, right_hand[0], right_hand[1] - 1, cheer)
    return palette_image(c.img, PUP_PALETTE)


def puppet_portrait():
    c = Canvas(32, 32, PUP_MAT, P_OUT)
    c.ellipse(16, 34, 13, 7, "sweater")
    c.shade()
    puppet_head(c, 16, 14, s=1.35)
    c.outline()
    ImageDraw.Draw(c.img).ellipse([22, 28, 27, 33], outline=P_ORANGE)
    return palette_image(c.img, PUP_PALETTE)


# --------------------------------------------------------------------------------------
# The mine: a stalactite, a mine cart of gold, and the blocks the wassie shoots apart.

MINE_PALETTE = [
    None,
    "#120e16",  # 1 outline
    "#2e2834",  # 2 rock dark
    "#4e4650",  # 3 rock
    "#7a6e78",  # 4 rock light
    "#5a3418",  # 5 wood dark
    "#93602e",  # 6 wood
    "#c48a4a",  # 7 wood light
    "#8a4204",  # 8 orange dark
    "#f7931a",  # 9 orange
    "#ffd040",  # 10 gold
    "#fff4b0",  # 11 glint
    "#272b44",  # 12 steel dark
    "#6a7290",  # 13 steel
    "#c05a30",  # 14 adobe
    "#e8a868",  # 15 adobe light
]
M_OUT, R_D, R, R_L, W_D, W, W_L, O_D, O, GOLD, GLINT, S_D, S, A, A_L = range(1, 16)
MINE_MAT = {"rock": (R_D, R, R_L), "wood": (W_D, W, W_L), "steel": (S_D, S, S)}


def stalactite_frames():
    frames = []
    for crack in (0, 1):
        c = Canvas(16, 32, MINE_MAT, M_OUT)
        c.poly([(1, 0), (14, 0), (11, 8), (9, 16), (8, 22), (6, 14), (4, 7)], "rock")
        c.shade()
        c.outline()
        # Drip rings and a gold fleck.
        c.px(7, 4, R_L)
        c.px(9, 9, R_D)
        c.px(5, 3, GOLD)
        if crack:
            # Cracks across the root, and grit falling off it.
            for x, y in [(3, 2), (4, 3), (5, 3), (6, 4), (10, 2), (9, 3), (8, 5)]:
                c.px(x, y, M_OUT)
            for x, y in [(2, 9), (12, 12), (4, 15)]:
                c.px(x, y, R_L)
        frames.append(palette_image(c.img, MINE_PALETTE))
    return frames


def cart_frames():
    frames = []
    for k in range(4):
        rolling = k >= 2
        c = Canvas(32, 16, MINE_MAT, M_OUT)
        # The load: gold and orange coins heaped over the rim.
        heap = [(6, 6), (11, 3), (16, 2), (21, 3), (26, 6)]
        c.poly(heap + [(26, 8), (6, 8)], "rock")
        c.poly([(4, 6), (28, 6), (26, 13), (6, 13)], "wood")
        c.shade()
        c.outline()
        # Repaint the heap gold.
        for y in range(2, 8):
            for x in range(6, 27):
                if c.img.getpixel((x, y)) in (R_D, R, R_L):
                    c.px(x, y, GOLD if (x + y) % 5 else O)
        for x, y in [(10, 4), (17, 3), (22, 5)]:
            c.px(x, y, GLINT)
        # Steel bands and wheels.
        c.line([(5, 9), (27, 9)], S)
        for wx in (9, 23):
            ImageDraw.Draw(c.img).ellipse([wx - 3, 10, wx + 3, 15], fill=S_D, outline=M_OUT)
            a = (k % 2) * 45 + (0 if not rolling else 22)
            dx, dy = round(2 * math.cos(math.radians(a))), round(2 * math.sin(math.radians(a)))
            c.px(wx + dx, 12 + dy, S)
            c.px(wx - dx, 12 - dy, S)
        if not rolling and k == 1:
            c.px(14, 2, GLINT)
        if rolling:
            # Sparks off the rails.
            for x, y in ([(3, 14), (1, 13)] if k == 2 else [(2, 15), (4, 13)]):
                c.px(x, y, GOLD)
        frames.append(palette_image(c.img, MINE_PALETTE))
    return frames


def brick_block():
    """Adobe bricks in a dark frame: three courses, the joints staggered."""
    img = Image.new("L", (16, 16), 0)
    d = ImageDraw.Draw(img)
    d.rectangle([0, 0, 15, 15], fill=M_OUT)
    d.rectangle([1, 1, 14, 14], fill=A)
    for y in (5, 10):
        d.line([(1, y), (14, y)], fill=W_D)
    for y0, xs in [(1, (7,)), (6, (4, 11)), (11, (7,))]:
        for x in xs:
            d.line([(x, y0), (x, y0 + 3)], fill=W_D)
    # Sunlit tops of each course, and a crack.
    for y in (1, 6, 11):
        d.line([(1, y), (14, y)], fill=A_L)
    for x, y in [(3, 7), (4, 8), (5, 8), (12, 2), (11, 3)]:
        img.putpixel((x, y), W_D)
    return palette_image(img, MINE_PALETTE)


def ore_block():
    """A block of the mine's rock shot through with a gold vein and an orange ₿ glint."""
    c = Canvas(16, 16, MINE_MAT, M_OUT)
    c.rect(1, 1, 14, 14, "rock")
    c.shade()
    img = c.img
    rng = random.Random(21)
    for x, y in [(3, 4), (4, 5), (5, 5), (6, 6), (7, 7), (8, 7), (9, 8), (10, 9), (11, 9), (12, 10), (4, 11), (5, 12), (11, 4)]:
        img.putpixel((x, y), GOLD)
    for x, y in [(5, 6), (8, 8), (11, 10)]:
        img.putpixel((x, y), O)
    for x, y in [(4, 4), (10, 8)]:
        img.putpixel((x, y), GLINT)
    for _ in range(6):
        x, y = rng.randrange(2, 14), rng.randrange(2, 14)
        if img.getpixel((x, y)) == R:
            img.putpixel((x, y), R_D)
    return palette_image(img, MINE_PALETTE)


# --------------------------------------------------------------------------------------
# Backgrounds.

SKY = ["#24143e", "#3a1a52", "#5a2260", "#86305e", "#b8485a", "#e0704a", "#f49a48", "#ffc66a", "#ffe6a0", "#fff4d8"]


def satoshi_sky():
    """A desert sunset: violet overhead down to gold at the horizon, a big orange sun
    with a darker ₿ cut into it, a few stars still out."""
    w, h = 256, 160
    img = Image.new("RGB", (w, h))
    px = img.load()
    bands = [(0, 0), (20, 1), (38, 2), (56, 3), (72, 4), (88, 5), (102, 6), (116, 7), (128, 8)]
    tops = dict((i, t) for t, i in bands)
    for y in range(h):
        band = max(i for top, i in bands if y >= top)
        nxt = min(band + 1, 8)
        edge = tops.get(nxt, h) if nxt != band else h
        for x in range(w):
            near = nxt != band and edge - y <= 3
            bayer = [[0, 2], [3, 1]][y % 2][x % 2]
            use = nxt if near and bayer >= edge - y else band
            px[x, y] = hexrgb(SKY[use])
    rng = random.Random(9)
    for _ in range(26):
        x, y = rng.randrange(w), rng.randrange(40)
        px[x, y] = hexrgb(SKY[9])
    # The sun, low over the mesas.
    sun = Image.new("L", (w, h), 0)
    sd = ImageDraw.Draw(sun)
    cx, cy, r = 180, 100, 26
    sd.ellipse([cx - r, cy - r, cx + r, cy + r], fill=1)
    sd.ellipse([cx - r + 3, cy - r + 3, cx + r - 3, cy + r - 3], fill=2)
    # A ₿ in the sun, from a 6x9 bitmap at three pixels a dot.
    mark = [".X.X..", "XXXXX.", ".X...X", ".X...X", ".XXXX.", ".X...X", ".X...X", "XXXXX.", ".X.X.."]
    bx, by = cx - 9, cy - 13
    for j, row in enumerate(mark):
        for i, ch in enumerate(row):
            if ch == "X":
                sd.rectangle([bx + i * 3, by + j * 3, bx + i * 3 + 2, by + j * 3 + 2], fill=3)
    sp = sun.load()
    for y in range(h):
        for x in range(w):
            v = sp[x, y]
            if v == 1:
                px[x, y] = hexrgb(SKY[8])
            elif v == 2:
                px[x, y] = hexrgb(SKY[9])
            elif v == 3 and (x - cx) ** 2 + (y - cy) ** 2 <= (r - 3) ** 2:
                px[x, y] = hexrgb(SKY[6])
    return img


MESA = [None, "#2a1426", "#4a1e2c", "#6e2a2e", "#9a3c30", "#c0602e", "#e08c4a", "#1a0c1c", "#3a6a3a", "#ffd040"]


def satoshi_mesas():
    """Mesas in two depths, flat-topped and banded, saguaros and a mine headframe in
    front. Transparent above."""
    w, h = 256, 160
    img = Image.new("L", (w, h), 0)
    d = ImageDraw.Draw(img)
    rng = random.Random(31)
    # The far row: wide mesas with sloping skirts, in muted violet-red.
    for x0, top, width in [(-20, 92, 70), (60, 104, 40), (110, 86, 60), (190, 98, 80)]:
        d.polygon([(x0 - 14, h), (x0, top), (x0 + width, top), (x0 + width + 14, h)], fill=2)
        d.line([(x0, top), (x0 + width, top)], fill=3)
        for y in range(top + 8, h, 9):
            d.line([(x0 - (y - top) // 2 + 2, y), (x0 + width + (y - top) // 2 - 2, y)], fill=3)
    # The near row: buttes, a lit rim along the top and strata.
    for x0, top, width in [(20, 120, 30), (96, 128, 24), (150, 116, 36), (226, 126, 40)]:
        d.polygon([(x0 - 8, h), (x0, top), (x0 + width, top), (x0 + width + 8, h)], fill=4)
        d.line([(x0, top), (x0 + width, top)], fill=6)
        for y in range(top + 5, h, 6):
            d.line([(x0 - (y - top) // 3, y), (x0 + width + (y - top) // 3, y)], fill=5)
    # The ground.
    d.rectangle([0, 146, w, h], fill=1)
    d.line([(0, 146), (w, 146)], fill=3)
    # Saguaros.
    for cx, base, hgt in [(64, 146, 22), (132, 146, 16), (210, 146, 26)]:
        d.rectangle([cx - 1, base - hgt, cx + 1, base], fill=7)
        d.rectangle([cx - 5, base - hgt + 8, cx - 4, base - hgt + 14], fill=7)
        d.line([(cx - 4, base - hgt + 13), (cx - 1, base - hgt + 13)], fill=7)
        d.rectangle([cx + 4, base - hgt + 5, cx + 5, base - hgt + 11], fill=7)
        d.line([(cx + 1, base - hgt + 10), (cx + 4, base - hgt + 10)], fill=7)
    # A mine headframe on a butte: the Satoshi mines, far off, with a lamp lit on top.
    hx, hb = 168, 116
    d.line([(hx - 8, hb), (hx, hb - 26)], fill=7)
    d.line([(hx + 8, hb), (hx, hb - 26)], fill=7)
    d.line([(hx - 5, hb - 8), (hx + 5, hb - 8)], fill=7)
    d.line([(hx - 3, hb - 16), (hx + 3, hb - 16)], fill=7)
    d.ellipse([hx - 3, hb - 30, hx + 3, hb - 24], outline=7)
    img.putpixel((hx, hb - 27), 9)
    # Specks of scrub on the ground.
    for _ in range(30):
        x, y = rng.randrange(w), rng.randrange(148, h)
        img.putpixel((x, y), 8 if rng.random() < 0.5 else 2)
    return palette_image(img, MESA)


CAVE = ["#0c0a10", "#16121c", "#221c2a", "#30283a", "#40364a", "#3a2412", "#5a3418", "#93602e", "#ffd040", "#f7931a", "#fff4b0"]


def mine_back():
    """The mine's rock wall: dark cobbles, timber frames every 128 px with a lantern
    hanging from each, and gold glinting in the rock."""
    w, h = 256, 160
    img = Image.new("L", (w, h), 1)
    d = ImageDraw.Draw(img)
    rng = random.Random(5)
    # Cobbles, tiling across the picture's edges so it repeats without a seam.
    for _ in range(140):
        x, y = rng.randrange(w), rng.randrange(h)
        rx, ry = rng.randrange(6, 12), rng.randrange(4, 8)
        for ox in (-w, 0, w):
            for oy in (-h, 0, h):
                d.ellipse([x + ox - rx, y + oy - ry, x + ox + rx, y + oy + ry], fill=2, outline=0)
                d.arc([x + ox - rx + 1, y + oy - ry + 1, x + ox + rx - 1, y + oy + ry - 1], 200, 290, fill=3)
    # Gold flecks.
    for _ in range(40):
        x, y = rng.randrange(w), rng.randrange(h)
        img.putpixel((x, y), 8 if rng.random() < 0.6 else 9)
    # Timber frames: two posts and a cap, every 128 px.
    for fx in (40, 168):
        for post in (fx, fx + 56):
            d.rectangle([post, 20, post + 5, h], fill=6)
            d.line([(post, 20), (post, h)], fill=7)
            d.line([(post + 5, 20), (post + 5, h)], fill=5)
        d.rectangle([fx - 4, 14, fx + 61, 21], fill=6)
        d.line([(fx - 4, 14), (fx + 61, 14)], fill=7)
        d.line([(fx - 4, 21), (fx + 61, 21)], fill=5)
        # A lantern hanging from the cap.
        lx = fx + 28
        d.line([(lx, 22), (lx, 30)], fill=5)
        d.rectangle([lx - 3, 30, lx + 3, 38], fill=5)
        d.rectangle([lx - 2, 31, lx + 2, 37], fill=9)
        d.rectangle([lx - 1, 32, lx + 1, 35], fill=10)
        # Its glow on the rock behind it.
        for ang in range(0, 360, 20):
            for r in (8, 10):
                gx, gy = lx + r * math.cos(math.radians(ang)), 34 + r * math.sin(math.radians(ang))
                if 0 <= gx < w and 0 <= gy < h and img.getpixel((int(gx), int(gy))) in (1, 2):
                    img.putpixel((int(gx), int(gy)), 4 if r == 8 else 3)
    return palette_image(img, CAVE).convert("RGB")


def main():
    SPRITES.mkdir(parents=True, exist_ok=True)
    BACKGROUNDS.mkdir(parents=True, exist_ok=True)
    frames = [palette_image(f, KING_PALETTE) for f in inscriber_frames()]
    sheet(frames, 64, 64, 8).save(SPRITES / "inscriber.png")
    inscriber_portrait().save(SPRITES / "inscriber-portrait.png")
    sheet(laser_frames(), 32, 8, 2).save(SPRITES / "inscriber-laser.png")
    sheet(sat_frames(), 16, 16, 4).save(SPRITES / "sat-glyph.png")
    pup = [puppet(False, 0), puppet(False, 1), puppet(True, -1, 2), puppet(True, 0, 2)]
    sheet(pup, 32, 32, 4).save(SPRITES / "btc-puppet.png")
    puppet_portrait().save(SPRITES / "btc-puppet-portrait.png")
    sheet(stalactite_frames(), 16, 32, 2).save(SPRITES / "stalactite.png")
    sheet(cart_frames(), 32, 16, 4).save(SPRITES / "mine-cart.png")
    brick_block().save(SPRITES / "brick-block.png")
    ore_block().save(SPRITES / "ore-block.png")
    satoshi_sky().save(BACKGROUNDS / "satoshi-sky.png")
    satoshi_mesas().save(BACKGROUNDS / "satoshi-mesas.png")
    mine_back().save(BACKGROUNDS / "satoshi-mine-back.png")
    print("wrote Satoshi Canyon's first-pass art to", SPRITES, "and", BACKGROUNDS)


if __name__ == "__main__":
    main()
