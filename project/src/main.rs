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
mod screens;
mod sound;
mod splash;
/// The studio levels' screen (Sol City, Ethereum City), drawn from their rooms: shared
/// with the `sol-city` ROM.
#[path = "sol_city/draw.rs"]
mod studio_screen;

pecten_studio_rt::include_data!();
pecten_studio_rt::include_gfx!();
pecten_studio_rt::include_audio!();

use diamond_hands_logic::{Events, Game, Input, Scene, level::Theme};

#[agb::entry]
fn main(mut gba: agb::Gba) -> ! {
    let mut store = save::Store::new(&mut gba.save);
    let (best, difficulty, files) = store.load();
    let mut game = boxed(|| Game::with_save(best, difficulty, files));
    if cfg!(feature = "dev") {
        // Every level but the last counts as beaten on the map, so all of them are open.
        game.dev_cleared = diamond_hands_logic::levels::ALL_CLEARED >> 1;
    }
    let mut gfx = gba.graphics.get();
    let mut splash = Some(boxed(|| splash::Splash::new(&mut gfx)));
    let mut drawer: Option<Drawer> = None;
    // Blank frames still to go out after a drawer is dropped, before the next is built.
    let mut settling = 0u8;
    // The studio's map, read once: Sol City's screen draws from it.
    let studio_world: &'static _ = Box::leak(boxed(data::world));
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

        // A drawer is built once the splash's tiles are gone, and swapped when the game
        // moves between a level painted in the studio and everything else: no two of
        // them fit in VRAM at once. The frame that drops one goes out blank, and so does
        // the next: committing the first frees the tiles, but agb holds on to the sprites
        // of the last two frames shown, so the old drawer's sprites (a boss's 64x64
        // frames, the plasma shield's bubble) stay in VRAM until the second.
        // Building the renderer over them ran out of sprite VRAM when pausing in BORED
        // KONG's fight. The frame after builds the other drawer. It runs long, which is
        // fine on a level card, a clear screen or the pause menu.
        let want = Drawer::wanted(&game);
        if splash.is_none() && drawer.as_ref().is_some_and(|d| d.kind() != want) {
            drawer = None;
            settling = 1;
        } else if splash.is_none() && drawer.is_none() && settling > 0 {
            settling -= 1;
        } else if splash.is_none() && drawer.is_none() {
            // Every frame the old drawer showed is still cached in sprite VRAM: clear
            // them out so the new one's sprites pack together instead of filling gaps.
            art::flush_sprite_cache();
            drawer = Some(match want {
                DrawerKind::Classic => Drawer::Classic(boxed(|| render::Renderer::new(&mut gfx))),
                DrawerKind::Studio(theme) => Drawer::Studio(boxed(|| studio_screen::Screen::new(studio_world, theme))),
            });
        }
        if let Some(Drawer::Studio(screen)) = &mut drawer {
            screen.prepare(&game, &mut gfx);
        }
        // For GBA-Studio's Play panel (dev builds): where Sol City's wassie is, or that
        // the game is somewhere the studio has no map of.
        match &drawer {
            Some(Drawer::Studio(screen)) => screen.publish(&game),
            _ => studio_screen::publish_away(),
        }
        let mut frame = gfx.frame();
        if game.scene == Scene::Splash {
            if let Some(splash) = &splash {
                splash.draw(&game, &mut frame);
            }
        } else if splash.take().is_none() {
            match &mut drawer {
                Some(Drawer::Classic(renderer)) => renderer.draw(&game, &mut frame),
                Some(Drawer::Studio(screen)) => screen.draw(&game, &mut frame),
                None => {}
            }
        }
        frame.commit();
    }
}

/// What draws the frame: the code-drawn renderer (menus, map, the first two levels), or
/// the studio's screen for a level painted in GBA-Studio (Sol City, Ethereum City).
enum Drawer {
    Classic(Box<render::Renderer>),
    Studio(Box<studio_screen::Screen>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DrawerKind {
    Classic,
    /// The studio screen of one level: going to another studio level makes a new one.
    Studio(Theme),
}

impl Drawer {
    fn kind(&self) -> DrawerKind {
        match self {
            Drawer::Classic(_) => DrawerKind::Classic,
            Drawer::Studio(screen) => DrawerKind::Studio(screen.theme()),
        }
    }

    /// The studio screen draws a studio level from its card to its clear screen; the
    /// pause menu, the map and the menus stay with the renderer, which has them.
    fn wanted(game: &Game) -> DrawerKind {
        let in_level = matches!(game.scene, Scene::Intro | Scene::Arrival | Scene::Play | Scene::Dialog | Scene::LevelClear);
        let theme = game.world.level.theme;
        if in_level && theme.studio() { DrawerKind::Studio(theme) } else { DrawerKind::Classic }
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
