//! Scene flow around the levels: title ⇄ options, title → level intro → play ⇄ pause →
//! level clear → next level, or back to the title after the last one.

use crate::{
    levels,
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
}

/// Frames the level card shows before play starts on its own.
pub const INTRO_FRAMES: u32 = 150;
/// Frames the level-clear screen ignores input, so a held button doesn't skip it.
pub const CLEAR_LOCK_FRAMES: u32 = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Scene {
    Title,
    Play,
    Paused,
    LevelClear,
    Options,
    Intro,
}

/// Title menu rows, in [`Game::cursor`] order.
pub mod title_menu {
    pub const PLAY: u8 = 0;
    pub const OPTIONS: u8 = 1;
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
    /// Highlighted row of the current menu.
    pub cursor: u8,
    /// Frames spent in the current scene, for timed screens.
    timer: u32,
}

impl Game {
    pub fn new(best: u32) -> Self {
        Self::with_difficulty(best, Difficulty::default())
    }

    pub fn with_difficulty(best: u32, difficulty: Difficulty) -> Self {
        Self {
            scene: Scene::Title,
            world: World::new(0, difficulty),
            frame: 0,
            best,
            banked: 0,
            new_best: false,
            difficulty,
            cursor: title_menu::PLAY,
            timer: 0,
        }
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
            Scene::Title => {
                self.move_cursor(input, title_menu::LEN);
                if confirm {
                    match self.cursor {
                        title_menu::PLAY => {
                            self.banked = 0;
                            self.start_level(0);
                        }
                        _ => {
                            self.scene = Scene::Options;
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
                    self.enter(Scene::Play);
                }
            }
            Scene::Play => {
                if input.start_pressed() {
                    self.enter(Scene::Paused);
                    return events;
                }
                events = self.world.step(input);
                if events.contains(Events::LEVEL_CLEAR) {
                    self.finish_level(&mut events);
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
                    if (next as usize) < levels::ALL.len() {
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

    /// Up and Down move through a menu of `len` rows, wrapping at the ends.
    fn move_cursor(&mut self, input: &Input, len: u8) {
        if input.pressed & crate::keys::UP != 0 {
            self.cursor = (self.cursor + len - 1) % len;
        } else if input.pressed & crate::keys::DOWN != 0 {
            self.cursor = (self.cursor + 1) % len;
        }
    }

    fn return_to_title(&mut self, cursor: u8) {
        self.enter(Scene::Title);
        self.cursor = cursor;
        self.world = World::new(0, self.difficulty);
    }

    fn start_level(&mut self, index: u8) {
        self.world = World::new(index, self.difficulty);
        self.enter(Scene::Intro);
    }

    fn finish_level(&mut self, events: &mut Events) {
        self.enter(Scene::LevelClear);
        let score = self.score();
        self.new_best = score > self.best;
        if self.new_best {
            self.best = score;
            events.insert(Events::SAVE);
        }
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

    #[test]
    fn play_shows_the_level_card_then_starts() {
        let mut game = Game::new(0);
        tap(&mut game, keys::START);
        assert_eq!(game.scene, Scene::Intro);
        for _ in 0..INTRO_FRAMES {
            game.step(&Input::default());
        }
        assert_eq!(game.scene, Scene::Play);
    }

    #[test]
    fn intro_can_be_skipped_after_a_moment() {
        let mut game = Game::new(0);
        tap(&mut game, keys::A);
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Intro, "too soon to skip");
        for _ in 0..20 {
            game.step(&Input::default());
        }
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Play);
    }

    #[test]
    fn pause_freezes_the_world() {
        let mut game = Game::new(0);
        tap(&mut game, keys::START);
        for _ in 0..INTRO_FRAMES {
            game.step(&Input::default());
        }
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
        let mut game = Game::new(0);
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
        tap(&mut game, keys::UP);
        tap(&mut game, keys::A);
        assert_eq!(game.world.difficulty, Difficulty::Chill);
    }

    #[test]
    fn b_leaves_options_and_cursor_wraps() {
        let mut game = Game::new(0);
        tap(&mut game, keys::UP);
        assert_eq!(game.cursor, title_menu::OPTIONS);
        tap(&mut game, keys::START);
        assert_eq!(game.scene, Scene::Options);
        tap(&mut game, keys::B);
        assert_eq!(game.scene, Scene::Title);
    }

    #[test]
    fn clearing_the_last_level_records_best_and_returns_to_title() {
        let mut game = Game::new(0);
        tap(&mut game, keys::START);
        game.enter(Scene::Play);
        game.world.sats = 12;
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
