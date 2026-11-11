//! One playthrough of a level: the wassie, the villains, the shots, the boss, and how
//! they interact.

use alloc::{boxed::Box, vec};

use crate::{
    FX, SCREEN_H, SCREEN_W, TILE,
    boss::{Boss, BossCtx},
    enemy::{self, Actor, ActorKind, Ctx, MAX_ACTORS},
    fx::{FxKind, FxPool},
    game::{Progress, keys},
    level::Level,
    levels,
    physics::Body,
    player::{
        DIAGONAL, GRAVITY_FX, HOVER_FRAMES, MAX_FALL_FX, MAX_HP, PLAYER_H, PLAYER_W, Player, PlayerState,
        RUN_MAX_FX, STOMP_BOUNCE_FX,
    },
    rng::Rng,
    shot::{self, MAX_SHOTS, Shot, ShotKind},
};

/// Frames B must be held for the charged beam.
pub const CHARGE_FRAMES: u16 = 45;
/// Frames between blaster bolts.
pub const FIRE_COOLDOWN: u8 = 10;
pub const BEAM_COOLDOWN: u8 = 24;
/// Bolts the wassie may have in flight at once.
pub const MAX_PLAYER_SHOTS: usize = 3;
const BOLT_FX: i32 = 4 * FX;
const BEAM_FX: i32 = 6 * FX;
/// Frames the knocked-out pose shows before respawning.
pub const DEATH_FRAMES: u16 = 90;
/// Frames an ordinary shot jolts the screen for, a pixel each way.
pub const KICK_FRAMES: u8 = 4;
/// How fast a knocked-out wassie skids to a stop on the floor, per frame.
const DEAD_SKID_FX: i32 = 24;
/// Frames between falling into a pit and respawning.
pub const PIT_FRAMES: u16 = 30;
pub const SPIKE_DAMAGE: i32 = 2;
/// Electric fences run on a cycle: dead, a warning blink, then live for a few seconds.
/// Live, a fence's curtain of lightning is a wall from its emitters up to the rock
/// above: touching it shocks, and nothing gets through until it goes dead again.
const FENCE_CYCLE: u32 = 360;
const FENCE_WARN: u32 = 60;
const FENCE_LIVE: u32 = 150;

/// The fences' state at a frame: 0 dead, 1 warning blink, 2 live.
pub fn fence_state(frames: u32) -> u8 {
    let t = frames % FENCE_CYCLE;
    let dead = FENCE_CYCLE - FENCE_WARN - FENCE_LIVE;
    if t < dead {
        0
    } else if t < dead + FENCE_WARN {
        (t / 6 % 2) as u8
    } else {
        2
    }
}
/// Frames until a fence goes live again, or 0 while it is warning or live.
pub fn fence_open_for(frames: u32) -> u32 {
    let t = frames % FENCE_CYCLE;
    (FENCE_CYCLE - FENCE_WARN - FENCE_LIVE).saturating_sub(t)
}

pub const PIT_DAMAGE: i32 = 2;
pub const HEAL_AMOUNT: i32 = 2;
/// The camera keeps the wassie's centre inside this band of the screen.
const CAMERA_BAND: (i32, i32) = (96, 136);
/// And vertically, in a level taller than the screen: a little above the middle, so
/// more shows below than above when dropping down a shaft.
const CAMERA_BAND_Y: (i32, i32) = (56, 100);
/// How fast the camera pans to another storey of a level built in storeys, in pixels
/// a frame: faster than the wassie falls.
const CAMERA_PAN: i32 = 10;
/// How far from the elevator's car its terminal may stand for the car's call button
/// to work it, in pixels. A terminal further off (a breaker somewhere else) has to be
/// switched on where it stands.
const CALL_REACH: i32 = 64;
/// The bit of the file's part mask that records the jetpack, found in a level that
/// hides it. The other bits are CL207's plasma cells.
pub const GEAR_JETPACK: u16 = 1 << 15;

/// The plasma cells in a part mask, without the gear bits.
pub fn cells(parts: u16) -> u16 {
    parts & !GEAR_JETPACK
}
/// Actors wake once this close to the screen edges.
const WAKE_MARGIN: i32 = 16;
/// Frames between jetpack exhaust puffs.
const PUFF_EVERY: u32 = 6;
/// Frames between dust puffs kicked up by running and braking.
const DUST_EVERY: u32 = 10;
const SKID_EVERY: u32 = 4;

/// Buttons for one frame: what is held and what went down this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub held: u16,
    pub pressed: u16,
}

impl Input {
    /// Maps this frame's and last frame's held buttons to game input. The ROM and the
    /// emulator harness both use this, so they agree on what a frame's keys mean.
    pub fn from_keys(held: u16, previous: u16) -> Self {
        Self {
            held,
            pressed: held & !previous,
        }
    }

    /// -1, 0 or 1. Holding both directions cancels out.
    pub fn dir_x(&self) -> i32 {
        (self.held & keys::RIGHT != 0) as i32 - (self.held & keys::LEFT != 0) as i32
    }

    pub fn up(&self) -> bool {
        self.held & keys::UP != 0
    }

    pub fn down(&self) -> bool {
        self.held & keys::DOWN != 0
    }

    /// Stand still and aim with the d-pad.
    pub fn plant(&self) -> bool {
        self.held & keys::PLANT != 0
    }

    pub fn jump_pressed(&self) -> bool {
        self.pressed & keys::JUMP != 0
    }

    pub fn jump_held(&self) -> bool {
        self.held & keys::JUMP != 0
    }

    pub fn fire_pressed(&self) -> bool {
        self.pressed & keys::FIRE != 0
    }

    pub fn fire_held(&self) -> bool {
        self.held & keys::FIRE != 0
    }

    pub fn start_pressed(&self) -> bool {
        self.pressed & keys::START != 0
    }

    /// Up went down this frame: works a terminal or calls the elevator.
    pub fn up_pressed(&self) -> bool {
        self.pressed & keys::UP != 0
    }
}

/// Things that happened during a step, for sound effects and saving.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events(pub u32);

impl Events {
    pub const JUMP: Events = Events(1 << 0);
    pub const SHOOT: Events = Events(1 << 1);
    pub const BEAM: Events = Events(1 << 2);
    /// The wassie took damage.
    pub const HIT: Events = Events(1 << 3);
    pub const DIE: Events = Events(1 << 4);
    pub const ENEMY_DOWN: Events = Events(1 << 5);
    pub const PICKUP: Events = Events(1 << 6);
    pub const CHECKPOINT: Events = Events(1 << 7);
    pub const BOSS_START: Events = Events(1 << 8);
    pub const BOSS_HIT: Events = Events(1 << 9);
    pub const BOSS_DOWN: Events = Events(1 << 10);
    pub const LEVEL_CLEAR: Events = Events(1 << 11);
    pub const RESPAWN: Events = Events(1 << 12);
    /// Progress or an option changed and should be written to SRAM.
    pub const SAVE: Events = Events(1 << 13);
    /// The boss slammed the ground.
    pub const SLAM: Events = Events(1 << 14);
    /// The wassie touched down from a real fall.
    pub const LAND: Events = Events(1 << 15);
    /// Menus: the cursor moved, something was picked, the menu backed out, or the
    /// pick isn't allowed (a locked place on the map).
    pub const MENU_MOVE: Events = Events(1 << 16);
    pub const MENU_OK: Events = Events(1 << 17);
    pub const MENU_BACK: Events = Events(1 << 18);
    pub const MENU_DENY: Events = Events(1 << 19);
    /// A gold coin, picked up or paid out as a bounty.
    pub const COIN: Events = Events(1 << 20);
    /// A plasma cell for CL207's shield.
    pub const PART: Events = Events(1 << 21);
    /// The elevator terminal powered up.
    pub const POWER: Events = Events(1 << 22);
    /// The elevator doors opened, or it set off.
    pub const LIFT: Events = Events(1 << 23);
    /// SAM swung the Cucumber Hit.
    pub const SWING: Events = Events(1 << 24);
    /// The electric fences went live with one on screen.
    pub const FENCE_ON: Events = Events(1 << 25);
    /// A live fence shocked the wassie.
    pub const SHOCK: Events = Events(1 << 26);
    /// The wassie picked up a piece of gear (the jetpack).
    pub const GEAR: Events = Events(1 << 27);

    pub fn contains(self, other: Events) -> bool {
        self.0 & other.0 == other.0 && other.0 != 0
    }

    pub fn insert(&mut self, other: Events) {
        self.0 |= other.0;
    }
}

/// Picked in the options screen and saved with the best score.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Difficulty {
    /// Half damage, slower enemy fire.
    Chill,
    #[default]
    Normal,
    /// Double damage, faster enemy fire.
    Degen,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Chill, Difficulty::Normal, Difficulty::Degen];

    pub fn damage(self, base: i32) -> i32 {
        match self {
            Difficulty::Chill => (base + 1) / 2,
            Difficulty::Normal => base,
            Difficulty::Degen => base * 2,
        }
    }

    /// Scales a delay between enemy shots.
    pub fn fire_interval(self, base: u16) -> u16 {
        match self {
            Difficulty::Chill => base * 3 / 2,
            Difficulty::Normal => base,
            Difficulty::Degen => base * 2 / 3,
        }
    }

    pub fn from_u8(value: u8) -> Option<Difficulty> {
        Difficulty::ALL.get(value as usize).copied()
    }

    pub fn next(self) -> Difficulty {
        Difficulty::ALL[(self as usize + 1) % Difficulty::ALL.len()]
    }

    pub fn previous(self) -> Difficulty {
        Difficulty::ALL[(self as usize + Difficulty::ALL.len() - 1) % Difficulty::ALL.len()]
    }
}

/// A blank actor table, made on the heap without passing through the stack. The table
/// is 5 KB, and on the GBA the stack shares 32 KB of IWRAM with the sound mixer:
/// building worlds with it inline overflowed there.
fn new_actors() -> Box<[Actor; MAX_ACTORS]> {
    let table: Box<[Actor]> = vec![Actor::EMPTY; MAX_ACTORS].into_boxed_slice();
    match table.try_into() {
        Ok(table) => table,
        Err(_) => unreachable!("the vec has MAX_ACTORS entries"),
    }
}

/// Whether the wassie, with hitbox `r`, can work the terminal at `t`: standing at it,
/// within a few pixels either side, rather than having to overlap its narrow kiosk.
fn reaches_terminal(r: &crate::physics::Rect, t: &crate::physics::Rect) -> bool {
    (r.centre_x() - t.centre_x()).abs() <= t.w / 2 + 10 && r.bottom() > t.y && r.y < t.bottom()
}

/// Coins SAM's stash spills when he goes down.
pub const BOSS_STASH: u32 = 10;
/// The elevator car: 3 tiles wide and 4 tall.
pub const LIFT_W: i32 = 24;
pub const LIFT_H: i32 = 32;
/// Frames the terminal takes to bring the power up, the doors to slide, and the ride.
pub const LIFT_POWER_FRAMES: u16 = 90;
pub const LIFT_DOOR_FRAMES: u16 = 16;
pub const LIFT_RIDE_FRAMES: u16 = 72;
/// Sealed walls the camera can be held between.
const MAX_SEALS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LiftState {
    /// No power: the terminal is off.
    Dark,
    /// The terminal is on and the lights come up floor by floor.
    Powering,
    Opening,
    /// Waiting with its doors open for the wassie to step in and press Up.
    Open,
    /// The wassie is in; the doors shut.
    Closing,
    /// Going up. Halfway, the car arrives at the top.
    Riding,
    /// The doors open on the upper floor.
    Arriving,
    /// Done: the wassie has stepped out upstairs.
    Done,
}

/// The elevator between the casino floor and the floors above.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lift {
    /// Left edge and floor of the car downstairs, and of the one it arrives at.
    pub car: (i32, i32),
    pub dest: (i32, i32),
    pub state: LiftState,
    /// Frames into the state.
    pub timer: u16,
}

impl Lift {
    /// Whether the wassie is inside, out of sight and out of control.
    pub fn riding(&self) -> bool {
        matches!(self.state, LiftState::Closing | LiftState::Riding | LiftState::Arriving)
    }

    /// How far the doors of the car at `dest` (or downstairs) are open, 0 shut to 16.
    pub fn doors(&self, upstairs: bool) -> u16 {
        let t = self.timer.min(LIFT_DOOR_FRAMES);
        match (self.state, upstairs) {
            (LiftState::Opening, false) => t,
            (LiftState::Open, false) => LIFT_DOOR_FRAMES,
            (LiftState::Closing, false) => LIFT_DOOR_FRAMES - t,
            (LiftState::Arriving, true) => t,
            (LiftState::Done, true) => LIFT_DOOR_FRAMES,
            _ => 0,
        }
    }

    /// Whether the wassie stands in the doorway of the car downstairs.
    fn at_door(&self, r: &crate::physics::Rect) -> bool {
        let (x, feet) = self.car;
        (x + 4..x + LIFT_W - 4).contains(&r.centre_x()) && (feet - 4..=feet).contains(&r.bottom())
    }

    /// Whether a terminal at `t` stands by the car downstairs, where the car's call
    /// button can work it.
    fn beside(&self, t: &crate::physics::Rect) -> bool {
        let (x, feet) = self.car;
        (t.centre_x() - (x + LIFT_W / 2)).abs() <= CALL_REACH && (t.bottom() - feet).abs() <= TILE
    }
}

#[derive(Clone, Debug)]
pub struct World {
    pub level: &'static Level,
    pub level_index: u8,
    pub difficulty: Difficulty,
    pub player: Player,
    /// On the heap, filled there: see `new_actors`.
    pub actors: Box<[Actor; MAX_ACTORS]>,
    /// Slots of `actors` the map filled, from the first; those after stay empty. The
    /// per-frame loops stop there instead of walking all of them in slow EWRAM.
    used: usize,
    pub shots: [Shot; MAX_SHOTS],
    pub boss: Boss,
    pub fx: FxPool,
    /// Frames of screen shake left: kept for big hits, not every shot.
    pub shake: u8,
    /// Frames of the light jolt each ordinary shot gives the screen.
    pub kick: u8,
    /// Left edge of the screen in level pixels.
    pub camera_x: i32,
    /// Top edge of the screen in level pixels: 0 in a level one screen tall.
    pub camera_y: i32,
    /// Where the camera's top settles in the boss arena.
    pub arena_y: i32,
    /// The level hides the jetpack: the wassie starts without it.
    pub jetpack_here: bool,
    /// Tile the wassie respawns on, and how many checkpoints have been lit.
    pub checkpoint: (i32, i32),
    pub checkpoints_lit: u8,
    /// The boss arena has closed behind the wassie.
    pub arena: bool,
    /// The boss has words for the wassie once the camera settles on the arena.
    pub talk_due: bool,
    /// The boss has had its say this visit, so a retry goes straight to the fight.
    pub talked: bool,
    pub diamonds: u32,
    /// Gold coins picked up and paid out in this level.
    pub coins: u32,
    /// Shield parts this file owns, one bit each, including any picked up here.
    pub parts: u16,
    pub kills: u32,
    pub lift: Option<Lift>,
    /// Sealed walls, as (first column, column after), left to right.
    seals: [(i32, i32); MAX_SEALS],
    /// Frames played, for the clear time.
    pub frames: u32,
    pub cleared: bool,
    /// Test helper: the wassie takes no damage.
    pub god: bool,
    rng: Rng,
}

impl World {
    pub fn new(level_index: u8, difficulty: Difficulty) -> Self {
        Self::with_level(levels::get(level_index), level_index, difficulty)
    }

    pub fn with_level(level: &'static Level, level_index: u8, difficulty: Difficulty) -> Self {
        // One pass over the map's markers for its actors and the places that matter.
        let mut actors = new_actors();
        let mut count = 0;
        let (mut start, mut car, mut dest, mut jetpack, mut boss_row) = (None, None, None, false, None);
        for (c, tx, ty) in level.markers() {
            match c {
                b'P' if start.is_none() => start = Some((tx, ty)),
                b'L' if car.is_none() => car = Some((tx, ty)),
                b'l' if dest.is_none() => dest = Some((tx, ty)),
                b'J' => jetpack = true,
                b'B' if boss_row.is_none() => boss_row = Some(ty),
                _ => {}
            }
            if let Some(kind) = ActorKind::from_marker(c)
                && count < MAX_ACTORS
            {
                actors[count] = Actor::spawn(kind, tx, ty);
                count += 1;
            }
        }
        let start = start.unwrap_or((2, level.height() - 2));
        let lift = car.map(|(tx, ty)| {
            let dest = dest.unwrap_or((tx, ty));
            Lift {
                car: (tx * TILE, (ty + 1) * TILE),
                dest: (dest.0 * TILE, (dest.1 + 1) * TILE),
                state: LiftState::Dark,
                timer: 0,
            }
        });
        let mut seals = [(i32::MAX, i32::MAX); MAX_SEALS];
        let mut sealed = 0;
        let mut tx = 0;
        while tx < level.width() && sealed < MAX_SEALS {
            if level.sealed(tx) {
                let start = tx;
                while tx < level.width() && level.sealed(tx) {
                    tx += 1;
                }
                seals[sealed] = (start, tx);
                sealed += 1;
            }
            tx += 1;
        }
        let mut world = Self {
            level,
            level_index,
            difficulty,
            player: Player::new(Body::standing_on_tile(start.0, start.1, PLAYER_W, PLAYER_H)),
            actors,
            used: count,
            shots: [Shot::EMPTY; MAX_SHOTS],
            boss: Boss::new(level),
            fx: FxPool::new(),
            shake: 0,
            kick: 0,
            camera_x: 0,
            camera_y: 0,
            arena_y: 0,
            jetpack_here: jetpack,
            checkpoint: start,
            checkpoints_lit: 0,
            arena: false,
            talk_due: false,
            talked: false,
            diamonds: 0,
            coins: 0,
            parts: 0,
            kills: 0,
            lift,
            seals,
            frames: 0,
            cleared: false,
            god: false,
            rng: Rng::new(0x5EED ^ level_index as u32),
        };
        world.player.jetpack = !world.jetpack_here;
        if let Some(ty) = boss_row {
            // The arena's floor sits four rows above the bottom of the screen, as in a
            // level one screen tall.
            world.arena_y = ((ty + 5) * TILE - SCREEN_H).clamp(0, world.max_camera_y());
        }
        world.snap_camera();
        world
    }

    /// The actors the map placed: every slot that can hold one.
    pub fn placed_actors(&self) -> &[Actor] {
        &self.actors[..self.used]
    }

    fn max_camera_y(&self) -> i32 {
        (self.level.px_height() - SCREEN_H).max(0)
    }

    /// Whether the wassie owns the jetpack in this level.
    pub fn has_jetpack(&self) -> bool {
        !self.jetpack_here || self.parts & GEAR_JETPACK != 0
    }

    /// The actors taken so far, one bit per actor slot: pickups collected, villains
    /// beaten for good, beacons lit and terminals on. Saved with the run so an
    /// explorable level comes back exactly as it was left.
    pub fn taken(&self) -> u128 {
        self.actors[..self.used].iter().enumerate().fold(0, |mask, (i, a)| {
            let done = a.killed || (matches!(a.kind, ActorKind::Checkpoint | ActorKind::Terminal) && a.state == enemy::state::LIT);
            if done { mask | 1 << i } else { mask }
        })
    }

    pub fn score(&self) -> u32 {
        Self::score_for(self.diamonds, self.coins, self.kills, self.boss.is_beaten())
    }

    pub fn score_for(diamonds: u32, coins: u32, kills: u32, boss_beaten: bool) -> u32 {
        let boss = if boss_beaten { 1000 } else { 0 };
        (diamonds + coins) * 10 + kills * 50 + boss
    }

    /// The file already owns the parts in `mask`: those here are gone from the map.
    pub fn own_parts(&mut self, mask: u16) {
        self.parts |= mask;
        if mask & GEAR_JETPACK != 0 {
            for a in self.actors[..self.used].iter_mut().filter(|a| a.kind == ActorKind::Jetpack) {
                a.alive = false;
                a.killed = true;
            }
        }
        self.player.jetpack = self.has_jetpack();
        let base = self.level.parts_base;
        for (n, a) in self.actors[..self.used].iter_mut().filter(|a| a.kind == ActorKind::Part).enumerate() {
            if mask & (1 << (base as usize + n)) != 0 {
                a.alive = false;
                a.killed = true;
            }
        }
    }

    /// Shield parts this level holds, and how many of them the file owns.
    pub fn parts_here(&self) -> (u32, u32) {
        let total = self.actors[..self.used].iter().filter(|a| a.kind == ActorKind::Part).count() as u32;
        let base = self.level.parts_base as u32;
        let mask = ((1u32 << total) - 1) << base;
        (total, (self.parts as u32 & mask).count_ones())
    }

    /// The level of a saved run, picked up at its last beacon. Everything that started
    /// behind the beacon was dealt with on the way: pickups and villains there are gone,
    /// and the beacons there are lit.
    pub fn resume(progress: &Progress, difficulty: Difficulty) -> Self {
        let mut world = Self::new(progress.level, difficulty);
        world.pick_up(progress);
        world
    }

    fn pick_up(&mut self, progress: &Progress) {
        let Some((tx, ty)) = progress.checkpoint else {
            return;
        };
        let edge = if self.level.explore {
            // No left-to-right order to go by: put back exactly what was taken.
            for (i, a) in self.actors[..self.used].iter_mut().enumerate() {
                if progress.taken & (1 << i) == 0 {
                    continue;
                }
                if matches!(a.kind, ActorKind::Checkpoint | ActorKind::Terminal) {
                    a.state = enemy::state::LIT;
                } else {
                    a.alive = false;
                    a.killed = true;
                }
            }
            i32::MIN
        } else {
            (tx + 1) * TILE
        };
        for a in self.actors[..self.used].iter_mut().filter(|a| a.alive) {
            if a.home.0 + a.kind.size().0 / 2 >= edge {
                continue;
            }
            if matches!(a.kind, ActorKind::Checkpoint | ActorKind::Terminal) {
                a.state = enemy::state::LIT;
            } else {
                a.alive = false;
                a.killed = true;
            }
        }
        self.checkpoint = (tx, ty);
        self.checkpoints_lit = progress.beacons;
        self.diamonds = progress.diamonds;
        self.coins = progress.coins;
        self.kills = progress.kills;
        self.frames = progress.frames;
        if let Some(mut lift) = self.lift {
            let powered = self.actors[..self.used].iter().any(|a| a.kind == ActorKind::Terminal && a.state == enemy::state::LIT);
            if self.level.explore {
                // Upstairs, it was ridden; downstairs with its power on, it waits open.
                if self.upstairs(&lift, tx * TILE + TILE / 2, (ty + 1) * TILE) {
                    lift.state = LiftState::Done;
                } else if powered {
                    lift.state = LiftState::Open;
                    lift.timer = LIFT_DOOR_FRAMES;
                }
            } else if edge > lift.car.0 {
                // A beacon past the elevator: it was ridden on the way.
                lift.state = if edge > lift.dest.0 { LiftState::Done } else { LiftState::Open };
            }
            self.lift = Some(lift);
        }
        self.place_player(MAX_HP);
        self.snap_camera();
    }

    pub fn player_shots(&self) -> usize {
        self.shots
            .iter()
            .filter(|s| s.active && s.kind.from_player())
            .count()
    }

    pub fn step(&mut self, input: &Input) -> Events {
        let mut events = Events::default();
        if self.cleared {
            return events;
        }
        self.frames += 1;
        self.fx.update();
        self.shake = self.shake.saturating_sub(1);
        self.kick = self.kick.saturating_sub(1);

        if self.player.is_dead() {
            // The count only starts once the body has come down: a wassie knocked out
            // in the air drops to the ground first, and only then lies still.
            if self.fall_out(&mut events) {
                self.player.dead_timer = self.player.dead_timer.saturating_sub(1);
            }
            if self.player.dead_timer == 0 {
                self.respawn(&mut events);
            }
            return events;
        }
        if self.step_lift(input, &mut events) {
            return events;
        }

        self.player.update(input, self.level, &mut events);
        if self.player.hovering && self.frames % PUFF_EVERY == 0 {
            let r = self.player.body.rect();
            self.fx.spawn(FxKind::Puff, r.centre_x() - self.player.facing * 9, r.bottom() - 4);
        }
        self.kick_up_dust(&events);
        self.work_terminals(input, &mut events);
        self.enter_arena(&mut events);
        self.fire(input, &mut events);
        self.update_camera();
        self.update_actors();
        let player_rect = self.player.body.rect();
        events.insert(self.boss.update(&mut BossCtx {
            level: self.level,
            player: player_rect,
            shots: &mut self.shots,
            rng: &mut self.rng,
            difficulty: self.difficulty,
        }));
        if events.contains(Events::LEVEL_CLEAR) {
            self.cleared = true;
            return events;
        }
        if events.contains(Events::SLAM) {
            self.shake = self.shake.max(10);
        }
        if events.contains(Events::BOSS_DOWN) {
            self.shake = 24;
            if self.level.coins {
                self.coins += BOSS_STASH;
                events.insert(Events::COIN);
            }
        }
        // The boss comes apart in a string of blasts while it goes down; in a casino,
        // its stash of coins spills out between them.
        if self.boss.state == crate::boss::BossState::Dying && self.boss.timer % 5 == 0 {
            let r = self.boss.rect();
            let x = r.x + self.rng.below(r.w as u32) as i32;
            let y = r.y + self.rng.below(r.h as u32) as i32;
            if self.boss.timer % 10 == 0 {
                self.fx.spawn(FxKind::Explosion, x, y);
            } else if self.level.coins {
                self.fx.spawn(FxKind::CoinPop, x, y);
            }
        }
        for s in self.shots.iter_mut().filter(|s| s.active) {
            s.update(self.level);
            let r = s.rect();
            if !s.active && s.kind.from_player() && s.life > 0 {
                // Ran into rock.
                self.fx.spawn(FxKind::Impact, r.centre_x(), r.centre_y());
            }
            if !s.active && matches!(s.kind, ShotKind::Cucumber | ShotKind::MegaCucumber) {
                self.fx.spawn(FxKind::Splat, r.centre_x(), r.centre_y());
            }
            let off_side = r.right() < self.camera_x - 16 || r.x > self.camera_x + SCREEN_W + 16;
            // Well above or below the screen too, in a tall level.
            let off_end = r.bottom() < self.camera_y - SCREEN_H || r.y > self.camera_y + 2 * SCREEN_H;
            if off_side || off_end {
                s.active = false;
            }
        }
        self.collide(&mut events);
        self.hazards(&mut events);
        events
    }

    /// Runs the elevator. Returns true while the wassie rides it, when nothing else
    /// moves.
    fn step_lift(&mut self, input: &Input, events: &mut Events) -> bool {
        let Some(mut lift) = self.lift else {
            return false;
        };
        lift.timer = lift.timer.saturating_add(1);
        let next = |lift: &mut Lift, state| {
            lift.state = state;
            lift.timer = 0;
        };
        let r = self.player.body.rect();
        match lift.state {
            LiftState::Dark => {
                let on = self.actors[..self.used].iter().any(|a| a.kind == ActorKind::Terminal && a.state == enemy::state::LIT);
                if on {
                    next(&mut lift, LiftState::Powering);
                }
            }
            LiftState::Powering if lift.timer >= LIFT_POWER_FRAMES => {
                next(&mut lift, LiftState::Opening);
                // Its doors are heard only near it, not from a breaker far away.
                let (x, feet) = lift.car;
                let (cx, cy) = (self.camera_x + SCREEN_W / 2, self.camera_y + SCREEN_H / 2);
                if (x - cx).abs() < SCREEN_W && (feet - cy).abs() < SCREEN_H {
                    events.insert(Events::LIFT);
                }
            }
            LiftState::Opening if lift.timer >= LIFT_DOOR_FRAMES => next(&mut lift, LiftState::Open),
            // Up held counts too, so walking in while aiming up still boards.
            LiftState::Open if input.up() && self.player.grounded && self.player.hurt == 0 && lift.at_door(&r) =>
            {
                next(&mut lift, LiftState::Closing);
                // Step to the middle of the car, facing out.
                let (x, feet) = lift.car;
                self.player.body.x_fx = (x + LIFT_W / 2 - PLAYER_W / 2) * FX;
                self.player.body.y_fx = (feet - PLAYER_H) * FX;
                self.player.body.vx_fx = 0;
                self.player.body.vy_fx = 0;
                self.player.state = PlayerState::Idle;
                self.player.charge = 0;
                self.shots = [Shot::EMPTY; MAX_SHOTS];
                events.insert(Events::LIFT);
            }
            LiftState::Closing if lift.timer >= LIFT_DOOR_FRAMES => next(&mut lift, LiftState::Riding),
            LiftState::Riding => {
                if lift.timer == LIFT_RIDE_FRAMES / 2 {
                    // Out of sight, the car reaches the top.
                    let (x, feet) = lift.dest;
                    self.player.body.x_fx = (x + LIFT_W / 2 - PLAYER_W / 2) * FX;
                    self.player.body.y_fx = (feet - PLAYER_H) * FX;
                    self.player.facing = 1;
                    self.fx.clear();
                    self.snap_camera();
                }
                if lift.timer >= LIFT_RIDE_FRAMES {
                    next(&mut lift, LiftState::Arriving);
                    events.insert(Events::LIFT);
                }
            }
            LiftState::Arriving if lift.timer >= LIFT_DOOR_FRAMES => {
                next(&mut lift, LiftState::Done);
                // Upstairs is where the wassie comes back after a fall now, unless a
                // beacon further on is lit: a respawn on the casino floor would leave
                // it below a lift it has already taken.
                let arrived = ((lift.dest.0 + LIFT_W / 2).div_euclid(TILE), lift.dest.1.div_euclid(TILE) - 1);
                let (cx, cy) = self.checkpoint;
                let further = if self.level.explore {
                    self.upstairs(&lift, cx * TILE + TILE / 2, (cy + 1) * TILE)
                } else {
                    cx >= arrived.0
                };
                if !further {
                    self.checkpoint = arrived;
                }
            }
            // Back downstairs after the ride (a resumed run, say): the car waits open again.
            LiftState::Done if self.level.explore && !self.upstairs(&lift, r.centre_x(), r.bottom()) => {
                lift.state = LiftState::Open;
                lift.timer = LIFT_DOOR_FRAMES;
            }
            LiftState::Done if !self.level.explore && r.centre_x() < lift.dest.0 && lift.car.0 < lift.dest.0 => {
                lift.state = LiftState::Open;
                lift.timer = LIFT_DOOR_FRAMES;
            }
            _ => {}
        }
        let riding = lift.riding();
        self.lift = Some(lift);
        riding
    }

    /// Whether a wassie with its centre at `x` and feet at `feet` is up where the
    /// elevator goes. In a level that runs left to right that is past the car; in an
    /// explorable one, the floor it arrives on and above, from the car's column on.
    fn upstairs(&self, lift: &Lift, x: i32, feet: i32) -> bool {
        if self.level.explore {
            feet <= lift.dest.1 && x + TILE >= lift.dest.0
        } else {
            x >= lift.dest.0
        }
    }

    /// Up at a dark terminal switches it on, whether pressed there or held on the way.
    /// So does Up at the dark elevator's door, if its terminal stands by the car: the
    /// call button works it too.
    fn work_terminals(&mut self, input: &Input, events: &mut Events) {
        if !input.up() || self.player.hurt > 0 {
            return;
        }
        let r = self.player.body.rect();
        let dark_car = self
            .lift
            .filter(|l| l.state == LiftState::Dark && self.player.grounded && l.at_door(&r));
        for a in self.actors[..self.used].iter_mut() {
            let called = dark_car.is_some_and(|l| l.beside(&a.rect()));
            if a.kind == ActorKind::Terminal
                && a.alive
                && a.state != enemy::state::LIT
                && (called || reaches_terminal(&r, &a.rect()))
            {
                a.state = enemy::state::LIT;
                let ar = a.rect();
                self.fx.spawn(FxKind::Sparks, ar.centre_x(), ar.y + 4);
                events.insert(Events::POWER);
            }
        }
    }

    /// Whether the wassie stands where Up would do something: a dark terminal, or the
    /// open elevator. For the prompt over it.
    pub fn up_prompt(&self) -> Option<(i32, i32)> {
        let r = self.player.body.rect();
        let dark = |a: &&Actor| a.kind == ActorKind::Terminal && a.alive && a.state != enemy::state::LIT;
        if let Some(a) = self.actors[..self.used].iter().filter(dark).find(|a| reaches_terminal(&r, &a.rect())) {
            let ar = a.rect();
            return Some((ar.centre_x(), ar.y - 4));
        }
        let lift = self.lift?;
        let callable = |a: &Actor| dark(&a) && lift.beside(&a.rect());
        let ready = lift.state == LiftState::Open || (lift.state == LiftState::Dark && self.actors[..self.used].iter().any(callable));
        (ready && lift.at_door(&r)).then_some((lift.car.0 + LIFT_W / 2, lift.car.1 - LIFT_H - 4))
    }

    /// Whether the wassie stands by the dark elevator with no terminal there to power
    /// it: its power is switched on somewhere else. For the sign on the car.
    pub fn lift_unpowered(&self) -> bool {
        let Some(lift) = self.lift else {
            return false;
        };
        let r = self.player.body.rect();
        let (x, feet) = lift.car;
        let near = (r.centre_x() - (x + LIFT_W / 2)).abs() <= 48 && (r.bottom() - feet).abs() <= 2 * TILE;
        let terminal_here = self.actors[..self.used].iter().any(|a| a.kind == ActorKind::Terminal && lift.beside(&a.rect()));
        lift.state == LiftState::Dark && near && !terminal_here
    }

    /// Dust at the wassie's feet: a burst on take-off and on a hard landing, a puff
    /// behind the heel every few strides, and a spray in front while skidding.
    fn kick_up_dust(&mut self, events: &Events) {
        let p = &self.player;
        let r = p.body.rect();
        let (x, feet) = (r.centre_x(), r.bottom() - 3);
        if events.contains(Events::JUMP) || events.contains(Events::LAND) {
            self.fx.spawn(FxKind::Kick, x, feet);
        } else if p.braking && self.frames % SKID_EVERY == 0 {
            let ahead = p.body.vx_fx.signum();
            self.fx.spawn(FxKind::Dust, x + ahead * 6, feet);
        } else if p.state == PlayerState::Run
            && p.body.vx_fx.abs() >= RUN_MAX_FX
            && self.frames % DUST_EVERY == 0
        {
            let behind = -p.body.vx_fx.signum();
            self.fx.spawn(FxKind::Dust, x + behind * 6, feet);
        }
    }

    fn enter_arena(&mut self, events: &mut Events) {
        if !self.boss.present {
            return;
        }
        let (left, _) = self.boss.arena;
        // In a tall level, other floors may run under the arena: it closes only on
        // the wassie up on its own floor.
        let centre = self.player.body.rect().centre_y();
        let level_with_it = (self.arena_y..self.arena_y + SCREEN_H).contains(&centre);
        if !self.arena && level_with_it && self.player.body.x() >= left + 2 * TILE {
            self.arena = true;
            self.talk_due = !self.talked && !self.level.boss_talk.is_empty();
            self.boss.wake();
            events.insert(Events::BOSS_START);
        }
        if self.arena && self.player.body.x() < left {
            self.player.body.x_fx = left * FX;
            self.player.body.vx_fx = self.player.body.vx_fx.max(0);
        }
    }

    fn fire(&mut self, input: &Input, events: &mut Events) {
        let p = &mut self.player;
        if p.hurt > 0 {
            return;
        }
        let (ax, ay) = p.aim.dir();
        let (dx, dy) = (ax * p.facing, ay);
        let speed = |base: i32| if p.aim.diagonal() { base * DIAGONAL / FX } else { base };
        let muzzle = p.muzzle();

        if input.fire_held() {
            p.charge = p.charge.saturating_add(1);
        }
        let release = !input.fire_held() && p.charge > 0;
        if release && p.charge >= CHARGE_FRAMES && p.fire_cooldown == 0 {
            let fired = shot::spawn(
                &mut self.shots,
                ShotKind::Beam,
                muzzle.0,
                muzzle.1,
                dx * speed(BEAM_FX),
                dy * speed(BEAM_FX),
            );
            if fired {
                p.fire_cooldown = BEAM_COOLDOWN;
                p.recoil = 10;
                self.shake = self.shake.max(4);
                events.insert(Events::BEAM);
            }
        }
        if release {
            p.charge = 0;
        }

        let in_flight = self
            .shots
            .iter()
            .filter(|s| s.active && s.kind == ShotKind::Bolt)
            .count();
        if input.fire_pressed()
            && p.fire_cooldown == 0
            && in_flight < MAX_PLAYER_SHOTS
            && shot::spawn(
                &mut self.shots,
                ShotKind::Bolt,
                muzzle.0,
                muzzle.1,
                dx * speed(BOLT_FX),
                dy * speed(BOLT_FX),
            )
        {
            p.fire_cooldown = FIRE_COOLDOWN;
            p.recoil = 6;
            self.kick = KICK_FRAMES;
            events.insert(Events::SHOOT);
        }
    }

    /// Where the camera's top starts in `level`: on the wassie standing at its start.
    pub fn start_camera_y(level: &Level) -> i32 {
        let (tx, ty) = level.find(b'P').unwrap_or((2, level.height() - 2));
        let r = Body::standing_on_tile(tx, ty, PLAYER_W, PLAYER_H).rect();
        let (lo, hi) = Self::camera_y_range(level, r.centre_y());
        (r.centre_y() - (CAMERA_BAND_Y.0 + CAMERA_BAND_Y.1) / 2).clamp(lo, hi)
    }

    /// The tops the camera may take with the wassie's centre at pixel row `y`: the
    /// whole level, or in a level built in storeys ([`Level::bands`]), the storey it
    /// is in.
    pub fn camera_y_range(level: &Level, y: i32) -> (i32, i32) {
        let max = (level.px_height() - SCREEN_H).max(0);
        if level.bands.is_empty() {
            return (0, max);
        }
        let (top, bottom) = level.band(y);
        let lo = (top * TILE).min(max);
        (lo, (bottom * TILE - SCREEN_H).clamp(lo, max))
    }

    fn snap_camera(&mut self) {
        let r = self.player.body.rect();
        self.camera_x = r.centre_x() - (CAMERA_BAND.0 + CAMERA_BAND.1) / 2;
        self.camera_y = r.centre_y() - (CAMERA_BAND_Y.0 + CAMERA_BAND_Y.1) / 2;
        self.clamp_camera();
        let (lo, hi) = Self::camera_y_range(self.level, r.centre_y());
        self.camera_y = self.camera_y.clamp(lo, hi);
    }

    /// The stretch of level the camera may show: up to and including the sealed walls
    /// either side of the wassie.
    pub fn view_bounds(&self) -> (i32, i32) {
        let x = self.player.body.rect().centre_x();
        let mut bounds = (0, self.level.px_width());
        for &(start, end) in self.seals.iter().take_while(|s| s.0 != i32::MAX) {
            if end * TILE <= x {
                bounds.0 = start * TILE;
            } else if start * TILE >= x {
                bounds.1 = bounds.1.min(end * TILE);
            }
        }
        bounds
    }

    fn clamp_camera(&mut self) {
        let (lo, hi) = self.view_bounds();
        let max = (hi - SCREEN_W).max(lo);
        self.camera_x = self.camera_x.clamp(lo, max);
        self.camera_y = self.camera_y.clamp(0, self.max_camera_y());
    }

    fn update_camera(&mut self) {
        if self.arena {
            let target = self.boss.arena.0;
            self.camera_x += (target - self.camera_x).clamp(-3, 3);
            self.camera_y += (self.arena_y - self.camera_y).clamp(-3, 3);
            return;
        }
        let r = self.player.body.rect();
        let centre = r.centre_x();
        if centre < self.camera_x + CAMERA_BAND.0 {
            self.camera_x = centre - CAMERA_BAND.0;
        } else if centre > self.camera_x + CAMERA_BAND.1 {
            self.camera_x = centre - CAMERA_BAND.1;
        }
        let middle = r.centre_y();
        let mut target = self.camera_y;
        if middle < target + CAMERA_BAND_Y.0 {
            target = middle - CAMERA_BAND_Y.0;
        } else if middle > target + CAMERA_BAND_Y.1 {
            target = middle - CAMERA_BAND_Y.1;
        }
        // Into another storey, the camera pans over to it.
        let (lo, hi) = Self::camera_y_range(self.level, middle);
        let target = target.clamp(lo, hi);
        self.camera_y += (target - self.camera_y).clamp(-CAMERA_PAN, CAMERA_PAN);
        self.clamp_camera();
    }

    fn update_actors(&mut self) {
        let mut ctx = Ctx {
            level: self.level,
            player: self.player.body.rect(),
            shots: &mut self.shots,
            difficulty: self.difficulty,
            camera_x: self.camera_x,
        };
        let view = (self.camera_x - WAKE_MARGIN, self.camera_x + SCREEN_W + WAKE_MARGIN);
        let view_y = (self.camera_y - WAKE_MARGIN, self.camera_y + SCREEN_H + WAKE_MARGIN);
        for a in self.actors[..self.used].iter_mut().filter(|a| a.alive) {
            let r = a.rect();
            let visible = r.right() > view.0 && r.x < view.1 && r.bottom() > view_y.0 && r.y < view_y.1;
            if !a.awake && visible {
                a.awake = true;
            }
            // Villains far off screen stop, so the CPU and object budget stay bounded.
            let far = r.right() < view.0 - SCREEN_W / 2
                || r.x > view.1 + SCREEN_W / 2
                || r.bottom() < view_y.0 - SCREEN_H / 2
                || r.y > view_y.1 + SCREEN_H / 2;
            if a.awake && !far {
                a.update(&mut ctx);
            }
        }
    }

    fn collide(&mut self, events: &mut Events) {
        // The wassie's shots against villains and the boss. Only the few villains awake
        // can be hit: listed once, in slot order, rather than every shot walking every
        // slot. A villain a shot downs is still listed, so each one is checked again.
        let mut threats = [0u8; MAX_ACTORS];
        let mut count = 0;
        for (i, a) in self.actors[..self.used].iter().enumerate() {
            if a.is_threat() {
                threats[count] = i as u8;
                count += 1;
            }
        }
        for s in self.shots.iter_mut().filter(|s| s.active && s.kind.from_player()) {
            let r = s.rect();
            for &i in &threats[..count] {
                let (a, i) = (&mut self.actors[i as usize], i as usize);
                if !a.is_threat() || !r.overlaps(&a.rect()) || s.hits & (1u128 << i) != 0 {
                    continue;
                }
                let ar = a.rect();
                self.fx.spawn(FxKind::Impact, r.centre_x(), r.centre_y());
                if a.damage(s.kind.damage()) {
                    self.kills += 1;
                    if let Some(wreck) = a.kind.wreck() {
                        // It goes down where it stood, with a burst of sparks.
                        self.fx.spawn_facing(wreck, ar.centre_x(), ar.bottom(), a.facing < 0);
                        self.fx.spawn(FxKind::Sparks, ar.centre_x(), ar.centre_y());
                    } else {
                        self.fx.spawn(FxKind::Explosion, ar.centre_x(), ar.centre_y());
                    }
                    events.insert(Events::ENEMY_DOWN);
                    if self.level.coins {
                        self.coins += a.kind.bounty();
                        self.fx.spawn(FxKind::CoinPop, ar.centre_x(), ar.y);
                        events.insert(Events::COIN);
                    }
                }
                if s.kind.pierces() {
                    s.hits |= 1u128 << i;
                } else {
                    s.active = false;
                    break;
                }
            }
            if s.active && !s.hit_boss && self.boss.vulnerable() && r.overlaps(&self.boss.rect()) {
                self.fx.spawn(FxKind::Impact, r.centre_x(), r.centre_y());
                self.boss.damage(s.kind.damage(), events);
                if s.kind.pierces() {
                    s.hit_boss = true;
                } else {
                    s.active = false;
                }
            }
        }

        let player = self.player.body.rect();
        let mut hurt: Option<(i32, i32, bool)> = None;

        // Plasma rounds, bombs, shockwaves, missiles and cucumbers against the wassie.
        for s in self.shots.iter_mut().filter(|s| s.active && !s.kind.from_player()) {
            let r = s.rect();
            if r.overlaps(&player.inset(1)) && self.player.invuln == 0 {
                hurt = Some((s.kind.damage(), r.centre_x() - s.vx_fx.signum() * 16, s.kind.heavy()));
                s.active = false;
                if matches!(s.kind, ShotKind::Cucumber | ShotKind::MegaCucumber) {
                    self.fx.spawn(FxKind::Splat, r.centre_x(), r.centre_y());
                    if s.kind.heavy() {
                        self.shake = self.shake.max(12);
                    }
                }
            }
        }

        // Touching villains, pickups and checkpoints.
        let mut checkpoint_reached = None;
        let mut part_taken = None;
        for (i, a) in self.actors[..self.used].iter_mut().enumerate() {
            if !a.alive || !player.overlaps(&a.rect()) {
                continue;
            }
            match a.kind {
                ActorKind::Diamond => {
                    a.alive = false;
                    a.killed = true;
                    self.diamonds += 1;
                    let r = a.rect();
                    self.fx.spawn(FxKind::Glint, r.centre_x(), r.centre_y());
                    events.insert(Events::PICKUP);
                }
                ActorKind::Heal => {
                    a.alive = false;
                    a.killed = true;
                    let r = a.rect();
                    self.fx.spawn(FxKind::Glint, r.centre_x(), r.centre_y());
                    self.player.hp = (self.player.hp + HEAL_AMOUNT).min(MAX_HP);
                    events.insert(Events::PICKUP);
                }
                ActorKind::Coin => {
                    a.alive = false;
                    a.killed = true;
                    self.coins += 1;
                    let r = a.rect();
                    self.fx.spawn(FxKind::CoinPop, r.centre_x(), r.centre_y());
                    events.insert(Events::COIN);
                }
                ActorKind::Part => {
                    a.alive = false;
                    a.killed = true;
                    part_taken = Some(i);
                    let r = a.rect();
                    self.fx.spawn(FxKind::Glint, r.centre_x(), r.centre_y());
                    events.insert(Events::PART);
                }
                ActorKind::Jetpack => {
                    a.alive = false;
                    a.killed = true;
                    self.parts |= GEAR_JETPACK;
                    self.player.jetpack = true;
                    self.player.hover_fuel = HOVER_FRAMES;
                    let r = a.rect();
                    self.fx.spawn(FxKind::Glint, r.centre_x(), r.centre_y());
                    self.fx.spawn(FxKind::Sparks, r.centre_x(), r.centre_y());
                    events.insert(Events::GEAR);
                }
                ActorKind::Terminal => {}
                ActorKind::Checkpoint => {
                    if a.state != enemy::state::LIT {
                        a.state = enemy::state::LIT;
                        checkpoint_reached = Some(i);
                    }
                }
                _ if !a.awake => {}
                _ => {
                    let falling = self.player.body.vy_fx > 0;
                    let stomp = falling && player.bottom() - a.rect().y <= 8;
                    if stomp {
                        let ar = a.rect();
                        self.fx.spawn(FxKind::Impact, player.centre_x(), player.bottom());
                        if a.damage(2) {
                            self.kills += 1;
                            self.fx.spawn(FxKind::Explosion, ar.centre_x(), ar.centre_y());
                            events.insert(Events::ENEMY_DOWN);
                            if self.level.coins {
                                self.coins += a.kind.bounty();
                                events.insert(Events::COIN);
                            }
                        }
                        self.player.body.vy_fx = STOMP_BOUNCE_FX;
                        // A stomp relights the jetpack, for chaining bounces.
                        self.player.hovering = false;
                        self.player.hover_fuel = HOVER_FRAMES;
                    } else if self.player.invuln == 0 {
                        hurt = Some((a.kind.contact_damage(), a.rect().centre_x(), false));
                    }
                }
            }
        }
        if let Some(i) = part_taken {
            let n = self.actors[..i].iter().filter(|a| a.kind == ActorKind::Part).count();
            self.parts |= 1 << (self.level.parts_base as usize + n);
        }
        if let Some(i) = checkpoint_reached {
            let flag = self.actors[i].rect();
            self.checkpoint = (flag.centre_x().div_euclid(TILE), (flag.bottom() - 1).div_euclid(TILE));
            self.checkpoints_lit += 1;
            events.insert(Events::CHECKPOINT);
        }

        if let Some((damage, heavy)) = self.boss.contact()
            && player.overlaps(&self.boss.rect().inset(2))
        {
            hurt = Some((damage, self.boss.rect().centre_x(), heavy));
        }

        if let Some((damage, from_x, heavy)) = hurt {
            self.knock_player(damage, from_x, heavy, events);
        }
    }

    #[cfg(test)]
    fn hurt_player(&mut self, damage: i32, from_x: i32, events: &mut Events) {
        self.knock_player(damage, from_x, false, events);
    }

    /// Hurts and throws the wassie unless it is still blinking from the last hit.
    /// Returns whether it was hurt.
    fn knock_player(&mut self, damage: i32, from_x: i32, heavy: bool, events: &mut Events) -> bool {
        if self.god {
            return false;
        }
        if !self.player.knocked(self.difficulty.damage(damage), from_x, heavy) {
            return false;
        }
        let r = self.player.body.rect();
        self.fx.spawn(FxKind::Sparks, r.centre_x(), r.centre_y());
        events.insert(Events::HIT);
        if self.player.hp <= 0 {
            self.die(DEATH_FRAMES, events);
        }
        true
    }

    /// A knocked-out wassie under gravity: it keeps the knockback's arc, skids to a
    /// stop once down, and stays inside a closed arena. Whether it is down: on the
    /// floor, or out of the bottom of the level.
    fn fall_out(&mut self, events: &mut Events) -> bool {
        let level = self.level;
        let p = &mut self.player;
        if p.body.rect().y > level.px_height() {
            return true;
        }
        let was_down = p.grounded;
        let body = &mut p.body;
        body.vy_fx = (body.vy_fx + GRAVITY_FX).min(MAX_FALL_FX);
        if was_down {
            body.vx_fx = crate::player::approach(body.vx_fx, 0, DEAD_SKID_FX);
        }
        let contacts = body.step(level);
        if self.arena && body.x() < self.boss.arena.0 {
            body.x_fx = self.boss.arena.0 * FX;
            body.vx_fx = 0;
        }
        p.grounded = contacts.ground;
        if p.grounded && !was_down {
            let r = p.body.rect();
            self.fx.spawn(FxKind::Kick, r.centre_x(), r.bottom() - 3);
            self.shake = self.shake.max(4);
            events.insert(Events::LAND);
        }
        p.grounded
    }

    fn die(&mut self, frames: u16, events: &mut Events) {
        self.player.hp = self.player.hp.max(0);
        self.player.state = PlayerState::Dead;
        self.player.dead_timer = frames;
        // Keep the knockback: the body is thrown, then falls. A grounded death still
        // pops up off the floor and lands again.
        self.player.grounded = false;
        self.player.body.vx_fx = self.player.body.vx_fx.clamp(-2 * FX, 2 * FX);
        let r = self.player.body.rect();
        self.fx.spawn(FxKind::Explosion, r.centre_x(), r.centre_y());
        self.shake = self.shake.max(6);
        events.insert(Events::DIE);
    }

    fn hazards(&mut self, events: &mut Events) {
        let r = self.player.body.rect();
        let live = fence_state(self.frames) == 2;
        if let Some((x0, x1)) = self.level.fence_hit(r.x, r.y, r.w, r.h).filter(|_| live && !self.player.is_dead()) {
            // Thrown back out the side it came in by, shocked or not: a wassie still
            // blinking from the last jolt can't slip through either.
            let mid = (x0 + x1) / 2;
            let body = &mut self.player.body;
            if r.centre_x() < mid {
                body.x_fx = (x0 - r.w - 1) * FX;
                body.vx_fx = body.vx_fx.min(0);
            } else {
                body.x_fx = (x1 + 1) * FX;
                body.vx_fx = body.vx_fx.max(0);
            }
            if self.knock_player(SPIKE_DAMAGE, mid, true, events) {
                events.insert(Events::SHOCK);
            }
        }
        // The current coming on, heard only with a fence in view.
        let switched_on = live && self.frames > 0 && fence_state(self.frames - 1) != 2;
        if switched_on && self.level.fence_between(self.camera_x, self.camera_x + SCREEN_W) {
            events.insert(Events::FENCE_ON);
        }
        let r = self.player.body.rect();
        if r.y > self.level.px_height() && !self.player.is_dead() {
            if !self.god {
                self.player.hp -= self.difficulty.damage(PIT_DAMAGE);
            }
            events.insert(Events::HIT);
            // Out of the pit at the checkpoint with what health is left, or knocked out.
            let frames = if self.player.hp > 0 { PIT_FRAMES } else { DEATH_FRAMES };
            self.die(frames, events);
        }
    }

    /// Where a fresh wassie stands at the last checkpoint: its centre x and feet.
    pub fn start_feet(&self) -> (i32, i32) {
        let (tx, ty) = self.checkpoint;
        let r = Body::standing_on_tile(tx, ty, PLAYER_W, PLAYER_H).rect();
        (r.centre_x(), r.bottom())
    }

    /// A fresh wassie at full health on the last checkpoint.
    pub fn reset_player(&mut self) {
        self.place_player(MAX_HP);
    }

    /// A fresh wassie with `hp`, standing on the last checkpoint.
    fn place_player(&mut self, hp: i32) {
        let (tx, ty) = self.checkpoint;
        self.player = Player::new(Body::standing_on_tile(tx, ty, PLAYER_W, PLAYER_H));
        self.player.hp = hp;
        self.player.jetpack = self.has_jetpack();
    }

    /// Back to the last checkpoint. A knockout restores full health; a pit keeps what
    /// was left. Living villains return to their posts.
    fn respawn(&mut self, events: &mut Events) {
        let hp = if self.player.hp > 0 { self.player.hp } else { MAX_HP };
        self.place_player(hp);
        self.player.invuln = crate::player::INVULN_FRAMES;
        for a in self.actors[..self.used].iter_mut() {
            a.reset();
        }
        self.shots = [Shot::EMPTY; MAX_SHOTS];
        self.fx.clear();
        if self.arena {
            self.arena = false;
            self.talk_due = false;
            self.boss.reset();
        }
        self.snap_camera();
        events.insert(Events::RESPAWN);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{boss::BossState, enemy::state};
    use crate::player::Player;

    static RANGE: Level = Level {
        rows: &[
            "....................................................................",
            "....................................................................",
            "....................................................................",
            "....................................................................",
            "....................................................................",
            "....................................................................",
            "....................................................................",
            "....................................................................",
            "....................................................................",
            "....................................................................",
            "....................................................................",
            "....................................................................",
            "....................................................................",
            "....................................................................",
            "....................................................................",
            "...P.....o..+.....C.........b.....................^^...........s....",
            "#############################...#########################.#########",
            "#############################...#########################.#########",
            "#############################...#########################.#########",
            "#############################...#########################.#########",
        ],
        ..Level::BASE
    };

    fn world() -> World {
        let mut w = World::with_level(&RANGE, 0, Difficulty::Normal);
        w.step(&Input::default());
        w
    }

    fn hold(w: &mut World, keys: u16, frames: usize) -> Events {
        let mut events = Events::default();
        for _ in 0..frames {
            events.insert(w.step(&Input::from_keys(keys, keys)));
        }
        events
    }

    fn tap(w: &mut World, keys: u16) -> Events {
        let events = w.step(&Input::from_keys(keys, 0));
        w.step(&Input::default());
        events
    }

    #[test]
    fn a_resumed_run_starts_at_its_beacon_with_whats_behind_it_gone() {
        let mut w = world();
        w.pick_up(&Progress {
            checkpoint: Some((18, 15)),
            beacons: 1,
            diamonds: 1,
            kills: 2,
            ..Progress::default()
        });
        assert_eq!(w.player.body.rect().centre_x().div_euclid(TILE), 18);
        assert_eq!(w.player.hp, MAX_HP);
        assert_eq!((w.checkpoints_lit, w.diamonds, w.kills), (1, 1, 2));
        let of = |kind| w.actors.iter().find(|a| a.kind == kind).unwrap();
        assert!(of(ActorKind::Diamond).killed && of(ActorKind::Heal).killed);
        assert_eq!(of(ActorKind::Checkpoint).state, state::LIT);
        assert!(of(ActorKind::Sentry).alive, "villains ahead of the beacon stay");
    }

    #[test]
    fn collects_sats_and_heals_and_lights_checkpoints() {
        let mut w = world();
        w.god = true;
        w.player.hp = 3;
        let events = hold(&mut w, keys::RIGHT, 120);
        assert!(events.contains(Events::PICKUP));
        assert!(events.contains(Events::CHECKPOINT));
        assert_eq!(w.diamonds, 1);
        assert_eq!(w.player.hp, 3 + HEAL_AMOUNT);
        assert_eq!(w.checkpoints_lit, 1);
        assert_eq!(w.checkpoint, (18, 15));
        assert!(w.actors.iter().any(|a| a.kind == ActorKind::Checkpoint && a.state == state::LIT));
    }

    #[test]
    fn camera_follows_and_clamps() {
        let mut w = world();
        assert_eq!(w.camera_x, 0);
        w.god = true;
        hold(&mut w, keys::RIGHT, 200);
        let centre = w.player.body.rect().centre_x();
        assert!(w.camera_x > 0);
        assert!((CAMERA_BAND.0..=CAMERA_BAND.1).contains(&(centre - w.camera_x)));
        hold(&mut w, keys::LEFT, 400);
        assert_eq!(w.camera_x, 0);
    }

    #[test]
    fn bolts_fire_with_a_cooldown_and_a_cap() {
        let mut w = world();
        assert!(tap(&mut w, keys::B).contains(Events::SHOOT));
        assert!(!tap(&mut w, keys::B).contains(Events::SHOOT), "cooling down");
        hold(&mut w, 0, FIRE_COOLDOWN as usize);
        assert!(tap(&mut w, keys::B).contains(Events::SHOOT));
        assert_eq!(w.player_shots(), 2);
        let bolt = w.shots.iter().find(|s| s.active).unwrap();
        assert!(bolt.vx_fx > 0, "fires the way the wassie faces");
    }

    #[test]
    fn up_aims_the_blaster_skyward() {
        let mut w = world();
        tap(&mut w, keys::UP | keys::B);
        let bolt = w.shots.iter().find(|s| s.active).unwrap();
        assert_eq!(bolt.vx_fx, 0);
        assert!(bolt.vy_fx < 0);
    }

    #[test]
    fn up_and_forward_fires_diagonally_at_the_same_speed() {
        let mut w = world();
        tap(&mut w, keys::UP | keys::RIGHT | keys::B);
        let bolt = w.shots.iter().find(|s| s.active).unwrap();
        assert!(bolt.vx_fx > 0 && bolt.vy_fx < 0);
        assert_eq!(bolt.vx_fx, -bolt.vy_fx);
        let speed_sq = 2 * bolt.vx_fx * bolt.vx_fx;
        assert!((speed_sq - BOLT_FX * BOLT_FX).abs() < BOLT_FX * BOLT_FX / 50, "{speed_sq}");
        let (w_, h) = (bolt.rect().w, bolt.rect().h);
        assert_eq!(w_, h, "diagonal lasers have a square hitbox");
    }

    #[test]
    fn down_aims_fire_from_the_air() {
        let mut w = world();
        tap(&mut w, keys::A);
        w.step(&Input::from_keys(keys::DOWN | keys::LEFT | keys::B, 0));
        let bolt = w.shots.iter().find(|s| s.active).unwrap();
        assert!(bolt.vx_fx < 0 && bolt.vy_fx > 0, "down and back the way it faces");
    }

    #[test]
    fn hovering_leaves_a_trail_of_exhaust() {
        let mut w = world();
        tap(&mut w, keys::A);
        let mut puffs = 0;
        for _ in 0..60 {
            w.step(&Input::from_keys(keys::A, keys::A));
            puffs += w.fx.active().filter(|f| f.kind == FxKind::Puff && f.age == 0).count();
        }
        assert!(puffs >= 4, "{puffs} puffs");
    }

    #[test]
    fn stomping_refuels_the_jetpack() {
        let mut w = world();
        let sentry = w.actors.iter().position(|a| a.kind == ActorKind::Sentry).unwrap();
        let r = w.actors[sentry].rect();
        w.actors[sentry].awake = true;
        w.player.hover_fuel = 0;
        w.player.body.x_fx = r.x * FX;
        w.player.body.y_fx = (r.y - PLAYER_H + 4) * FX;
        w.player.body.vy_fx = 2 * FX;
        w.collide(&mut Events::default());
        assert_eq!(w.player.hover_fuel, HOVER_FRAMES);
    }

    #[test]
    fn holding_b_charges_a_piercing_beam() {
        let mut w = world();
        hold(&mut w, keys::B, CHARGE_FRAMES as usize + 5);
        let events = w.step(&Input::from_keys(0, keys::B));
        assert!(events.contains(Events::BEAM));
        assert!(w.shots.iter().any(|s| s.active && s.kind == ShotKind::Beam));
        assert_eq!(w.player.charge, 0);
    }

    #[test]
    fn sentry_is_shot_down_and_counts() {
        let mut w = world();
        w.god = true;
        // Walk up to the sentry and keep firing.
        let mut events = Events::default();
        for i in 0..400 {
            let keys = if w.player.body.x() < 170 { keys::RIGHT } else { 0 };
            let fire = if i % 2 == 0 { keys::B } else { 0 };
            events.insert(w.step(&Input::from_keys(keys | fire, if fire == 0 { keys::B } else { 0 })));
        }
        assert!(events.contains(Events::ENEMY_DOWN));
        assert_eq!(w.kills, 1);
        assert_eq!(w.score(), w.diamonds * 10 + 50);
    }

    #[test]
    fn a_beaten_sentry_leaves_a_wreck_facing_its_way() {
        let mut w = world();
        let sentry = w.actors.iter().position(|a| a.kind == ActorKind::Sentry).unwrap();
        w.actors[sentry].awake = true;
        w.actors[sentry].hp = 1;
        w.actors[sentry].facing = -1;
        let r = w.actors[sentry].rect();
        shot::spawn(&mut w.shots, ShotKind::Bolt, r.centre_x(), r.centre_y(), FX, 0);
        w.collide(&mut Events::default());
        let wreck = w.fx.active().find(|f| f.kind == FxKind::Wreck).expect("a wreck");
        assert_eq!((wreck.x, wreck.y, wreck.flip), (r.centre_x(), r.bottom(), true));
        assert!(w.fx.active().all(|f| f.kind != FxKind::Explosion));
    }

    #[test]
    fn sentry_fires_from_its_cannon_then_recoils() {
        use crate::enemy::{SENTRY_MUZZLE, SENTRY_RECOIL_FRAMES, state};
        let mut w = world();
        let sentry = w.actors.iter().position(|a| a.kind == ActorKind::Sentry).unwrap();
        let a = &mut w.actors[sentry];
        a.awake = true;
        a.state = state::AIM;
        a.timer = 1;
        a.facing = -1;
        w.player.body.x_fx = (a.rect().x - 60) * FX;
        w.step(&Input::default());
        let a = w.actors[sentry];
        assert_eq!(a.state, state::RECOIL);
        let shot = w.shots.iter().find(|s| s.active && s.kind == ShotKind::Bullet).expect("a plasma round");
        let (r, sr) = (a.rect(), shot.rect());
        assert!((sr.centre_x() - (r.centre_x() - SENTRY_MUZZLE.0)).abs() <= 2, "{sr:?} from {r:?}");
        assert_eq!(sr.centre_y(), r.bottom() - SENTRY_MUZZLE.1);
        let x = a.body.x();
        for _ in 0..SENTRY_RECOIL_FRAMES {
            w.step(&Input::default());
        }
        assert_eq!(w.actors[sentry].body.x(), x, "stands still while recoiling");
        assert_eq!(w.actors[sentry].state, state::IDLE);
    }

    #[test]
    fn beam_hits_each_enemy_once() {
        let mut w = world();
        let sentry = w.actors.iter().position(|a| a.kind == ActorKind::Sentry).unwrap();
        w.actors[sentry].awake = true;
        w.actors[sentry].hp = 10;
        let r = w.actors[sentry].rect();
        shot::spawn(&mut w.shots, ShotKind::Beam, r.x - 30, r.centre_y(), 0, 0);
        let beam = w.shots.iter().position(|s| s.active).unwrap();
        w.shots[beam].vx_fx = FX;
        for _ in 0..40 {
            let mut events = Events::default();
            w.collide(&mut events);
            for s in w.shots.iter_mut().filter(|s| s.active) {
                s.x_fx += s.vx_fx;
            }
        }
        assert_eq!(w.actors[sentry].hp, 10 - ShotKind::Beam.damage());
    }

    #[test]
    fn stomping_bounces_and_damages() {
        let mut w = world();
        let sentry = w.actors.iter().position(|a| a.kind == ActorKind::Sentry).unwrap();
        let r = w.actors[sentry].rect();
        w.actors[sentry].awake = true;
        w.player.body.x_fx = r.x * FX;
        w.player.body.y_fx = (r.y - PLAYER_H + 4) * FX;
        w.player.body.vy_fx = 2 * FX;
        let mut events = Events::default();
        w.collide(&mut events);
        assert_eq!(w.actors[sentry].hp, ActorKind::Sentry.max_hp() - 2);
        assert_eq!(w.player.body.vy_fx, STOMP_BOUNCE_FX);
        assert_eq!(w.player.hp, MAX_HP);
    }

    #[test]
    fn spikes_hurt_and_difficulty_scales_damage() {
        for (difficulty, expected) in [
            (Difficulty::Chill, MAX_HP - 1),
            (Difficulty::Normal, MAX_HP - 2),
            (Difficulty::Degen, MAX_HP - 4),
        ] {
            let mut w = World::with_level(&RANGE, 0, difficulty);
            w.player.body.x_fx = 51 * TILE * FX;
            w.frames = FENCE_CYCLE - FENCE_LIVE;
            let events = w.step(&Input::default());
            assert!(events.contains(Events::HIT), "{difficulty:?}");
            assert_eq!(w.player.hp, expected, "{difficulty:?}");
        }
    }

    #[test]
    fn live_fence_is_a_wall_up_to_the_sky() {
        let mut w = World::with_level(&RANGE, 0, Difficulty::Normal);
        w.frames = FENCE_CYCLE - FENCE_LIVE;
        // High above the emitters, well over any jump: still in the curtain.
        w.player.body.x_fx = (50 * TILE) * FX;
        w.player.body.y_fx = 2 * TILE * FX;
        let events = w.step(&Input::default());
        assert!(events.contains(Events::HIT));
        let r = w.player.body.rect();
        assert!(r.right() <= 50 * TILE, "thrown back out the left: {r:?}");
        // Still blinking from the jolt, it can't slip through either.
        assert!(w.player.invuln > 0);
        for _ in 0..30 {
            w.player.body.x_fx = (50 * TILE - PLAYER_W + 2) * FX;
            w.player.body.vx_fx = 2 * FX;
            w.step(&Input::default());
            assert!(w.player.body.rect().right() <= 50 * TILE);
        }
    }

    #[test]
    fn fences_coming_on_are_heard_only_in_view_and_a_shock_is_its_own_sound() {
        // In view: the emitters at tile 50 are on screen from the start of RANGE.
        let mut w = World::with_level(&RANGE, 0, Difficulty::Normal);
        w.player.body.x_fx = 40 * TILE * FX;
        w.snap_camera();
        w.frames = FENCE_CYCLE - FENCE_LIVE - 1;
        let on = w.step(&Input::default());
        assert!(on.contains(Events::FENCE_ON));
        assert!(!w.step(&Input::default()).contains(Events::FENCE_ON), "once, as they switch");
        // Out of view: silent.
        let mut far = World::with_level(&RANGE, 0, Difficulty::Normal);
        far.frames = FENCE_CYCLE - FENCE_LIVE - 1;
        assert!(!far.step(&Input::default()).contains(Events::FENCE_ON));
        // Walking into the live curtain: a shock, but not again while still blinking.
        w.player.body.x_fx = (50 * TILE - PLAYER_W) * FX;
        w.player.body.vx_fx = 2 * FX;
        let shocked = w.step(&Input::default());
        assert!(shocked.contains(Events::SHOCK) && shocked.contains(Events::HIT));
        w.player.body.x_fx = (50 * TILE - PLAYER_W + 2) * FX;
        assert!(!w.step(&Input::default()).contains(Events::SHOCK));
    }

    #[test]
    fn fence_curtain_stops_at_rock() {
        assert_eq!(RANGE.fence_top(50, 15), 0);
        assert_eq!(RANGE.fence_hit(50 * TILE, 3 * TILE, 8, 8), Some((50 * TILE, 52 * TILE)));
        assert_eq!(RANGE.fence_hit(48 * TILE, 3 * TILE, 8, 8), None);
        // Inside the rock under the emitters: no curtain.
        assert_eq!(RANGE.fence_hit(50 * TILE, 17 * TILE, 8, 8), None);
    }

    #[test]
    fn dead_fences_are_harmless() {
        let mut w = World::with_level(&RANGE, 0, Difficulty::Normal);
        w.player.body.x_fx = 51 * TILE * FX;
        w.frames = 0;
        let events = w.step(&Input::default());
        assert!(!events.contains(Events::HIT));
        assert_eq!(w.player.hp, MAX_HP);
    }

    #[test]
    fn pit_costs_health_and_respawns_at_checkpoint() {
        let mut w = world();
        for a in w.actors.iter_mut().filter(|a| a.kind.is_enemy()) {
            a.alive = false;
            a.killed = true;
        }
        w.checkpoint = (18, 15);
        w.player.body.x_fx = 30 * TILE * FX;
        let mut events = Events::default();
        for _ in 0..120 {
            events.insert(w.step(&Input::default()));
        }
        assert!(events.contains(Events::RESPAWN));
        assert_eq!(w.player.hp, MAX_HP - PIT_DAMAGE);
        assert_eq!(w.player.body.rect().centre_x().div_euclid(TILE), 18);
    }

    #[test]
    fn knockout_respawns_with_full_health_and_resets_villains() {
        let mut w = world();
        let sentry = w.actors.iter().position(|a| a.kind == ActorKind::Sentry).unwrap();
        w.actors[sentry].awake = true;
        w.actors[sentry].body.x_fx += 40 * FX;
        w.player.hp = 1;
        let mut events = Events::default();
        w.hurt_player(2, 0, &mut events);
        assert!(events.contains(Events::DIE));
        assert!(w.player.is_dead());
        // The knockback pops the body up; it comes down before the count runs out.
        let respawned = (0..DEATH_FRAMES as usize + 30).any(|_| w.step(&Input::default()).contains(Events::RESPAWN));
        assert!(respawned);
        assert_eq!(w.player.hp, MAX_HP);
        assert!(!w.actors[sentry].awake);
        assert_eq!(w.actors[sentry].body.x(), w.actors[sentry].home.0);
    }

    #[test]
    fn a_wassie_knocked_out_in_the_air_falls_to_the_floor_first() {
        let mut w = world();
        let floor = w.player.body.y();
        w.player.body.y_fx -= 40 * FX;
        w.player.grounded = false;
        w.player.hp = 1;
        w.hurt_player(2, 0, &mut Events::default());
        assert!(w.player.is_dead());
        let mut landed_at = None;
        for i in 0..DEATH_FRAMES as usize {
            let events = w.step(&Input::default());
            assert!(!events.contains(Events::RESPAWN), "respawned in the air");
            if w.player.grounded {
                landed_at.get_or_insert(i);
            }
        }
        let landed_at = landed_at.expect("never came down");
        assert_eq!(w.player.body.y(), floor, "lying on the floor");
        assert!(landed_at > 10, "the fall took {landed_at} frames");
        // Down, the knockout runs its full count.
        let events = hold(&mut w, 0, landed_at + 2);
        assert!(events.contains(Events::RESPAWN));
    }

    #[test]
    fn god_mode_ignores_damage() {
        let mut w = world();
        w.god = true;
        let mut events = Events::default();
        w.hurt_player(5, 0, &mut events);
        assert_eq!(w.player.hp, MAX_HP);
    }

    #[test]
    fn difficulty_cycles() {
        for d in Difficulty::ALL {
            assert_eq!(d.next().previous(), d);
            assert_eq!(Difficulty::from_u8(d as u8), Some(d));
        }
        assert!(Difficulty::Degen.fire_interval(90) < Difficulty::Chill.fire_interval(90));
    }

    #[test]
    fn levels_without_a_boss_never_lock() {
        let mut w = world();
        assert!(!w.boss.present);
        assert_eq!(w.boss.state, BossState::Asleep);
        w.god = true;
        hold(&mut w, keys::RIGHT, 600);
        assert!(!w.arena);
    }

    #[test]
    fn hits_and_pickups_leave_effects() {
        let mut w = world();
        w.god = true;
        hold(&mut w, keys::RIGHT, 50);
        assert!(w.fx.active().any(|f| f.kind == FxKind::Glint) || w.diamonds > 0);
        let sentry = w.actors.iter().position(|a| a.kind == ActorKind::Sentry).unwrap();
        w.actors[sentry].awake = true;
        let r = w.actors[sentry].rect();
        shot::spawn(&mut w.shots, ShotKind::Bolt, r.centre_x(), r.centre_y(), 0, 0);
        let mut events = Events::default();
        w.collide(&mut events);
        assert!(w.fx.active().any(|f| f.kind == FxKind::Impact));
    }

    #[test]
    fn only_big_moments_shake_the_screen() {
        let mut w = world();
        tap(&mut w, keys::B);
        assert_eq!(w.shake, 0, "an ordinary bolt does not shake");
        assert!(w.kick > 0, "but it gives the screen a light jolt");
        hold(&mut w, 0, KICK_FRAMES as usize);
        assert_eq!(w.kick, 0, "which settles at once");
        hold(&mut w, 0, 20);
        hold(&mut w, keys::B, CHARGE_FRAMES as usize + 1);
        w.step(&Input::from_keys(0, keys::B));
        assert!(w.shake > 0, "a charged beam does");
    }

    /// The Trenches, with the wassie placed at tile `tx` on the floor and no villains.
    /// The Trenches with their bots gone and the wassie on the street storey's floor
    /// at column `tx`.
    fn trenches_at(tx: i32) -> World {
        trenches_on(tx, 39)
    }

    /// The Trenches with their bots gone and the wassie's feet in row `ty`.
    fn trenches_on(tx: i32, ty: i32) -> World {
        let mut w = World::new(1, Difficulty::Normal);
        for a in w.actors.iter_mut().filter(|a| a.kind.is_enemy()) {
            a.alive = false;
            a.killed = true;
        }
        w.player = Player::new(Body::standing_on_tile(tx, ty, PLAYER_W, PLAYER_H));
        w.snap_camera();
        w
    }

    #[test]
    fn guard_fires_a_burst_of_three_from_its_gun_then_holds_its_aim() {
        use crate::enemy::{GUARD_BURST_GAP, GUARD_MUZZLE, GUARD_RECOIL_FRAMES, state};
        let mut w = trenches_at(6);
        let guard = w.actors.iter().position(|a| a.kind == ActorKind::Guard).unwrap();
        let a = &mut w.actors[guard];
        a.alive = true;
        a.killed = false;
        a.awake = true;
        a.state = state::AIM;
        a.timer = 1;
        a.facing = 1;
        let r = a.rect();
        // Behind its back, so the burst flies clear, and close enough to keep it awake.
        w.player.body.x_fx = (r.x - 60) * FX;
        w.player.body.y_fx = (r.bottom() - PLAYER_H) * FX;
        w.snap_camera();
        w.god = true;
        let mut shots = 0;
        for _ in 0..(2 + 3 * GUARD_BURST_GAP as usize) {
            w.step(&Input::default());
            let fired = w.shots.iter().filter(|s| s.active && s.kind == ShotKind::Bullet).count();
            if fired > shots {
                let s = w.shots.iter().filter(|s| s.active && s.kind == ShotKind::Bullet).last().unwrap().rect();
                let r = w.actors[guard].rect();
                assert_eq!(s.centre_y(), r.bottom() - GUARD_MUZZLE.1);
                assert!((s.centre_x() - (r.centre_x() + GUARD_MUZZLE.0)).abs() <= 2, "{s:?} from {r:?}");
            }
            shots = shots.max(fired);
        }
        assert_eq!(shots, 3);
        assert_eq!(w.actors[guard].state, state::RECOIL);
        for _ in 0..GUARD_RECOIL_FRAMES {
            w.step(&Input::default());
        }
        assert_eq!(w.actors[guard].state, state::IDLE);
    }

    #[test]
    fn a_beaten_guard_leaves_a_ninja_wreck() {
        let mut w = trenches_at(6);
        let guard = w.actors.iter().position(|a| a.kind == ActorKind::Guard).unwrap();
        let a = &mut w.actors[guard];
        (a.alive, a.killed, a.awake, a.hp, a.facing) = (true, false, true, 1, 1);
        let r = a.rect();
        shot::spawn(&mut w.shots, ShotKind::Bolt, r.centre_x(), r.centre_y(), FX, 0);
        w.collide(&mut Events::default());
        let wreck = w.fx.active().find(|f| f.kind == FxKind::NinjaWreck).expect("a wreck");
        assert_eq!((wreck.x, wreck.y, wreck.flip), (r.centre_x(), r.bottom(), false));
    }

    #[test]
    fn coins_are_their_own_currency_and_bots_pay_bounties() {
        let mut w = trenches_at(6);
        hold(&mut w, keys::RIGHT, 40);
        assert!(w.coins >= 3, "picked up the street's first coins: {}", w.coins);
        assert_eq!(w.diamonds, 0, "coins never count as diamonds");
        let sentry = w.actors.iter().position(|a| a.kind == ActorKind::Sentry).unwrap();
        w.actors[sentry] = Actor::spawn(ActorKind::Sentry, 10, 39);
        w.actors[sentry].awake = true;
        let before = w.coins;
        let r = w.actors[sentry].rect();
        for _ in 0..3 {
            shot::spawn(&mut w.shots, ShotKind::Bolt, r.centre_x(), r.centre_y(), 0, 0);
            w.collide(&mut Events::default());
        }
        assert_eq!(w.coins, before + ActorKind::Sentry.bounty());
    }

    #[test]
    fn the_elevator_needs_the_breaker_in_the_sewers_then_rides_up_the_tower() {
        let level = levels::get(1);
        let (car_x, car_y) = level.find(b'L').unwrap();
        let mut w = trenches_on(car_x + 1, car_y);
        let lift = w.lift.unwrap();
        assert_eq!(lift.state, LiftState::Dark);
        assert!(w.lift_unpowered(), "the car says it has no power");
        assert!(w.up_prompt().is_none(), "and Up does nothing at it");
        assert!(!tap(&mut w, keys::UP).contains(Events::POWER));
        assert_eq!(w.lift.unwrap().state, LiftState::Dark);

        // Down in the sewers, Up at the breaker brings the power up.
        let (bx, by) = level.find(b'E').unwrap();
        w.player = Player::new(Body::standing_on_tile(bx, by, PLAYER_W, PLAYER_H));
        w.snap_camera();
        hold(&mut w, 0, 4);
        assert!(w.up_prompt().is_some());
        assert!(tap(&mut w, keys::UP).contains(Events::POWER));
        let events = hold(&mut w, 0, (LIFT_POWER_FRAMES + LIFT_DOOR_FRAMES + 4) as usize);
        assert!(!events.contains(Events::LIFT), "its doors aren't heard from the sewers");
        assert_eq!(w.lift.unwrap().state, LiftState::Open);

        // Back at the car, in and Up.
        w.player = Player::new(Body::standing_on_tile(car_x - 2, car_y, PLAYER_W, PLAYER_H));
        w.snap_camera();
        assert!(!w.lift_unpowered());
        let car = lift.car.0 + LIFT_W / 2;
        while w.player.body.rect().centre_x() < car - 2 {
            w.step(&Input::from_keys(keys::RIGHT, keys::RIGHT));
        }
        hold(&mut w, 0, 20);
        tap(&mut w, keys::UP);
        assert!(w.lift.unwrap().riding());
        hold(&mut w, keys::RIGHT, (LIFT_DOOR_FRAMES * 2 + LIFT_RIDE_FRAMES + 4) as usize);
        assert_eq!(w.lift.unwrap().state, LiftState::Done);
        let r = w.player.body.rect();
        assert!(r.bottom() <= lift.dest.1 && r.x >= lift.dest.0 - TILE, "stepped out at the top: {r:?}");
        assert!(w.camera_y < SCREEN_H, "the camera went up the tower with it: {}", w.camera_y);
    }

    /// A one-screen test floor: a terminal beside a car, and one off by itself.
    static LOBBY: Level = Level {
        rows: &[
            "............................................................",
            "............................................................",
            "............................................................",
            "............................................................",
            "............................................................",
            "............................................................",
            "............................................................",
            "............................................................",
            "............................................................",
            "............................................................",
            "............................................................",
            "............................................................",
            "............................................................",
            "............................................................",
            "............................................................",
            "..P.................E...L........................#...l......",
            "############################################################",
            "############################################################",
            "############################################################",
            "############################################################",
        ],
        ..Level::BASE
    };

    #[test]
    fn up_at_the_dark_elevator_calls_it_when_its_terminal_stands_by() {
        let mut w = World::with_level(&LOBBY, 0, Difficulty::Normal);
        let car = w.lift.unwrap().car;
        w.player = Player::new(Body::standing_on_tile(car.0 / TILE + 1, 15, PLAYER_W, PLAYER_H));
        hold(&mut w, 0, 10);
        assert!(!w.lift_unpowered());
        assert!(w.up_prompt().is_some(), "the dark car shows the Up bubble too");
        let events = tap(&mut w, keys::UP);
        assert!(events.contains(Events::POWER), "its call button powers it");
        assert_eq!(w.lift.unwrap().state, LiftState::Powering);

        let mut w = World::with_level(&LOBBY, 0, Difficulty::Normal);
        let terminal = LOBBY.find(b'E').unwrap();
        w.player = Player::new(Body::standing_on_tile(terminal.0, 15, PLAYER_W, PLAYER_H));
        w.player.body.x_fx += 12 * FX;
        hold(&mut w, 0, 10);
        assert!(w.up_prompt().is_some(), "a step to the side of the kiosk still reaches it");
        assert!(tap(&mut w, keys::UP).contains(Events::POWER));
    }

    #[test]
    fn plasma_cells_are_found_once_per_file() {
        let level = levels::get(1);
        let (tx, ty) = level.find(b'p').unwrap();
        let mut w = trenches_at(tx);
        w.player.body.y_fx = (ty * TILE - 10) * FX;
        let mut events = Events::default();
        w.collide(&mut events);
        assert!(events.contains(Events::PART));
        assert_eq!(w.parts, 1 << level.parts_base);
        assert_eq!(w.parts_here(), (3, 1));
        // A new visit with the cell owned doesn't show it again.
        let mut again = World::new(1, Difficulty::Normal);
        again.own_parts(w.parts);
        let first = again.actors.iter().find(|a| a.kind == ActorKind::Part).unwrap();
        assert!(!first.alive);
        assert_eq!(again.parts_here(), (3, 1));
    }

    #[test]
    fn heavy_hits_knock_the_wassie_much_further() {
        let mut light = trenches_at(20);
        let mut heavy = trenches_at(20);
        let x = light.player.body.rect().centre_x() + 10;
        light.knock_player(1, x, false, &mut Events::default());
        heavy.knock_player(1, x, true, &mut Events::default());
        assert!(heavy.player.body.vx_fx < light.player.body.vx_fx * 2, "away to the left, harder");
        assert!(heavy.player.hurt > light.player.hurt);
    }

    #[test]
    fn a_fall_after_the_ride_respawns_upstairs() {
        let level = levels::get(1);
        let (car_x, car_y) = level.find(b'L').unwrap();
        let mut w = trenches_on(car_x, car_y);
        let beacon = level.markers().find(|m| m.0 == b'C' && m.2 > car_y - 4).unwrap();
        w.checkpoint = (beacon.1, beacon.2);
        let mut lift = w.lift.unwrap();
        lift.state = LiftState::Open;
        w.lift = Some(lift);
        tap(&mut w, keys::UP);
        hold(&mut w, 0, (LIFT_DOOR_FRAMES * 2 + LIFT_RIDE_FRAMES + 4) as usize);
        assert_eq!(w.lift.unwrap().state, LiftState::Done);
        w.player.hp = 1;
        w.player.invuln = 0;
        w.hurt_player(2, 0, &mut Events::default());
        hold(&mut w, 0, DEATH_FRAMES as usize + 40);
        let r = w.player.body.rect();
        assert!(r.bottom() <= lift.dest.1, "back up the tower, not under the lift: {r:?}");
        assert_eq!(w.lift.unwrap().state, LiftState::Done);
    }

    #[test]
    fn a_resumed_trenches_run_finds_the_lift_as_it_left_it() {
        let level = levels::get(1);
        let mut w = World::new(1, Difficulty::Normal);
        // The breaker thrown, a beacon lit down on the casino floor.
        let breaker = w.actors.iter().position(|a| a.kind == ActorKind::Terminal).unwrap();
        w.actors[breaker].state = state::LIT;
        let floor = level.markers().find(|m| m.0 == b'C' && (24..44).contains(&m.2)).unwrap();
        let down = Progress { level: 1, checkpoint: Some((floor.1, floor.2)), taken: w.taken(), ..Progress::default() };
        let again = World::resume(&down, Difficulty::Normal);
        assert_eq!(again.lift.unwrap().state, LiftState::Open, "powered, it waits open");
        // And one lit up the tower: it was ridden.
        let top = level.markers().find(|m| m.0 == b'C' && m.2 < 20).unwrap();
        let up = Progress { checkpoint: Some((top.1, top.2)), ..down };
        let again = World::resume(&up, Difficulty::Normal);
        assert_eq!(again.lift.unwrap().state, LiftState::Done);
        assert!(again.camera_y < SCREEN_H);
        // Without the breaker, the lift is still dark.
        let dark = Progress { taken: 0, ..down };
        assert_eq!(World::resume(&dark, Difficulty::Normal).lift.unwrap().state, LiftState::Dark);
    }

    #[test]
    fn a_ridden_lift_reopens_if_the_wassie_is_downstairs_again() {
        let level = levels::get(1);
        let (car_x, car_y) = level.find(b'L').unwrap();
        let mut w = trenches_on(car_x - 2, car_y);
        let mut lift = w.lift.unwrap();
        lift.state = LiftState::Done;
        w.lift = Some(lift);
        w.step(&Input::default());
        assert_eq!(w.lift.unwrap().state, LiftState::Open);
        let car = lift.car.0 + LIFT_W / 2;
        while w.player.body.rect().centre_x() < car - 2 {
            w.step(&Input::from_keys(keys::RIGHT, keys::RIGHT));
        }
        hold(&mut w, 0, 20);
        assert!(w.up_prompt().is_some(), "the prompt shows at the car");
        tap(&mut w, keys::UP);
        assert!(w.lift.unwrap().riding(), "and it can be ridden again");
    }

}
