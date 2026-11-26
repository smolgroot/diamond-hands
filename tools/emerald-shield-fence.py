#!/usr/bin/env python3
"""First-pass pixel art for the plasma shield's emerald and HUD, and the zoo's fence posts.

Drawn by script until the author redraws them (the author's standing choice for new
first-pass art). Everything goes to project/generated/, never to project/assets/.
A sheet redrawn at the same size and frame layout drops straight in. Same script, same
pixels. Pillow only.

- sprites/emerald-gem.png: the green emerald that recharges the plasma shield. It is the
  diamond pickup (project/assets/sprites/pickups.png, frames 0-1) recoloured in the
  plasma shield's greens, so the two read as one family. 16x16, 2 frames:
  0 still, 1 glint.
- sprites/shield-icons.png: the plasma shield's charge in the HUD, one icon for every
  two charges. 8x8, 3 frames: 0 whole, 1 half (left half lit), 2 spent.
- sprites/zoo-fence.png: an electric fence post in the MagicEden zoo, 8x32, its feet on
  the bottom row and the post in the bottom 24 rows: 0 dead (lamps dark), 1 live (lamps
  lit). The game draws the author's lightning (fence-8x8) around it while it is live.

Run from the repository root: python3 tools/emerald-shield-fence.py
"""

from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
SPRITES = ROOT / "project" / "generated" / "sprites"
PICKUPS = ROOT / "project" / "assets" / "sprites" / "pickups.png"


def rgb(hexcode):
    return tuple(int(hexcode[i : i + 2], 16) for i in (1, 3, 5)) + (255,)


# The plasma shield's greens (generated/sprites/plasma-shield.png).
G_OUTLINE = rgb("#0b2e22")
G_SHADE = rgb("#147a48")
G_MAIN = rgb("#2fcf78")
G_LIGHT = rgb("#8df5b9")
G_WHITE = rgb("#f0fff6")
G_SPENT = rgb("#1d553a")

# The diamond's blues, and the green each becomes.
DIAMOND_TO_EMERALD = {
    rgb("#101030"): G_OUTLINE,
    rgb("#3070f8"): G_MAIN,
    rgb("#70e0f8"): G_LIGHT,
    rgb("#f8f8f8"): G_WHITE,
    rgb("#401868"): G_SHADE,
}


def emerald():
    sheet = Image.open(PICKUPS).convert("RGBA").crop((0, 0, 32, 16))
    out = Image.new("RGBA", sheet.size, (0, 0, 0, 0))
    for y in range(sheet.height):
        for x in range(sheet.width):
            px = sheet.getpixel((x, y))
            if px[3] == 0:
                continue
            if px not in DIAMOND_TO_EMERALD:
                raise SystemExit(f"pickups.png: colour {px} at {x},{y} has no green")
            out.putpixel((x, y), DIAMOND_TO_EMERALD[px])
    return out


# A heater shield, 8x8. O outline, H highlight, L lit, S shade.
SHIELD = [
    ".OOOOOO.",
    "OHLLLLSO",
    "OLLLLLSO",
    "OLLLLLSO",
    "OLLLLSSO",
    ".OLLLSO.",
    "..OLSO..",
    "...OO...",
]


def shield_icons():
    out = Image.new("RGBA", (24, 8), (0, 0, 0, 0))
    lit = {"O": G_OUTLINE, "H": G_WHITE, "L": G_MAIN, "S": G_SHADE}
    for frame in range(3):
        for y, row in enumerate(SHIELD):
            for x, c in enumerate(row):
                if c == ".":
                    continue
                spent = frame == 2 or (frame == 1 and x >= 4)
                colour = G_OUTLINE if c == "O" else (G_SPENT if spent else lit[c])
                out.putpixel((frame * 8 + x, y), colour)
    return out


# A fence post: an insulator on top, a pole, and the emitter housing at its foot.
# O outline, d dark steel, s steel, l light steel, M lamp.
POST = [
    "........",
    "........",
    "........",
    "........",
    "........",
    "........",
    "........",
    "........",
    "..OOOO..",
    ".OllssO.",
    ".OsMMdO.",
    "..OOOO..",
] + [
    "...OdO..",
] * 14 + [
    ".OOOOOO.",
    "OllllsdO",
    "OsMMMMdO",
    "OssssddO",
    "OddddddO",
    "OOOOOOOO",
]

STEEL = {"O": rgb("#101030"), "d": rgb("#303868"), "s": rgb("#6078b8"), "l": rgb("#c0d8f8")}
LAMP_OFF = rgb("#401868")
LAMP_ON = rgb("#70e0f8")


def fence_post():
    assert len(POST) == 32 and all(len(r) == 8 for r in POST), "the post is 8x32"
    out = Image.new("RGBA", (16, 32), (0, 0, 0, 0))
    for frame, lamp in enumerate((LAMP_OFF, LAMP_ON)):
        for y, row in enumerate(POST):
            for x, c in enumerate(row):
                if c != ".":
                    out.putpixel((frame * 8 + x, y), lamp if c == "M" else STEEL[c])
    return out


def main():
    SPRITES.mkdir(parents=True, exist_ok=True)
    emerald().save(SPRITES / "emerald-gem.png")
    shield_icons().save(SPRITES / "shield-icons.png")
    fence_post().save(SPRITES / "zoo-fence.png")


if __name__ == "__main__":
    main()
