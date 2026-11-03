//! Sheriff Ursa, the Canyon's boss: a bear with a badge and a revolver.
//!
//! The fight loops Walk → Aim (two volleys of three shots, alternating low and high) →
//! Telegraph → Charge on all fours into the arena wall → Stunned (double damage).
//! Below a third of her health she adds Leap → Slam, which sends shockwaves along the
//! ground and rains red candles. Every step is on timers, so the fight is deterministic.

use crate::{
    FX, SCREEN_W,
    level::Level,
    physics::{Body, Rect},
    player::GRAVITY_FX,
    rng::Rng,
    shot::{self, MAX_SHOTS, Shot, ShotKind},
    world::{Difficulty, Events},
};

pub const BOSS_W: i32 = 32;
/// Tall enough that bolts fired from the arena planks still reach her.
pub const BOSS_H: i32 = 44;
/// Height while charging on all fours: low enough to jump over.
pub const CHARGE_H: i32 = 22;
const _: () = assert!(CHARGE_H < crate::player::JUMP_APEX, "the charge must be jumpable");
pub const BOSS_HP: i32 = 30;
pub const CONTACT_DAMAGE: i32 = 2;
const WALK_FX: i32 = 3 * FX / 4;
const CHARGE_FX: i32 = 3 * FX;
const BULLET_FX: i32 = 2 * FX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BossState {
    /// Waiting for the player to enter the arena.
    Asleep,
    /// Arena locked; she tips her hat. Invulnerable.
    Intro,
    Walk,
    Aim,
    Telegraph,
    Charge,
    Stunned,
    Leap,
    Recover,
    Dying,
    Defeated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Boss {
    /// False on levels without a boss.
    pub present: bool,
    pub body: Body,
    pub hp: i32,
    pub state: BossState,
    pub timer: u16,
    pub facing: i32,
    pub flash: u8,
    volleys: u8,
    shots_left: u8,
    left_ground: bool,
    grounded: bool,
    home: (i32, i32),
    /// Arena x range in pixels.
    pub arena: (i32, i32),
}

pub struct BossCtx<'a> {
    pub level: &'a Level,
    pub player: Rect,
    pub shots: &'a mut [Shot; MAX_SHOTS],
    pub rng: &'a mut Rng,
    pub difficulty: Difficulty,
}

impl Boss {
    pub fn new(level: &Level) -> Self {
        let arena_x = level.arena_px();
        let (present, body) = match level.find(b'B') {
            Some((tx, ty)) => (true, Body::standing_on_tile(tx, ty, BOSS_W, BOSS_H)),
            None => (false, Body::default()),
        };
        Self {
            present,
            body,
            hp: BOSS_HP,
            state: BossState::Asleep,
            timer: 0,
            facing: -1,
            flash: 0,
            volleys: 0,
            shots_left: 0,
            left_ground: false,
            grounded: true,
            home: (body.x_fx, body.y_fx),
            arena: (arena_x, arena_x + SCREEN_W),
        }
    }

    /// Back to the start of the fight, for when the player respawns before winning.
    pub fn reset(&mut self) {
        if !self.present || self.is_beaten() {
            return;
        }
        self.body.x_fx = self.home.0;
        self.body.y_fx = self.home.1;
        self.body.vx_fx = 0;
        self.body.vy_fx = 0;
        self.set_height(BOSS_H);
        self.hp = BOSS_HP;
        self.state = BossState::Asleep;
        self.timer = 0;
        self.volleys = 0;
        self.flash = 0;
    }

    pub fn wake(&mut self) {
        if self.present && self.state == BossState::Asleep {
            self.state = BossState::Intro;
            self.timer = 90;
        }
    }

    pub fn rect(&self) -> Rect {
        self.body.rect()
    }

    pub fn is_beaten(&self) -> bool {
        matches!(self.state, BossState::Dying | BossState::Defeated)
    }

    pub fn is_active(&self) -> bool {
        self.present && !matches!(self.state, BossState::Asleep | BossState::Defeated)
    }

    /// 1 until a third of her health is gone, then 2.
    pub fn phase(&self) -> u8 {
        if self.hp * 3 > BOSS_HP { 1 } else { 2 }
    }

    pub fn vulnerable(&self) -> bool {
        self.is_active() && !matches!(self.state, BossState::Intro | BossState::Dying)
    }

    pub fn harmful(&self) -> bool {
        self.vulnerable() && self.state != BossState::Stunned
    }

    /// Applies a hit; stunned bears take double. Returns true if this hit beat her.
    pub fn damage(&mut self, amount: i32, events: &mut Events) -> bool {
        if !self.vulnerable() {
            return false;
        }
        let amount = if self.state == BossState::Stunned { amount * 2 } else { amount };
        self.hp -= amount;
        self.flash = 6;
        events.insert(Events::BOSS_HIT);
        if self.hp <= 0 {
            self.hp = 0;
            self.set_height(BOSS_H);
            self.body.vx_fx = 0;
            self.state = BossState::Dying;
            self.timer = 120;
            events.insert(Events::BOSS_DOWN);
            return true;
        }
        false
    }

    /// Changes the hitbox height, keeping the feet in place.
    fn set_height(&mut self, h: i32) {
        self.body.y_fx += (self.body.h - h) * FX;
        self.body.h = h;
    }

    fn face(&mut self, player: &Rect) {
        self.facing = if player.centre_x() < self.rect().centre_x() { -1 } else { 1 };
    }

    fn go(&mut self, state: BossState, timer: u16) {
        self.state = state;
        self.timer = timer;
    }

    pub fn update(&mut self, ctx: &mut BossCtx) -> Events {
        let mut events = Events::default();
        if !self.is_active() {
            return events;
        }
        self.flash = self.flash.saturating_sub(1);
        self.timer = self.timer.saturating_sub(1);
        let player = ctx.player;

        match self.state {
            BossState::Intro => {
                self.face(&player);
                if self.timer == 0 {
                    self.go(BossState::Walk, 80);
                }
            }
            BossState::Walk => {
                self.face(&player);
                let dist = (player.centre_x() - self.rect().centre_x()).abs();
                self.body.vx_fx = match dist {
                    d if d > 100 => self.facing * WALK_FX,
                    d if d < 64 => -self.facing * WALK_FX,
                    _ => 0,
                };
                if self.timer == 0 {
                    self.body.vx_fx = 0;
                    self.shots_left = 3;
                    self.go(BossState::Aim, 16);
                }
            }
            BossState::Aim => {
                self.face(&player);
                self.body.vx_fx = 0;
                if self.timer == 0 {
                    let r = self.rect();
                    // Low shots must be jumped. High ones pass over a standing wassie but
                    // track one up on a plank, so the planks are no hiding place.
                    let y = if self.shots_left % 2 == 1 {
                        r.bottom() - 6
                    } else {
                        player.centre_y().clamp(r.y + 4, r.bottom() - 30)
                    };
                    let x = r.centre_x() + self.facing * (BOSS_W / 2 + 4);
                    shot::spawn(ctx.shots, ShotKind::Bullet, x, y, self.facing * BULLET_FX, 0);
                    self.shots_left -= 1;
                    if self.shots_left > 0 {
                        self.timer = ctx.difficulty.fire_interval(14);
                    } else {
                        self.volleys += 1;
                        if self.volleys >= 2 {
                            self.volleys = 0;
                            self.go(BossState::Telegraph, 36);
                        } else {
                            self.go(BossState::Walk, ctx.difficulty.fire_interval(60));
                        }
                    }
                }
            }
            BossState::Telegraph => {
                self.face(&player);
                if self.timer == 0 {
                    self.set_height(CHARGE_H);
                    self.body.vx_fx = self.facing * CHARGE_FX;
                    self.go(BossState::Charge, 0);
                }
            }
            BossState::Charge => {
                let r = self.rect();
                let hit_wall = (self.facing < 0 && r.x <= self.arena.0 + 4)
                    || (self.facing > 0 && r.right() >= self.arena.1 - 4);
                if hit_wall {
                    self.body.vx_fx = 0;
                    self.set_height(BOSS_H);
                    self.go(BossState::Stunned, 100);
                }
            }
            BossState::Stunned => {
                if self.timer == 0 {
                    if self.phase() == 2 {
                        self.face(&player);
                        self.body.vy_fx = -5 * FX;
                        self.body.vx_fx = self.facing * 5 * FX / 4;
                        self.left_ground = false;
                        self.go(BossState::Leap, 0);
                    } else {
                        self.go(BossState::Walk, 60);
                    }
                }
            }
            BossState::Leap => {
                if self.left_ground && self.grounded {
                    self.slam(ctx);
                    self.body.vx_fx = 0;
                    self.go(BossState::Recover, 40);
                }
            }
            BossState::Recover => {
                if self.timer == 0 {
                    self.go(BossState::Walk, 60);
                }
            }
            BossState::Dying => {
                if self.timer == 0 {
                    self.state = BossState::Defeated;
                    events.insert(Events::LEVEL_CLEAR);
                }
                return events;
            }
            BossState::Asleep | BossState::Defeated => {}
        }

        self.body.vy_fx = (self.body.vy_fx + GRAVITY_FX).min(5 * FX);
        let contacts = self.body.step(ctx.level);
        self.grounded = contacts.ground;
        if !contacts.ground {
            self.left_ground = true;
        }
        // The arena edges are walls for the sheriff too.
        let min_x = self.arena.0 + 2;
        let max_x = self.arena.1 - 2 - BOSS_W;
        let x = self.body.x().clamp(min_x, max_x);
        if x != self.body.x() {
            self.body.x_fx = x * FX;
        }
        events
    }

    fn slam(&mut self, ctx: &mut BossCtx) {
        let r = self.rect();
        let y = r.bottom() - 6;
        for dir in [-1, 1] {
            shot::spawn(ctx.shots, ShotKind::Shockwave, r.centre_x() + dir * 20, y, dir * 2 * FX, 0);
        }
        let px = ctx.player.centre_x();
        for offset in [-48, 0, 48] {
            let jitter = ctx.rng.below(17) as i32 - 8;
            let x = (px + offset + jitter).clamp(self.arena.0 + 8, self.arena.1 - 8);
            shot::spawn(ctx.shots, ShotKind::Candle, x, 4, 0, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{levels, shot::Shot};

    struct Arena {
        boss: Boss,
        shots: [Shot; MAX_SHOTS],
        rng: Rng,
        player: Rect,
    }

    impl Arena {
        fn new() -> Self {
            let level = levels::get(0);
            let mut boss = Boss::new(level);
            boss.wake();
            let x = boss.arena.0 + 20;
            Self {
                boss,
                shots: [Shot::EMPTY; MAX_SHOTS],
                rng: Rng::new(7),
                player: Rect::new(x, 102, 10, 26),
            }
        }

        fn step(&mut self) -> Events {
            self.boss.update(&mut BossCtx {
                level: levels::get(0),
                player: self.player,
                shots: &mut self.shots,
                rng: &mut self.rng,
                difficulty: Difficulty::Normal,
            })
        }

        /// Steps until `state`, failing after `limit` frames.
        fn until(&mut self, state: BossState, limit: usize) {
            for _ in 0..limit {
                if self.boss.state == state {
                    return;
                }
                self.step();
            }
            panic!("never reached {state:?}, stuck in {:?}", self.boss.state);
        }

        fn shots_of(&self, kind: ShotKind) -> usize {
            self.shots.iter().filter(|s| s.active && s.kind == kind).count()
        }
    }

    #[test]
    fn intro_is_invulnerable() {
        let mut a = Arena::new();
        let mut events = Events::default();
        assert!(!a.boss.damage(5, &mut events));
        assert_eq!(a.boss.hp, BOSS_HP);
    }

    #[test]
    fn volleys_then_charges_low_into_the_wall_and_is_stunned() {
        let mut a = Arena::new();
        a.until(BossState::Aim, 400);
        a.until(BossState::Walk, 100);
        assert_eq!(a.shots_of(ShotKind::Bullet), 3, "one volley of three");
        a.until(BossState::Charge, 600);
        assert_eq!(a.boss.body.h, CHARGE_H, "low enough to jump over");
        a.until(BossState::Stunned, 200);
        assert_eq!(a.boss.body.h, BOSS_H);
        assert!(a.boss.rect().x <= a.boss.arena.0 + 4, "ran into the left wall");

        let mut events = Events::default();
        a.boss.damage(2, &mut events);
        assert_eq!(a.boss.hp, BOSS_HP - 4, "stunned bears take double");
        assert!(!a.boss.harmful(), "touching a dizzy bear is safe");
    }

    #[test]
    fn phase_two_leaps_and_slams() {
        let mut a = Arena::new();
        a.boss.hp = BOSS_HP / 3;
        assert_eq!(a.boss.phase(), 2);
        a.until(BossState::Stunned, 1500);
        a.until(BossState::Leap, 200);
        a.until(BossState::Recover, 200);
        assert_eq!(a.shots_of(ShotKind::Shockwave), 2);
        assert_eq!(a.shots_of(ShotKind::Candle), 3);
    }

    #[test]
    fn defeat_plays_out_then_clears_the_level() {
        let mut a = Arena::new();
        a.until(BossState::Walk, 200);
        let mut events = Events::default();
        assert!(a.boss.damage(BOSS_HP, &mut events));
        assert!(events.contains(Events::BOSS_DOWN));
        assert_eq!(a.boss.state, BossState::Dying);
        let mut cleared = false;
        for _ in 0..200 {
            cleared |= a.step().contains(Events::LEVEL_CLEAR);
        }
        assert!(cleared);
        assert_eq!(a.boss.state, BossState::Defeated);
        assert!(!a.boss.is_active());
    }

    #[test]
    fn reset_restores_a_fresh_fight() {
        let mut a = Arena::new();
        a.until(BossState::Charge, 1000);
        a.boss.hp = 9;
        a.boss.reset();
        assert_eq!(a.boss.hp, BOSS_HP);
        assert_eq!(a.boss.state, BossState::Asleep);
        assert_eq!(a.boss.body.h, BOSS_H);
    }
}
