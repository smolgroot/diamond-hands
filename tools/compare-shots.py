#!/usr/bin/env python3
"""Compares two directories of harness screenshots (PPM) pixel for pixel.

    DIAMOND_HANDS_SHOTS=<dir> cargo test -p diamond_hands_harness --release   # once per ROM
    python3 tools/compare-shots.py <original dir> <studio dir> [diff dir]

Prints every screenshot that differs, with how many pixels, and writes a side-by-side
picture of each (original, studio, differing pixels in red) when a diff dir is given.
Exits 1 if any differ or any is missing.
"""
import sys
from pathlib import Path


def read(path):
    data = path.read_bytes()
    header, rest = data.split(b"\n255\n", 1)
    w, h = map(int, header.split()[1:3])
    return w, h, rest


a, b = Path(sys.argv[1]), Path(sys.argv[2])
out = Path(sys.argv[3]) if len(sys.argv) > 3 else None
names = sorted(p.name for p in a.glob("*.ppm"))
missing = sorted(set(names) ^ {p.name for p in b.glob("*.ppm")})
bad = 0
for name in names:
    if name in missing:
        continue
    (w, h, x), (_, _, y) = read(a / name), read(b / name)
    diff = [i // 3 for i in range(0, len(x), 3) if x[i:i + 3] != y[i:i + 3]]
    if diff:
        bad += 1
        print(f"{name}: {len(diff)} pixels differ")
        if out:
            from PIL import Image
            out.mkdir(parents=True, exist_ok=True)
            ia, ib = Image.frombytes("RGB", (w, h), x), Image.frombytes("RGB", (w, h), y)
            mark = ia.copy()
            for i in diff:
                mark.putpixel((i % w, i // w), (255, 0, 0))
            sheet = Image.new("RGB", (w * 3 + 8, h))
            for n, im in enumerate((ia, ib, mark)):
                sheet.paste(im, (n * (w + 4), 0))
            sheet.save(out / name.replace(".ppm", ".png"))
for name in missing:
    print(f"{name}: only in one directory")
print(f"{len(names) - len(missing) - bad} of {len(names)} screenshots identical, {bad} differ, {len(missing)} unmatched")
sys.exit(1 if bad or missing else 0)
