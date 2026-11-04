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

use alloc::boxed::Box;

mod art;
mod font;
mod music;
mod probe;
mod render;
mod save;
mod sound;
mod splash;

use diamond_hands_logic::{Events, Game, Input, Scene};

#[agb::entry]
fn main(mut gba: agb::Gba) -> ! {
    let mut store = save::Store::new(&mut gba.save);
    let (best, difficulty, files) = store.load();
    let mut game = boxed(|| Game::with_save(best, difficulty, files));
    let mut gfx = gba.graphics.get();
    let mut splash = Some(boxed(|| splash::Splash::new(&mut gfx)));
    let mut renderer = None;
    // The tracker needs the mixer at 32768 Hz. The mixer comes before `Sfx`, which
    // keeps its DirectSound settings.
    let mut mixer = gba.mixer.mixer(agb::sound::mixer::Frequency::Hz32768);
    let mut music = music::Music::new();
    let mut sfx = sound::Sfx::new();
    let mut previous = 0;

    loop {
        // Keys are read once per frame, right after the previous frame's vblank, which
        // is what the emulator harness assumes when it replays inputs.
        let held = read_keys();
        let diamonds = game.world.diamonds;
        let events = game.step(&Input::from_keys(held, previous));
        previous = held;
        probe::publish(&game);
        sfx.play(events);
        // `PICKUP` also covers hearts, so a diamond is the count going up in play
        // (starting or continuing a level resets it outside `Play`).
        if game.scene == Scene::Play && game.world.diamonds > diamonds {
            sfx.diamond();
        }
        music.play(&game, &mut mixer);
        mixer.frame();

        if events.contains(Events::SAVE) {
            store.save(&game);
        }

        // Built once the splash's tiles are gone: both don't fit in VRAM at once. This
        // frame runs long, which is fine on the way into the title.
        if splash.is_none() && renderer.is_none() {
            renderer = Some(boxed(|| render::Renderer::new(&mut gfx)));
        }
        let mut frame = gfx.frame();
        if game.scene == Scene::Splash {
            if let Some(splash) = &splash {
                splash.draw(&game, &mut frame);
            }
        } else if splash.take().is_none()
            && let Some(renderer) = &mut renderer
        {
            renderer.draw(&game, &mut frame);
        }
        // The frame that drops the splash goes out blank; committing it frees the tiles.
        frame.commit();
    }
}

/// Builds a value in a stack frame of its own and moves it to the heap (EWRAM).
///
/// The stack shares the 32 KB of IWRAM with the sound mixer's buffers. Built inline,
/// these values and their temporaries kept many KB of `main`'s frame for the whole
/// game, and starting a level stacks whole `World`s on top of that.
#[inline(never)]
fn boxed<T>(make: impl FnOnce() -> T) -> Box<T> {
    Box::new(make())
}

/// Held buttons as active-high KEYINPUT bits, the layout `Input::from_keys` expects.
fn read_keys() -> u16 {
    const KEYINPUT: *const u16 = 0x0400_0130 as *const u16;
    // SAFETY: KEYINPUT is a read-only hardware register, always mapped.
    let raw = unsafe { KEYINPUT.read_volatile() };
    !raw & 0x03FF
}
