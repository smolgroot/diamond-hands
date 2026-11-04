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


# --- Title: A minor, ~128 BPM. The motif stated plainly. --------------------------

def title():
    song = Song("title", channels=6, speed=7, bpm=150)
    add_instruments(song, "kick", "snare", "hat", "crash", "sub", "lead", "pluck", "pad",
                    "coin")
    verse = [("A-", MINOR), ("F-", MAJOR), ("C-", MAJOR), ("G-", MAJOR)]
    turn = [("A-", MINOR), ("F-", MAJOR), ("G-", MAJOR), ("E-", MAJOR)]

    p0 = song.pattern(64)  # pad and coin sparkle
    pad(p0, verse)
    coins(p0, [0, 6, 10, 32, 38, 42, 60], ["E-6", "A-6", "C-7", "E-6", "G-6", "C-7", "E-7"])
    for bar, (root, _) in enumerate(verse):
        p0.put(bar * BAR, 2, f"{root}2 sub v36")

    p1 = song.pattern(64)  # the motif, slow and plain
    pad(p1, turn)
    for bar, (root, _) in enumerate(turn):
        p1.put(bar * BAR, 2, f"{root}2 sub v40")
    melody(p1, 3, motif("A-4", rhythm=2), "lead", 40)
    melody(p1, 3, [(0, "G#4", 8), (8, "B-4", 8)], "lead", 36, start=48)
    p1.put(56, 0, "C-5 snare v20")
    p1.put(60, 0, "C-5 snare v28")

    p2 = song.pattern(64)  # beat and arps join
    drums(p2, kicks=(0, 10), snares=(8,), hats=4, hat_vol=16, fill=3)
    p2.put(0, 1, "C-5 crash v30")
    for bar, (root, _) in enumerate(verse):
        p2.put(bar * BAR, 2, f"{root}2 sub v44")
        p2.put(bar * BAR + 10, 2, f"{root}2 sub v32")
    arp(p2, 4, verse, vol=18)
    melody(p2, 3, motif("A-4", rhythm=2), "lead", 42)
    melody(p2, 3, [(0, "D-5", 4), (4, "E-5", 4), (8, "G-5", 8)], "lead", 40, start=32,
           end=48)
    melody(p2, 3, [(0, "E-5", 16)], "lead", 38, start=48)

    p3 = song.pattern(64)  # up an octave with an echo
    drums(p3, kicks=(0, 10), snares=(8,), hats=2, hat_vol=14, fill=3)
    for bar, (root, _) in enumerate(turn):
        p3.put(bar * BAR, 2, f"{root}2 sub v44")
        p3.put(bar * BAR + 10, 2, f"{root}2 sub v32")
    arp(p3, 4, turn, vol=18, octave=5, figure=(0, 2, 1, 3))
    melody(p3, 3, motif("A-5", rhythm=2), "lead", 38)
    melody(p3, 3, [(0, "G#5", 8), (8, "B-5", 6)], "lead", 36, start=48)
    echo(p3, 3, 5, delay=3, vol=14)

    song.order = [p0, p1, p2, p3]
    song.restart = 1
    save(song)


# --- Badlands: A minor, ~149 BPM. The shooter groove. ------------------------------

A_SECTION = [("A-", MINOR), ("F-", MAJOR), ("G-", MAJOR), ("E-", MAJOR)]
B_SECTION = [("D-", MINOR), ("F-", MAJOR), ("G-", MAJOR), ("E-", MAJOR)]
HOOK = motif("A-4") + [(16, "E-5", 6), (22, "G-5", 2), (24, "E-5", 4), (28, "D-5", 2),
                       (30, "C-5", 2), (32, "A-4", 3), (35, "C-5", 3), (38, "E-5", 2),
                       (40, "A-5", 8), (48, "G#5", 8), (56, "E-5", 8)]
ANSWER = [(0, "F-5", 4), (4, "E-5", 2), (6, "D-5", 2), (8, "A-5", 8), (16, "A-5", 2),
          (18, "G-5", 2), (20, "F-5", 2), (22, "E-5", 2), (24, "C-5", 4), (28, "D-5", 4),
          (32, "B-4", 4), (36, "D-5", 4), (40, "G-5", 8), (48, "G#5", 4), (52, "B-5", 4),
          (56, "E-6", 8)]


def roots(chords):
    return [r for r, _ in chords]


def badlands():
    song = Song("badlands", channels=6, speed=6, bpm=150)
    add_instruments(song, "kick", "snare", "hat", "ohat", "crash", "riser", "bass", "lead",
                    "pluck", "pad", "coin")

    intro = song.pattern(64)  # engine starting: bass and hats, a pickup into the drop
    pump_bass(intro, roots(A_SECTION), vol=34)
    for r in range(0, 64, 2):
        intro.put(r, 1, f"C-5 hat v{12 + r // 4}")
    for r in (0, 16, 32):
        intro.put(r, 0, "C-5 kick v50")
    for i, r in enumerate(range(48, 64, 2)):
        intro.put(r, 0, f"C-5 snare v{22 + 4 * i}")
    coins(intro, [0, 8, 16, 24], ["A-6", "E-6", "C-7", "G-6"], vol=20)
    melody(intro, 3, [(0, "E-4", 2), (2, "G-4", 2), (4, "A-4", 2)], "lead", 36, start=58)

    groove = song.pattern(64)  # the full kit, arps, no lead
    groove.put(0, 1, "C-5 crash v34")
    drums(groove, open_hats=True, fill=3)
    groove.put(1, 1, "C-5 hat v22")
    groove.put(4, 3, "===")  # end the intro's pickup note
    pump_bass(groove, roots(A_SECTION))
    arp(groove, 4, A_SECTION, vol=20, step=1, figure=(0, 1, 2, 3))
    pad(groove, A_SECTION, vol=14)

    hook = song.pattern(64)  # the hero motif as a riff
    drums(hook, open_hats=True, fill=3)
    pump_bass(hook, roots(A_SECTION))
    arp(hook, 4, A_SECTION, vol=16, step=2)
    melody(hook, 3, HOOK, "lead", 40)
    echo(hook, 3, 5, vol=14)

    answer = song.pattern(64)
    drums(answer, open_hats=True, kicks=(0, 6, 10, 14), fill=3)
    pump_bass(answer, roots(B_SECTION))
    arp(answer, 4, B_SECTION, vol=16, step=2, figure=(0, 2, 3, 1))
    melody(answer, 3, ANSWER, "lead", 40)
    melody(answer, 3, [(0, "E-6", 4), (4, "D-6", 4), (8, "B-5", 4)], "lead", 38, start=56)
    echo(answer, 3, 5, vol=14)

    moon = song.pattern(64)  # breakdown: coins pile up, then the pump to the moon
    pad(moon, A_SECTION, vol=24)
    for bar in range(4):
        moon.put(bar * BAR, 0, "C-5 kick v56")
        moon.put(bar * BAR, 2, f"{A_SECTION[bar][0]}2 bass v40")
    sparkle = ["A-6", "C-7", "E-6", "G-6", "E-7", "D-7", "A-6", "C-7", "G#6", "B-6", "E-7",
               "G#6"]
    coins(moon, list(range(0, 48, 4)), sparkle, vol=22)
    moon.put(0, 3, "===")
    for i, r in enumerate(range(48, 64)):  # E major, rising sixteenths
        moon.put(r, 4, f"{up('E-5', [0, 4, 7, 12][i % 4] + 12 * (i // 8))} pluck v{18 + i}")
    for i, r in enumerate(range(48, 56, 2)):  # snare build: eighths, then sixteenths
        moon.put(r, 0, f"C-5 snare v{24 + 4 * i}")
    for r in range(56, 64):
        moon.put(r, 0, f"C-5 snare v{40 + (r - 56) * 3}")
    riser(moon, 32, 64)

    peak = song.pattern(64)  # the hook an octave up, everyone in
    peak.put(0, 1, "C-5 crash v40")
    drums(peak, open_hats=True, kicks=(0, 6, 10, 14), fill=3)
    peak.put(1, 1, "C-5 hat v24")
    pump_bass(peak, roots(A_SECTION), vol=46)
    arp(peak, 4, A_SECTION, vol=18, step=1, figure=(0, 1, 2, 3, 2, 1))
    melody(peak, 3, [(r, up(n, 12), l) for r, n, l in HOOK], "lead", 36)
    echo(peak, 3, 5, vol=12)

    outro = song.pattern(64)  # answer again, turning back to the groove
    drums(outro, open_hats=True, fill=3)
    pump_bass(outro, roots(B_SECTION))
    arp(outro, 4, B_SECTION, vol=16, step=1, figure=(0, 1, 2, 3))
    melody(outro, 3, ANSWER[:10], "lead", 38)
    melody(outro, 3, [(0, "B-4", 4), (4, "D-5", 4), (8, "E-5", 4), (12, "G#5", 4),
                      (16, "B-5", 8), (24, "E-5", 6)], "lead", 38, start=32)
    echo(outro, 3, 5, vol=12)

    song.order = [intro, groove, hook, answer, moon, peak, outro]
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
    title()
    badlands()
    boss()
    clear()
    down()
