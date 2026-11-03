//! Blaster sound effects on the GBA's built-in tone and noise channels.
//!
//! agb's mixer plays sampled audio, which needs WAV assets and CPU time every frame. The
//! hardware square-wave channel 1 has a pitch sweep instead: triggered once, it falls in
//! pitch and fades out on its own, which is the classic laser "pew". agb has no API for
//! these channels, so this writes their registers directly; nothing else in the game
//! uses sound.
//!
//! Pitches are register rates: frequency = 131072 / (2048 - rate) Hz.

use diamond_hands_logic::Events;

const SOUND1CNT_L: *mut u16 = 0x0400_0060 as *mut u16;
const SOUND1CNT_H: *mut u16 = 0x0400_0062 as *mut u16;
const SOUND1CNT_X: *mut u16 = 0x0400_0064 as *mut u16;
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

/// Bolt: a short, thin zap from about 2 kHz, falling fast.
const BOLT_RATE: u16 = 1984;
/// Charged beam: a fatter, lower tone with a long fall, over a burst of noise.
const BEAM_RATE: u16 = 1840;

pub struct Sfx(());

impl Sfx {
    /// Turns on the sound hardware with channels 1 (tone) and 4 (noise) at full
    /// volume on both speakers.
    pub fn new() -> Self {
        // SAFETY: these are the always-mapped sound registers, and nothing else in the
        // game touches them. The master enable must be set before the others accept
        // writes.
        unsafe {
            SOUNDCNT_X.write_volatile(1 << 7);
            // Left and right volume 7; channels 1 and 4 to the right (bits 8, 11) and
            // left (bits 12, 15).
            SOUNDCNT_L.write_volatile(0x77 | 1 << 8 | 1 << 11 | 1 << 12 | 1 << 15);
            // These channels at 100 % of the output.
            SOUNDCNT_H.write_volatile(2);
        }
        Self(())
    }

    /// Plays the sounds for this frame's events. A beam outranks a bolt on channel 1.
    pub fn play(&mut self, events: Events) {
        // SAFETY: as in `new`.
        unsafe {
            if events.contains(Events::BEAM) {
                SOUND1CNT_L.write_volatile(sweep_down(7, 2));
                SOUND1CNT_H.write_volatile(tone(0, 2, 2, 15));
                SOUND1CNT_X.write_volatile(BEAM_RATE | RESTART);
                // Noise, fading from volume 12; divider 3 and shift 2 make a bright hiss.
                SOUND4CNT_L.write_volatile(tone(0, 0, 2, 12));
                SOUND4CNT_H.write_volatile(3 | 2 << 4 | RESTART);
            } else if events.contains(Events::SHOOT) {
                SOUND1CNT_L.write_volatile(sweep_down(6, 1));
                SOUND1CNT_H.write_volatile(tone(32, 1, 1, 13));
                SOUND1CNT_X.write_volatile(BOLT_RATE | USE_LENGTH | RESTART);
            }
        }
    }
}
