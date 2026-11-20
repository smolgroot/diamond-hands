//! Sol City alone, built from its GBA-Studio project: every room, tile, sprite and song
//! comes from `studio/` and `assets/`, and every rule from `diamond_hands_logic`. This
//! crate reads the buttons, steps the game, and draws it with the studio's runtime.
//!
//! The port starts with Sol City (level 3), the first level painted in the studio: the
//! ROM boots straight into it. The earlier levels, the menus, the world map and saving
//! are still in the code-drawn ROM (`../game`) until they are moved over.
#![no_std]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;

use diamond_hands_logic::{Events, Game, Input, Scene, levels};

mod draw;
mod music;
#[path = "../sound.rs"]
mod sound;

pecten_studio_rt::include_data!();
pecten_studio_rt::include_gfx!();
pecten_studio_rt::include_audio!();

/// The level this ROM plays: Sol City.
const LEVEL: u8 = 2;

#[agb::entry]
fn main(mut gba: agb::Gba) -> ! {
    debug_assert!(core::ptr::eq(levels::ALL[LEVEL as usize], &levels::sol_city::SOL_CITY));
    let mut game = boxed(|| Game::new(0));
    game.play_level(LEVEL);
    game.world.god = cfg!(feature = "demo");
    let mut graphics = gba.graphics.get();
    // The tracker needs the mixer at 32768 Hz. The mixer comes before `Sfx`, which
    // keeps its DirectSound settings.
    let mut mixer = gba.mixer.mixer(agb::sound::mixer::Frequency::Hz32768);
    let mut music = music::Music::new();
    let mut sfx = sound::Sfx::new();
    let world: &'static _ = Box::leak(boxed(data::world));
    let mut screen = boxed(|| draw::Screen::new(world, &mut graphics));
    let mut previous = 0;
    let mut bot = diamond_hands_logic::bot::Bot::default();

    loop {
        let held = if cfg!(feature = "demo") { demo_keys(&game, &mut bot, previous) } else { read_keys() };
        let diamonds = game.world.diamonds;
        let events = game.step(&Input::from_keys(held, previous));
        previous = held;
        sfx.play(events);
        if game.scene == Scene::Play && game.world.diamonds > diamonds {
            sfx.diamond();
        }
        // Past the clear, the world map would open: this ROM has only Sol City for now,
        // so it says what comes next and starts over on Start.
        if game.scene == Scene::Map && Input::from_keys(held, 0).start_pressed() && events == Events::default() {
            game.play_level(LEVEL);
        }
        music.play(&game, &mut mixer);
        mixer.frame();

        let mut frame = graphics.frame();
        screen.draw(&game, &mut frame);
        frame.commit();
    }
}

/// Builds a value in a stack frame of its own and moves it to the heap (EWRAM): the
/// stack shares IWRAM with the sound mixer's buffers.
#[inline(never)]
fn boxed<T>(make: impl FnOnce() -> T) -> Box<T> {
    Box::new(make())
}

/// The demo's buttons: the route bot's in play, A every few frames in dialogue.
fn demo_keys(game: &Game, bot: &mut diamond_hands_logic::bot::Bot, previous: u16) -> u16 {
    use diamond_hands_logic::keys;
    match game.scene {
        Scene::Play => bot.keys(&game.world, previous),
        Scene::Dialog | Scene::Intro if game.frame % 8 == 0 => keys::A,
        _ => 0,
    }
}

/// Held buttons as active-high KEYINPUT bits, the layout `Input::from_keys` expects.
fn read_keys() -> u16 {
    const KEYINPUT: *const u16 = 0x0400_0130 as *const u16;
    // SAFETY: KEYINPUT is a read-only hardware register, always mapped.
    let raw = unsafe { KEYINPUT.read_volatile() };
    !raw & 0x03FF
}
