"""Converts the author's "Micro-Game Rush" MIDI stems into one GBA-ready XM. Run from anywhere:

    python3 game/music/src/micro_game_rush.py

Reads project/assets/audio/midi/Micro-Game Rush ({Bass,Drums,Keyboard}).mid and writes
project/assets/audio/micro-game-rush.xm. No SoundFont: every part is played by a
synthesized instrument from kit.py. They are placeholders; swap in the author's WAVs
with `synth.from_wav` when there are some.

What the conversion does, and why:
- Keeps three parts: the finger-bass track, the two GM drum tracks (channel 10) and the
  keyboard track. The other tracks in those files (saw leads, distortion guitars) are
  left out.
- 6 XM channels, which leaves 2 of the mixer's 8 voices for the game's sound effects:
      0 kick, crash    1 snare, clap, hats    2 bass    3-5 keyboard (3 voices)
- Timing: the MIDI is 480 ticks per beat at ~147-149 BPM (a drifting tempo map). Each
  row is a sixteenth (120 ticks) at speed 6 / BPM 150, i.e. 149.3 BPM locked to the
  frame rate, so the loop is ~1.5% shorter than the MIDI. Notes sit ~15 ticks ahead
  of the grid in the source, so they are shifted by +15 before rounding.
- Polyphony: the keyboard plays up to 11 notes at once. Each row keeps the 3 highest
  sounding notes; a held note stays on its channel and a note pushed out is cut. Bass
  is monophonic (newest note wins). Drums: on each channel the hit with the higher
  priority wins when two fall on the same row.
- MIDI velocity sets the volume column, scaled per part to leave mixer headroom.
"""

import struct
from pathlib import Path

from kit import GLOBAL_VOLUME, INSTRUMENTS, NAMES, ROOT, Song, synth

MIDI = ROOT / "project/assets/audio/midi"
OUT = ROOT / "project/assets/audio/micro-game-rush.xm"

TICKS_PER_ROW = 120          # 480 ticks per beat / 4 rows per beat
GRID_OFFSET = 15             # source notes run this many ticks early
SONG_TICKS = 84000           # end-of-track of every stem: 175 beats
KEY_VOICES = 3
BASS_TRANSPOSE = 0           # +12 makes the bass audible on the GBA's own speaker
MIN_KEY_VELOCITY = 40        # below this a keyboard note is treated as transcription noise

CH_KICK, CH_SNARE, CH_BASS, CH_KEYS = 0, 1, 2, 3

# GM drum key -> (channel, instrument, priority, gain). Higher priority wins a row.
DRUMS = {
    36: (CH_KICK, "kick", 3, 1.00),
    49: (CH_KICK, "crash", 1, 0.70),
    38: (CH_SNARE, "snare", 5, 0.85),
    39: (CH_SNARE, "clap", 4, 0.75),
    46: (CH_SNARE, "ohat", 3, 0.55),
    42: (CH_SNARE, "hat", 2, 0.55),
    69: (CH_SNARE, "hat", 1, 0.35),
}
BASS_GAIN = 0.85
KEYS_GAIN = 0.60


# --- MIDI reading (standard library only) ------------------------------------------

def _vlq(data, i):
    value = 0
    while True:
        byte = data[i]
        i += 1
        value = (value << 7) | (byte & 0x7F)
        if byte < 0x80:
            return value, i


def read_midi(path):
    """Returns one list per track of notes (start, end, channel, key, velocity) in ticks."""
    data = path.read_bytes()
    if data[:4] != b"MThd":
        raise ValueError(f"{path}: not a MIDI file")
    _, _, division = struct.unpack(">HHH", data[8:14])
    if division != 480:
        raise ValueError(f"{path}: expected 480 ticks per beat, got {division}")
    tracks, i = [], 14
    while i < len(data):
        length = struct.unpack(">I", data[i + 4:i + 8])[0]
        chunk, i = data[i + 8:i + 8 + length], i + 8 + length
        notes, held, j, status, tick = [], {}, 0, 0, 0
        while j < len(chunk):
            delta, j = _vlq(chunk, j)
            tick += delta
            byte = chunk[j]
            if byte == 0xFF:
                size, k = _vlq(chunk, j + 2)
                j = k + size
                continue
            if byte in (0xF0, 0xF7):
                size, k = _vlq(chunk, j + 1)
                j = k + size
                continue
            if byte & 0x80:
                status, j = byte, j + 1
            kind, channel = status >> 4, status & 0x0F
            if kind in (0xC, 0xD):
                j += 1
                continue
            key, velocity = chunk[j], chunk[j + 1]
            j += 2
            if kind == 0x9 and velocity > 0:
                held.setdefault((channel, key), []).append((tick, velocity))
            elif kind == 0x8 or kind == 0x9:
                if held.get((channel, key)):
                    start, vel = held[(channel, key)].pop(0)
                    notes.append((start, tick, channel, key, vel))
        tracks.append(sorted(notes))
    return tracks


def stem(name):
    return read_midi(MIDI / f"Micro-Game Rush ({name}).mid")


def row_of(tick):
    return round((tick + GRID_OFFSET) / TICKS_PER_ROW)


def note_name(key):
    octave = key // 12 - 1
    if not 0 <= octave <= 7:
        raise ValueError(f"MIDI key {key} is outside the XM range C-0..B-7")
    return f"{NAMES[key % 12]}{octave}"


def volume(velocity, gain):
    return max(1, min(64, round(velocity / 127 * 64 * gain)))


# --- Arranging onto the 6 channels -------------------------------------------------

def arrange(total_rows):
    """Returns {(row, channel): cell text}."""
    cells = {}

    # Drums: every channel-10 note of the Drums stem.
    best = {}
    for track in stem("Drums"):
        for start, _, channel, key, vel in track:
            if channel != 9 or key not in DRUMS:
                continue
            ch, ins, prio, gain = DRUMS[key]
            row = row_of(start)
            if row < total_rows and prio > best.get((row, ch), (0,))[0]:
                best[(row, ch)] = (prio, f"C-5 {ins} v{volume(vel, gain)}")
    cells.update({k: v[1] for k, v in best.items()})

    # Bass: the finger-bass track (MIDI channel 2), monophonic.
    bass = [n for t in stem("Bass") for n in t if n[2] == 1]
    place_voices(cells, bass, [CH_BASS], total_rows, BASS_GAIN, "bass", BASS_TRANSPOSE,
                 pick=lambda sounding: sorted(sounding, key=lambda n: -n[0])[:1])

    # Keyboard: the 3 highest sounding notes.
    keys = [n for t in stem("Keyboard") for n in t if n[4] >= MIN_KEY_VELOCITY]
    place_voices(cells, keys, list(range(CH_KEYS, CH_KEYS + KEY_VOICES)), total_rows,
                 KEYS_GAIN, "keys", 0,
                 pick=lambda sounding: sorted(sounding, key=lambda n: -n[3])[:KEY_VOICES])
    return cells


def place_voices(cells, notes, channels, total_rows, gain, ins, transpose, pick):
    """Gives each note a channel while it sounds; `pick` chooses which notes may sound.

    A note never moves to another channel: if `pick` drops it, it is cut, and it does not
    come back. A channel whose note ends (or is cut) gets a key-off unless a new note
    starts on it the same row.
    """
    quantized = []
    for start, end, _, key, vel in notes:
        s = row_of(start)
        e = max(s + 1, row_of(end))
        if s < total_rows:
            quantized.append((s, min(e, total_rows), key + transpose, vel))
    by_start = {}
    for n in quantized:
        by_start.setdefault(n[0], []).append(n)

    sounding, playing = [], {}   # playing: channel -> note
    for row in range(total_rows):
        # Same key starting again retriggers: the old one ends now.
        restarts = {n[2] for n in by_start.get(row, [])}
        sounding = [n for n in sounding if n[1] > row and n[2] not in restarts]
        sounding += by_start.get(row, [])
        keep = pick(sounding)
        keep_ids = {id(n) for n in keep}
        sounding = keep   # a note `pick` drops is cut for good

        freed = []
        for ch in channels:
            note = playing.get(ch)
            if note is not None and id(note) not in keep_ids:
                del playing[ch]
                freed.append(ch)
        free = [ch for ch in channels if ch not in playing]
        new = [n for n in keep if all(n is not p for p in playing.values())]
        # Spread new notes over free channels high to low so voices keep their register.
        new.sort(key=lambda n: -n[2])
        for ch, note in zip(free, new):
            playing[ch] = note
            cells[(row, ch)] = f"{note_name(note[2])} {ins} v{volume(note[3], gain)}"
        for ch in freed:
            if ch not in playing:
                cells[(row, ch)] = "==="


def main():
    total_rows = round(SONG_TICKS / TICKS_PER_ROW)   # 700 sixteenths
    song = Song("micro-game-rush", channels=6, speed=6, bpm=150)
    for name in ("kick", "crash", "snare", "ohat", "hat", "bass", "keys"):
        factory, kwargs = INSTRUMENTS[name]
        song.instrument(name, factory(), **kwargs)
    song.instrument("clap", synth.snare(length=0.12, seed=7), volume=50)

    cells = arrange(total_rows)
    patterns = []
    for first in range(0, total_rows, 64):
        rows = min(64, total_rows - first)
        p = song.pattern(rows)
        for (row, ch), text in cells.items():
            if first <= row < first + rows:
                p.put(row - first, ch, text)
        patterns.append(p)

    # Global volume like the rest of the soundtrack (kit.save), on a free cell of row 0.
    first = patterns[0].cells[0]
    free = next(c for c in first if c[3] == 0 and c[4] == 0)
    free[3], free[4] = 0x10, GLOBAL_VOLUME

    song.order = patterns
    song.restart = 0
    song.save(OUT)
    print(f"wrote {OUT.relative_to(ROOT)}: {total_rows} rows, {len(patterns)} patterns")


if __name__ == "__main__":
    main()
