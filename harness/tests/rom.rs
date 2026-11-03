//! Boots the built ROM on the gba-wasm core. Requires `dist/diamond-hands.gba`
//! (scripts/build-rom.sh) or `DIAMOND_HANDS_ROM`.
//!
//! When one of these fails, run the same ROM in mGBA. If it behaves there, the bug is
//! in gba-wasm: record it in gba-wasm (failing test or TASKS.md) and mark the test here
//! `#[ignore = "gba-wasm: <issue>"]`.

use diamond_hands_harness::{Runner, keys, read_rom};
use diamond_hands_logic::{
    Difficulty, Game, Input, PlayerState, Probe, Scene, bot,
    game::{INTRO_FRAMES, title_menu},
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
        other => panic!("unknown scene {other}"),
    }
}

fn boot_to_title(runner: &mut Runner) -> Probe {
    let title = runner.run_until(5 * SECOND, 0, |p| scene(p) == Scene::Title && p.frame > 0);
    screenshot(runner, "title");
    title
}

/// From power-on to control of the wassie in level 1.
fn start_level(runner: &mut Runner) -> Probe {
    boot_to_title(runner);
    runner.tap(keys::START);
    runner.run_until(10, 0, |p| scene(p) == Scene::Intro);
    runner.run_until(INTRO_FRAMES as usize + 10, 0, |p| scene(p) == Scene::Play)
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
    runner.frames(3, aim);
    screenshot(&runner, "aim-diagonal");
    let after = runner.expect_probe();
    assert_eq!(after.player_x, planted.player_x, "planted feet");
    assert_eq!(after.shots, 1);

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
    runner.tap(keys::START);
    let paused = runner.run_until(5, 0, |p| scene(p) == Scene::Paused);
    runner.frames(30, keys::RIGHT);
    assert_eq!(runner.expect_probe().player_x, paused.player_x);
    runner.tap(keys::START);
    runner.run_until(5, 0, |p| scene(p) == Scene::Play);
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
fn the_map_starts_the_badlands_and_shows_coming_soon_places() {
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
    assert_eq!(scene(&runner.expect_probe()), Scene::Map, "not built yet");
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
    let title = boot_to_title(&mut runner);

    let mut model = Game::new(title.best);
    while model.frame < title.frame {
        model.step(&Input::default());
    }
    assert_eq!(Probe::from_game(&model), title, "out of sync at the title screen");

    let mut previous = 0u16;
    let mut saw_hit = false;
    let mut saw_dialog = false;
    let mut dialog_frames = 0;
    for i in 0..(75 * SECOND) {
        let keys = match model.scene {
            Scene::Title if i == 20 => keys::START,
            Scene::Play => bot::keys_for(&model.world, previous),
            // Read each line for a moment, then press on.
            Scene::Dialog => {
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
        if model.scene == Scene::Dialog && dialog_frames % 50 == 49 {
            screenshot(&runner, &format!("dialog-{}", model.talk_line));
        }
        if model.scene == Scene::LevelClear {
            break;
        }
    }
    let end = runner.expect_probe();
    assert!(end.kills > 0 && end.diamonds > 0, "{end:?}");
    assert!(saw_hit, "the run should exercise damage too");
    assert!(saw_dialog, "URSA-9000 should have her say");
    assert_eq!(scene(&end), Scene::LevelClear, "the bot should beat the Badlands: {end:?}");
}
