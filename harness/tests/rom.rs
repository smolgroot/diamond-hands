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
        12 => Scene::Exchange,
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
    let mut files = runner.run_until(5 * SECOND, 0, |p| p.frame > entered.frame + 1);
    // Building the renderer and its first frames run long, by an amount that shifts
    // with what they load: wait until the ROM keeps pace, a game frame per video frame.
    let mut steady = 0;
    while steady < 3 {
        runner.frame(0);
        let p = runner.expect_probe();
        steady = if p.frame == files.frame + 1 { steady + 1 } else { 0 };
        files = p;
        assert!(files.frame < entered.frame + 60, "the ROM never kept pace");
    }
    screenshot(runner, "files");
    files
}

/// Presses `keys` like a player would: held until the game has read them, then let go
/// until it has seen them released. Menu screens can take several video frames to
/// draw (the map paints itself when it opens, a level repaints its backdrop), so a
/// one-frame tap may fall between two reads.
fn press(runner: &mut Runner, keys: u16) -> Probe {
    for held in [keys, 0] {
        let read = runner.expect_probe().frame + 2;
        runner.run_until(5 * SECOND, held, |p| p.frame >= read);
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
fn the_badlands_jetpack_is_lost_and_the_blaster_aims_diagonally() {
    let mut runner = Runner::new(&read_rom());
    let start = start_level(&mut runner);
    screenshot(&runner, "level-start");

    // The Badlands hide the jetpack in the deep: a held jump is only a jump.
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
    assert!(!hovered, "hovered without the jetpack");
    let rise = start.player_y - highest;
    assert!(rise <= JUMP_APEX + 1, "rose {rise}px");

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
        let item = (2 + i) % ITEMS.len();
        assert_eq!(probe.menu >> 8 & 0xFF, item as u32, "the list wraps");
        runner.frames(6, 0);
        assert_eq!(lit_names(&runner), [item], "only the selected name is lit");
        screenshot(&runner, &format!("menu-bag-{item}"));
        probe = press(&mut runner, keys::A);
        assert_eq!(menu(&probe), (pause_menu::BAG, item as u8, true));
        runner.frames(10, 0);
        screenshot(&runner, &format!("menu-inspect-{item}"));
        probe = press(&mut runner, keys::B);
        assert!(!menu(&probe).2, "B closes the card");
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
    let mut bot = bot::Bot::default();
    let mut world = Game::new(0).world;
    let lit = loop {
        // The route bot only reads the level and the wassie, which the probe can't
        // carry, so drive the ROM with a host model of the same run.
        let p = runner.expect_probe();
        if p.checkpoints > 0 {
            break p;
        }
        let keys = bot.keys(&world, previous);
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
        (resumed.player_x - lit.player_x).abs() < 32 && (resumed.player_y - lit.player_y).abs() < 32,
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
    // The map paints itself as it opens.
    let open = runner.expect_probe().frame + 2;
    runner.run_until(5 * SECOND, 0, |p| p.frame >= open);
    screenshot(&runner, "map-badlands");
    let first = runner.frame_hash();
    let moved = press(&mut runner, keys::RIGHT);
    runner.frames(2, 0);
    assert_eq!(moved.cursor, 1);
    assert_ne!(runner.frame_hash(), first, "the map shows the other place");
    screenshot(&runner, "map-solana");
    press(&mut runner, keys::A);
    runner.frames(2, 0);
    assert_eq!(scene(&runner.expect_probe()), Scene::Map, "locked on a new file");
    press(&mut runner, keys::LEFT);
    press(&mut runner, keys::A);
    runner.run_until(10, 0, |p| scene(p) == Scene::Intro);
}

/// The bag rows whose name shows in white, the selection's colour (the others are
/// steel). The rows are where `game/src/art/menu.rs` puts them: from y = `LIST_Y`
/// every `LIST_STEP`, names from `LIST_NAME_X` to `RIGHT` (the badges are on the line
/// below).
fn lit_names(runner: &Runner) -> Vec<usize> {
    let fb = runner.framebuffer();
    let white = |x: usize, y: usize| fb[(y * 240 + x) * 4..][..3] == [255, 255, 255];
    (0..ITEMS.len())
        .filter(|i| {
            let top = 24 + 16 * i;
            (top..top + 8).any(|y| (128..232).any(|x| white(x, y)))
        })
        .collect()
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
    clear_badlands_with_keys(runner).0
}

/// [`clear_badlands_in_step`], also returning the keys held on the last frame.
fn clear_badlands_with_keys(runner: &mut Runner) -> (Game, u16) {
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
    let mut bot = bot::Bot::default();
    let mut saw_hit = false;
    let mut saw_dialog = false;
    let mut dialog_frames = 0;
    let mut arrival_frames = 0;
    let mut beats_seen = 0;
    let mut waypoint = 0;
    // The explorable Badlands are a long way round, with a knockout or two on the way.
    for i in 0..(240 * SECOND) {
        let keys = match model.scene {
            Scene::Files => keys::A,
            Scene::Title if i == 20 => keys::START,
            Scene::Play => bot.keys(&model.world, previous),
            // Read each line for a moment, then press on.
            Scene::Dialog | Scene::Arrival if model.talking() => {
                dialog_frames += 1;
                if dialog_frames % 50 == 0 { keys::A } else { 0 }
            }
            _ => 0,
        };
        step_in_sync(runner, &mut model, keys, previous, &format!("step {i}"));
        previous = keys;
        // Each stop on the way round: the pit, the caves, the deep, the jetpack, the
        // shaft's girders and the plateau.
        if bot.step() != waypoint {
            waypoint = bot.step();
            screenshot(runner, &format!("b1-{waypoint:02}"));
        }
        saw_hit |= model.world.player.hp < diamond_hands_logic::player::MAX_HP;
        saw_dialog |= model.scene == Scene::Dialog;
        let talk = if model.scene == Scene::Arrival { "arrival-talk" } else { "dialog" };
        if model.talking() && dialog_frames % 50 == 49 {
            screenshot(runner, &format!("{talk}-{}", model.talk_line));
        }
        // The route bot waiting at an electric fence: its curtain warning and live.
        let w = &model.world;
        let r = w.player.body.rect();
        if model.scene == Scene::Play && i % 8 == 0 && w.level.fence_hit(r.right(), r.y, 24, r.h).is_some() {
            screenshot(runner, &format!("fence-{i:05}"));
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
    (model, previous)
}

/// Steps the ROM and the model one game frame with `keys`. The ROM may take several
/// video frames over one (entering a level repaints its backdrop), so the keys are held
/// until it has read them; and after a frame that ran long it may catch up with two
/// game frames in one video frame, both reading the same keys, so the model steps as
/// often. Panics if the two disagree.
fn step_in_sync(runner: &mut Runner, model: &mut Game, keys: u16, previous: u16, what: &str) {
    let next = model.frame + 1;
    let probe = runner.run_until(10 * SECOND, keys, |p| p.frame >= next);
    model.step(&Input::from_keys(keys, previous));
    while model.frame < probe.frame {
        model.step(&Input::from_keys(keys, keys));
    }
    assert_eq!(probe, Probe::from_game(model), "ROM diverged from the model: {what}");
}

/// Level 2 end to end on the ROM, in step with the model: CL207's call, the street, the
/// casino floor, down to the back room and the sewers, the breaker, back up the shaft,
/// the elevator, the pause menu up the tower, SAM's fight on the rooftop, the penguin's
/// exchange (accepted) and the clear, which unlocks level 3 on the map. Screenshots
/// each area with `DIAMOND_HANDS_SHOTS` set.
#[test]
fn the_trenches_play_through_to_the_exchange_in_step_with_the_model() {
    use diamond_hands_logic::{boss::BossState, game::Deal, world::LiftState};
    let mut runner = Runner::new(&read_rom());
    let (mut model, mut previous) = clear_badlands_with_keys(&mut runner);
    let mut bot = bot::Bot::default();
    let mut seen = std::collections::BTreeSet::new();
    let mut shot = |runner: &Runner, name: &str| {
        if seen.insert(name.to_string()) {
            screenshot(runner, name);
        }
    };
    let mut wallet_before = None;
    let mut diamonds_here = 0;
    let mut coins_offered = 0;
    let (mut rode, mut swung, mut crashed, mut overloaded) = (false, false, false, false);
    let mut talk_frames = 0u32;
    let mut paused = false;
    for i in 0..(400 * SECOND) {
        let level2 = model.world.level_index == 1;
        let keys = match model.scene {
            Scene::LevelClear if model.scene_frames() > diamond_hands_logic::game::CLEAR_LOCK_FRAMES + 4 => {
                if i % 2 == 0 { keys::A } else { 0 }
            }
            Scene::Map if model.scene_frames() > MAP_WALK_FRAMES + 8 && model.cleared == 0b1 => {
                if i % 2 == 0 { keys::A } else { 0 }
            }
            // Up the tower, with every storey's terrain made, the pause menu opens and
            // closes: background VRAM is at its fullest here.
            Scene::Play if level2 && !paused && model.world.camera_y < 100 && model.world.camera_x >= 1600 => {
                paused = true;
                keys::START
            }
            // Every page: the bag, an item's card, the wallet and the controls.
            Scene::Paused => match model.scene_frames() {
                20 => {
                    shot(&runner, "t2-pause-tower");
                    keys::A
                }
                40 => {
                    shot(&runner, "t2-pause-tower-card");
                    keys::B
                }
                60 | 80 => keys::RIGHT,
                100 => {
                    shot(&runner, "t2-pause-tower-controls");
                    keys::START
                }
                _ => 0,
            },
            Scene::Play => bot.keys(&model.world, previous),
            Scene::Dialog | Scene::Exchange if model.talking() => {
                talk_frames += 1;
                if talk_frames % 40 == 0 { keys::A } else { 0 }
            }
            _ => 0,
        };
        let what = format!("step {i}, {:?}", model.scene);
        step_in_sync(&mut runner, &mut model, keys, previous, &what);
        previous = keys;
        if model.scene == Scene::Map && model.cleared == 0b11 {
            if model.scene_frames() == MAP_WALK_FRAMES / 2 {
                shot(&runner, "t2-map-walk");
            }
            if model.scene_frames() == MAP_WALK_FRAMES + 20 {
                shot(&runner, "t2-map-ethereum-unlocked");
                break;
            }
            continue;
        }
        if !level2 && model.world.level_index != 1 {
            continue;
        }
        let w = &model.world;
        match model.scene {
            Scene::Intro if model.scene_frames() == 60 => shot(&runner, "t2-intro"),
            Scene::Dialog if model.ringing() && model.scene_frames() == 30 => shot(&runner, "t2-call-ringing"),
            Scene::Dialog if model.talking() && model.talk_line < 2 && talk_frames % 40 == 30 => {
                shot(&runner, &format!("t2-call-{}", model.talk_line))
            }
            Scene::Dialog if model.talking() && talk_frames % 40 == 30 && w.arena => {
                shot(&runner, &format!("t2-sam-talk-{}", model.talk_line))
            }
            Scene::Play => {
                let cam = w.camera_x;
                // Which storey the camera is on: the tower, the street, the back room or
                // the sewers.
                let storey = match w.camera_y {
                    0..=100 => 0,
                    101..=300 => 1,
                    301..=450 => 2,
                    _ => 3,
                };
                let places = [
                    (1, 150, "t2-street"),
                    (1, 420, "t2-street-awning"),
                    (1, 640, "t2-casino-entrance"),
                    (1, 760, "t2-casino-tables"),
                    (1, 860, "t2-casino-sunken-bar"),
                    (1, 1000, "t2-casino-balcony"),
                    (1, 1120, "t2-secret-stage"),
                    (1, 1220, "t2-casino-hole"),
                    (2, 1150, "t2-back-mezzanine"),
                    (2, 1000, "t2-back-ramp"),
                    (2, 850, "t2-back-stage"),
                    (2, 700, "t2-back-slots"),
                    (3, 640, "t2-sewer-drain"),
                    (3, 800, "t2-sewer-channel"),
                    (3, 1000, "t2-sewer-pipe"),
                    (3, 1300, "t2-sewer-breaker"),
                    (0, 1560, "t2-suites"),
                    (0, 1700, "t2-suites-vault"),
                ];
                for (on, x, name) in places {
                    if storey == on && cam >= x && cam < x + 4 {
                        shot(&runner, name);
                    }
                }
                let r = w.player.body.rect();
                let in_shaft = (196 * 8..203 * 8).contains(&r.centre_x());
                if in_shaft && w.player.grounded {
                    shot(&runner, &format!("t2-shaft-{storey}"));
                }
                let r = w.player.body.rect();
                if w.level.fence_hit(r.right(), r.y, 24, r.h).is_some() {
                    let state = diamond_hands_logic::world::fence_state(w.frames);
                    shot(&runner, &format!("t2-fence-{}-{state}", r.x / 256));
                }
                if let Some(lift) = w.lift {
                    match lift.state {
                        LiftState::Powering if lift.timer == 40 => shot(&runner, "t2-lift-powering"),
                        LiftState::Open if lift.timer == 4 => shot(&runner, "t2-lift-open"),
                        LiftState::Riding if lift.timer == 12 => shot(&runner, "t2-lift-riding"),
                        LiftState::Arriving if lift.timer == 10 => shot(&runner, "t2-lift-arriving"),
                        LiftState::Done => rode = true,
                        _ => {}
                    }
                }
                let b = &w.boss;
                match b.state {
                    BossState::Throw if b.timer == 10 => shot(&runner, "t2-sam-throw"),
                    BossState::Windup if b.timer == 20 => {
                        shot(&runner, if b.phase() == 1 { "t2-sam-windup" } else { "t2-sam-windup-overloaded" })
                    }
                    BossState::Swing => {
                        swung = true;
                        if b.timer == 12 {
                            shot(&runner, "t2-sam-cucumber-hit");
                        }
                    }
                    BossState::Barrage => {
                        crashed = true;
                        if b.timer == 50 {
                            shot(&runner, "t2-sam-market-crash");
                        }
                    }
                    BossState::Overload => {
                        overloaded = true;
                        if b.timer == 30 {
                            shot(&runner, "t2-sam-overload");
                        }
                    }
                    BossState::Dying if b.timer == 90 => shot(&runner, "t2-sam-defeat"),
                    BossState::Dying if b.timer == 20 => shot(&runner, "t2-sam-down"),
                    _ => {}
                }
            }
            Scene::Exchange => {
                wallet_before.get_or_insert(model.wallet);
                diamonds_here = model.world.diamonds;
                let t = model.scene_frames();
                match model.exchange.deal {
                    Deal::Walk if t == 40 => shot(&runner, "t2-penguin-arrives"),
                    Deal::Pitch if talk_frames % 40 == 30 => shot(&runner, &format!("t2-penguin-pitch-{}", model.talk_line)),
                    Deal::Offer => {
                        coins_offered = model.exchange.coins;
                        if t == 20 {
                            shot(&runner, "t2-exchange-offer");
                        }
                    }
                    Deal::Payout if t == 45 => shot(&runner, "t2-exchange-payout"),
                    Deal::Bye if t == 30 => shot(&runner, "t2-exchange-bye"),
                    _ => {}
                }
            }
            Scene::LevelClear if model.scene_frames() == 60 => shot(&runner, "t2-level-clear"),
            _ => {}
        }
    }
    let end = runner.expect_probe();
    assert!(paused, "the pause menu opened up the tower");
    assert!(rode, "the bot rode the elevator");
    assert!(swung && crashed && overloaded, "SAM used his attacks: hit {swung}, crash {crashed}, overload {overloaded}");
    assert!(coins_offered > 0, "the penguin had coins to change");
    assert_eq!(end.cleared, 0b11, "the Trenches are cleared: {end:?}");
    assert_eq!((scene(&end), end.cursor), (Scene::Map, 2), "the map moves on to level 3");
    let expected = wallet_before.unwrap() + coins_offered * diamond_hands_logic::game::EXCHANGE_RATE + diamonds_here;
    assert_eq!(end.wallet, expected, "8 diamonds a coin, plus the level's own, banked once");
    assert_eq!(end.purse, 0);
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
    // CL207 calls on the first visit; Start ignores the call.
    let call = runner.run_until(INTRO_FRAMES as usize + SECOND, 0, |p| scene(p) != Scene::Intro);
    assert_eq!((scene(&call), call.level), (Scene::Dialog, 1), "CL207 calls in the Trenches");
    let play = press(&mut runner, keys::START);
    assert_eq!((scene(&play), play.level), (Scene::Play, 1));
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
    // The map paints itself as it opens; the A from the title must be seen let go.
    rebooted.run_until(5 * SECOND, 0, |p| p.frame >= map.frame + 2);
    let intro = press(&mut rebooted, keys::A);
    assert_eq!((scene(&intro), intro.level), (Scene::Intro, 1));
}


