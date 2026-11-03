//! Diamond Hands: a wassie with a laser blaster takes on the bears, level by level.
//!
//! All game rules live in `diamond_hands_logic`; this crate reads the buttons, steps the
//! game once per frame, and draws the result.
#![no_std]
#![no_main]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]
#![cfg_attr(test, test_runner(agb::test_runner::test_runner))]

extern crate alloc;

mod art;
mod font;
mod probe;
mod render;
mod save;

use diamond_hands_logic::{Events, Game, Input};

#[agb::entry]
fn main(mut gba: agb::Gba) -> ! {
    let mut store = save::Store::new(&mut gba.save);
    let (best, difficulty) = store.load();
    let mut game = Game::with_difficulty(best, difficulty);
    let mut gfx = gba.graphics.get();
    let mut renderer = render::Renderer::new(&mut gfx);
    let mut previous = 0;

    loop {
        // Keys are read once per frame, right after the previous frame's vblank, which
        // is what the emulator harness assumes when it replays inputs.
        let held = read_keys();
        let events = game.step(&Input::from_keys(held, previous));
        previous = held;
        probe::publish(&game);

        if events.contains(Events::SAVE) {
            store.save(game.best, game.difficulty);
        }

        let mut frame = gfx.frame();
        renderer.draw(&game, &mut frame);
        frame.commit();
    }
}

/// Held buttons as active-high KEYINPUT bits, the layout `Input::from_keys` expects.
fn read_keys() -> u16 {
    const KEYINPUT: *const u16 = 0x0400_0130 as *const u16;
    // SAFETY: KEYINPUT is a read-only hardware register, always mapped.
    let raw = unsafe { KEYINPUT.read_volatile() };
    !raw & 0x03FF
}
