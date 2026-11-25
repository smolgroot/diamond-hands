#!/usr/bin/env python3
"""Cuts the Bad Cop bot (Ethereum City's riot-police villain) out of the author's mockup,
`.claude/mockup/bad-cop-bot-mockup.png`, into GBA-sized sheets for the studio.

The mockup draws each pose about 4x the game's size on a plum backdrop. Each pose is
brought down to a 32x32 frame by taking the commonest colour of every 4x4 block (thin
outlines and the red visor survive, where sampling one point could miss them), with its
feet on the frame's bottom row and its body centred. The thrown canister is left out of
the attack poses: the game draws it as its own sprite. All frames share one palette of at
most 15 colours plus transparency.

Writes, for `sprite_create` in the studio:

- project/generated/sprites/bad-cop.png: 8 frames a row, 32x32 each, in this order:
  idle 0-3, walk 4-9, attack 10-15, hit 16-19, defeat 20-24
- project/generated/sprites/bad-cop-shot.png: the canister, 16x16, 5 frames spinning

    python3 tools/eth-city/bad-cop-sheet.py   # needs Pillow, numpy and scipy
"""
from collections import Counter
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

ROOT = Path(__file__).resolve().parents[2]
MOCKUP = ROOT / ".claude/mockup/bad-cop-bot-mockup.png"
OUT = ROOT / "project/generated/sprites"
SCALE = 4
FRAME = 32
BACKDROP = np.array([22, 6, 38])

# (row name, top, bottom, left, right) of each band of poses in the mockup.
BANDS = [
    ("idle", 110, 235, 175, 1520),
    ("walk", 265, 400, 175, 1520),
    ("attack", 435, 580, 175, 1380),
    ("hit", 610, 770, 175, 1030),
    ("defeat", 810, 960, 175, 1030),
]
SHOTS = (660, 760, 1060, 1520)


def backdrop(px):
    return np.abs(px.astype(int) - BACKDROP).sum(-1) <= 40


def poses(img, top, bottom, left, right):
    """The poses in a band, left to right: (bbox of the whole pose, bbox of its body)."""
    mask = ~backdrop(img[top:bottom, left:right])
    joined = ndimage.binary_dilation(mask, iterations=5)
    lab, _ = ndimage.label(joined)
    found = []
    for i, s in enumerate(ndimage.find_objects(lab), 1):
        h, w = s[0].stop - s[0].start, s[1].stop - s[1].start
        if h > 60 and w > 60:  # a bot, not a spark or the canister
            found.append((s[1].start + left, s[0].start + top, s[1].stop + left, s[0].stop + top, i))
    found.sort()
    out = []
    for x0, y0, x1, y1, i in found:
        # The body is the biggest blob in the pose: the canister flies off on its own.
        sub = (lab[y0 - top : y1 - top, x0 - left : x1 - left] == i) & mask[y0 - top : y1 - top, x0 - left : x1 - left]
        sl, n = ndimage.label(ndimage.binary_dilation(sub, iterations=2))
        sizes = ndimage.sum(sub, sl, range(1, n + 1))
        big = (sl == (int(np.argmax(sizes)) + 1)) & sub
        ys, xs = np.nonzero(big)
        out.append(((x0, y0, x1, y1), (x0 + xs.min(), y0 + ys.min(), x0 + xs.max() + 1, y0 + ys.max() + 1), big, (x0, y0)))
    return out


def shrink(img, keep, x0, y0):
    """A FRAME x FRAME picture from the mockup window at (x0, y0): each pixel the commonest
    colour of its SCALE x SCALE block, transparent where the backdrop wins."""
    out = np.zeros((FRAME, FRAME, 4), np.uint8)
    for fy in range(FRAME):
        for fx in range(FRAME):
            ys, xs = y0 + fy * SCALE, x0 + fx * SCALE
            block = img[ys : ys + SCALE, xs : xs + SCALE].reshape(-1, 3)
            kept = keep[ys : ys + SCALE, xs : xs + SCALE].reshape(-1)
            colours = [tuple(c) for c, k in zip(block, kept) if k]
            if len(colours) * 2 < SCALE * SCALE:
                continue
            c = Counter(colours).most_common(1)[0][0]
            out[fy, fx] = (*c, 255)
    return out


def silhouette(img):
    """Everything that is part of a figure: the clearly coloured pixels, their holes
    filled, grown by the width of the mockup's dark outline (which is close to the
    backdrop's plum, so a colour test alone would drop it)."""
    strong = ~backdrop(img)
    closed = ndimage.binary_closing(strong, iterations=2)
    filled = ndimage.binary_fill_holes(closed)
    grown = ndimage.binary_dilation(filled, iterations=3)
    # Grown only into dark pixels: the outline, not the backdrop around it.
    dark = img.astype(int).sum(-1) < 110
    return filled | (grown & dark & ~nearly_backdrop(img))


def nearly_backdrop(px):
    return np.abs(px.astype(int) - BACKDROP).sum(-1) <= 6


def main():
    img = np.array(Image.open(MOCKUP).convert("RGB"))
    keep_all = silhouette(img)
    frames = []
    for name, top, bottom, left, right in BANDS:
        for whole, body, big, origin in poses(img, top, bottom, left, right):
            keep = np.zeros(keep_all.shape, bool)
            if name == "attack":
                # Only the body: the canister is a sprite of its own.
                ox, oy = origin
                body_only = ndimage.binary_dilation(big, iterations=4)
                keep[oy : oy + big.shape[0], ox : ox + big.shape[1]] = body_only & keep_all[oy : oy + big.shape[0], ox : ox + big.shape[1]]
            else:
                x0, y0, x1, y1 = whole
                keep[y0:y1, x0:x1] = keep_all[y0:y1, x0:x1]
            bx0, by0, bx1, by1 = body
            # Feet on the bottom row, the body's middle at column 15.
            wx = (bx0 + bx1) // 2 - 15 * SCALE - SCALE // 2
            wy = by1 - FRAME * SCALE
            frames.append(shrink(img, keep, wx, wy))
        print(name, "poses so far:", len(frames))
    assert len(frames) == 25, len(frames)

    # The canister: five spins along the parabola panel.
    top, bottom, left, right = SHOTS
    mask = ~backdrop(img[top:bottom, left:right])
    lab, _ = ndimage.label(ndimage.binary_dilation(mask, iterations=3))
    shots = []
    for s in sorted(ndimage.find_objects(lab), key=lambda s: s[1].start):
        y0, y1, x0, x1 = s[0].start + top, s[0].stop + top, s[1].start + left, s[1].stop + left
        if y1 - y0 < 20:
            continue
        keep = np.zeros(keep_all.shape, bool)
        keep[y0:y1, x0:x1] = keep_all[y0:y1, x0:x1]
        cx, cy = (x0 + x1) // 2, (y0 + y1) // 2
        f = shrink(img, keep, cx - 8 * SCALE, cy - 8 * SCALE)
        shots.append(f[:16, :16])
    assert len(shots) == 5, len(shots)

    # One shared palette of 15 colours for each sheet. The visor and the siren's red
    # are the bot's tells: they keep slots of their own rather than being averaged away.
    ACCENTS = [(255, 46, 120), (255, 120, 170)]

    def accent(c):
        r, g, b = c
        if r > 150 and g < 110 and r > b:
            return ACCENTS[0] if r < 235 or g < 60 else ACCENTS[1]
        return None

    def sheet(pics, w, h, cols):
        rows = (len(pics) + cols - 1) // cols
        canvas = Image.new("RGBA", (cols * w, rows * h), (0, 0, 0, 0))
        for i, p in enumerate(pics):
            canvas.paste(Image.fromarray(p, "RGBA"), ((i % cols) * w, (i // cols) * h))
        px = np.array(canvas)
        alpha = px[..., 3] > 0
        hot = np.zeros(alpha.shape, int) - 1
        for y, x in zip(*np.nonzero(alpha)):
            a = accent(tuple(int(v) for v in px[y, x, :3]))
            if a is not None:
                hot[y, x] = ACCENTS.index(a)
        base = Image.fromarray(px[..., :3], "RGB")
        q = np.array(base.quantize(colors=15 - len(ACCENTS), method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE).convert("RGB"))
        for i, a in enumerate(ACCENTS):
            q[hot == i] = a
        out = np.zeros((*q.shape[:2], 4), np.uint8)
        out[..., :3] = q
        out[..., 3] = np.where(alpha, 255, 0)
        return Image.fromarray(out, "RGBA")

    OUT.mkdir(parents=True, exist_ok=True)
    sheet(frames, FRAME, FRAME, 8).save(OUT / "bad-cop.png")
    sheet(shots, 16, 16, 5).save(OUT / "bad-cop-shot.png")
    print("wrote", OUT / "bad-cop.png", "and", OUT / "bad-cop-shot.png")


if __name__ == "__main__":
    main()
