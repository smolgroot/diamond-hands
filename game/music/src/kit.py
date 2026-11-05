"""Shared instruments, note helpers and the hero motif for the Diamond Hands soundtrack.

Every instrument is synthesized here, so the soundtrack has no third-party samples.
"""

import math
import random
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / ".claude/skills/gba-music/scripts"))

from xm_writer import Sample, Song, synth  # noqa: E402

MUSIC = ROOT / "game/music"
NAMES = ["C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-"]
ROWS_PER_BAR = 16


def semis(note):
    """'A-4' -> semitone number (C-0 = 0)."""
    return int(note[2]) * 12 + NAMES.index(note[:2])


def name(n):
    return f"{NAMES[n % 12]}{n // 12}"


def up(note, k):
    """Transposes a note name by k semitones."""
    return name(semis(note) + k)


# The hero motif: 1, b3, 5, 4 in a 3-3-2 rhythm, then a held note. Cues restate it in
# their own key, rhythm and mood. (row, interval from the tonic, length in rows)
MOTIF = [(0, 0, 3), (3, 3, 3), (6, 7, 2), (8, 5, 8)]


# --- Instruments -------------------------------------------------------------------

def _loop(fn, cycles, frames_per_cycle=32):
    """A loop of `cycles` periods of fn(t), tuned so that C-4 plays at C-4."""
    n = cycles * frames_per_cycle
    data = [fn(i / frames_per_cycle) for i in range(n)]
    peak = max(abs(x) for x in data) or 1
    return Sample([x / peak * 0.95 for x in data], 261.6256 * n / cycles, "C-4", loop_start=0)


def _saw(t):
    return 1 - 2 * (t % 1)


def reese():
    """Two saws 96 and 97 cycles to the loop: a slow, wide detuned beat."""
    n = 96 * 32
    data = [0.5 * _saw(96 * i / n) + 0.5 * _saw(97 * i / n) for i in range(n)]
    return Sample(data, 261.6256 * n / 96.5, "C-4", loop_start=0)


def coin():
    """Bell partials at 1x and 2.75x the fundamental: a bright crypto 'ting'."""
    return _loop(lambda t: math.sin(2 * math.pi * t) + 0.6 * math.sin(2 * math.pi * 2.75 * t),
                 8)


def metal():
    """Inharmonic partials (1, 2.31, 3.69): the bears' machinery clanging."""
    return _loop(lambda t: math.sin(2 * math.pi * t) + 0.7 * math.sin(2 * math.pi * 2.3125 * t)
                 + 0.5 * math.copysign(1, math.sin(2 * math.pi * 3.6875 * t)), 16)


def riser():
    """Looped white noise; pitch it up with 1xx and swell it with Axy."""
    rng = random.Random(7)
    return Sample([rng.uniform(-1, 1) for _ in range(2048)], 16726, "C-5", loop_start=0)


def ohat():
    return synth.hat(length=0.2, seed=5)


def _lowpass(sample, a=0.35):
    """One-pole low-pass: takes the fizz off a noise drum so it sits in a lofi mix."""
    out, y = [], 0.0
    for x in sample.frames:
        y += a * (x - y)
        out.append(y)
    peak = max(abs(v) for v in out) or 1
    sample.frames = [v / peak * 0.9 for v in out]
    return sample


def lofi_kick():
    return synth.kick(length=0.22, start_hz=95, end_hz=48)


def lofi_snare():
    return _lowpass(synth.snare(length=0.2, seed=4), 0.3)


def lofi_hat():
    return _lowpass(synth.hat(length=0.06, seed=9), 0.25)


def keys():
    """Electric-piano tine: a sine with a soft 2nd and a quiet bell partial."""
    return _loop(lambda t: math.sin(2 * math.pi * t) + 0.15 * math.sin(4 * math.pi * t), 1, 64)


def mellow():
    """Soft triangle-ish lead: warm and dull, no square-wave edge."""
    return _loop(lambda t: math.sin(2 * math.pi * t) + 0.08 * math.sin(6 * math.pi * t), 1, 64)


INSTRUMENTS = {
    # name: (sample factory, keyword args for Song.instrument)
    "kick": (synth.kick, dict(volume=64)),
    "snare": (synth.snare, dict(volume=56)),
    "hat": (synth.hat, dict(volume=48)),
    "ohat": (ohat, dict(volume=40)),
    "crash": (lambda: synth.noise(length=1.0, decay=3.5), dict(volume=40)),
    "riser": (riser, dict(volume=32)),
    "bass": (synth.saw, dict(volume=56, envelope=[(0, 64), (4, 44), (12, 30), (40, 20)])),
    "sub": (synth.triangle, dict(volume=64, envelope=[(0, 64), (20, 50)], sustain=1)),
    "reese": (reese, dict(volume=56, envelope=[(0, 64), (6, 52)], sustain=1)),
    "lead": (lambda: synth.square(0.25), dict(volume=48, envelope=[(0, 64), (8, 50)],
                                              sustain=1)),
    "saw": (synth.saw, dict(volume=44, envelope=[(0, 64), (10, 46)], sustain=1)),
    "pluck": (lambda: synth.square(0.125), dict(volume=48,
                                                envelope=[(0, 64), (3, 36), (14, 0)])),
    "pad": (synth.triangle, dict(volume=48, envelope=[(0, 0), (24, 56), (48, 44)],
                                 sustain=2)),
    "coin": (coin, dict(volume=48, envelope=[(0, 64), (6, 30), (30, 0)])),
    "metal": (metal, dict(volume=52, envelope=[(0, 64), (8, 28), (36, 0)])),
    "lkick": (lofi_kick, dict(volume=60)),
    "lsnare": (lofi_snare, dict(volume=50)),
    "lhat": (lofi_hat, dict(volume=40)),
    "lbass": (synth.sine, dict(volume=60, envelope=[(0, 64), (6, 50), (30, 36), (60, 0)])),
    "keys": (keys, dict(volume=52, envelope=[(0, 64), (6, 44), (30, 26), (80, 0)])),
    "soft": (mellow, dict(volume=48, envelope=[(0, 40), (5, 64), (16, 52)], sustain=2)),
    "haze": (synth.sine, dict(volume=40, envelope=[(0, 0), (30, 56), (70, 44)], sustain=2)),
    "siren": (lambda: synth.square(0.5), dict(volume=36, envelope=[(0, 64), (1, 64)],
                                                          sustain=1)),
}


def add_instruments(song, *names):
    for n in names:
        factory, kwargs = INSTRUMENTS[n]
        song.instrument(n, factory(), **kwargs)


# --- Writing helpers ---------------------------------------------------------------

def melody(p, ch, notes, ins, vol, start=0, vibrato="424", end=None):
    """Places (row, note, length) notes; adds vibrato on long notes and key-offs on rests.

    A note runs until the next one starts or `length` rows pass, whichever is first.
    """
    for i, (row, note, length) in enumerate(notes):
        r = start + row
        if r >= p.rows:
            break
        p.put(r, ch, f"{note} {ins} v{vol}")
        nxt = notes[i + 1][0] + start if i + 1 < len(notes) else end
        off = r + length
        if length >= 6 and vibrato:
            # agb clears vibrato every row, so it goes on each row of the held note.
            for rr in range(r + 2, min(off, nxt or p.rows, p.rows)):
                p.put(rr, ch, f"--- {vibrato}")
        if off < p.rows and (nxt is None or off < nxt):
            p.put(off, ch, "===")


def motif(key_note, scale_fix=None, rhythm=1, octave=0):
    """The hero motif as (row, note, length) in a key. scale_fix maps intervals to new
    ones (e.g. {7: 6} for a flattened, menacing fifth)."""
    fix = scale_fix or {}
    return [(row * rhythm, up(key_note, fix.get(iv, iv) + 12 * octave), length * rhythm)
            for row, iv, length in MOTIF]


def echo(p, src, dst, delay=3, vol=16, pan="8C0"):
    """Copies a channel onto another, `delay` rows later and quieter, panned aside."""
    first = True
    for row in range(p.rows - delay):
        note, ins, _, eff, param = p.cells[row][src]
        if not (note or eff == 4):
            continue
        cell = p.cells[row + delay][dst]
        cell[0], cell[1] = note, ins
        if note and note != 97:
            cell[2] = 0x10 + vol
        if first:
            cell[3], cell[4] = 0x8, int(pan[1:], 16)
            first = False
        elif eff == 4:
            cell[3], cell[4] = eff, param


def arp(p, ch, chords, ins="pluck", vol=22, figure=(0, 1, 2, 3, 2, 1, 0, 2), step=2,
        bars=None, octave=4):
    """Broken-chord figure, one note every `step` rows, over one chord per bar.

    chords: (root note, intervals) per bar, e.g. ("A-", (0, 3, 7, 12)).
    """
    for bar, (root, ivs) in enumerate(chords):
        if bars is not None and bar not in bars:
            continue
        base = semis(f"{root}{octave}")
        for k, r in enumerate(range(0, ROWS_PER_BAR, step)):
            iv = ivs[figure[k % len(figure)] % len(ivs)]
            row = bar * ROWS_PER_BAR + r
            if row < p.rows:
                p.put(row, ch, f"{name(base + iv)} {ins} v{vol}")


# Global volume every song starts at: headroom for the blaster effects, which play on
# the DMG channels at full volume on top of the music.
GLOBAL_VOLUME = 0x30


def save(song):
    first = song.order[0].cells[0]
    free = next(c for c in first if c[3] == 0 and c[4] == 0)
    free[3], free[4] = 0x10, GLOBAL_VOLUME  # Gxx
    path = MUSIC / f"{song.name}.xm"
    song.save(path)
    print(f"wrote {path.relative_to(ROOT)}")
