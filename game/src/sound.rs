//! Sound effects on the GBA's built-in tone and noise channels: the blaster on channels 1
//! and 4, the diamond chime on channel 2, the electric fences' crackle on channel 4.
//!
//! agb's mixer plays sampled audio, which needs WAV assets and CPU time every frame. The
//! hardware square-wave channel 1 has a pitch sweep instead: triggered once, it falls in
//! pitch and fades out on its own, which is the classic laser "pew". agb has no API for
//! these channels, so this writes their registers directly. The music (`music`) plays
//! on agb's mixer, which owns DirectSound A and B in the same control registers.
//!
//! Pitches are register rates: frequency = 131072 / (2048 - rate) Hz.

use diamond_hands_logic::Events;

const SOUND1CNT_L: *mut u16 = 0x0400_0060 as *mut u16;
const SOUND1CNT_H: *mut u16 = 0x0400_0062 as *mut u16;
const SOUND1CNT_X: *mut u16 = 0x0400_0064 as *mut u16;
const SOUND2CNT_L: *mut u16 = 0x0400_0068 as *mut u16;
const SOUND2CNT_H: *mut u16 = 0x0400_006C as *mut u16;
const SOUND4CNT_L: *mut u16 = 0x0400_0078 as *mut u16;
const SOUND4CNT_H: *mut u16 = 0x0400_007C as *mut u16;
const SOUNDCNT_L: *mut u16 = 0x0400_0080 as *mut u16;
const SOUNDCNT_H: *mut u16 = 0x0400_0082 as *mut u16;
const SOUNDCNT_X: *mut u16 = 0x0400_0084 as *mut u16;

/// Sweep register: shift `n` (bits 0-2), pitch going down (bit 3), one step every
/// `time` x 7.8 ms (bits 4-6).
const fn sweep_down(shift: u16, time: u16) -> u16 {
    shift | 1 << 3 | time << 4
}

/// Duty/length/envelope register: sound length `(64 - length) / 256` s (bits 0-5),
/// duty 12.5/25/50/75 % (bits 6-7), fading one volume step every `step` / 64 s
/// (bits 8-10) from `volume` (bits 12-15).
const fn tone(length: u16, duty: u16, step: u16, volume: u16) -> u16 {
    length | duty << 6 | step << 8 | volume << 12
}

const RESTART: u16 = 1 << 15;
const USE_LENGTH: u16 = 1 << 14;

/// Bolt: a short, round zap from about 1 kHz, falling fast.
const BOLT_RATE: u16 = rate(1000);
/// Charged beam: a fat, low tone with a long fall, over a burst of noise.
const BEAM_RATE: u16 = rate(420);

/// The register rate for a pitch in Hz, rounded.
const fn rate(hz: u32) -> u16 {
    2048 - ((131_072 + hz / 2) / hz) as u16
}

/// Diamond chime pitches: A minor pentatonic from E5, the music's key. A chime plays a
/// rung and then the one two above it; each diamond picked up soon after the last
/// starts a rung higher.
const LADDER: [u16; 10] = [
    rate(659), // E5
    rate(784), // G5
    rate(880), // A5
    rate(1047), // C6
    rate(1175), // D6
    rate(1319), // E6
    rate(1568), // G6
    rate(1760), // A6
    rate(2093), // C7
    rate(2349), // D7
];
/// Frames after a diamond within which the next one climbs the ladder.
const CHAIN_FRAMES: u16 = 50;
/// Frames between the chime's two notes.
const CHIME_GAP: u8 = 4;
/// Frames between a coin's two notes: quicker than a diamond's.
const COIN_GAP: u8 = 3;

/// Noise register: clock divider `div` (bits 0-2), the 7-step LFSR (bit 3: a buzzy,
/// metallic noise rather than a hiss) and clock shift `shift` (bits 4-7). Lower values
/// are higher-pitched.
const fn buzz(div: u16, shift: u16) -> u16 {
    div | 1 << 3 | shift << 4
}

/// A crackle: the noise channel retriggered every few frames, each burst quieter, so
/// it stutters like an arc instead of hissing once.
#[derive(Clone, Copy)]
struct Crackle {
    /// Bursts left after this one.
    bursts: u8,
    /// Frames between bursts, and until the next.
    gap: u8,
    wait: u8,
    volume: u16,
    /// Volume lost each burst.
    fade: u16,
    noise: u16,
}

impl Crackle {
    /// The fences coming on: three dry "bz" ticks.
    const FENCE_ON: Crackle = Crackle { bursts: 3, gap: 3, wait: 0, volume: 8, fade: 1, noise: buzz(2, 3) };
    /// The wassie shocked: a loud, bright stutter that dies away.
    const SHOCK: Crackle = Crackle { bursts: 6, gap: 3, wait: 0, volume: 14, fade: 2, noise: buzz(1, 1) };

    /// Plays this frame's burst, if it's time. Returns what's left of the crackle.
    fn step(mut self) -> Option<Crackle> {
        if self.wait > 0 {
            self.wait -= 1;
            return Some(self);
        }
        // SAFETY: as in `Sfx::new`.
        unsafe {
            SOUND4CNT_L.write_volatile(tone(0, 0, 1, self.volume));
            SOUND4CNT_H.write_volatile(self.noise | RESTART);
        }
        if self.bursts == 0 {
            return None;
        }
        self.bursts -= 1;
        self.wait = self.gap - 1;
        self.volume = self.volume.saturating_sub(self.fade);
        Some(self)
    }
}

/// The shock's hum under the crackle: a thin square wave at mains-ish 110 Hz, sagging.
const SHOCK_RATE: u16 = rate(110);

pub struct Sfx {
    /// The fences' crackle, while it plays.
    crackle: Option<Crackle>,
    /// Rungs climbed by the current chain of diamonds.
    chain: u8,
    /// Frames since the last diamond, saturating.
    since_diamond: u16,
    /// The chime's second note: frames until it plays, and its rate.
    pending: Option<(u8, u16)>,
}

impl Sfx {
    /// Turns on channels 1, 2 (tones) and 4 (noise) at full volume on both speakers.
    /// Call after creating the mixer, which rewrites `SOUNDCNT_H` when it starts.
    pub fn new() -> Self {
        // SAFETY: these are the always-mapped sound registers; apart from agb's mixer,
        // nothing else in the game touches them. The master enable must be set before
        // the others accept writes.
        unsafe {
            SOUNDCNT_X.write_volatile(1 << 7);
            // Left and right volume 7; channels 1, 2 and 4 to the right (bits 8, 9, 11)
            // and left (bits 12, 13, 15).
            SOUNDCNT_L.write_volatile(0x77 | 0b1011 << 8 | 0b1011 << 12);
            // These channels at 100 % of the output (bits 0-1). The rest of the register
            // is the mixer's DirectSound setup: clearing it would silence the music.
            SOUNDCNT_H.write_volatile(SOUNDCNT_H.read_volatile() & !0b11 | 0b10);
        }
        Self {
            crackle: None,
            chain: 0,
            since_diamond: u16::MAX,
            pending: None,
        }
    }

    /// Starts the diamond chime, a rung higher if the last diamond was moments ago.
    pub fn diamond(&mut self) {
        self.chain = if self.since_diamond < CHAIN_FRAMES {
            (self.chain + 1).min(LADDER.len() as u8 - 3)
        } else {
            0
        };
        self.since_diamond = 0;
        let rung = self.chain as usize;
        // A soft, quick "ting", then a fuller note two rungs up.
        Self::chime(LADDER[rung], tone(0, 2, 1, 8));
        self.pending = Some((CHIME_GAP, LADDER[rung + 2]));
    }

    fn chime(rate: u16, envelope: u16) {
        // SAFETY: as in `new`.
        unsafe {
            SOUND2CNT_L.write_volatile(envelope);
            SOUND2CNT_H.write_volatile(rate | RESTART);
        }
    }

    /// Plays the sounds for this frame's events, and the rest of a chime. A beam
    /// outranks a bolt on channel 1. Call once per frame.
    pub fn play(&mut self, events: Events) {
        self.since_diamond = self.since_diamond.saturating_add(1);
        if let Some((frames, rate)) = self.pending {
            if frames > 1 {
                self.pending = Some((frames - 1, rate));
            } else {
                Self::chime(rate, tone(0, 2, 3, 10));
                self.pending = None;
            }
        }
        self.menu(events);
        self.casino(events);
        // A shock outranks the fences coming on; the beam's rumble outranks both.
        if events.contains(Events::SHOCK) {
            self.crackle = Some(Crackle::SHOCK);
        } else if events.contains(Events::FENCE_ON) && self.crackle.is_none() {
            self.crackle = Some(Crackle::FENCE_ON);
        }
        if events.contains(Events::BEAM) {
            self.crackle = None;
        }
        self.crackle = self.crackle.and_then(Crackle::step);
        // SAFETY: as in `new`.
        unsafe {
            if events.contains(Events::SHOCK) {
                SOUND1CNT_L.write_volatile(sweep_down(3, 4));
                SOUND1CNT_H.write_volatile(tone(0, 0, 3, 12));
                SOUND1CNT_X.write_volatile(SHOCK_RATE | RESTART);
            } else if events.contains(Events::BEAM) {
                SOUND1CNT_L.write_volatile(sweep_down(7, 2));
                SOUND1CNT_H.write_volatile(tone(0, 2, 2, 12));
                SOUND1CNT_X.write_volatile(BEAM_RATE | RESTART);
                // Noise, fading from volume 12; divider 3 and shift 4 make a low rumble, not a hiss.
                SOUND4CNT_L.write_volatile(tone(0, 0, 2, 9));
                SOUND4CNT_H.write_volatile(3 | 4 << 4 | RESTART);
            } else if events.contains(Events::SHOOT) {
                SOUND1CNT_L.write_volatile(sweep_down(6, 1));
                SOUND1CNT_H.write_volatile(tone(32, 2, 1, 10));
                SOUND1CNT_X.write_volatile(BOLT_RATE | USE_LENGTH | RESTART);
            }
        }
    }

    /// The casino's sounds on the chime's channel: a bright "ka-ching" for a gold coin
    /// (higher and snappier than the diamond chime), a rising power-up for the elevator
    /// terminal, a "ding-dong" for its doors and ride, and a sparkle for a plasma cell.
    fn casino(&mut self, events: Events) {
        if events.contains(Events::PART) {
            Self::chime(LADDER[5], tone(0, 2, 2, 11));
            self.pending = Some((CHIME_GAP * 2, LADDER[9]));
        } else if events.contains(Events::POWER) {
            Self::chime(rate(330), tone(0, 2, 3, 11));
            self.pending = Some((CHIME_GAP * 2, rate(660)));
        } else if events.contains(Events::LIFT) {
            Self::chime(rate(1319), tone(0, 2, 3, 10));
            self.pending = Some((CHIME_GAP * 3, rate(1047)));
        } else if events.contains(Events::COIN) {
            Self::chime(rate(1976), tone(0, 2, 1, 10));
            self.pending = Some((COIN_GAP, rate(2637)));
        }
    }

    /// Menu feedback on the chime's channel: a tick for the cursor, a rising pair to
    /// pick, a falling pair to back out, and a low buzz for a locked place.
    fn menu(&mut self, events: Events) {
        if events.contains(Events::MENU_OK) {
            Self::chime(LADDER[3], tone(0, 2, 1, 9));
            self.pending = Some((CHIME_GAP, LADDER[5]));
        } else if events.contains(Events::MENU_BACK) {
            Self::chime(LADDER[3], tone(0, 2, 1, 8));
            self.pending = Some((CHIME_GAP, LADDER[0]));
        } else if events.contains(Events::MENU_DENY) {
            Self::chime(rate(165), tone(0, 3, 2, 10));
        } else if events.contains(Events::MENU_MOVE) {
            Self::chime(LADDER[2], tone(0, 1, 1, 6));
        }
    }
}
