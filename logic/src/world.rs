//! One playthrough of a level: the wassie, the villains, the shots, the boss, and how
//! they interact.

use alloc::{boxed::Box, vec};

use crate::{
    FX, SCREEN_W, TILE,
    boss::{Boss, BossCtx},
    enemy::{self, Actor, ActorKind, Ctx, MAX_ACTORS},
    fx::{FxKind, FxPool},
    game::{Progress, keys},
    level::{Level, Tile},
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
pub const PIT_DAMAGE: i32 = 2;
pub const HEAL_AMOUNT: i32 = 2;
/// The camera keeps the wassie's centre inside this band of the screen.
const CAMERA_BAND: (i32, i32) = (96, 136);
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
}

#[derive(Clone, Debug)]
pub struct World {
    pub level: &'static Level,
    pub level_index: u8,
    pub difficulty: Difficulty,
    pub player: Player,
    /// On the heap, filled there: see `new_actors`.
    pub actors: Box<[Actor; MAX_ACTORS]>,
    pub shots: [Shot; MAX_SHOTS],
    pub boss: Boss,
    pub fx: FxPool,
    /// Frames of screen shake left: kept for big hits, not every shot.
    pub shake: u8,
    /// Frames of the light jolt each ordinary shot gives the screen.
    pub kick: u8,
    /// Left edge of the screen in level pixels.
    pub camera_x: i32,
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
        let start = level.find(b'P').unwrap_or((2, level.height() - 2));
        let lift = level.find(b'L').map(|(tx, ty)| {
            let dest = level.find(b'l').unwrap_or((tx, ty));
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
            actors: new_actors(),
            shots: [Shot::EMPTY; MAX_SHOTS],
            boss: Boss::new(level),
            fx: FxPool::new(),
            shake: 0,
            kick: 0,
            camera_x: 0,
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
        let mut count = 0;
        for (ty, row) in level.rows.iter().enumerate() {
            for (tx, c) in row.bytes().enumerate() {
                if let Some(kind) = ActorKind::from_marker(c)
                    && count < MAX_ACTORS
                {
                    world.actors[count] = Actor::spawn(kind, tx as i32, ty as i32);
                    count += 1;
                }
            }
        }
        world.snap_camera();
        world
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
        let base = self.level.parts_base;
        for (n, a) in self.actors.iter_mut().filter(|a| a.kind == ActorKind::Part).enumerate() {
            if mask & (1 << (base as usize + n)) != 0 {
                a.alive = false;
                a.killed = true;
            }
        }
    }

    /// Shield parts this level holds, and how many of them the file owns.
    pub fn parts_here(&self) -> (u32, u32) {
        let total = self.actors.iter().filter(|a| a.kind == ActorKind::Part).count() as u32;
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
        let edge = (tx + 1) * TILE;
        for a in self.actors.iter_mut().filter(|a| a.alive) {
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
        if let Some(lift) = &mut self.lift
            && edge > lift.car.0
        {
            // A beacon past the elevator: it was ridden on the way.
            lift.state = if edge > lift.dest.0 { LiftState::Done } else { LiftState::Open };
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
            if r.right() < self.camera_x - 16 || r.x > self.camera_x + SCREEN_W + 16 {
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
                let on = self.actors.iter().any(|a| a.kind == ActorKind::Terminal && a.state == enemy::state::LIT);
                if on {
                    next(&mut lift, LiftState::Powering);
                }
            }
            LiftState::Powering if lift.timer >= LIFT_POWER_FRAMES => {
                next(&mut lift, LiftState::Opening);
                events.insert(Events::LIFT);
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
                if self.checkpoint.0 < arrived.0 {
                    self.checkpoint = arrived;
                }
            }
            // Back downstairs after the ride (a resumed run, say): the car waits open again.
            LiftState::Done if r.centre_x() < lift.dest.0 && lift.car.0 < lift.dest.0 => {
                lift.state = LiftState::Open;
                lift.timer = LIFT_DOOR_FRAMES;
            }
            _ => {}
        }
        let riding = lift.riding();
        self.lift = Some(lift);
        riding
    }

    /// Up at a dark terminal switches it on, whether pressed there or held on the way.
    /// So does Up at the dark elevator's door: its call button works the terminal too.
    fn work_terminals(&mut self, input: &Input, events: &mut Events) {
        if !input.up() || self.player.hurt > 0 {
            return;
        }
        let r = self.player.body.rect();
        let at_dark_car = self
            .lift
            .is_some_and(|l| l.state == LiftState::Dark && self.player.grounded && l.at_door(&r));
        for a in self.actors.iter_mut() {
            if a.kind == ActorKind::Terminal
                && a.alive
                && a.state != enemy::state::LIT
                && (at_dark_car || reaches_terminal(&r, &a.rect()))
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
        if let Some(a) = self.actors.iter().filter(dark).find(|a| reaches_terminal(&r, &a.rect())) {
            let ar = a.rect();
            return Some((ar.centre_x(), ar.y - 4));
        }
        let lift = self.lift?;
        let ready = lift.state == LiftState::Open || (lift.state == LiftState::Dark && self.actors.iter().any(|a| dark(&a)));
        (ready && lift.at_door(&r)).then_some((lift.car.0 + LIFT_W / 2, lift.car.1 - LIFT_H - 4))
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
        if !self.arena && self.player.body.x() >= left + 2 * TILE {
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

    fn snap_camera(&mut self) {
        let centre = self.player.body.rect().centre_x();
        self.camera_x = centre - (CAMERA_BAND.0 + CAMERA_BAND.1) / 2;
        self.clamp_camera();
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
    }

    fn update_camera(&mut self) {
        if self.arena {
            let target = self.boss.arena.0;
            self.camera_x += (target - self.camera_x).clamp(-3, 3);
            return;
        }
        let centre = self.player.body.rect().centre_x();
        if centre < self.camera_x + CAMERA_BAND.0 {
            self.camera_x = centre - CAMERA_BAND.0;
        } else if centre > self.camera_x + CAMERA_BAND.1 {
            self.camera_x = centre - CAMERA_BAND.1;
        }
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
        for a in self.actors.iter_mut().filter(|a| a.alive) {
            let r = a.rect();
            let visible = r.right() > view.0 && r.x < view.1;
            if !a.awake && visible {
                a.awake = true;
            }
            // Villains far off screen stop, so the CPU and object budget stay bounded.
            let far = r.right() < view.0 - SCREEN_W / 2 || r.x > view.1 + SCREEN_W / 2;
            if a.awake && !far {
                a.update(&mut ctx);
            }
        }
    }

    fn collide(&mut self, events: &mut Events) {
        // The wassie's shots against villains and the boss.
        for s in self.shots.iter_mut().filter(|s| s.active && s.kind.from_player()) {
            let r = s.rect();
            for (i, a) in self.actors.iter_mut().enumerate() {
                if !a.is_threat() || !r.overlaps(&a.rect()) || s.hits & (1u128 << i) != 0 {
                    continue;
                }
                let ar = a.rect();
                self.fx.spawn(FxKind::Impact, r.centre_x(), r.centre_y());
                if a.damage(s.kind.damage()) {
                    self.kills += 1;
                    self.fx.spawn(FxKind::Explosion, ar.centre_x(), ar.centre_y());
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
        for (i, a) in self.actors.iter_mut().enumerate() {
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

    fn hurt_player(&mut self, damage: i32, from_x: i32, events: &mut Events) {
        self.knock_player(damage, from_x, false, events);
    }

    fn knock_player(&mut self, damage: i32, from_x: i32, heavy: bool, events: &mut Events) {
        if self.god {
            return;
        }
        if self.player.knocked(self.difficulty.damage(damage), from_x, heavy) {
            let r = self.player.body.rect();
            self.fx.spawn(FxKind::Sparks, r.centre_x(), r.centre_y());
            events.insert(Events::HIT);
            if self.player.hp <= 0 {
                self.die(DEATH_FRAMES, events);
            }
        }
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
        let spiked = fence_state(self.frames) == 2
            && crate::physics::tiles_in(&r.inset(2)).any(|(tx, ty)| self.level.tile(tx, ty) == Tile::Spike);
        if spiked {
            let from_x = r.centre_x() - self.player.facing * 8;
            self.hurt_player(SPIKE_DAMAGE, from_x, events);
        }
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
    }

    /// Back to the last checkpoint. A knockout restores full health; a pit keeps what
    /// was left. Living villains return to their posts.
    fn respawn(&mut self, events: &mut Events) {
        let hp = if self.player.hp > 0 { self.player.hp } else { MAX_HP };
        self.place_player(hp);
        self.player.invuln = crate::player::INVULN_FRAMES;
        for a in self.actors.iter_mut() {
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
    fn trenches_at(tx: i32) -> World {
        let mut w = World::new(1, Difficulty::Normal);
        for a in w.actors.iter_mut().filter(|a| a.kind.is_enemy()) {
            a.alive = false;
            a.killed = true;
        }
        w.player = Player::new(Body::standing_on_tile(tx, 15, PLAYER_W, PLAYER_H));
        w.snap_camera();
        w
    }

    #[test]
    fn coins_are_their_own_currency_and_bots_pay_bounties() {
        let mut w = trenches_at(6);
        hold(&mut w, keys::RIGHT, 40);
        assert!(w.coins >= 3, "picked up the street's first coins: {}", w.coins);
        assert_eq!(w.diamonds, 0, "coins never count as diamonds");
        let sentry = w.actors.iter().position(|a| a.kind == ActorKind::Sentry).unwrap();
        w.actors[sentry] = Actor::spawn(ActorKind::Sentry, 10, 15);
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
    fn the_elevator_needs_its_terminal_then_rides_to_the_upper_floor() {
        let terminal = levels::get(1).find(b'E').unwrap();
        let mut w = trenches_at(terminal.0);
        let lift = w.lift.unwrap();
        assert_eq!(lift.state, LiftState::Dark);
        let (lo, hi) = w.view_bounds();
        assert!(hi <= lift.dest.0, "the camera stays downstairs: {lo}..{hi}");
        assert!(w.camera_x + SCREEN_W <= hi);

        // Up at the car does nothing without power.
        let mut events = tap(&mut w, keys::UP);
        assert!(events.contains(Events::POWER), "Up at the terminal powers it");
        events = hold(&mut w, 0, (LIFT_POWER_FRAMES + LIFT_DOOR_FRAMES + 4) as usize);
        assert!(events.contains(Events::LIFT));
        assert_eq!(w.lift.unwrap().state, LiftState::Open);

        // Walk into the car and press Up.
        let car = lift.car.0 + LIFT_W / 2;
        while w.player.body.rect().centre_x() < car - 2 {
            w.step(&Input::from_keys(keys::RIGHT, keys::RIGHT));
        }
        hold(&mut w, 0, 20);
        tap(&mut w, keys::UP);
        assert!(w.lift.unwrap().riding());
        let frames = w.frames;
        hold(&mut w, keys::LEFT, (LIFT_DOOR_FRAMES * 2 + LIFT_RIDE_FRAMES + 4) as usize);
        assert_eq!(w.lift.unwrap().state, LiftState::Done);
        assert!(w.frames > frames);
        let r = w.player.body.rect();
        assert!((r.centre_x() - (lift.dest.0 + LIFT_W / 2)).abs() <= 4, "stepped out upstairs at {}", r.x);
        assert!(w.camera_x >= w.view_bounds().0 && w.view_bounds().0 > lift.car.0, "the camera moved up with it");
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
        let mut w = trenches_at(level.find(b'L').unwrap().0);
        let beacon = level.markers().filter(|m| m.0 == b'C').nth(1).unwrap();
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
        assert!(w.player.body.x() >= lift.dest.0, "back upstairs, not under the lift: x={}", w.player.body.x());
    }

    #[test]
    fn a_ridden_lift_reopens_if_the_wassie_is_downstairs_again() {
        let level = levels::get(1);
        let mut w = trenches_at(level.find(b'L').unwrap().0 - 2);
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

    #[test]
    fn up_at_the_dark_elevator_calls_it_and_the_terminal_reaches_further() {
        let level = levels::get(1);
        let mut w = trenches_at(level.find(b'L').unwrap().0 + 1);
        hold(&mut w, 0, 10);
        assert!(w.up_prompt().is_some(), "the dark car shows the Up bubble too");
        let events = tap(&mut w, keys::UP);
        assert!(events.contains(Events::POWER), "its call button powers it");
        assert_eq!(w.lift.unwrap().state, LiftState::Powering);

        let terminal = level.find(b'E').unwrap();
        let mut w = trenches_at(terminal.0);
        w.player.body.x_fx += 12 * FX;
        hold(&mut w, 0, 10);
        assert!(w.up_prompt().is_some(), "a step to the side of the kiosk still reaches it");
        assert!(tap(&mut w, keys::UP).contains(Events::POWER));
    }
}
