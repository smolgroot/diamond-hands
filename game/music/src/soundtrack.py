"""Writes every Diamond Hands music cue to game/music/*.xm. Run from anywhere:

    python3 game/music/src/soundtrack.py

All cues run at 150 BPM (one tracker tick per video frame) and set the tempo with the
speed. Channel layout, kept the same across cues where possible:
    0 kick/snare   1 hats, risers, crashes   2 bass   3 lead   4 arps/stabs   5 pad/echo
"""

from kit import (ROWS_PER_BAR, Song, add_instruments, arp, echo, melody, motif, save,
                 up)

BAR = ROWS_PER_BAR
MINOR = (0, 3, 7, 12)
MAJOR = (0, 4, 7, 12)


def drums(p, bars=range(4), kicks=(0, 6, 10), snares=(4, 12), hats=2, open_hats=False,
          hat_vol=22, fill=None):
    """A bar-by-bar beat. `fill` is the bar that ends in a snare roll instead."""
    for bar in bars:
        b = bar * BAR
        for r in kicks:
            p.put(b + r, 0, "C-5 kick v60")
        for r in snares:
            p.put(b + r, 0, "C-5 snare v46")
        if hats:
            for r in range(0, BAR, hats):
                if open_hats and r % 4 == 2:
                    p.put(b + r, 1, f"C-5 ohat v{hat_vol}")
                else:
                    p.put(b + r, 1, f"C-5 hat v{hat_vol + (6 if r % 4 == 0 else 0)}")
        if bar == fill:
            for i, r in enumerate(range(8, BAR)):
                p.put(b + r, 0, f"C-5 snare v{24 + 3 * i}")


def pump_bass(p, roots, octave=2, vol=44, bars=None):
    """Octave-pumping eighths with sixteenth ghosts: the level's engine."""
    rows = [(0, 0, vol), (2, 0, vol - 8), (3, 12, vol - 16), (4, 0, vol), (6, 12, vol - 6),
            (8, 0, vol), (10, 0, vol - 8), (11, 12, vol - 16), (12, 0, vol), (14, 12, vol - 6)]
    for bar, root in enumerate(roots):
        if bars is not None and bar not in bars:
            continue
        for r, iv, v in rows:
            p.put(bar * BAR + r, 2, f"{up(f'{root}{octave}', iv)} bass v{v}")


def pad(p, chords, vol=20, ch=5, octave=3):
    """One swelling chord per bar on a single voice, using the 0xy arpeggio."""
    for bar, (root, ivs) in enumerate(chords):
        fx = f"0{ivs[1]:X}{ivs[2]:X}"
        p.put(bar * BAR, ch, f"{root}{octave} pad v{vol} {fx}")
        for r in range(1, BAR):
            p.put(bar * BAR + r, ch, f"--- {fx}")


def riser(p, start, end, ch=1):
    """Noise climbing in pitch and volume up to `end`, cut on the downbeat after."""
    p.put(start, ch, "C-4 riser v4")
    for r in range(start + 1, end):
        p.put(r, ch, f"--- v{4 + (r - start) * 40 // (end - start)} 103")
    if end < p.rows:
        p.put(end, ch, "===")


def coins(p, rows, notes, ch=4, vol=26):
    for r, n in zip(rows, notes):
        p.put(r, ch, f"{n} coin v{vol}")


# --- Lofi cyberpunk: title and level. A minor, ~80 BPM, swung. ---------------------
# Speed 11 with F0C/F0A on alternate rows: 4 rows a beat, ~81 BPM, and a lazy shuffle.
# Channels: 0 kick/snare, 1 hats (carries the swing), 2 sine bass, 3 lead,
#           4 and 5 the 3rd and 7th of each chord (a shell voicing, like a Rhodes comp).

# Chords as (bass root, 3rd, 7th), all low and warm: Am7, Fmaj7, Dm7, E7.
LOFI_CHORDS = [("A-", "C-4", "G-4"), ("F-", "A-3", "E-4"), ("D-", "F-3", "C-4"),
               ("E-", "G#3", "D-4")]
LOFI_B = [("D-", "F-3", "C-4"), ("F-", "A-3", "E-4"), ("G-", "B-3", "F-4"),
          ("E-", "G#3", "D-4")]
LOFI_NOTES = {"A-": "A", "F-": "F"}


def lmelody(p, ch, notes, ins, vol, **kw):
    """The lead, an octave below how it is written here: warm, not piercing."""
    melody(p, ch, [(r, up(n, -12), ln) for r, n, ln in notes], ins, vol, **kw)


def swing(p, ch=1):
    for r in range(p.rows):
        cell = p.cells[r][ch]
        cell[3], cell[4] = 0xF, 0x0C if r % 2 == 0 else 0x0A


def lofi_beat(p, bars=range(4), hats=True, ghost=True, fill_bar=None):
    for bar in bars:
        b = bar * BAR
        p.put(b, 0, "C-5 lkick v56")
        p.put(b + 7, 0, "C-5 lkick v34")
        p.put(b + 10, 0, "C-5 lkick v46")
        p.put(b + 4, 0, "C-5 lsnare v38")
        p.put(b + 12, 0, "C-5 lsnare v40")
        if ghost:
            p.put(b + 15, 0, "C-5 lsnare v14")
        if hats:
            for r in range(0, BAR, 2):
                p.put(b + r, 1, f"C-5 lhat v{14 if r % 4 == 0 else 8}")
        if bar == fill_bar:
            for i, r in enumerate((12, 13, 14, 15)):
                p.put(b + r, 0, f"C-5 lsnare v{22 + 6 * i}")


def lofi_bass(p, chords, vol=46):
    for bar, (root, _, _) in enumerate(chords):
        b = bar * BAR
        p.put(b, 2, f"{root}2 lbass v{vol}")
        p.put(b + 6, 2, f"{root}2 lbass v{vol - 14}")
        p.put(b + 10, 2, f"{up(root + '2', 7)} lbass v{vol - 8}")
        p.put(b + 14, 2, f"{root}3 lbass v{vol - 20}")


def lofi_keys(p, chords, vol=24, offbeat=True):
    """The 3rd and 7th on their own voices; played on the 1 and the 'and of 2'."""
    for bar, (_, third, seventh) in enumerate(chords):
        b = bar * BAR
        for r in ((0, 6) if offbeat else (0,)):
            p.put(b + r, 4, f"{third} keys v{vol}")
            p.put(b + r, 5, f"{seventh} keys v{vol - 3}")


def lofi_haze(p, chords, vol=14):
    """A slow pad swelling on each chord's seventh."""
    for bar, (_, _, seventh) in enumerate(chords):
        p.put(bar * BAR, 5, f"{seventh} haze v{vol}")


def lofi_title():
    song = Song("title", channels=6, speed=11, bpm=150)
    add_instruments(song, "lkick", "lsnare", "lhat", "lbass", "keys", "soft", "haze")

    p0 = song.pattern(64)  # just the keys, a slow bass and the hats: rain on neon
    lofi_keys(p0, LOFI_CHORDS, vol=26, offbeat=False)
    lofi_bass(p0, LOFI_CHORDS, vol=40)
    for bar in range(4):
        for r in range(0, BAR, 4):
            p0.put(bar * BAR + r, 1, f"C-5 lhat v{8 if r else 12}")
    swing(p0)

    p1 = song.pattern(64)  # the motif, slow and plain, over a soft beat
    lofi_beat(p1, hats=False, ghost=False)
    for bar in range(4):
        p1.put(bar * BAR, 0, "C-5 lkick v50")
    lofi_bass(p1, LOFI_CHORDS)
    lofi_keys(p1, LOFI_CHORDS)
    lmelody(p1, 3, [(0, "A-4", 6), (6, "C-5", 4), (10, "E-5", 6), (16, "D-5", 14),
                   (32, "A-4", 6), (38, "C-5", 4), (42, "D-5", 6), (48, "C-5", 14)],
           "soft", 34, vibrato="412")
    swing(p1)

    p2 = song.pattern(64)  # full groove, the motif answers itself a step lower
    lofi_beat(p2, fill_bar=3)
    lofi_bass(p2, LOFI_B)
    lofi_keys(p2, LOFI_B)
    lmelody(p2, 3, [(0, "F-4", 6), (6, "A-4", 4), (10, "C-5", 6), (16, "B-4", 14),
                   (32, "E-4", 6), (38, "G#4", 4), (42, "B-4", 6), (48, "A-4", 14)],
           "soft", 34, vibrato="412")
    swing(p2)

    song.order = [p0, p1, p2, p1]
    song.restart = 1
    save(song)


def lofi_level():
    song = Song("badlands", channels=6, speed=11, bpm=150)
    add_instruments(song, "lkick", "lsnare", "lhat", "lbass", "keys", "soft", "haze",
                    "coin")

    intro = song.pattern(64)  # keys and a heartbeat kick
    lofi_keys(intro, LOFI_CHORDS, vol=26, offbeat=False)
    lofi_bass(intro, LOFI_CHORDS, vol=40)
    for bar in range(4):
        intro.put(bar * BAR, 0, "C-5 lkick v46")
        for r in range(2, BAR, 4):
            intro.put(bar * BAR + r, 1, "C-5 lhat v10")
    swing(intro)

    groove = song.pattern(64)  # the beat drops in, no lead
    lofi_beat(groove, fill_bar=3)
    lofi_bass(groove, LOFI_CHORDS)
    lofi_keys(groove, LOFI_CHORDS)
    swing(groove)

    hook = song.pattern(64)  # the hero motif, low and cool
    lofi_beat(hook, fill_bar=3)
    lofi_bass(hook, LOFI_CHORDS)
    lofi_keys(hook, LOFI_CHORDS)
    lmelody(hook, 3, [(0, "A-4", 6), (6, "C-5", 4), (10, "E-5", 6), (16, "D-5", 10),
                     (28, "C-5", 4), (32, "A-4", 6), (38, "C-5", 4), (42, "D-5", 4),
                     (46, "C-5", 2), (48, "A-4", 14)], "soft", 36, vibrato="412")
    swing(hook)

    answer = song.pattern(64)  # a different corner of the city: D minor turnaround
    lofi_beat(answer, fill_bar=3)
    lofi_bass(answer, LOFI_B)
    lofi_keys(answer, LOFI_B)
    lmelody(answer, 3, [(0, "D-5", 8), (8, "F-5", 4), (12, "E-5", 4), (16, "C-5", 10),
                       (28, "A-4", 4), (32, "B-4", 6), (38, "D-5", 4), (42, "F-5", 6),
                       (48, "E-5", 14)], "soft", 36, vibrato="412")
    swing(answer)

    drift = song.pattern(64)  # breakdown: no drums, haze and falling coin chimes
    lofi_keys(drift, LOFI_CHORDS, vol=24, offbeat=False)
    lofi_bass(drift, LOFI_CHORDS, vol=36)
    lofi_haze(drift, LOFI_CHORDS)
    for r, n in zip((2, 10, 20, 26, 34, 42, 52, 58),
                    ("E-5", "A-5", "G-5", "E-5", "D-5", "G-5", "E-5", "C-5")):
        drift.put(r, 3, f"{up(n, -12)} coin v14")
    for bar in range(4):
        drift.put(bar * BAR, 0, "C-5 lkick v34")
    swing(drift)

    song.order = [intro, groove, hook, answer, hook, drift, answer]
    song.restart = 1
    save(song)


# --- Boss: BERA 3000. E Phrygian, ~179 BPM, industrial. -----------------------------

def boss():
    song = Song("boss", channels=6, speed=5, bpm=150)
    add_instruments(song, "kick", "snare", "hat", "crash", "reese", "saw", "metal", "siren",
                    "riser")

    # Two patterns of threat for the dialogue: heartbeat, drone, clangs and a siren.
    wake = song.pattern(64)
    wake.put(0, 2, "E-1 reese v12")
    for r in range(1, 64, 2):
        wake.put(r, 2, f"--- v{12 + r * 28 // 64}")
    for bar in range(4):
        wake.put(bar * BAR, 0, "C-5 kick v48")
        wake.put(bar * BAR + 3, 0, "C-5 kick v30")
        wake.put(bar * BAR + 8, 4, f"{['E-3', 'F-3', 'E-3', 'A#2'][bar]} metal v40")
    wake.put(16, 3, "E-4 siren v16")
    for r in range(17, 64):
        wake.put(r, 3, "--- 103" if (r // 8) % 2 == 0 else "--- 203")
    wake.put(63, 3, "===")

    alarm = song.pattern(64)
    alarm.put(0, 2, "E-1 reese v44")
    for bar in range(4):
        alarm.put(bar * BAR, 0, "C-5 kick v54")
        alarm.put(bar * BAR + 3, 0, "C-5 kick v34")
        alarm.put(bar * BAR + 8, 0, "C-5 snare v30")
        alarm.put(bar * BAR + 8, 4, f"{['E-3', 'F-3', 'G-3', 'A#2'][bar]} metal v44")
        for r in range(0, BAR, 2):
            alarm.put(bar * BAR + r, 1, f"C-5 hat v{10 + bar * 4}")
    alarm.put(0, 3, "E-5 siren v18")
    for r in range(1, 48):
        alarm.put(r, 3, "--- 106" if (r // 4) % 2 == 0 else "--- 206")
    alarm.put(48, 3, "===")
    for i, r in enumerate(range(48, 64)):
        alarm.put(r, 0, f"C-5 snare v{26 + i * 2}")

    # The motif, warped: flat fifth, and the Phrygian flat second in the bass.
    warped = motif("E-4", scale_fix={7: 6})

    def chug(p, roots_, vol=46):
        figure = [(0, 0), (2, 0), (3, 1), (4, 0), (6, 0), (7, 3), (8, 0), (10, 0), (11, 1),
                  (12, 0), (14, 3)]
        for bar, root in enumerate(roots_):
            for r, iv in figure:
                p.put(bar * BAR + r, 2, f"{up(root, iv)} reese v{vol if r % 4 == 0 else vol - 10}")

    def fight_drums(p, fill=3):
        for bar in range(4):
            b = bar * BAR
            for r in (0, 3, 6, 8, 11, 14):
                p.put(b + r, 0, "C-5 kick v58")
            for r in (4, 12):
                p.put(b + r, 0, "C-5 snare v48")
            for r in range(BAR):
                p.put(b + r, 1, f"C-5 hat v{22 if r % 2 == 0 else 12}")
            if bar == fill:
                for i, r in enumerate(range(8, BAR)):
                    p.put(b + r, 0, f"C-5 snare v{30 + 3 * i}")

    def stabs(p, notes, vol=36):
        for bar, n in enumerate(notes):
            p.put(bar * BAR + 6, 4, f"{n} metal v{vol}")
            p.put(bar * BAR + 14, 4, f"{up(n, 6)} metal v{vol - 8}")

    fight = song.pattern(64)
    fight.put(0, 1, "C-5 crash v40")
    fight_drums(fight)
    chug(fight, ["E-1", "E-1", "F-1", "E-1"])
    stabs(fight, ["E-4", "E-4", "F-4", "E-4"])
    melody(fight, 3, warped + [(16, "G-4", 4), (20, "F-4", 4), (24, "E-4", 8)], "saw", 38)
    melody(fight, 3, [(r + 32, n, l) for r, n, l in warped]
           + [(48, "C-5", 4), (52, "A#4", 4), (56, "B-4", 8)], "saw", 38)

    rage = song.pattern(64)
    fight_drums(rage)
    chug(rage, ["G-1", "F-1", "E-1", "F-1"])
    stabs(rage, ["G-4", "F-4", "E-4", "F-4"], vol=40)
    melody(rage, 3, [(0, "B-4", 4), (4, "C-5", 4), (8, "D-5", 4), (12, "E-5", 4),
                     (16, "F-5", 8), (24, "E-5", 4), (28, "D-5", 4),
                     (32, "E-5", 3), (35, "G-5", 3), (38, "A#5", 2), (40, "A-5", 8),
                     (48, "G-5", 4), (52, "F-5", 4), (56, "E-5", 8)], "saw", 38)
    rage.put(48, 5, "E-5 siren v14")
    for r in range(49, 64):
        rage.put(r, 5, "--- 104")
    rage.put(63, 5, "===")

    grind = song.pattern(64)  # half-time drop, then the build back in
    for bar in range(4):
        grind.put(bar * BAR, 0, "C-5 kick v62")
        grind.put(bar * BAR + 8, 0, "C-5 snare v50")
        grind.put(bar * BAR, 2, f"{['E-1', 'F-1', 'E-1', 'A#0'][bar]} reese v52")
        grind.put(bar * BAR + 2, 4, f"{['E-3', 'F-3', 'E-3', 'A#2'][bar]} metal v48")
        grind.put(bar * BAR + 10, 4, f"{['E-4', 'F-4', 'E-4', 'A#3'][bar]} metal v34")
    melody(grind, 3, [(0, "E-5", 12), (16, "F-5", 12), (32, "E-5", 8), (40, "A#4", 8)],
           "saw", 34)
    riser(grind, 48, 64)
    for i, r in enumerate(range(48, 64)):
        grind.put(r, 0, f"C-5 snare v{24 + i * 2}")

    song.order = [wake, alarm, fight, rage, fight, grind]
    song.restart = 2
    save(song)


# --- Clear: C major victory sting, then a chill results loop. ----------------------

def clear():
    song = Song("clear", channels=6, speed=6, bpm=150)
    add_instruments(song, "kick", "snare", "hat", "crash", "bass", "lead", "pluck", "pad",
                    "coin")

    sting = song.pattern(48)
    # The motif in major, a triumphant leap, a crash and a coin shower.
    melody(sting, 3, [(0, "C-5", 3), (3, "E-5", 3), (6, "G-5", 2), (8, "F-5", 4),
                      (12, "A-5", 4), (16, "C-6", 24)], "lead", 46, vibrato="434")
    for r, n in [(0, "F-2"), (8, "G-2"), (16, "C-2")]:
        sting.put(r, 2, f"{n} bass v48")
    pad(sting, [("F-", MAJOR), ("C-", MAJOR), ("C-", MAJOR)], vol=22)
    for i, r in enumerate(range(8, 16)):
        sting.put(r, 0, f"C-5 snare v{28 + 4 * i}")
    sting.put(0, 0, "C-5 kick v60")
    sting.put(16, 0, "C-5 kick v64")
    sting.put(16, 1, "C-5 crash v44")
    coins(sting, list(range(16, 40, 2)),
          ["C-6", "E-6", "G-6", "C-7", "E-7", "G-7", "E-7", "C-7", "G-6", "C-7", "E-7", "G-7"],
          vol=24)
    for r in range(40, 48):
        sting.put(r, 3, "--- A02")

    chill = song.pattern(64)  # results screen: laid-back groove on the same chords
    prog = [("F-", MAJOR), ("G-", MAJOR), ("E-", MINOR), ("A-", MINOR)]
    for bar, (root, _) in enumerate(prog):
        b = bar * BAR
        chill.put(b, 0, "C-5 kick v44")
        chill.put(b + 10, 0, "C-5 kick v32")
        chill.put(b + 8, 0, "C-5 snare v30")
        chill.put(b, 2, f"{root}2 bass v40")
        chill.put(b + 6, 2, f"{root}3 bass v28")
        chill.put(b + 10, 2, f"{root}2 bass v34")
        for r in range(0, BAR, 4):
            chill.put(b + r + 2, 1, "C-5 hat v14")
    arp(chill, 4, prog, vol=14, step=2, figure=(0, 2, 1, 3), octave=5)
    pad(chill, prog, vol=16)
    melody(chill, 3, [(0, "C-5", 3), (3, "E-5", 3), (6, "G-5", 2), (8, "F-5", 8),
                      (32, "B-4", 3), (35, "D-5", 3), (38, "G-5", 2), (40, "E-5", 8)],
           "lead", 30)

    song.order = [sting, chill]
    song.restart = 1
    save(song)


# --- Down: knocked out. 1.5 s of falling motif, then silence. ----------------------

def down():
    song = Song("down", channels=4, speed=6, bpm=150)
    add_instruments(song, "kick", "bass", "lead", "metal")
    fall = song.pattern(16)
    fall.put(0, 0, "C-5 kick v60")
    fall.put(0, 3, "E-3 metal v40")
    for r, n in [(0, "D-5"), (3, "C-5"), (6, "A#4")]:
        fall.put(r, 1, f"{n} lead v42")
    fall.put(8, 1, "A-4 lead v42")
    for r in range(9, 15):
        fall.put(r, 1, "--- 204")
    fall.put(15, 1, "===")
    for r, n in [(0, "A-2"), (6, "E-2"), (8, "A-1")]:
        fall.put(r, 2, f"{n} bass v48")
    fall.put(15, 2, "===")
    silence = song.pattern(16)
    silence.put(15, 0, "--- B01")  # hold here until the game picks the next cue
    song.order = [fall, silence]
    song.restart = 1
    save(song)


if __name__ == "__main__":
    lofi_title()
    lofi_level()
    boss()
    clear()
    down()
