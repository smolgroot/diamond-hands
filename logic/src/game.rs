//! Scene flow around the levels: splash → save files → title ⇄ options, title → level
//! intro → (arrival or a call from CL207, on a first visit) → play ⇄ pause menu →
//! (the penguin's exchange, in a casino level) → level clear → world map → the next
//! level. Play stops for the boss's dialogue when the arena closes.
//!
//! Two currencies: diamonds, banked into the file's wallet when a level is cleared,
//! and gold coins from the casino. At the end of a casino level the penguin cashier
//! offers to change the coins into diamonds at [`EXCHANGE_RATE`]; coins kept instead
//! stay in the file's purse for next time.
//!
//! The world map is the hub. Levels unlock in order: clearing one opens the next, and
//! any unlocked level can be played (or replayed) from the map. Each save file keeps
//! which levels it has cleared, and every run saves to the chosen file at each beacon
//! and level clear.

use alloc::boxed::Box;

use crate::{
    arrival::{self, Beat},
    inventory::ITEMS,
    levels::{self, Cleared},
    talk::{Line, Speaker},
    world::{Difficulty, Events, Input, World},
};

/// Diamonds the penguin pays for each gold coin.
pub const EXCHANGE_RATE: u32 = 8;
/// Frames CL207's call rings before the wassie picks up on its own.
pub const CALL_RING_FRAMES: u32 = 80;
/// Frames the penguin takes to waddle in, and the payout counts up for.
pub const PENGUIN_WALK_FRAMES: u32 = 70;
pub const PAYOUT_FRAMES: u32 = 90;

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
    /// Menus: accept, and go back.
    pub const CONFIRM: u16 = A;
    pub const BACK: u16 = B;
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
/// Frames the level-clear screen shows before the world map opens on its own.
pub const CLEAR_AUTO_FRAMES: u32 = 420;
/// Frames the wassie takes to walk the road to a newly unlocked place on the map.
pub const MAP_WALK_FRAMES: u32 = 72;
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
    /// After a casino boss: the penguin cashier changes coins into diamonds.
    Exchange,
}

/// What the dialogue box is talking through in [`Scene::Dialog`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Talk {
    /// The boss, when the arena closes.
    Boss,
    /// CL207 calls when the level starts.
    Call,
}

/// Where the penguin's exchange is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Deal {
    /// Waddling in from the right.
    Walk,
    /// The pitch.
    Pitch,
    /// The offer is on the table: exchange, or keep the coins.
    Offer,
    /// Accepted: the coins count down as the diamonds count up.
    Payout,
    /// A parting line.
    Bye,
}

/// Rows of the penguin's offer, in [`Game::cursor`] order.
pub mod offer_menu {
    pub const EXCHANGE: u8 = 0;
    pub const KEEP: u8 = 1;
    pub const LEN: u8 = 2;
}

/// The exchange in progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exchange {
    pub deal: Deal,
    /// Coins on the counter: this run's (unless it's a replay) and the purse's.
    pub coins: u32,
    /// What the wassie chose, once it has: `Some(true)` exchanged.
    pub accepted: Option<bool>,
}

impl Exchange {
    /// Diamonds the coins on the counter are worth.
    pub fn diamonds(&self) -> u32 {
        self.coins.saturating_mul(EXCHANGE_RATE)
    }
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
    /// The levels this file has cleared, which also unlock the ones after them.
    pub cleared: Cleared,
    /// Gold coins picked up in this level so far.
    pub coins: u32,
    /// Diamonds banked from cleared levels and the penguin's exchange.
    pub wallet: u32,
    /// Gold coins kept from earlier exchanges.
    pub purse: u32,
    /// CL207's shield parts found, one bit each, and the gear found in levels that
    /// hide it (see [`crate::world::GEAR_JETPACK`]).
    pub parts: u16,
    /// What the run had taken at its beacon, one bit per actor slot (see
    /// [`World::taken`]). Only an explorable level reads it back.
    pub taken: u128,
}

impl Progress {
    /// Score so far.
    pub fn score(&self) -> u32 {
        self.banked + World::score_for(self.diamonds, self.coins, self.kills, false)
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

/// The pause menu's tabs, in [`Menu::tab`] order.
pub mod pause_menu {
    pub const CONTROLS: u8 = 0;
    pub const BAG: u8 = 1;
    pub const WALLET: u8 = 2;
    pub const TABS: u8 = 3;
}

/// Where the pause menu is. It stays put between pauses, as a bag's pocket does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Menu {
    pub tab: u8,
    /// Highlighted row of the bag, an index into [`ITEMS`].
    pub item: u8,
    /// A opened the highlighted item's detail card.
    pub inspecting: bool,
}

impl Menu {
    pub const NEW: Menu = Menu { tab: pause_menu::BAG, item: 0, inspecting: false };
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
    /// On the heap: see the crate root.
    pub world: Box<World>,
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
    /// The levels the active file has cleared.
    pub cleared: Cleared,
    /// Dev builds: levels the map treats as cleared, so the ones after them can be
    /// played without beating them. Never saved; the file's own clears are untouched.
    pub dev_cleared: Cleared,
    /// This level was already cleared before this visit: it banks no score again.
    pub replay: bool,
    /// The place a clear just unlocked, marked "NEW" on the map until played.
    pub fresh: Option<u8>,
    /// The map places the wassie walks between after a clear, while it walks.
    pub map_walk: Option<(u8, u8)>,
    pub menu: Menu,
    /// Buttons held when the pause menu closed. Play ignores each until it is let go,
    /// so the B or A that closed the menu doesn't also fire or jump.
    latch: u16,
    /// Dialogue line showing, and how many of its characters are typed out.
    pub talk_line: u8,
    pub talk_shown: u16,
    /// The saved runs, and the file this session plays in.
    pub files: [Option<Progress>; FILES],
    pub file: u8,
    /// PLAY picks up the file's run rather than starting over.
    pub resume: bool,
    /// Where the arrival scene is, and whether this level start plays it (or CL207's
    /// call, in a level with no arrival).
    pub beat: Beat,
    arrival_due: bool,
    /// What [`Scene::Dialog`] is talking through.
    pub talk: Talk,
    /// The penguin's exchange, while it runs, and how it went after.
    pub exchange: Exchange,
    /// The active file's banked diamonds, kept gold coins and shield parts.
    pub wallet: u32,
    pub purse: u32,
    pub parts: u16,
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
            world: new_world(0, difficulty),
            frame: 0,
            best,
            banked: 0,
            new_best: false,
            difficulty,
            cursor: title_menu::PLAY,
            cleared: 0,
            dev_cleared: 0,
            replay: false,
            fresh: None,
            map_walk: None,
            menu: Menu::NEW,
            latch: 0,
            talk_line: 0,
            talk_shown: 0,
            files,
            file: 0,
            resume: false,
            beat: Beat::Landing,
            arrival_due: false,
            talk: Talk::Boss,
            exchange: Exchange { deal: Deal::Walk, coins: 0, accepted: None },
            wallet: 0,
            purse: 0,
            parts: 0,
            timer: 0,
        }
    }

    /// The run PLAY will pick up, if any: the active file's, unless the player chose to
    /// start over.
    pub fn resumable(&self) -> Option<Progress> {
        self.files[self.file as usize].filter(|_| self.resume)
    }

    /// Whether PLAY picks up where the file left off rather than starting a new run.
    pub fn can_continue(&self) -> bool {
        self.resumable().is_some() || self.cleared != 0
    }

    /// Frames spent in the current scene.
    pub fn scene_frames(&self) -> u32 {
        self.timer
    }

    /// The levels the world map treats as cleared: the file's, and in a dev build the
    /// ones it opens up.
    pub fn map_cleared(&self) -> Cleared {
        self.cleared | self.dev_cleared
    }

    /// The map place of the first level the file hasn't cleared.
    fn frontier_place(&self) -> u8 {
        levels::place_of(levels::frontier(self.map_cleared())) as u8
    }

    /// Whether the splash is showing its "PRESS START" prompt, and for how many frames.
    pub fn splash_prompt(&self) -> Option<u32> {
        (self.scene == Scene::Splash && self.timer >= SPLASH_PROMPT_FRAMES)
            .then(|| self.timer - SPLASH_PROMPT_FRAMES)
    }

    /// The lines being talked through: the boss's, the arrival's, CL207's call, or the
    /// penguin's.
    pub fn talk_lines(&self) -> &'static [Line] {
        match self.scene {
            Scene::Arrival => self.arrival().map_or(&[], |a| a.lines),
            Scene::Exchange => match self.exchange.deal {
                Deal::Pitch => &PITCH,
                Deal::Offer => &OFFER,
                Deal::Bye if self.exchange.coins == 0 => &BROKE,
                Deal::Bye if self.exchange.accepted == Some(true) => &THANKS,
                Deal::Bye => &KEPT,
                _ => &[],
            },
            _ if self.talk == Talk::Call => self.world.level.call,
            _ => self.world.level.boss_talk,
        }
    }

    /// Whether a line of dialogue is up.
    pub fn talking(&self) -> bool {
        match self.scene {
            Scene::Dialog => self.talk != Talk::Call || self.timer >= CALL_RING_FRAMES,
            Scene::Arrival => matches!(self.beat, Beat::Talk | Beat::Reply),
            Scene::Exchange => matches!(self.exchange.deal, Deal::Pitch | Deal::Offer | Deal::Bye),
            _ => false,
        }
    }

    /// CL207's phone is ringing, before the wassie picks up.
    pub fn ringing(&self) -> bool {
        self.scene == Scene::Dialog && self.talk == Talk::Call && self.timer < CALL_RING_FRAMES
    }

    /// What the arrival scene shows now, while it plays.
    pub fn arrival_view(&self) -> Option<arrival::View> {
        let start = self.world.start_feet();
        (self.scene == Scene::Arrival).then(|| {
            let mut v = arrival::view(self.beat, self.timer, start, start.1);
            if self.world.jetpack_here {
                // This level hides the jetpack: CL207 only has the blaster to give.
                v.jetpack = false;
                if matches!(v.thrown, Some((arrival::Gear::Jetpack, _, _))) {
                    v.thrown = None;
                }
            }
            v
        })
    }

    fn arrival(&self) -> Option<&'static arrival::Arrival> {
        levels::arrival(self.world.level_index)
    }

    /// Score so far in this playthrough.
    pub fn score(&self) -> u32 {
        self.banked + self.world.score()
    }

    /// The file's diamonds: banked, plus those picked up in this level so far.
    pub fn diamond_balance(&self) -> u32 {
        self.wallet + if self.replay { 0 } else { self.world.diamonds }
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
                        self.choose_file(false);
                    }
                }
            }
            Scene::FileChoice => {
                self.move_cursor(input, file_menu::LEN);
                if back {
                    self.open_files();
                } else if confirm {
                    // Starting over leaves the file as it is until the new run saves.
                    self.choose_file(self.cursor == file_menu::CONTINUE);
                }
            }
            Scene::Title => {
                self.move_cursor(input, title_menu::LEN);
                if back {
                    self.open_files();
                } else if confirm {
                    match self.cursor {
                        // Mid-level runs pick up at their beacon; between levels,
                        // the map opens on the next one; a new run starts at the first.
                        title_menu::PLAY => match self.resumable() {
                            Some(progress) if progress.checkpoint.is_some() => self.continue_run(progress),
                            _ if self.map_cleared() != 0 => self.open_map(self.frontier_place()),
                            _ => self.start_level(0),
                        },
                        title_menu::MAP => self.open_map(self.frontier_place()),
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
                    let due = core::mem::take(&mut self.arrival_due);
                    if due && self.arrival().is_some() {
                        self.enter(Scene::Arrival);
                        self.beat = Beat::Landing;
                    } else if due && !self.world.level.call.is_empty() {
                        self.talk = Talk::Call;
                        self.talk_line = 0;
                        self.talk_shown = 0;
                        self.enter(Scene::Dialog);
                    } else {
                        self.enter(Scene::Play);
                    }
                }
            }
            Scene::Arrival => self.step_arrival(input, &mut events),
            Scene::Play => {
                self.latch &= input.held;
                let input = &Input { held: input.held & !self.latch, pressed: input.pressed & !self.latch };
                if input.start_pressed() {
                    self.menu.inspecting = false;
                    self.enter(Scene::Paused);
                    events.insert(Events::MENU_OK);
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
                            complete: self.cleared == levels::ALL_CLEARED,
                            cleared: self.cleared,
                            coins: world.coins,
                            wallet: self.wallet,
                            purse: self.purse,
                            parts: world.parts,
                            taken: world.taken(),
                        },
                        &mut events,
                    );
                }
                if events.contains(Events::LEVEL_CLEAR) {
                    if self.world.level.coins {
                        self.open_exchange();
                    } else {
                        self.finish_level(&mut events);
                    }
                    return events;
                }
                // The boss speaks once the camera has settled on the arena.
                let world = &mut self.world;
                if world.talk_due && world.camera_x == world.boss.arena.0 && world.camera_y == world.arena_y {
                    world.talk_due = false;
                    world.talked = true;
                    self.talk = Talk::Boss;
                    self.talk_line = 0;
                    self.talk_shown = 0;
                    self.enter(Scene::Dialog);
                }
            }
            Scene::Dialog => {
                let lines = self.talk_lines();
                if self.ringing() {
                    // A picks up; Start ignores the call altogether.
                    if input.start_pressed() {
                        self.enter(Scene::Play);
                    } else if input.pressed & crate::keys::A != 0 && self.timer > 10 {
                        self.timer = CALL_RING_FRAMES;
                    }
                } else if input.start_pressed()
                    || self.talk_line as usize >= lines.len()
                    || self.talk(input, lines, lines.len())
                {
                    self.enter(Scene::Play);
                }
            }
            Scene::Exchange => self.step_exchange(input, &mut events),
            Scene::Map => self.step_map(input, &mut events),
            Scene::Paused => self.step_menu(input, &mut events),
            Scene::LevelClear => {
                let skip = confirm && self.timer >= CLEAR_LOCK_FRAMES;
                if skip || self.timer >= CLEAR_AUTO_FRAMES {
                    if !self.replay {
                        self.banked += self.world.score();
                    }
                    let from = levels::place_of(self.world.level_index) as u8;
                    let to = self.fresh.unwrap_or_else(|| self.frontier_place());
                    self.open_map(to);
                    self.map_walk = (from != to && self.fresh == Some(to)).then_some((from, to));
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
                let hidden = self.world.jetpack_here;
                if arrival::caught(self.timer).is_some_and(|g| g == arrival::Gear::Blaster || !hidden) {
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

    /// The penguin's exchange, which opens when a casino boss is beaten: the coins on
    /// the counter are this run's (unless it's a replay, which banks nothing) and
    /// those kept from before.
    fn open_exchange(&mut self) {
        let run = if self.replay { 0 } else { self.world.coins };
        self.exchange = Exchange { deal: Deal::Walk, coins: run.saturating_add(self.purse), accepted: None };
        // The fight is over: nothing left in flight hangs in the air while he talks.
        self.world.shots = [crate::shot::Shot::EMPTY; crate::shot::MAX_SHOTS];
        self.world.fx.clear();
        self.world.player.recoil = 0;
        self.world.player.charge = 0;
        self.enter(Scene::Exchange);
    }

    fn deal(&mut self, deal: Deal) {
        self.exchange.deal = deal;
        self.timer = 0;
        self.talk_line = 0;
        self.talk_shown = 0;
    }

    /// One frame of the exchange. The deal is struck once, on the frame A confirms the
    /// offer, and saved with the level's clear when the penguin says goodbye.
    fn step_exchange(&mut self, input: &Input, events: &mut Events) {
        let pressed = |key| input.pressed & key != 0;
        match self.exchange.deal {
            Deal::Walk => {
                if self.timer >= PENGUIN_WALK_FRAMES || (input.start_pressed() && self.timer > 10) {
                    let next = if self.exchange.coins == 0 { Deal::Bye } else { Deal::Pitch };
                    self.deal(next);
                }
            }
            Deal::Pitch => {
                if input.start_pressed() || self.talk(input, &PITCH, PITCH.len()) {
                    self.deal(Deal::Offer);
                    self.cursor = offer_menu::EXCHANGE;
                }
            }
            Deal::Offer => {
                // The question is up at once; the choice is made with the pad.
                self.talk_shown = OFFER[0].len() as u16;
                let move_keys = crate::keys::UP | crate::keys::DOWN | crate::keys::LEFT | crate::keys::RIGHT;
                if pressed(move_keys) {
                    self.cursor = (self.cursor + 1) % offer_menu::LEN;
                    events.insert(Events::MENU_MOVE);
                } else if pressed(crate::keys::A) {
                    let accept = self.cursor == offer_menu::EXCHANGE;
                    self.strike(accept);
                    events.insert(if accept { Events::MENU_OK } else { Events::MENU_BACK });
                    self.deal(if accept { Deal::Payout } else { Deal::Bye });
                }
            }
            Deal::Payout => {
                if self.timer % 6 == 0 {
                    events.insert(Events::COIN);
                }
                if self.timer >= PAYOUT_FRAMES || (pressed(crate::keys::A) && self.timer > 10) {
                    self.deal(Deal::Bye);
                }
            }
            Deal::Bye => {
                let lines = self.talk_lines();
                if input.start_pressed() || self.talk(input, lines, lines.len()) {
                    self.finish_level(events);
                }
            }
        }
    }

    /// Settles the exchange, once: accepting pays the coins' worth into the wallet and
    /// empties the purse; keeping them puts every coin on the counter in the purse.
    fn strike(&mut self, accept: bool) {
        if self.exchange.accepted.is_some() {
            return;
        }
        self.exchange.accepted = Some(accept);
        if accept {
            self.wallet = self.wallet.saturating_add(self.exchange.diamonds());
            self.purse = 0;
        } else {
            self.purse = self.exchange.coins;
        }
    }

    /// Play starts with the wassie standing on its start, armed.
    fn end_arrival(&mut self) {
        self.world.reset_player();
        self.enter(Scene::Play);
    }

    /// The world map: Left and Right (or Up and Down) walk between places, A plays an
    /// unlocked one, B goes back to the title.
    fn step_map(&mut self, input: &Input, events: &mut Events) {
        let last = levels::MAP.len() as u8 - 1;
        let pressed = |key| input.pressed & key != 0;
        let was = self.cursor;
        if pressed(crate::keys::RIGHT | crate::keys::DOWN) {
            self.cursor = (self.cursor + 1).min(last);
        } else if pressed(crate::keys::LEFT | crate::keys::UP) {
            self.cursor = self.cursor.saturating_sub(1);
        }
        if self.cursor != was {
            self.map_walk = None;
            events.insert(Events::MENU_MOVE);
        }
        let place = &levels::MAP[self.cursor as usize];
        if pressed(crate::keys::BACK) {
            events.insert(Events::MENU_BACK);
            self.return_to_title(title_menu::MAP);
        } else if input.start_pressed() || pressed(crate::keys::CONFIRM) {
            match place.level.filter(|_| place.playable(self.map_cleared())) {
                Some(index) => {
                    events.insert(Events::MENU_OK);
                    self.start_level(index);
                }
                None => events.insert(Events::MENU_DENY),
            }
        }
    }

    /// The pause menu. Left and Right (or L and R) switch tabs; in the bag, Up and Down
    /// pick an item and A inspects it. B backs out of an inspected item, else closes
    /// the menu, as Start always does.
    fn step_menu(&mut self, input: &Input, events: &mut Events) {
        use crate::keys::{BACK, CONFIRM, DOWN, L, LEFT, R, RIGHT, UP};
        let pressed = |key| input.pressed & key != 0;
        let menu = &mut self.menu;
        if input.start_pressed() || (pressed(BACK) && !menu.inspecting) {
            events.insert(Events::MENU_BACK);
            self.latch = input.held;
            // A charge held into the menu would fire on the way out.
            self.world.player.charge = 0;
            self.enter(Scene::Play);
            return;
        }
        if menu.inspecting {
            if pressed(BACK | CONFIRM) {
                menu.inspecting = false;
                events.insert(Events::MENU_BACK);
            }
            return;
        }
        let tabs = pause_menu::TABS;
        let items = ITEMS.len() as u8;
        if pressed(LEFT | L) {
            menu.tab = (menu.tab + tabs - 1) % tabs;
            events.insert(Events::MENU_MOVE);
        } else if pressed(RIGHT | R) {
            menu.tab = (menu.tab + 1) % tabs;
            events.insert(Events::MENU_MOVE);
        } else if menu.tab == pause_menu::BAG {
            if pressed(UP) {
                menu.item = (menu.item + items - 1) % items;
                events.insert(Events::MENU_MOVE);
            } else if pressed(DOWN) {
                menu.item = (menu.item + 1) % items;
                events.insert(Events::MENU_MOVE);
            } else if pressed(CONFIRM) {
                menu.inspecting = true;
                events.insert(Events::MENU_OK);
            }
        }
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

    /// Picks up the chosen file (or starts it over) and goes to the title.
    fn choose_file(&mut self, resume: bool) {
        self.resume = resume;
        let run = self.resumable().unwrap_or_default();
        self.cleared = run.cleared;
        self.banked = run.banked;
        self.wallet = run.wallet;
        self.purse = run.purse;
        self.parts = run.parts;
        self.fresh = None;
        self.return_to_title(title_menu::PLAY);
    }

    fn open_map(&mut self, place: u8) {
        self.enter(Scene::Map);
        self.cursor = place;
        self.map_walk = None;
        self.world = new_world(0, self.difficulty);
    }

    fn return_to_title(&mut self, cursor: u8) {
        self.enter(Scene::Title);
        self.cursor = cursor;
        self.world = new_world(0, self.difficulty);
    }

    /// Starts level `index` at its beginning. Its arrival plays on a first visit only.
    fn start_level(&mut self, index: u8) {
        self.world = new_world(index, self.difficulty);
        self.world.own_parts(self.parts);
        self.replay = levels::is_cleared(self.cleared, index);
        self.arrival_due = !self.replay;
        if self.fresh == Some(levels::place_of(index) as u8) {
            self.fresh = None;
        }
        self.enter(Scene::Intro);
    }

    /// Picks up a saved run: its level, at its last beacon.
    fn continue_run(&mut self, progress: Progress) {
        self.banked = progress.banked;
        self.world = Box::new(World::resume(&progress, self.difficulty));
        self.world.own_parts(self.parts | progress.parts);
        self.replay = levels::is_cleared(self.cleared, progress.level);
        self.arrival_due = false;
        self.enter(Scene::Intro);
    }

    /// Writes the active file.
    fn save_run(&mut self, progress: Progress, events: &mut Events) {
        self.files[self.file as usize] = Some(progress);
        self.resume = true;
        events.insert(Events::SAVE);
    }

    /// Marks the level cleared, which unlocks the next, and saves. A replay of a cleared
    /// level banks no score and can't set a best.
    /// Marks the level cleared, which unlocks the next, banks the level's diamonds in
    /// the wallet and saves. A replay of a cleared level banks no score or diamonds and
    /// can't set a best. Shield parts are kept either way: each can only be found once.
    fn finish_level(&mut self, events: &mut Events) {
        self.enter(Scene::LevelClear);
        let index = self.world.level_index;
        let score = self.score();
        self.new_best = !self.replay && score > self.best;
        if self.new_best {
            self.best = score;
            events.insert(Events::SAVE);
        }
        self.parts |= self.world.parts;
        if !self.replay {
            self.wallet = self.wallet.saturating_add(self.world.diamonds);
            self.cleared |= 1 << index;
            let next = index + 1;
            self.fresh = levels::MAP
                .iter()
                .position(|p| p.order == next)
                .filter(|_| levels::is_reached(self.cleared, next))
                .map(|i| i as u8);
        }
        let progress = Progress {
            level: levels::frontier(self.cleared),
            banked: if self.replay { self.banked } else { score },
            complete: self.cleared == levels::ALL_CLEARED,
            cleared: self.cleared,
            wallet: self.wallet,
            purse: self.purse,
            parts: self.parts,
            ..Progress::default()
        };
        self.save_run(progress, events);
    }
}

/// A level's world, made in a stack frame of its own and moved straight to the heap.
#[inline(never)]
fn new_world(level: u8, difficulty: Difficulty) -> Box<World> {
    Box::new(World::new(level, difficulty))
}

/// The penguin cashier's lines.
static PITCH: [Line; 3] = [
    Line { speaker: Speaker::Penguin, rows: &["HONK! NICE WIN, PAL.", "THE HOUSE IS CLOSED."] },
    Line { speaker: Speaker::Penguin, rows: &["I CHANGE CASINO GOLD", "FOR REAL DIAMONDS."] },
    Line { speaker: Speaker::Penguin, rows: &["ONE COIN BUYS EIGHT.", "NO FEES. NO RUGS."] },
];
static OFFER: [Line; 1] = [Line { speaker: Speaker::Penguin, rows: &["CASH OUT YOUR COINS?", "   EXCHANGE THEM", "   KEEP MY COINS"] }];
static THANKS: [Line; 1] = [Line { speaker: Speaker::Penguin, rows: &["PLEASURE DOING", "BUSINESS. WADDLE ON!"] }];
static KEPT: [Line; 1] = [Line { speaker: Speaker::Penguin, rows: &["HODLING GOLD, EH?", "MY BOOTH IS ALWAYS", "OPEN."] }];
static BROKE: [Line; 1] = [Line { speaker: Speaker::Penguin, rows: &["NO COINS? COME BACK", "WHEN YOU WIN BIG."] }];

/// Every line the penguin can say.
pub static MERCHANT_LINES: [&[Line]; 5] = [&PITCH, &OFFER, &THANKS, &KEPT, &BROKE];

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

    /// Clears the level being played, then waits out the clear screen into the map.
    fn clear_level(game: &mut Game) -> Events {
        let mut events = Events::default();
        game.finish_level(&mut events);
        for _ in 0..CLEAR_AUTO_FRAMES {
            game.step(&Input::default());
        }
        assert_eq!(game.scene, Scene::Map, "the map opens on its own");
        events
    }

    #[test]
    fn clearing_level_one_opens_the_map_with_level_two_unlocked() {
        let mut game = at_title();
        tap(&mut game, keys::START);
        play_level(&mut game);
        game.world.diamonds = 5;
        let level_score = game.world.score();
        let events = clear_level(&mut game);
        assert!(events.contains(Events::SAVE));
        assert_eq!(game.cleared, 0b1);
        use levels::PlaceStatus::*;
        assert_eq!(levels::MAP.each_ref().map(|p| p.status(game.cleared)), [Cleared, Open, Soon]);
        assert_eq!(game.cursor, 1, "the cursor waits on the new place");
        assert_eq!(game.fresh, Some(1));
        assert_eq!(game.map_walk, Some((0, 1)), "the wassie walks the road there");
        assert_eq!(game.banked, level_score);

        let file = game.files[0].expect("saved on clear");
        assert_eq!((file.cleared, file.level, file.checkpoint), (0b1, 1, None));
        assert!(!file.complete);

        tap(&mut game, keys::A);
        assert_eq!((game.scene, game.world.level_index), (Scene::Intro, 1));
        assert_eq!(game.fresh, None, "no longer new once played");
        assert!(!game.replay);
    }

    #[test]
    fn locked_places_refuse_and_cleared_ones_replay() {
        let mut game = at_title();
        tap(&mut game, keys::DOWN);
        tap(&mut game, keys::A);
        assert_eq!((game.scene, game.cursor), (Scene::Map, 0));
        tap(&mut game, keys::RIGHT);
        assert!(tap(&mut game, keys::A).contains(Events::MENU_DENY));
        assert_eq!(game.scene, Scene::Map, "level 2 is locked on a new file");
        tap(&mut game, keys::RIGHT);
        tap(&mut game, keys::RIGHT);
        assert_eq!(game.cursor as usize, levels::MAP.len() - 1, "stops at the east edge");
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Map, "unbuilt places don't start");

        game.cleared = 0b1;
        game.cursor = 0;
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Intro);
        assert!(game.replay && !game.arrival_due, "a replay skips the arrival");
        play_level_card(&mut game);
        assert_eq!(game.scene, Scene::Play);
        let banked = game.banked;
        game.world.diamonds = 50;
        clear_level(&mut game);
        assert_eq!(game.banked, banked, "a replay banks nothing");
        assert!(!game.new_best);
        assert_eq!(game.cleared, 0b1);
    }

    #[test]
    fn clearing_every_level_marks_the_file_complete() {
        let mut game = at_title();
        game.cleared = 0b1;
        game.cursor = title_menu::PLAY;
        tap(&mut game, keys::START);
        assert_eq!((game.scene, game.cursor), (Scene::Map, 1), "PLAY opens the map between levels");
        tap(&mut game, keys::A);
        play_level(&mut game);
        clear_level(&mut game);
        let file = game.files[0].expect("saved on clear");
        assert!(file.complete);
        assert_eq!(file.cleared, levels::ALL_CLEARED);
        assert_eq!(game.fresh, Some(2), "level 3 shows unlocked, though not built");
        assert_eq!(game.map_walk, Some((1, 2)), "the wassie walks on to it");
        assert_eq!(levels::MAP[2].status(game.cleared), levels::PlaceStatus::Upcoming);
        assert!(tap(&mut game, keys::A).contains(Events::MENU_DENY), "but it can't be played yet");
        tap(&mut game, keys::B);
        assert_eq!(game.scene, Scene::Title);
        tap(&mut game, keys::UP);
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Map, "a finished file replays from the map");
    }

    #[test]
    fn progression_comes_back_with_the_file() {
        let mut game = at_title();
        tap(&mut game, keys::START);
        play_level(&mut game);
        clear_level(&mut game);
        // Power cycle: a new game from what the cartridge holds.
        let mut game = Game::with_save(game.best, game.difficulty, game.files);
        game.open_files();
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::FileChoice);
        tap(&mut game, keys::A);
        assert_eq!((game.scene, game.cleared), (Scene::Title, 0b1));
        assert!(game.can_continue());
        tap(&mut game, keys::A);
        assert_eq!((game.scene, game.cursor), (Scene::Map, 1));
        assert!(levels::MAP[1].playable(game.cleared));

        // NEW GAME starts from nothing, but only overwrites the file once it saves.
        game.open_files();
        tap(&mut game, keys::A);
        tap(&mut game, keys::DOWN);
        tap(&mut game, keys::A);
        assert_eq!((game.cleared, game.banked), (0, 0));
        assert_eq!(game.files[0].map(|f| f.cleared), Some(0b1));
    }

    #[test]
    fn start_opens_the_menu_and_its_tabs_and_items_cycle() {
        let mut game = at_title();
        tap(&mut game, keys::START);
        play_level(&mut game);
        let x = game.world.player.body.x();
        assert!(tap(&mut game, keys::START).contains(Events::MENU_OK));
        assert_eq!((game.scene, game.menu.tab), (Scene::Paused, pause_menu::BAG));
        for _ in 0..ITEMS.len() {
            tap(&mut game, keys::DOWN);
        }
        assert_eq!(game.menu.item, 0, "the bag wraps");
        tap(&mut game, keys::UP);
        assert_eq!(game.menu.item as usize, ITEMS.len() - 1);
        tap(&mut game, keys::A);
        assert!(game.menu.inspecting);
        tap(&mut game, keys::RIGHT);
        assert_eq!(game.menu.tab, pause_menu::BAG, "tabs hold still while inspecting");
        tap(&mut game, keys::B);
        assert_eq!(game.scene, Scene::Paused, "B closes the card first");
        assert!(!game.menu.inspecting);
        tap(&mut game, keys::RIGHT);
        assert_eq!(game.menu.tab, pause_menu::WALLET);
        tap(&mut game, keys::R);
        assert_eq!(game.menu.tab, pause_menu::CONTROLS);
        tap(&mut game, keys::L);
        tap(&mut game, keys::LEFT);
        assert_eq!(game.menu.tab, pause_menu::BAG);
        tap(&mut game, keys::START);
        assert_eq!(game.scene, Scene::Play);
        assert_eq!(game.world.player.body.x(), x, "nothing moved");
        tap(&mut game, keys::START);
        assert_eq!(game.menu.tab, pause_menu::BAG, "the menu remembers its tab");
    }

    #[test]
    fn closing_the_menu_with_b_neither_fires_nor_charges() {
        let mut game = at_title();
        tap(&mut game, keys::START);
        play_level(&mut game);
        tap(&mut game, keys::START);
        assert_eq!(game.scene, Scene::Paused);
        // B goes down and is held well past a full charge.
        game.step(&Input::from_keys(keys::B, 0));
        assert_eq!(game.scene, Scene::Play);
        for _ in 0..crate::world::CHARGE_FRAMES + 10 {
            game.step(&Input::from_keys(keys::B, keys::B));
        }
        assert_eq!(game.world.player.charge, 0, "the held B is ignored");
        game.step(&Input::default());
        assert_eq!(game.world.player_shots(), 0);
        let events = game.step(&Input::from_keys(keys::B, 0));
        assert!(events.contains(Events::SHOOT), "a fresh press fires again");
    }

    #[test]
    fn a_charge_held_into_the_menu_is_dropped() {
        let mut game = at_title();
        tap(&mut game, keys::START);
        play_level(&mut game);
        for i in 0..crate::world::CHARGE_FRAMES + 5 {
            game.step(&Input::from_keys(keys::B, if i == 0 { 0 } else { keys::B }));
        }
        game.step(&Input::from_keys(keys::B | keys::START, keys::B));
        assert_eq!(game.scene, Scene::Paused);
        game.step(&Input::from_keys(keys::START, 0));
        game.step(&Input::default());
        assert_eq!(game.scene, Scene::Play);
        assert!(!game.step(&Input::default()).contains(Events::BEAM));
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

    /// From the level card into play, skipping the arrival or CL207's call.
    fn play_level(game: &mut Game) {
        play_level_card(game);
        if matches!(game.scene, Scene::Arrival | Scene::Dialog) {
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
    fn a_skipped_clear_screen_opens_the_map_and_b_leaves_it() {
        let mut game = at_title();
        tap(&mut game, keys::DOWN);
        tap(&mut game, keys::A);
        tap(&mut game, keys::A);
        assert_eq!(game.scene, Scene::Intro);
        play_level(&mut game);
        let mut events = Events::default();
        game.finish_level(&mut events);
        for _ in 0..CLEAR_LOCK_FRAMES {
            game.step(&Input::default());
        }
        tap(&mut game, keys::A);
        assert_eq!((game.scene, game.cursor), (Scene::Map, 1));
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
    fn a_clear_records_best_and_locks_input_at_first() {
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
        assert_eq!(game.scene, Scene::Map);
    }

    /// A game at SAM's defeat in the Trenches, with `coins` picked up there, `purse`
    /// kept from before and `wallet` banked.
    fn at_the_exchange(coins: u32, purse: u32, wallet: u32, replay: bool) -> Game {
        let mut game = at_title();
        game.cleared = if replay { 0b11 } else { 0b1 };
        game.purse = purse;
        game.wallet = wallet;
        game.start_level(1);
        game.enter(Scene::Play);
        game.world.coins = coins;
        game.world.diamonds = 3;
        game.open_exchange();
        game
    }

    /// Reads lines with A until the exchange reaches `deal` or ends.
    fn exchange_until(game: &mut Game, deal: Deal) {
        for _ in 0..2000 {
            if game.scene != Scene::Exchange || game.exchange.deal == deal {
                return;
            }
            if game.talking() && game.talk_lines().get(game.talk_line as usize).is_some_and(|l| game.talk_shown as usize >= l.len()) {
                tap(game, keys::A);
            } else {
                game.step(&Input::default());
            }
        }
        panic!("stuck in {:?}", game.exchange.deal);
    }

    #[test]
    fn beating_a_casino_boss_opens_the_exchange_before_the_clear() {
        let mut game = at_title();
        game.cleared = 0b1;
        game.start_level(1);
        play_level(&mut game);
        game.world.coins = 7;
        game.world.boss.state = crate::boss::BossState::Dying;
        game.world.boss.timer = 1;
        game.world.boss.present = true;
        game.world.arena = true;
        game.step(&Input::default());
        assert_eq!(game.scene, Scene::Exchange);
        assert_eq!((game.exchange.coins, game.exchange.diamonds()), (7, 7 * EXCHANGE_RATE));
        assert!(game.files[0].is_none() || game.files[0].unwrap().cleared == 0b1, "not saved until the deal is done");
    }

    #[test]
    fn exchanging_pays_eight_diamonds_a_coin_once() {
        let mut game = at_the_exchange(25, 0, 100, false);
        exchange_until(&mut game, Deal::Offer);
        assert_eq!(game.exchange.diamonds(), 200);
        assert_eq!(game.cursor, offer_menu::EXCHANGE);
        tap(&mut game, keys::A);
        assert_eq!(game.exchange.deal, Deal::Payout);
        assert_eq!(game.wallet, 300);
        game.strike(true);
        assert_eq!(game.wallet, 300, "a struck deal stays struck");
        // Mashing A runs through the payout and the goodbye, and pays nothing twice.
        for _ in 0..20 {
            tap(&mut game, keys::A);
        }
        assert_eq!(game.scene, Scene::LevelClear);
        assert_eq!(game.wallet, 303, "the level's own diamonds are banked too");
        let file = game.files[0].unwrap();
        assert_eq!((file.wallet, file.purse, file.cleared), (303, 0, 0b11), "saved with the clear");
        assert_eq!(levels::MAP[2].status(file.cleared), levels::PlaceStatus::Upcoming);
    }

    #[test]
    fn keeping_the_coins_carries_them_to_the_next_exchange() {
        let mut game = at_the_exchange(25, 4, 0, false);
        exchange_until(&mut game, Deal::Offer);
        assert_eq!(game.exchange.coins, 29);
        tap(&mut game, keys::DOWN);
        assert_eq!(game.cursor, offer_menu::KEEP);
        tap(&mut game, keys::A);
        exchange_until(&mut game, Deal::Walk);
        assert_eq!(game.scene, Scene::LevelClear);
        assert_eq!((game.wallet, game.purse), (3, 29));
        assert_eq!(game.files[0].unwrap().purse, 29);

        // A replay pays nothing for its own coins, but the purse can still be changed.
        let mut replay = at_the_exchange(50, 29, 3, true);
        assert_eq!(replay.exchange.coins, 29, "a replay's coins aren't on the counter");
        exchange_until(&mut replay, Deal::Offer);
        tap(&mut replay, keys::A);
        exchange_until(&mut replay, Deal::Walk);
        assert_eq!((replay.wallet, replay.purse), (3 + 29 * EXCHANGE_RATE, 0), "no diamonds banked from a replay");
    }

    #[test]
    fn no_coins_means_no_offer() {
        let mut game = at_the_exchange(0, 0, 0, false);
        exchange_until(&mut game, Deal::Offer);
        assert_eq!(game.scene, Scene::LevelClear, "the penguin just says goodbye");
        assert_eq!(game.exchange.accepted, None);
    }

    #[test]
    fn the_wallet_and_parts_come_back_with_the_file() {
        let mut game = at_the_exchange(2, 0, 0, false);
        game.world.parts = 0b101;
        exchange_until(&mut game, Deal::Offer);
        tap(&mut game, keys::A);
        exchange_until(&mut game, Deal::Walk);
        let mut again = Game::with_save(game.best, game.difficulty, game.files);
        again.open_files();
        tap(&mut again, keys::A);
        tap(&mut again, keys::A);
        assert_eq!((again.wallet, again.purse, again.parts), (3 + 2 * EXCHANGE_RATE, 0, 0b101));
        assert_eq!(again.diamond_balance(), again.wallet);
    }

    #[test]
    fn cl207_calls_on_a_first_visit_to_the_trenches() {
        let mut game = at_title();
        game.cleared = 0b1;
        game.start_level(1);
        play_level_card(&mut game);
        assert_eq!((game.scene, game.talk), (Scene::Dialog, Talk::Call));
        assert!(game.ringing() && !game.talking());
        for _ in 0..CALL_RING_FRAMES {
            game.step(&Input::default());
        }
        assert!(game.talking());
        assert_eq!(game.talk_lines()[0].speaker, Speaker::Cl207);
        let mut lines = 0;
        while game.scene == Scene::Dialog {
            tap(&mut game, keys::A);
            lines += 1;
            assert!(lines < 100);
        }
        assert_eq!(game.scene, Scene::Play);

        // A replay goes straight to play.
        let mut game = at_title();
        game.cleared = 0b11;
        game.start_level(1);
        play_level_card(&mut game);
        assert_eq!(game.scene, Scene::Play);
    }

    #[test]
    fn a_dev_build_opens_the_trenches_on_a_new_file_without_saving_a_clear() {
        let mut game = at_title();
        game.dev_cleared = 0b1;
        tap(&mut game, keys::A);
        assert_eq!((game.scene, game.cursor), (Scene::Map, 1), "PLAY opens the map on the Trenches");
        tap(&mut game, keys::A);
        assert_eq!((game.scene, game.world.level_index), (Scene::Intro, 1));
        assert!(!game.replay, "a first visit: CL207 calls, the clear counts");
        assert_eq!(game.cleared, 0, "the file hasn't cleared the Badlands");
    }
}
