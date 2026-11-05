//! Boots the built ROM on the gba-wasm core. Requires `dist/diamond-hands.gba`
//! (scripts/build-rom.sh) or `DIAMOND_HANDS_ROM`.
//!
//! When one of these fails, run the same ROM in mGBA. If it behaves there, the bug is
//! in gba-wasm: record it in gba-wasm (failing test or TASKS.md) and mark the test here
//! `#[ignore = "gba-wasm: <issue>"]`.

use diamond_hands_harness::{Runner, keys, read_rom};
use diamond_hands_logic::{
    Difficulty, Game, Input, PlayerState, Probe, Scene, bot,
    game::{CLEAR_AUTO_FRAMES, INTRO_FRAMES, MAP_WALK_FRAMES, SPLASH_PROMPT_FRAMES, pause_menu, title_menu},
    inventory::ITEMS,
    player::JUMP_APEX,
};

const SECOND: usize = 60;

fn scene(probe: &Probe) -> Scene {
    match probe.scene {
        0 => Scene::Title,
        1 => Scene::Play,
        2 => Scene::Paused,
        3 => Scene::LevelClear,
        4 => Scene::Options,
        5 => Scene::Intro,
        6 => Scene::Dialog,
        7 => Scene::Map,
        8 => Scene::Splash,
        9 => Scene::Files,
        10 => Scene::FileChoice,
        11 => Scene::Arrival,
        other => panic!("unknown scene {other}"),
    }
}

/// Power-on to the splash's "PRESS START" prompt.
fn boot_to_splash_prompt(runner: &mut Runner) -> Probe {
    let prompt = SPLASH_PROMPT_FRAMES + 1;
    let ready = runner.run_until(5 * SECOND, 0, |p| scene(p) == Scene::Splash && p.frame > prompt);
    screenshot(runner, "splash");
    ready
}

/// Power-on, through the splash, to the title in file 1, which must be empty.
fn boot_to_title(runner: &mut Runner) -> Probe {
    boot_to_splash_prompt(runner);
    boot_to_files_from_prompt(runner);
    runner.tap(keys::A);
    let title = runner.run_until(5, 0, |p| scene(p) == Scene::Title);
    runner.frames(2, 0);
    screenshot(runner, "title");
    title
}

/// From the splash prompt to the file menu with its first frame drawn: the renderer is
/// built on the way in, which takes a few frames, and keys are ignored meanwhile.
fn boot_to_files_from_prompt(runner: &mut Runner) -> Probe {
    runner.tap(keys::START);
    let entered = runner.run_until(5, 0, |p| scene(p) == Scene::Files);
    let files = runner.run_until(5 * SECOND, 0, |p| p.frame > entered.frame + 1);
    screenshot(runner, "files");
    files
}

/// Presses `keys` like a player would: held until the game has read them, then let go
/// until it has seen them released. Menu screens can take several video frames to
/// draw, so a one-frame tap may fall between two reads.
fn press(runner: &mut Runner, keys: u16) -> Probe {
    for held in [keys, 0] {
        let read = runner.expect_probe().frame + 2;
        runner.run_until(30, held, |p| p.frame >= read);
    }
    runner.expect_probe()
}

/// The pause menu's state from the probe: tab, item, and whether it is inspected.
fn menu(probe: &Probe) -> (u8, u8, bool) {
    (probe.menu as u8, (probe.menu >> 8) as u8, probe.menu >> 16 != 0)
}

/// From power-on to control of the wassie in level 1, skipping the arrival.
fn start_level(runner: &mut Runner) -> Probe {
    boot_to_title(runner);
    runner.tap(keys::START);
    runner.run_until(10, 0, |p| scene(p) == Scene::Intro);
    runner.run_until(INTRO_FRAMES as usize + 10, 0, |p| scene(p) == Scene::Arrival);
    runner.tap(keys::START);
    runner.run_until(5, 0, |p| scene(p) == Scene::Play)
}

#[test]
fn boots_to_title_and_counts_frames() {
    let mut runner = Runner::new(&read_rom());
    let first = boot_to_title(&mut runner);
    runner.frames(30, 0);
    let later = runner.expect_probe();
    assert_eq!(scene(&later), Scene::Title);
    assert!(later.frame >= first.frame + 29, "game loop stalled: {first:?} → {later:?}");
}

#[test]
fn the_splash_ignores_start_until_its_prompt_shows() {
    let mut runner = Runner::new(&read_rom());
    runner.run_until(5 * SECOND, 0, |p| p.frame > 0);
    runner.tap(keys::START);
    runner.frames(10, 0);
    assert_eq!(scene(&runner.expect_probe()), Scene::Splash, "too soon to skip");
    boot_to_title(&mut runner);
}

#[test]
fn play_shows_the_level_card_then_hands_over_control() {
    let mut runner = Runner::new(&read_rom());
    let start = start_level(&mut runner);
    assert_eq!(start.level, 0);
    assert_eq!(start.hp, diamond_hands_logic::player::MAX_HP);
}

#[test]
fn the_wassie_runs_and_jumps() {
    let mut runner = Runner::new(&read_rom());
    let start = start_level(&mut runner);
    runner.frames(12, keys::A);
    let airborne = runner.expect_probe();
    assert_eq!(airborne.player_state, PlayerState::Jump as u32);
    assert!(airborne.player_y < start.player_y - 20, "only reached y={}", airborne.player_y);

    runner.frames(40, 0);
    runner.frames(30, keys::RIGHT);
    let moved = runner.expect_probe();
    assert!(moved.player_x > start.player_x + 20, "{start:?} → {moved:?}");
}

#[test]
fn holding_a_hovers_and_the_blaster_aims_diagonally() {
    let mut runner = Runner::new(&read_rom());
    let start = start_level(&mut runner);
    screenshot(&runner, "level-start");

    // A held jump climbs past a plain jump's apex on the jetpack.
    let mut highest = start.player_y;
    let mut hovered = false;
    for i in 0..70 {
        runner.frame(keys::A);
        let p = runner.expect_probe();
        highest = highest.min(p.player_y);
        hovered |= p.player_state == PlayerState::Hover as u32;
        if i == 40 {
            screenshot(&runner, "hover");
        }
    }
    assert!(hovered, "never lit the jetpack");
    let rise = start.player_y - highest;
    assert!(rise > JUMP_APEX + 16, "rose {rise}px");

    // Planted with R, Up+Right fires up and to the right without walking.
    runner.frames(60, 0);
    let planted = runner.expect_probe();
    let aim = keys::R | keys::UP | keys::RIGHT;
    runner.frames(3, aim);
    runner.frame(aim | keys::B);
    // A mine rolls in here, and the bolt can leave the muzzle right on top of it: a
    // bolt in flight or a kill both mean it fired.
    let fired = runner.expect_probe();
    assert!(fired.shots == 1 || fired.kills > planted.kills, "{fired:?}");
    runner.frames(3, aim);
    screenshot(&runner, "aim-diagonal");
    let after = runner.expect_probe();
    assert_eq!(after.player_x, planted.player_x, "planted feet");

    // Down+Left in the air fires down and back.
    runner.frames(20, 0);
    runner.frames(8, keys::A);
    runner.frame(keys::DOWN | keys::LEFT | keys::B);
    runner.frames(3, keys::DOWN | keys::LEFT);
    screenshot(&runner, "aim-down");
    runner.frames(3, keys::UP);
    runner.frame(keys::UP | keys::B);
    runner.frames(3, keys::UP);
    screenshot(&runner, "aim-up");
}

#[test]
fn b_fires_the_blaster() {
    let mut runner = Runner::new(&read_rom());
    start_level(&mut runner);
    runner.tap(keys::B);
    assert_eq!(runner.expect_probe().shots, 1);
}

#[test]
fn start_pauses_and_resumes() {
    let mut runner = Runner::new(&read_rom());
    start_level(&mut runner);
    runner.frames(10, keys::RIGHT);
    let paused = press(&mut runner, keys::START);
    assert_eq!(scene(&paused), Scene::Paused);
    runner.frames(30, keys::RIGHT);
    assert_eq!(runner.expect_probe().player_x, paused.player_x);
    let resumed = press(&mut runner, keys::START);
    assert_eq!(scene(&resumed), Scene::Play);
    // It keeps its run-up, so it coasts a little in the frames since.
    assert!((resumed.player_x - paused.player_x).abs() <= 4, "the level carries on where it stopped");
}

/// The pause menu: tabs, the bag's items and the inspect card, and B and Start backing
/// out, with the world frozen throughout. Screenshots each page.
#[test]
fn the_pause_menu_shows_the_controls_bag_and_wallet() {
    let mut runner = Runner::new(&read_rom());
    start_level(&mut runner);
    runner.frames(20, keys::RIGHT);
    let before = runner.expect_probe();
    let paused = press(&mut runner, keys::START);
    assert_eq!((scene(&paused), menu(&paused)), (Scene::Paused, (pause_menu::BAG, 0, false)));
    runner.frames(20, 0);
    screenshot(&runner, "menu-bag");

    let mut probe = press(&mut runner, keys::DOWN);
    assert_eq!(menu(&probe), (pause_menu::BAG, 1, false));
    runner.frames(6, 0);
    screenshot(&runner, "menu-bag-jetpack");
    probe = press(&mut runner, keys::A);
    assert_eq!(menu(&probe), (pause_menu::BAG, 1, true));
    runner.frames(10, 0);
    screenshot(&runner, "menu-inspect-jetpack");
    probe = press(&mut runner, keys::B);
    assert_eq!((scene(&probe), menu(&probe)), (Scene::Paused, (pause_menu::BAG, 1, false)), "B closes the card");
    for i in 0..ITEMS.len() {
        probe = press(&mut runner, keys::DOWN);
        assert_eq!(probe.menu >> 8 & 0xFF, ((2 + i) % ITEMS.len()) as u32, "the list wraps");
    }

    probe = press(&mut runner, keys::RIGHT);
    assert_eq!(menu(&probe).0, pause_menu::WALLET);
    runner.frames(10, 0);
    screenshot(&runner, "menu-wallet");
    probe = press(&mut runner, keys::RIGHT);
    assert_eq!(menu(&probe).0, pause_menu::CONTROLS, "tabs wrap");
    runner.frames(10, 0);
    screenshot(&runner, "menu-controls");

    assert_eq!((probe.player_x, probe.diamonds), (before.player_x, before.diamonds), "frozen");
    probe = press(&mut runner, keys::B);
    assert_eq!(scene(&probe), Scene::Play, "B closes the menu");
    assert_eq!(probe.shots, 0, "and doesn't fire on the way out");
    probe = press(&mut runner, keys::START);
    assert_eq!((scene(&probe), menu(&probe).0), (Scene::Paused, pause_menu::CONTROLS), "it reopens where it was");
    probe = press(&mut runner, keys::START);
    assert_eq!(scene(&probe), Scene::Play);
}

#[test]
fn difficulty_persists_across_a_power_cycle() {
    let rom = read_rom();
    let mut runner = Runner::new(&rom);
    let title = boot_to_title(&mut runner);
    assert_eq!(title.difficulty, Difficulty::Normal as u32);

    runner.tap(keys::UP);
    assert_eq!(runner.expect_probe().cursor, title_menu::OPTIONS as u32);
    runner.tap(keys::A);
    runner.run_until(5, 0, |p| scene(p) == Scene::Options);
    runner.tap(keys::LEFT);
    assert_eq!(runner.expect_probe().difficulty, Difficulty::Chill as u32);
    runner.tap(keys::B);
    runner.run_until(5, 0, |p| scene(p) == Scene::Title);
    runner.frames(10, 0);
    let save = runner.save_data();
    assert!(!save.is_empty(), "cartridge has no backup memory: SRAM marker missing?");

    let mut rebooted = Runner::with_save(&rom, Some(&save));
    let title = boot_to_title(&mut rebooted);
    assert_eq!(title.difficulty, Difficulty::Chill as u32);
}

#[test]
fn a_file_continues_from_its_beacon_after_a_power_cycle() {
    let rom = read_rom();
    let mut runner = Runner::new(&rom);
    start_level(&mut runner);
    let mut previous = 0;
    let mut world = Game::new(0).world;
    let lit = loop {
        // The route bot only reads the level and the wassie, which the probe can't
        // carry, so drive the ROM with a host model of the same run.
        let p = runner.expect_probe();
        if p.checkpoints > 0 {
            break p;
        }
        let keys = bot::keys_for(&world, previous);
        runner.frame(keys);
        world.step(&Input::from_keys(keys, previous));
        previous = keys;
        assert!(runner.expect_probe().frame < 60 * SECOND as u32, "no beacon reached");
    };
    runner.frames(10, 0);
    let save = runner.save_data();

    let mut rebooted = Runner::with_save(&rom, Some(&save));
    boot_to_splash_prompt(&mut rebooted);
    boot_to_files_from_prompt(&mut rebooted);
    rebooted.tap(keys::A);
    rebooted.run_until(5, 0, |p| scene(p) == Scene::FileChoice);
    screenshot(&rebooted, "file-choice");
    rebooted.tap(keys::A);
    rebooted.run_until(5, 0, |p| scene(p) == Scene::Title);
    rebooted.tap(keys::START);
    let resumed = rebooted.run_until(INTRO_FRAMES as usize + 30, 0, |p| scene(p) == Scene::Play);
    assert_eq!(resumed.checkpoints, lit.checkpoints);
    assert_eq!(resumed.diamonds, lit.diamonds);
    assert!(
        (resumed.player_x - lit.player_x).abs() < 32,
        "should stand at the beacon: {lit:?} → {resumed:?}"
    );
}

#[test]
fn the_map_starts_the_badlands_and_keeps_the_trenches_locked() {
    let mut runner = Runner::new(&read_rom());
    boot_to_title(&mut runner);
    runner.tap(keys::DOWN);
    assert_eq!(runner.expect_probe().cursor, title_menu::MAP as u32);
    runner.tap(keys::A);
    runner.run_until(5, 0, |p| scene(p) == Scene::Map);
    runner.frames(2, 0);
    screenshot(&runner, "map-badlands");
    let first = runner.frame_hash();
    runner.tap(keys::RIGHT);
    runner.frames(2, 0);
    assert_eq!(runner.expect_probe().cursor, 1);
    assert_ne!(runner.frame_hash(), first, "the map shows the other place");
    screenshot(&runner, "map-solana");
    runner.tap(keys::A);
    runner.frames(2, 0);
    assert_eq!(scene(&runner.expect_probe()), Scene::Map, "locked on a new file");
    runner.tap(keys::LEFT);
    runner.tap(keys::A);
    runner.run_until(10, 0, |p| scene(p) == Scene::Intro);
}

/// Saves the screen as a binary PPM when `DIAMOND_HANDS_SHOTS` names a directory.
fn screenshot(runner: &Runner, name: &str) {
    let Some(dir) = std::env::var_os("DIAMOND_HANDS_SHOTS") else {
        return;
    };
    let mut out = b"P6\n240 160\n255\n".to_vec();
    for px in runner.framebuffer().chunks(4) {
        out.extend_from_slice(&px[..3]);
    }
    let path = std::path::Path::new(&dir).join(format!("{name}.ppm"));
    std::fs::write(&path, out).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

/// The strongest check: the ROM and the host-side `Game` take the same inputs and must
/// agree on every field of the probe, every frame, through the route bot's run of the
/// Badlands: running, jumping, shooting, getting hit, the boss's dialogue and the
/// fight. With `DIAMOND_HANDS_SHOTS` set it also saves screenshots along the way.
#[test]
fn rom_matches_host_model_frame_by_frame() {
    let mut runner = Runner::new(&read_rom());
    clear_badlands_in_step(&mut runner);
}

/// Plays file 1 from power-on through the Badlands with the route bot, stepping the
/// host model alongside and checking the two agree every frame, up to the level-clear
/// screen. Returns the model.
fn clear_badlands_in_step(runner: &mut Runner) -> Game {
    let ready = boot_to_splash_prompt(runner);
    let mut model = Game::new(ready.best);
    while model.frame < ready.frame {
        model.step(&Input::default());
    }
    assert_eq!(Probe::from_game(&model), ready, "out of sync at the splash");

    // The ROM takes the next frame's Start and then builds its renderer, so it runs
    // slower than the emulator until the file menu is up. Catch the model up there.
    let files = boot_to_files_from_prompt(runner);
    model.step(&Input::from_keys(keys::START, 0));
    while model.frame < files.frame {
        model.step(&Input::default());
    }
    assert_eq!(Probe::from_game(&model), files, "out of sync at the file menu");

    let mut previous = 0u16;
    let mut saw_hit = false;
    let mut saw_dialog = false;
    let mut dialog_frames = 0;
    let mut arrival_frames = 0;
    let mut beats_seen = 0;
    for i in 0..(100 * SECOND) {
        let keys = match model.scene {
            Scene::Files => keys::A,
            Scene::Title if i == 20 => keys::START,
            Scene::Play => bot::keys_for(&model.world, previous),
            // Read each line for a moment, then press on.
            Scene::Dialog | Scene::Arrival if model.talking() => {
                dialog_frames += 1;
                if dialog_frames % 50 == 0 { keys::A } else { 0 }
            }
            _ => 0,
        };
        runner.frame(keys);
        model.step(&Input::from_keys(keys, previous));
        previous = keys;
        saw_hit |= model.world.player.hp < diamond_hands_logic::player::MAX_HP;

        let probe = runner.expect_probe();
        assert_eq!(probe, Probe::from_game(&model), "ROM diverged from the model at step {i}");
        saw_dialog |= model.scene == Scene::Dialog;
        let talk = if model.scene == Scene::Arrival { "arrival-talk" } else { "dialog" };
        if model.talking() && dialog_frames % 50 == 49 {
            screenshot(runner, &format!("{talk}-{}", model.talk_line));
        }
        if model.scene == Scene::Play && model.world.boss.is_active() && i % 10 == 0 {
            screenshot(runner, &format!("boss-{i:05}"));
        }
        if model.scene == Scene::Arrival {
            beats_seen |= 1 << model.beat as u32;
            arrival_frames += 1;
            if !model.talking() && arrival_frames % 30 == 0 {
                screenshot(runner, &format!("arrival-{arrival_frames:04}"));
            }
        }
        if model.scene == Scene::LevelClear {
            break;
        }
    }
    let end = runner.expect_probe();
    assert!(end.kills > 0 && end.diamonds > 0, "{end:?}");
    assert!(saw_hit, "the run should exercise damage too");
    assert!(saw_dialog, "BERA 3000 should have her say");
    assert_eq!(beats_seen, 0b11_1111, "CL207's welcome should play every beat");
    assert_eq!(scene(&end), Scene::LevelClear, "the bot should beat the Badlands: {end:?}");
    model
}

/// Clearing the Badlands: the clear screen hands over to the world map on its own,
/// where the Badlands shows cleared and the wassie walks to the newly open Trenches.
/// Level 2 plays, and its pause menu works with both levels' terrain in VRAM, the
/// tightest it gets. After a power cycle the file still has the Trenches open.
#[test]
fn clearing_the_badlands_opens_the_map_and_unlocks_the_trenches_for_good() {
    let rom = read_rom();
    let mut runner = Runner::new(&rom);
    clear_badlands_in_step(&mut runner);
    for (frames, name) in [(20, "level-clear-burst"), (60, "level-clear-tally"), (60, "level-clear-rain")] {
        runner.frames(frames, 0);
        screenshot(&runner, name);
    }
    let map = runner.run_until(CLEAR_AUTO_FRAMES as usize + SECOND, 0, |p| scene(p) == Scene::Map);
    assert_eq!((map.cleared, map.cursor), (0b1, 1), "the Badlands is cleared, the Trenches picked");
    runner.frames(MAP_WALK_FRAMES as usize / 2, 0);
    screenshot(&runner, "map-walk");
    runner.frames(MAP_WALK_FRAMES as usize, 0);
    screenshot(&runner, "map-unlocked");

    let intro = press(&mut runner, keys::A);
    assert_eq!((scene(&intro), intro.level), (Scene::Intro, 1));
    let play = runner.run_until(INTRO_FRAMES as usize + SECOND, 0, |p| scene(p) == Scene::Play);
    assert_eq!(play.level, 1, "no arrival scene in the Trenches");
    runner.frames(60, keys::RIGHT);
    screenshot(&runner, "trenches");

    // Every page of the menu, with the most terrain loaded.
    let x = press(&mut runner, keys::START).player_x;
    for keys in [keys::A, keys::B, keys::DOWN, keys::A, keys::B, keys::RIGHT, keys::RIGHT, keys::RIGHT] {
        let probe = press(&mut runner, keys);
        assert_eq!(scene(&probe), Scene::Paused, "{probe:?}");
    }
    let resumed = press(&mut runner, keys::START);
    assert_eq!(scene(&resumed), Scene::Play);
    assert!((resumed.player_x - x).abs() <= 4, "carries on from where it paused");

    runner.frames(10, 0);
    let save = runner.save_data();
    let mut rebooted = Runner::with_save(&rom, Some(&save));
    boot_to_splash_prompt(&mut rebooted);
    boot_to_files_from_prompt(&mut rebooted);
    rebooted.tap(keys::A);
    rebooted.run_until(5, 0, |p| scene(p) == Scene::FileChoice);
    rebooted.tap(keys::A);
    let title = rebooted.run_until(5, 0, |p| scene(p) == Scene::Title);
    assert_eq!(title.cleared, 0b1, "the file remembers the clear");
    rebooted.tap(keys::A);
    let map = rebooted.run_until(5, 0, |p| scene(p) == Scene::Map);
    assert_eq!(map.cursor, 1, "PLAY opens the map on the Trenches");
    let intro = press(&mut rebooted, keys::A);
    assert_eq!((scene(&intro), intro.level), (Scene::Intro, 1));
}
