//! Mirrors game state into RAM for the emulator harness (see `harness/`).

use diamond_hands_logic::{Game, Probe};

// Pinned to EWRAM: the harness scans EWRAM first, and the stack (which may hold stale
// temporaries with the same magic) lives in IWRAM.
#[unsafe(no_mangle)]
#[unsafe(link_section = ".ewram")]
#[used]
static mut DH_PROBE: Probe = Probe::BOOT;

pub fn publish(game: &Game) {
    // SAFETY: single-threaded, and nothing else holds a reference to DH_PROBE.
    unsafe { (&raw mut DH_PROBE).write_volatile(Probe::from_game(game)) }
}
