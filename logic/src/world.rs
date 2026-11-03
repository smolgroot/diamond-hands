//! One playthrough of a level: the wassie, the villains, the shots, the boss, and how
//! they interact.

use crate::{
    FX, SCREEN_W, TILE,
    boss::{self, Boss, BossCtx},
    enemy::{self, Actor, ActorKind, Ctx, MAX_ACTORS},
    game::keys,
    level::{Level, Tile},
    levels,
    physics::Body,
    player::{MAX_HP, PLAYER_H, PLAYER_W, Player, PlayerState, STOMP_BOUNCE_FX},
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
/// Frames between falling into a pit and respawning.
pub const PIT_FRAMES: u16 = 30;
pub const SPIKE_DAMAGE: i32 = 2;
pub const PIT_DAMAGE: i32 = 2;
pub const HEAL_AMOUNT: i32 = 2;
/// The camera keeps the wassie's centre inside this band of the screen.
const CAMERA_BAND: (i32, i32) = (96, 136);
/// Actors wake once this close to the screen edges.
const WAKE_MARGIN: i32 = 16;

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
}

/// Things that happened during a step, for sound effects and saving.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Events(pub u16);

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

#[derive(Clone, Debug)]
pub struct World {
    pub level: &'static Level,
    pub level_index: u8,
    pub difficulty: Difficulty,
    pub player: Player,
    pub actors: [Actor; MAX_ACTORS],
    pub shots: [Shot; MAX_SHOTS],
    pub boss: Boss,
    /// Left edge of the screen in level pixels.
    pub camera_x: i32,
    /// Tile the wassie respawns on, and how many checkpoints have been lit.
    pub checkpoint: (i32, i32),
    pub checkpoints_lit: u8,
    /// The boss arena has closed behind the wassie.
    pub arena: bool,
    pub sats: u32,
    pub kills: u32,
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
        let mut actors = [Actor::EMPTY; MAX_ACTORS];
        let mut count = 0;
        for (ty, row) in level.rows.iter().enumerate() {
            for (tx, c) in row.bytes().enumerate() {
                if let Some(kind) = ActorKind::from_marker(c)
                    && count < MAX_ACTORS
                {
                    actors[count] = Actor::spawn(kind, tx as i32, ty as i32);
                    count += 1;
                }
            }
        }
        let mut world = Self {
            level,
            level_index,
            difficulty,
            player: Player::new(Body::standing_on_tile(start.0, start.1, PLAYER_W, PLAYER_H)),
            actors,
            shots: [Shot::EMPTY; MAX_SHOTS],
            boss: Boss::new(level),
            camera_x: 0,
            checkpoint: start,
            checkpoints_lit: 0,
            arena: false,
            sats: 0,
            kills: 0,
            frames: 0,
            cleared: false,
            god: false,
            rng: Rng::new(0x5EED ^ level_index as u32),
        };
        world.snap_camera();
        world
    }

    pub fn score(&self) -> u32 {
        let boss = if self.boss.is_beaten() { 1000 } else { 0 };
        self.sats * 10 + self.kills * 50 + boss
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

        if self.player.is_dead() {
            self.player.dead_timer = self.player.dead_timer.saturating_sub(1);
            if self.player.dead_timer == 0 {
                self.respawn(&mut events);
            }
            return events;
        }

        self.player.update(input, self.level, &mut events);
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
        for s in self.shots.iter_mut().filter(|s| s.active) {
            s.update(self.level);
            let r = s.rect();
            if r.right() < self.camera_x - 16 || r.x > self.camera_x + SCREEN_W + 16 {
                s.active = false;
            }
        }
        self.collide(&mut events);
        self.hazards(&mut events);
        events
    }

    fn enter_arena(&mut self, events: &mut Events) {
        if !self.boss.present {
            return;
        }
        let (left, _) = self.boss.arena;
        if !self.arena && self.player.body.x() >= left + 2 * TILE {
            self.arena = true;
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
        let r = p.body.rect();
        let (dx, dy) = if p.aim_up { (0, -1) } else { (p.facing, 0) };
        // The barrel's tip in the wassie sprite (see game/src/art/wassie.rs).
        let muzzle = if p.aim_up {
            (r.centre_x() + p.facing * 10, r.y - 6)
        } else {
            (r.centre_x() + p.facing * 17, r.y + 13)
        };

        if input.fire_held() {
            p.charge = p.charge.saturating_add(1);
        }
        let release = !input.fire_held() && p.charge > 0;
        if release && p.charge >= CHARGE_FRAMES && p.fire_cooldown == 0 {
            let fired = shot::spawn(&mut self.shots, ShotKind::Beam, muzzle.0, muzzle.1, dx * BEAM_FX, dy * BEAM_FX);
            if fired {
                p.fire_cooldown = BEAM_COOLDOWN;
                p.recoil = 10;
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
            && shot::spawn(&mut self.shots, ShotKind::Bolt, muzzle.0, muzzle.1, dx * BOLT_FX, dy * BOLT_FX)
        {
            p.fire_cooldown = FIRE_COOLDOWN;
            p.recoil = 6;
            events.insert(Events::SHOOT);
        }
    }

    fn snap_camera(&mut self) {
        let centre = self.player.body.rect().centre_x();
        self.camera_x = centre - (CAMERA_BAND.0 + CAMERA_BAND.1) / 2;
        self.clamp_camera();
    }

    fn clamp_camera(&mut self) {
        let max = (self.level.px_width() - SCREEN_W).max(0);
        self.camera_x = self.camera_x.clamp(0, max);
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
                if a.damage(s.kind.damage()) {
                    self.kills += 1;
                    events.insert(Events::ENEMY_DOWN);
                }
                if s.kind.pierces() {
                    s.hits |= 1u128 << i;
                } else {
                    s.active = false;
                    break;
                }
            }
            if s.active && !s.hit_boss && self.boss.vulnerable() && r.overlaps(&self.boss.rect()) {
                self.boss.damage(s.kind.damage(), events);
                if s.kind.pierces() {
                    s.hit_boss = true;
                } else {
                    s.active = false;
                }
            }
        }

        let player = self.player.body.rect();
        let mut hurt: Option<(i32, i32)> = None;

        // Bullets, dust, shockwaves and candles against the wassie.
        for s in self.shots.iter_mut().filter(|s| s.active && !s.kind.from_player()) {
            let r = s.rect();
            if r.overlaps(&player.inset(1)) && self.player.invuln == 0 {
                hurt = Some((s.kind.damage(), r.centre_x()));
                s.active = false;
            }
        }

        // Touching villains, pickups and checkpoints.
        let mut checkpoint_reached = None;
        for (i, a) in self.actors.iter_mut().enumerate() {
            if !a.alive || !player.overlaps(&a.rect()) {
                continue;
            }
            match a.kind {
                ActorKind::Sat => {
                    a.alive = false;
                    a.killed = true;
                    self.sats += 1;
                    events.insert(Events::PICKUP);
                }
                ActorKind::Heal => {
                    a.alive = false;
                    a.killed = true;
                    self.player.hp = (self.player.hp + HEAL_AMOUNT).min(MAX_HP);
                    events.insert(Events::PICKUP);
                }
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
                        if a.damage(2) {
                            self.kills += 1;
                            events.insert(Events::ENEMY_DOWN);
                        }
                        self.player.body.vy_fx = STOMP_BOUNCE_FX;
                    } else if self.player.invuln == 0 {
                        hurt = Some((a.kind.contact_damage(), a.rect().centre_x()));
                    }
                }
            }
        }
        if let Some(i) = checkpoint_reached {
            let flag = self.actors[i].rect();
            self.checkpoint = (flag.centre_x().div_euclid(TILE), (flag.bottom() - 1).div_euclid(TILE));
            self.checkpoints_lit += 1;
            events.insert(Events::CHECKPOINT);
        }

        if self.boss.harmful() && player.overlaps(&self.boss.rect().inset(2)) {
            hurt = Some((boss::CONTACT_DAMAGE, self.boss.rect().centre_x()));
        }

        if let Some((damage, from_x)) = hurt {
            self.hurt_player(damage, from_x, events);
        }
    }

    fn hurt_player(&mut self, damage: i32, from_x: i32, events: &mut Events) {
        if self.god {
            return;
        }
        if self.player.hurt_by(self.difficulty.damage(damage), from_x) {
            events.insert(Events::HIT);
            if self.player.hp <= 0 {
                self.die(DEATH_FRAMES, events);
            }
        }
    }

    fn die(&mut self, frames: u16, events: &mut Events) {
        self.player.hp = self.player.hp.max(0);
        self.player.state = PlayerState::Dead;
        self.player.dead_timer = frames;
        self.player.body.vx_fx = 0;
        events.insert(Events::DIE);
    }

    fn hazards(&mut self, events: &mut Events) {
        let r = self.player.body.rect();
        let spiked = crate::physics::tiles_in(&r.inset(2))
            .any(|(tx, ty)| self.level.tile(tx, ty) == Tile::Spike);
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

    /// Back to the last checkpoint. A knockout restores full health; a pit keeps what
    /// was left. Living villains return to their posts.
    fn respawn(&mut self, events: &mut Events) {
        let (tx, ty) = self.checkpoint;
        let hp = if self.player.hp > 0 { self.player.hp } else { MAX_HP };
        self.player = Player::new(Body::standing_on_tile(tx, ty, PLAYER_W, PLAYER_H));
        self.player.hp = hp;
        self.player.invuln = crate::player::INVULN_FRAMES;
        for a in self.actors.iter_mut() {
            a.reset();
        }
        self.shots = [Shot::EMPTY; MAX_SHOTS];
        if self.arena {
            self.arena = false;
            self.boss.reset();
        }
        self.snap_camera();
        events.insert(Events::RESPAWN);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{boss::BossState, enemy::state, level::Theme};

    static RANGE: Level = Level {
        name: "TEST",
        subtitle: "",
        theme: Theme::Canyon,
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
    fn collects_sats_and_heals_and_lights_checkpoints() {
        let mut w = world();
        w.god = true;
        w.player.hp = 3;
        let events = hold(&mut w, keys::RIGHT, 120);
        assert!(events.contains(Events::PICKUP));
        assert!(events.contains(Events::CHECKPOINT));
        assert_eq!(w.sats, 1);
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
    fn holding_b_charges_a_piercing_beam() {
        let mut w = world();
        hold(&mut w, keys::B, CHARGE_FRAMES as usize + 5);
        let events = w.step(&Input::from_keys(0, keys::B));
        assert!(events.contains(Events::BEAM));
        assert!(w.shots.iter().any(|s| s.active && s.kind == ShotKind::Beam));
        assert_eq!(w.player.charge, 0);
    }

    #[test]
    fn bandit_is_shot_down_and_counts() {
        let mut w = world();
        w.god = true;
        // Walk up to the bandit and keep firing.
        let mut events = Events::default();
        for i in 0..400 {
            let keys = if w.player.body.x() < 170 { keys::RIGHT } else { 0 };
            let fire = if i % 2 == 0 { keys::B } else { 0 };
            events.insert(w.step(&Input::from_keys(keys | fire, if fire == 0 { keys::B } else { 0 })));
        }
        assert!(events.contains(Events::ENEMY_DOWN));
        assert_eq!(w.kills, 1);
        assert_eq!(w.score(), w.sats * 10 + 50);
    }

    #[test]
    fn beam_hits_each_enemy_once() {
        let mut w = world();
        let bandit = w.actors.iter().position(|a| a.kind == ActorKind::Bandit).unwrap();
        w.actors[bandit].awake = true;
        w.actors[bandit].hp = 10;
        let r = w.actors[bandit].rect();
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
        assert_eq!(w.actors[bandit].hp, 10 - ShotKind::Beam.damage());
    }

    #[test]
    fn stomping_bounces_and_damages() {
        let mut w = world();
        let bandit = w.actors.iter().position(|a| a.kind == ActorKind::Bandit).unwrap();
        let r = w.actors[bandit].rect();
        w.actors[bandit].awake = true;
        w.player.body.x_fx = r.x * FX;
        w.player.body.y_fx = (r.y - PLAYER_H + 4) * FX;
        w.player.body.vy_fx = 2 * FX;
        let mut events = Events::default();
        w.collide(&mut events);
        assert_eq!(w.actors[bandit].hp, ActorKind::Bandit.max_hp() - 2);
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
            let events = w.step(&Input::default());
            assert!(events.contains(Events::HIT), "{difficulty:?}");
            assert_eq!(w.player.hp, expected, "{difficulty:?}");
        }
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
        let bandit = w.actors.iter().position(|a| a.kind == ActorKind::Bandit).unwrap();
        w.actors[bandit].awake = true;
        w.actors[bandit].body.x_fx += 40 * FX;
        w.player.hp = 1;
        let mut events = Events::default();
        w.hurt_player(2, 0, &mut events);
        assert!(events.contains(Events::DIE));
        assert!(w.player.is_dead());
        let events = hold(&mut w, 0, DEATH_FRAMES as usize);
        assert!(events.contains(Events::RESPAWN));
        assert_eq!(w.player.hp, MAX_HP);
        assert!(!w.actors[bandit].awake);
        assert_eq!(w.actors[bandit].body.x(), w.actors[bandit].home.0);
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
}
