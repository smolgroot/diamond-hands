//! Scene flow around the levels: splash → save files → title ⇄ options, title → level
//! intro → (arrival, on a fresh start) → play ⇄ pause → level clear → next level, or back
//! to the title after the last one. Play stops for the boss's dialogue when the arena closes. The world map (title →
//! map) starts any open level on its own, and a clear there returns to the map.
//!
//! A run started from the title saves to the chosen file at every beacon and level
//! clear; map runs are free play and never touch the files.

use crate::{
    arrival::{self, Beat},
    levels,
    talk::Line,
    world::{Difficulty, Events, Input, World},
};

/// KEYINPUT bit positions, active high (pressed = 1).
pub mod keys {
    pub const A: u16 = 1 << 0;
    pub const B: u16 = 1 << 1;
    pub const SELECT: u16 = 1 << 2;
    pub const START: u16 = 1 << 3;
    pub const RIGHT: u16 = 1 << 4;
    pub const LEFT: u16 = 1 << 5;
    pub const UP: u16 = 1 << 6;
    pub const DOWN: u16 = 1 << 7;
    pub const R: u16 = 1 << 8;
    pub const L: u16 = 1 << 9;

    pub const JUMP: u16 = A;
    pub const FIRE: u16 = B;
    /// Either shoulder: plant the feet and aim.
    pub const PLANT: u16 = L | R;
}

/// Frames the splash shows the cover alone before "PRESS START" appears and the
/// game starts listening, so a button held through power-on doesn't skip it.
pub const SPLASH_PROMPT_FRAMES: u32 = 90;
/// Frames the level card shows before play starts on its own.
pub const INTRO_FRAMES: u32 = 150;
/// Frames the level-clear screen ignores input, so a held button doesn't skip it.
pub const CLEAR_LOCK_FRAMES: u32 = 60;
/// Save files on the cartridge.
pub const FILES: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Scene {
    Title,
    Play,
    Paused,
    LevelClear,
    Options,
    Intro,
    /// The boss and the wassie talk; the world waits.
    Dialog,
    /// The world map: pick any open level.
    Map,
    /// The cover art, shown at power-on until Start is pressed.
    Splash,
    /// Pick a save file.
    Files,
    /// A file with a run in it: continue it or start over.
    FileChoice,
    /// The level's arrival scene: dropped off, welcomed, and handed the gear.
    Arrival,
}

/// A run saved to a file: where to pick it up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    pub level: u8,
    /// Tile of the last beacon lit, or `None` for the level's start.
    pub checkpoint: Option<(i32, i32)>,
    pub beacons: u8,
    /// Score from levels cleared before this one.
    pub banked: u32,
    pub diamonds: u32,
    pub kills: u32,
    /// Frames played in this level.
    pub frames: u32,
    /// Every level is cleared; `banked` is the final score.
    pub complete: bool,
}

impl Progress {
    /// Score so far.
    pub fn score(&self) -> u32 {
        self.banked + World::score_for(self.diamonds, self.kills, false)
    }
}

/// Title menu rows, in [`Game::cursor`] order.
pub mod title_menu {
    pub const PLAY: u8 = 0;
    pub const MAP: u8 = 1;
    pub const OPTIONS: u8 = 2;
    pub const LEN: u8 = 3;
}

/// Rows of the continue-or-start-over choice, in [`Game::cursor`] order.
pub mod file_menu {
    pub const CONTINUE: u8 = 0;
    pub const NEW_GAME: u8 = 1;
    pub const LEN: u8 = 2;
}

/// Options menu rows, in [`Game::cursor`] order.
pub mod options_menu {
    pub const DIFFICULTY: u8 = 0;
    pub const BACK: u8 = 1;
    pub const LEN: u8 = 2;
}

#[derive(Clone, Debug)]
pub struct Game {
    pub scene: Scene,
    pub world: World,
    /// Frames since power-on.
    pub frame: u32,
    /// Best total score across a full playthrough.
    pub best: u32,
    /// Score banked from cleared levels in this playthrough.
    pub banked: u32,
    /// Whether the last cleared level set `best`.
    pub new_best: bool,
    pub difficulty: Difficulty,
    /// Highlighted row of the current menu, or place on the map.
    pub cursor: u8,
    /// The level was picked on the map, so clearing it returns there.
    pub from_map: bool,
    /// Dialogue line showing, and how many of its characters are typed out.
    pub talk_line: u8,
    pub talk_shown: u16,
    /// The saved runs, and the file this session plays in.
    pub files: [Option<Progress>; FILES],
    pub file: u8,
    /// PLAY picks up the file's run rather than starting over.
    pub resume: bool,
    /// Where the arrival scene is, and whether this level start plays it.
    pub beat: Beat,
    arrival_due: bool,
    /// Frames spent in the current scene, for timed screens.
    timer: u32,
}

impl Game {
    pub fn new(best: u32) -> Self {
        Self::with_save(best, Difficulty::default(), [None; FILES])
    }

    pub fn with_save(best: u32, difficulty: Difficulty, files: [Option<Progress>; FILES]) -> Self {
        Self {
            scene: Scene::Splash,
            world: World::new(0, difficulty),
            frame: 0,
            best,
            banked: 0,
            new_best: false,
            difficulty,
            cursor: title_menu::PLAY,
            from_map: false,
            talk_line: 0,
            talk_shown: 0,
            files,
            file: 0,
            resume: false,
            beat: Beat::Landing,
            arrival_due: false,
            timer: 0,
        }
    }

    /// The run PLAY will pick up, if any: the active file's, unless it is finished or
    /// the player chose to start over.
    pub fn resumable(&self) -> Option<Progress> {
        self.files[self.file as usize].filter(|p| self.resume && !p.complete)
    }

    /// Whether the splash is showing its "PRESS START" prompt, and for how many frames.
    pub fn splash_prompt(&self) -> Option<u32> {
        (self.scene == Scene::Splash && self.timer >= SPLASH_PROMPT_FRAMES)
            .then(|| self.timer - SPLASH_PROMPT_FRAMES)
    }

    /// The lines being talked through: the boss's, or the arrival's.
    pub fn talk_lines(&self) -> &'static [Line] {
        match self.arrival() {
            Some(arrival) if self.scene == Scene::Arrival => arrival.lines,
            _ => self.world.level.boss_talk,
        }
    }

    /// Whether a line of dialogue is up.
    pub fn talking(&self) -> bool {
        self.scene == Scene::Dialog
            || (self.scene == Scene::Arrival && matches!(self.beat, Beat::Talk | Beat::Reply))
    }

    /// What the arrival scene shows now, while it plays.
    pub fn arrival_view(&self) -> Option<arrival::View> {
        let start = self.world.start_feet();
        (self.scene == Scene::Arrival).then(|| arrival::view(self.beat, self.timer, start, start.1))
    }

    fn arrival(&self) -> Option<&'static arrival::Arrival> {
        levels::arrival(self.world.level_index)
    }

    /// Score so far in this playthrough.
    pub fn score(&self) -> u32 {
        self.banked + self.world.score()
    }

    pub fn step(&mut self, input: &Input) -> Events {
        self.frame = self.frame.wrapping_add(1);
        self.timer += 1;
        let mut events = Events::default();
        let confirm = input.start_pressed() || input.pressed & crate::keys::A != 0;
        let back = input.pressed & crate::keys::B != 0;

        match self.scene {
            Scene::Splash => {
                if confirm && self.timer > SPLASH_PROMPT_FRAMES {
                    self.open_files();
                }
            }
            Scene::Files => {
                self.move_cursor(input, FILES as u8);
                if confirm {
                    self.file = self.cursor;
                    if self.files[self.file as usize].is_some() {
                        self.enter(Scene::FileChoice);
                        self.cursor = file_menu::CONTINUE;
                    } else {
                        self.resume = false;
                        self.return_to_title(title_menu::PLAY);
                    }
                }
            }
            Scene::FileChoice => {
                self.move_cursor(input, file_menu::LEN);
                if back {
                    self.open_files();
                } else if confirm {
                    // Starting over leaves the file as it is until the new run saves.
                    self.resume = self.cursor == file_menu::CONTINUE;
                    self.return_to_title(title_menu::PLAY);
                }
            }
            Scene::Title => {
                self.move_cursor(input, title_menu::LEN);
                if back {
                    self.open_files();
                } else if confirm {
                    match self.cursor {
                        title_menu::PLAY => {
                            self.from_map = false;
                            match self.resumable() {
                                Some(progress) => self.continue_run(progress),
                                None => {
                                    self.banked = 0;
                                    self.start_level(0);
                                }
                            }
                        }
                        title_menu::MAP => self.open_map(0),
                        _ => {
                            self.enter(Scene::Options);
                            self.cursor = options_menu::DIFFICULTY;
                        }
                    }
                }
            }
            Scene::Options => {
                self.move_cursor(input, options_menu::LEN);
                let on_row = self.cursor == options_menu::DIFFICULTY;
                let pressed = |key| input.pressed & key != 0;
                let difficulty = if on_row && pressed(crate::keys::LEFT) {
                    self.difficulty.previous()
                } else if on_row && (pressed(crate::keys::RIGHT) || pressed(crate::keys::A)) {
                    self.difficulty.next()
                } else {
                    self.difficulty
                };
                if difficulty != self.difficulty {
                    self.difficulty = difficulty;
                    events.insert(Events::SAVE);
                }
                if back || input.start_pressed() || (confirm && !on_row) {
                    self.return_to_title(title_menu::OPTIONS);
                }
            }
            Scene::Intro => {
                if self.timer >= INTRO_FRAMES || (confirm && self.timer > 20) {
                    if self.arrival_due && self.arrival().is_some() {
                        self.arrival_due = false;
                        self.enter(Scene::Arrival);
                        self.beat = Beat::Landing;
                    } else {
                        self.enter(Scene::Play);
                    }
                }
            }
            Scene::Arrival => self.step_arrival(input, &mut events),
            Scene::Play => {
                if input.start_pressed() {
                    self.enter(Scene::Paused);
                    return events;
                }
                events = self.world.step(input);
                if events.contains(Events::CHECKPOINT) {
                    let world = &self.world;
                    self.save_run(
                        Progress {
                            level: world.level_index,
                            checkpoint: Some(world.checkpoint),
                            beacons: world.checkpoints_lit,
                            banked: self.banked,
                            diamonds: world.diamonds,
                            kills: world.kills,
                            frames: world.frames,
                            complete: false,
                        },
                        &mut events,
                    );
                }
                if events.contains(Events::LEVEL_CLEAR) {
                    self.finish_level(&mut events);
                }
                // The boss speaks once the camera has settled on the arena.
                let world = &mut self.world;
                if world.talk_due && world.camera_x == world.boss.arena.0 {
                    world.talk_due = false;
                    world.talked = true;
                    self.talk_line = 0;
                    self.talk_shown = 0;
                    self.enter(Scene::Dialog);
                }
            }
            Scene::Dialog => {
                let lines = self.world.level.boss_talk;
                if input.start_pressed()
                    || self.talk_line as usize >= lines.len()
                    || self.talk(input, lines, lines.len())
                {
                    self.enter(Scene::Play);
                }
            }
            Scene::Map => {
                let last = levels::MAP.len() as u8 - 1;
                let pressed = |key| input.pressed & key != 0;
                if pressed(crate::keys::RIGHT | crate::keys::DOWN) {
                    self.cursor = (self.cursor + 1).min(last);
                } else if pressed(crate::keys::LEFT | crate::keys::UP) {
                    self.cursor = self.cursor.saturating_sub(1);
                }
                if back {
                    self.return_to_title(title_menu::MAP);
                } else if confirm && let Some(index) = levels::MAP[self.cursor as usize].level {
                    self.banked = 0;
                    self.from_map = true;
                    self.start_level(index);
                }
            }
            Scene::Paused => {
                if input.start_pressed() {
                    self.enter(Scene::Play);
                }
            }
            Scene::LevelClear => {
                if confirm && self.timer >= CLEAR_LOCK_FRAMES {
                    let next = self.world.level_index + 1;
                    if self.from_map {
                        self.open_map(levels::place_of(self.world.level_index) as u8);
                    } else if (next as usize) < levels::ALL.len() {
                        self.banked += self.world.score();
                        self.start_level(next);
                    } else {
                        self.return_to_title(title_menu::PLAY);
                    }
                }
            }
        }
        events
    }

    fn enter(&mut self, scene: Scene) {
        self.scene = scene;
        self.timer = 0;
    }

    /// Types out the current line; A finishes it, then turns the page. Returns true
    /// when A dismisses the line before `stop`, leaving `talk_line` at `stop`.
    fn talk(&mut self, input: &Input, lines: &[Line], stop: usize) -> bool {
        let len = lines.get(self.talk_line as usize).map_or(0, |l| l.len()) as u16;
        if input.pressed & crate::keys::A == 0 {
            self.talk_shown = (self.talk_shown + 1).min(len);
            return false;
        }
        if self.talk_shown < len {
            self.talk_shown = len;
            return false;
        }
        self.talk_line += 1;
        self.talk_shown = 0;
        self.talk_line as usize >= stop
    }

    /// One frame of the arrival. The world waits; the scene moves the wassie itself.
    fn step_arrival(&mut self, input: &Input, events: &mut Events) {
        let Some(script) = self.arrival() else {
            return self.end_arrival();
        };
        if input.start_pressed() {
            return self.end_arrival();
        }
        let next = |game: &mut Game, beat| {
            game.beat = beat;
            game.timer = 0;
        };
        match self.beat {
            Beat::Landing if self.timer >= arrival::LANDING_FRAMES => next(self, Beat::Drop),
            Beat::Landing => {}
            Beat::Drop => {
                let start = self.world.start_feet();
                let (feet, landed) = arrival::drop_feet(self.timer, start);
                let p = &mut self.world.player;
                p.body.y_fx = (feet - p.body.h) * crate::FX;
                p.facing = 1;
                if !landed {
                    p.state = crate::PlayerState::Fall;
                    p.body.vy_fx = crate::FX;
                } else if p.state == crate::PlayerState::Fall {
                    p.state = crate::PlayerState::Idle;
                    p.body.vy_fx = 0;
                    p.land = arrival::LAND_SQUASH;
                    events.insert(Events::LAND);
                } else {
                    p.land = p.land.saturating_sub(1);
                }
                if self.timer >= arrival::DROP_FRAMES {
                    self.talk_line = 0;
                    self.talk_shown = 0;
                    next(self, Beat::Talk);
                }
            }
            Beat::Talk => {
                if self.talk(input, script.lines, script.gift_after) {
                    next(self, Beat::Gift);
                }
            }
            Beat::Gift => {
                if arrival::caught(self.timer).is_some() {
                    events.insert(Events::PICKUP);
                }
                if self.timer >= arrival::GIFT_FRAMES {
                    next(self, Beat::Reply);
                }
            }
            Beat::Reply => {
                if self.talk(input, script.lines, script.lines.len()) {
                    next(self, Beat::Liftoff);
                }
            }
            Beat::Liftoff if self.timer >= arrival::LIFTOFF_FRAMES => self.end_arrival(),
            Beat::Liftoff => {}
        }
    }

    /// Play starts with the wassie standing on its start, armed.
    fn end_arrival(&mut self) {
        self.world.reset_player();
        self.enter(Scene::Play);
    }

    /// Up and Down move through a menu of `len` rows, wrapping at the ends.
    fn move_cursor(&mut self, input: &Input, len: u8) {
        if input.pressed & crate::keys::UP != 0 {
            self.cursor = (self.cursor + len - 1) % len;
        } else if input.pressed & crate::keys::DOWN != 0 {
            self.cursor = (self.cursor + 1) % len;
        }
    }

    fn open_files(&mut self) {
        self.enter(Scene::Files);
        self.cursor = self.file;
    }

    fn open_map(&mut self, place: u8) {
        self.enter(Scene::Map);
        self.cursor = place;
        self.world = World::new(0, self.difficulty);
    }

    fn return_to_title(&mut self, cursor: u8) {
        self.enter(Scene::Title);
        self.cursor = cursor;
        self.world = World::new(0, self.difficulty);
    }

    fn start_level(&mut self, index: u8) {
        self.world = World::new(index, self.difficulty);
        self.arrival_due = true;
        self.enter(Scene::Intro);
    }

    /// Picks up a saved run: its level, at its last beacon.
    fn continue_run(&mut self, progress: Progress) {
        self.banked = progress.banked;
        self.world = World::resume(&progress, self.difficulty);
        self.arrival_due = false;
        self.enter(Scene::Intro);
    }

    /// Writes the active file, unless this is free play from the map.
    fn save_run(&mut self, progress: Progress, events: &mut Events) {
        if self.from_map {
            return;
        }
        self.files[self.file as usize] = Some(progress);
        self.resume = true;
        events.insert(Events::SAVE);
    }

    fn finish_level(&mut self, events: &mut Events) {
        self.enter(Scene::LevelClear);
        let score = self.score();
        self.new_best = score > self.best;
        if self.new_best {
            self.best = score;
            events.insert(Events::SAVE);
        }
        let next = self.world.level_index + 1;
        let progress = if (next as usize) < levels::ALL.len() {
            Progress { level: next, banked: score, ..Progress::default() }
        } else {
            Progress { banked: score, complete: true, ..Progress::default() }
        };
        self.save_run(progress, events);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys;

    fn tap(game: &mut Game, key: u16) -> Events {
        let events = game.step(&Input::from_keys(key, 0));
        game.step(&Input::default());
        events
    }

    /// A new game, past the splash.
    fn at_title() -> Game {
        let mut game = Game::new(0);
        game.enter(Scene::Title);
        game
    }

    #[test]
    fn the_splash_waits_for_its_prompt_then_start_opens_the_title() {
        let mut game = Game::new(0);
        assert_eq!(game.scene, Scene::Splash);
        tap(&mut game, keys::START);
        assert_eq!(game.scene, Scene::Splash, "too soon: the prompt isn't up");
        assert_eq!(game.splash_prompt(), None);
        while game.splash_prompt().is_none() {
            game.step(&Input::default());
        }
        tap(&mut game, keys::START);
        assert_eq!(game.scene, Scene::Files);
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Title, "an empty file goes straight to the title");
        assert_eq!(game.cursor, title_menu::PLAY);
        assert_eq!(game.resumable(), None);
    }

    /// A game at the file menu, with a run at the first beacon saved in file 2.
    fn with_saved_run() -> (Game, Progress) {
        let saved = Progress {
            checkpoint: Some((40, 15)),
            beacons: 1,
            banked: 0,
            diamonds: 3,
            kills: 1,
            frames: 600,
            ..Progress::default()
        };
        let mut game = Game::with_save(0, Difficulty::Normal, [None, Some(saved), None]);
        game.open_files();
        tap(&mut game, keys::DOWN);
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::FileChoice);
        (game, saved)
    }

    #[test]
    fn continue_picks_up_the_files_run_at_its_beacon() {
        let (mut game, saved) = with_saved_run();
        tap(&mut game, keys::A);
        assert_eq!((game.scene, game.file), (Scene::Title, 1));
        assert_eq!(game.resumable(), Some(saved));
        tap(&mut game, keys::START);
        assert_eq!(game.scene, Scene::Intro);
        assert_eq!(game.world.checkpoint, (40, 15));
        assert_eq!(game.score(), saved.score());
    }

    #[test]
    fn a_new_game_only_replaces_the_file_once_it_saves() {
        let (mut game, saved) = with_saved_run();
        tap(&mut game, keys::DOWN);
        tap(&mut game, keys::A);
        assert_eq!(game.resumable(), None);
        tap(&mut game, keys::START);
        assert_eq!(game.world.checkpoints_lit, 0, "a fresh run");
        assert_eq!(game.files[1], Some(saved), "untouched until a beacon");

        let mut events = Events::default();
        let progress = Progress { beacons: 1, ..Progress::default() };
        game.save_run(progress, &mut events);
        assert!(events.contains(Events::SAVE));
        assert_eq!(game.files[1], Some(progress));
        assert_eq!(game.resumable(), Some(progress));
    }

    #[test]
    fn b_backs_out_of_the_file_choice_and_the_title() {
        let (mut game, _) = with_saved_run();
        tap(&mut game, keys::B);
        assert_eq!((game.scene, game.cursor), (Scene::Files, 1));
        let mut game = at_title();
        tap(&mut game, keys::B);
        assert_eq!(game.scene, Scene::Files);
    }

    #[test]
    fn map_runs_never_write_a_file() {
        let mut game = at_title();
        tap(&mut game, keys::DOWN);
        tap(&mut game, keys::A);
        tap(&mut game, keys::A);
        assert!(game.from_map);
        let mut events = Events::default();
        game.save_run(Progress::default(), &mut events);
        game.finish_level(&mut events);
        assert!(!events.contains(Events::SAVE));
        assert_eq!(game.files, [None; FILES]);
    }

    #[test]
    fn clearing_the_last_level_marks_the_file_complete() {
        let mut game = at_title();
        tap(&mut game, keys::START);
        let mut events = Events::default();
        game.finish_level(&mut events);
        assert!(events.contains(Events::SAVE));
        let file = game.files[0].expect("saved on clear");
        assert!(file.complete);
        assert_eq!(file.banked, game.score());
        assert_eq!(game.resumable(), None, "a finished run starts over");
    }

    #[test]
    fn play_shows_the_level_card_then_the_arrival() {
        let mut game = at_title();
        tap(&mut game, keys::START);
        assert_eq!(game.scene, Scene::Intro);
        for _ in 0..INTRO_FRAMES {
            game.step(&Input::default());
        }
        assert_eq!((game.scene, game.beat), (Scene::Arrival, Beat::Landing));
    }

    #[test]
    fn intro_can_be_skipped_after_a_moment() {
        let mut game = at_title();
        tap(&mut game, keys::A);
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Intro, "too soon to skip");
        for _ in 0..20 {
            game.step(&Input::default());
        }
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Arrival);
    }

    /// Plays the arrival through, reading each line with A, and checks each beat in turn.
    #[test]
    fn the_arrival_drops_the_wassie_off_and_arms_it() {
        let mut game = at_title();
        tap(&mut game, keys::START);
        play_level_card(&mut game);
        let start = game.world.start_feet();
        let (mut beats, mut seen) = ([Beat::Landing; 6], 0);
        let mut lines_read = 0;
        let mut armed_before_gift = false;
        for _ in 0..5000 {
            if game.scene != Scene::Arrival {
                break;
            }
            if seen == 0 || beats[seen - 1] != game.beat {
                beats[seen] = game.beat;
                seen += 1;
            }
            let view = game.arrival_view().unwrap();
            armed_before_gift |= view.blaster && game.beat < Beat::Gift;
            if game.talking() && game.talk_shown as usize == game.talk_lines()[game.talk_line as usize].len() {
                lines_read += 1;
                tap(&mut game, keys::A);
            } else {
                game.step(&Input::default());
            }
        }
        use Beat::*;
        assert_eq!((seen, beats), (6, [Landing, Drop, Talk, Gift, Reply, Liftoff]));
        assert_eq!(lines_read, levels::arrival(0).unwrap().lines.len());
        assert!(!armed_before_gift);
        assert_eq!(game.scene, Scene::Play);
        let r = game.world.player.body.rect();
        assert_eq!((r.centre_x(), r.bottom()), start, "standing on the start");
    }

    #[test]
    fn start_skips_the_arrival_but_a_continue_never_plays_it() {
        let mut game = at_title();
        tap(&mut game, keys::START);
        play_level_card(&mut game);
        for _ in 0..arrival::LANDING_FRAMES + 30 {
            game.step(&Input::default());
        }
        tap(&mut game, keys::START);
        assert_eq!(game.scene, Scene::Play);
        let r = game.world.player.body.rect();
        assert_eq!((r.centre_x(), r.bottom()), game.world.start_feet());

        let (mut game, _) = with_saved_run();
        tap(&mut game, keys::A);
        tap(&mut game, keys::START);
        play_level_card(&mut game);
        assert_eq!(game.scene, Scene::Play);
    }

    #[test]
    fn pause_freezes_the_world() {
        let mut game = at_title();
        tap(&mut game, keys::START);
        play_level(&mut game);
        tap(&mut game, keys::START);
        assert_eq!(game.scene, Scene::Paused);
        let frames = game.world.frames;
        for _ in 0..30 {
            game.step(&Input::from_keys(keys::RIGHT, keys::RIGHT));
        }
        assert_eq!(game.world.frames, frames);
        tap(&mut game, keys::START);
        assert_eq!(game.scene, Scene::Play);
    }

    #[test]
    fn options_change_difficulty_and_return() {
        let mut game = at_title();
        tap(&mut game, keys::DOWN);
        tap(&mut game, keys::DOWN);
        assert_eq!(game.cursor, title_menu::OPTIONS);
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Options);
        assert!(tap(&mut game, keys::RIGHT).contains(Events::SAVE));
        assert_eq!(game.difficulty, Difficulty::Degen);
        tap(&mut game, keys::LEFT);
        tap(&mut game, keys::LEFT);
        assert_eq!(game.difficulty, Difficulty::Chill);
        tap(&mut game, keys::DOWN);
        assert!(!tap(&mut game, keys::RIGHT).contains(Events::SAVE), "Back row ignores Left/Right");
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Title);
        assert_eq!(game.cursor, title_menu::OPTIONS);
        tap(&mut game, keys::DOWN);
        tap(&mut game, keys::A);
        assert_eq!(game.world.difficulty, Difficulty::Chill);
    }

    #[test]
    fn b_leaves_options_and_cursor_wraps() {
        let mut game = at_title();
        tap(&mut game, keys::UP);
        assert_eq!(game.cursor, title_menu::OPTIONS);
        tap(&mut game, keys::START);
        assert_eq!(game.scene, Scene::Options);
        tap(&mut game, keys::B);
        assert_eq!(game.scene, Scene::Title);
    }

    fn play_level_card(game: &mut Game) {
        for _ in 0..INTRO_FRAMES {
            game.step(&Input::default());
        }
    }

    /// From the level card into play, skipping the arrival.
    fn play_level(game: &mut Game) {
        play_level_card(game);
        if game.scene == Scene::Arrival {
            tap(game, keys::START);
        }
        assert_eq!(game.scene, Scene::Play);
    }

    #[test]
    fn the_boss_talks_once_the_arena_closes_then_the_fight_resumes() {
        let mut game = at_title();
        tap(&mut game, keys::START);
        play_level(&mut game);
        let world = &mut game.world;
        world.god = true;
        let arena = world.boss.arena.0;
        world.player.body.x_fx = (arena + 3 * crate::TILE) * crate::FX;
        let mut frames = 0;
        while game.scene == Scene::Play {
            game.step(&Input::default());
            frames += 1;
            assert!(frames < 1000, "no dialogue");
        }
        assert_eq!(game.scene, Scene::Dialog);
        assert_eq!(game.world.camera_x, arena, "the camera settled first");
        let frozen = game.world.frames;
        let lines = game.world.level.boss_talk;

        // A finishes the typing, then turns the page.
        tap(&mut game, keys::A);
        assert_eq!(game.talk_shown as usize, lines[0].len());
        tap(&mut game, keys::A);
        assert_eq!((game.talk_line, game.talk_shown), (1, 1));
        for _ in 0..200 {
            game.step(&Input::default());
        }
        assert_eq!(game.talk_shown as usize, lines[1].len(), "types out on its own");
        for _ in 0..2 * lines.len() {
            if game.scene == Scene::Dialog {
                tap(&mut game, keys::A);
            }
        }
        assert_eq!(game.scene, Scene::Play);
        assert_eq!(game.world.frames, frozen + 1, "the world waited");
        assert!(!game.world.talk_due && game.world.talked);
    }

    #[test]
    fn start_skips_the_dialogue() {
        let mut game = at_title();
        game.scene = Scene::Dialog;
        tap(&mut game, keys::START);
        assert_eq!(game.scene, Scene::Play);
    }

    #[test]
    fn the_map_starts_open_levels_and_returns_after_a_clear() {
        let mut game = at_title();
        tap(&mut game, keys::DOWN);
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Map);
        assert_eq!(game.cursor, 0);
        tap(&mut game, keys::RIGHT);
        tap(&mut game, keys::RIGHT);
        tap(&mut game, keys::RIGHT);
        assert_eq!(game.cursor as usize, levels::MAP.len() - 1, "stops at the east edge");
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Map, "unbuilt places don't start");
        tap(&mut game, keys::LEFT);
        tap(&mut game, keys::LEFT);
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Intro);
        assert!(game.from_map);

        play_level(&mut game);
        let mut events = Events::default();
        game.finish_level(&mut events);
        for _ in 0..CLEAR_LOCK_FRAMES {
            game.step(&Input::default());
        }
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Map);
        assert_eq!(game.cursor, 0);
        tap(&mut game, keys::B);
        assert_eq!(game.scene, Scene::Title);
        assert_eq!(game.cursor, title_menu::MAP);
    }

    #[test]
    fn every_built_level_has_a_place_on_the_map() {
        for index in 0..levels::ALL.len() as u8 {
            assert_eq!(levels::MAP.iter().filter(|p| p.level == Some(index)).count(), 1);
        }
    }

    #[test]
    fn clearing_the_last_level_records_best_and_returns_to_title() {
        let mut game = at_title();
        tap(&mut game, keys::START);
        game.enter(Scene::Play);
        game.world.diamonds = 12;
        let mut events = Events::default();
        game.finish_level(&mut events);
        assert!(events.contains(Events::SAVE));
        assert_eq!(game.scene, Scene::LevelClear);
        assert_eq!(game.best, 120);
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::LevelClear, "input locked at first");
        for _ in 0..CLEAR_LOCK_FRAMES {
            game.step(&Input::default());
        }
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Title);
    }
}
