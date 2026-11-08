    //! The bosses waiting at the end of each level.
//!
//! BERA 3000, the Badlands boss, is a bear-shaped war mech with an arm cannon. The
//! fight loops Walk → Aim (two volleys of three shots, alternating low and high) →
//! Telegraph → Charge low on its treads into the arena wall → Stalled (double damage).
//! Below a third of its armour it adds Leap → Slam, which sends shockwaves along the
//! ground and rains missiles.
//!
//! SAM, the Trenches boss, is a cyborg exchange founder who fights with cucumbers. He
//! paces, then works through a fixed pattern of three attacks: the Cucumber Throw (an
//! arcing cucumber), the Cucumber Hit (a telegraphed lunge that hurls a giant cucumber
//! flat along the floor, after which he is winded and takes double damage), and the
//! Market Crash (red candles rain from the charts while his implants fire magenta
//! energy). At half health his implants overload: he gets faster, throws in volleys,
//! and his Cucumber Hit sends shockwaves along the floor too.
//!
//! Every step is on timers, so the fights are deterministic.

use crate::{
    FX, SCREEN_W,
    level::{BossKind, Level},
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

/// SAM's hitbox: stocky, and half again the wassie's height.
pub const SAM_W: i32 = 24;
pub const SAM_H: i32 = 40;
pub const SAM_HP: i32 = 40;
/// Running into SAM mid-lunge.
pub const SWING_DAMAGE: i32 = 3;
/// Frames SAM takes to come apart once beaten.
pub const SAM_DYING_FRAMES: u16 = 150;
const SAM_WALK_FX: i32 = 3 * FX / 5;
const SAM_LUNGE_FX: i32 = 7 * FX / 2;
/// Frames a thrown cucumber takes to come down where the wassie stood.
const THROW_FRAMES: i32 = 44;
/// SAM's attacks, in the order he works through them, before and after he overloads.
/// He opens with his signature, the Cucumber Hit.
const SAM_PATTERN: [[BossState; 6]; 2] = [
    [BossState::Windup, BossState::Throw, BossState::Throw, BossState::Crash, BossState::Throw, BossState::Windup],
    [BossState::Windup, BossState::Crash, BossState::Throw, BossState::Windup, BossState::Throw, BossState::Crash],
];
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
    /// SAM winds up a cucumber, then throws it.
    Throw,
    /// SAM's Cucumber Hit tell: he crouches and his aura crackles.
    Windup,
    /// The Cucumber Hit itself: a lunge and the giant cucumber.
    Swing,
    /// Out of breath after the Cucumber Hit: double damage, harmless to touch.
    Winded,
    /// MARKET CRASH tell: arms up, the screens turn red.
    Crash,
    /// MARKET CRASH: candles rain and energy flies.
    Barrage,
    /// Half health: the implants overload. Invulnerable for a moment.
    Overload,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Boss {
    pub kind: BossKind,
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
    /// How far through its attack pattern it is.
    step: u8,
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
        let kind = level.boss;
        let (w, h) = Self::size(kind);
        let (present, body) = match level.find(b'B') {
            Some((tx, ty)) => (true, Body::standing_on_tile(tx, ty, w, h)),
            None => (false, Body::default()),
        };
        Self {
            kind,
            present,
            body,
            hp: Self::full_hp(kind),
            state: BossState::Asleep,
            timer: 0,
            facing: -1,
            flash: 0,
            volleys: 0,
            shots_left: 0,
            left_ground: false,
            grounded: true,
            step: 0,
            home: (body.x_fx, body.y_fx),
            arena: (arena_x, arena_x + SCREEN_W),
        }
    }

    fn size(kind: BossKind) -> (i32, i32) {
        match kind {
            BossKind::Bera => (BOSS_W, BOSS_H),
            BossKind::Sam => (SAM_W, SAM_H),
        }
    }

    fn full_hp(kind: BossKind) -> i32 {
        match kind {
            BossKind::Bera => BOSS_HP,
            BossKind::Sam => SAM_HP,
        }
    }

    /// Full health, for the health bar.
    pub fn max_hp(&self) -> i32 {
        Self::full_hp(self.kind)
    }

    /// Height standing up.
    fn standing_h(&self) -> i32 {
        Self::size(self.kind).1
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
        self.set_height(self.standing_h());
        self.hp = self.max_hp();
        self.state = BossState::Asleep;
        self.timer = 0;
        self.volleys = 0;
        self.step = 0;
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

    /// BERA: 1 until two thirds of her armour are gone, then 2. SAM: 1 until half his
    /// health is gone, then 2.
    pub fn phase(&self) -> u8 {
        let lasts = match self.kind {
            BossKind::Bera => self.hp * 3 > BOSS_HP,
            BossKind::Sam => self.hp * 2 > SAM_HP,
        };
        if lasts { 1 } else { 2 }
    }

    /// Whether touching it hurts, and how much; heavy hits knock the wassie flying.
    pub fn contact(&self) -> Option<(i32, bool)> {
        if !self.harmful() {
            return None;
        }
        Some(match self.state {
            BossState::Swing => (SWING_DAMAGE, true),
            _ => (CONTACT_DAMAGE, false),
        })
    }

    pub fn vulnerable(&self) -> bool {
        self.is_active() && !matches!(self.state, BossState::Intro | BossState::Dying | BossState::Overload)
    }

    /// Stalled or winded: harmless to touch, and taking double damage.
    pub fn dazed(&self) -> bool {
        matches!(self.state, BossState::Stunned | BossState::Winded)
    }

    pub fn harmful(&self) -> bool {
        self.vulnerable() && !self.dazed()
    }

    /// Applies a hit; a stunned mech or a winded SAM takes double. Returns true if this
    /// hit beat it.
    pub fn damage(&mut self, amount: i32, events: &mut Events) -> bool {
        if !self.vulnerable() {
            return false;
        }
        let amount = if self.dazed() { amount * 2 } else { amount };
        let phase = self.phase();
        self.hp -= amount;
        self.flash = 6;
        events.insert(Events::BOSS_HIT);
        if self.hp <= 0 {
            self.hp = 0;
            self.set_height(self.standing_h());
            self.body.vx_fx = 0;
            self.state = BossState::Dying;
            self.timer = match self.kind {
                BossKind::Bera => 120,
                BossKind::Sam => SAM_DYING_FRAMES,
            };
            events.insert(Events::BOSS_DOWN);
            return true;
        }
        if self.kind == BossKind::Sam && phase == 1 && self.phase() == 2 {
            // The implants overload: he stops whatever he was doing.
            self.body.vx_fx = 0;
            self.go(BossState::Overload, 50);
            events.insert(Events::SLAM);
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
        if self.state == BossState::Dying {
            if self.timer == 0 {
                self.state = BossState::Defeated;
                events.insert(Events::LEVEL_CLEAR);
            }
            return events;
        }
        match self.kind {
            BossKind::Bera => self.bera(ctx, &mut events),
            BossKind::Sam => self.sam(ctx, &mut events),
        }

        self.body.vy_fx = (self.body.vy_fx + GRAVITY_FX).min(5 * FX);
        let contacts = self.body.step(ctx.level);
        self.grounded = contacts.ground;
        if !contacts.ground {
            self.left_ground = true;
        }
        // The arena edges are walls for the boss too.
        let min_x = self.arena.0 + 2;
        let max_x = self.arena.1 - 2 - self.body.w;
        let x = self.body.x().clamp(min_x, max_x);
        if x != self.body.x() {
            self.body.x_fx = x * FX;
        }
        events
    }

    fn bera(&mut self, ctx: &mut BossCtx, events: &mut Events) {
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
                    events.insert(Events::SLAM);
                    self.body.vx_fx = 0;
                    self.go(BossState::Recover, 40);
                }
            }
            BossState::Recover => {
                if self.timer == 0 {
                    self.go(BossState::Walk, 60);
                }
            }
            _ => {}
        }
    }

    /// One frame of SAM. Timers run shorter once he has overloaded.
    fn sam(&mut self, ctx: &mut BossCtx, events: &mut Events) {
        let player = ctx.player;
        let fast = self.phase() == 2;
        let pick = |slow: u16, quick: u16| if fast { quick } else { slow };
        match self.state {
            BossState::Intro | BossState::Overload => {
                self.face(&player);
                self.body.vx_fx = 0;
                if self.timer == 0 {
                    self.go(BossState::Walk, 50);
                }
            }
            BossState::Walk => {
                self.face(&player);
                let dist = (player.centre_x() - self.rect().centre_x()).abs();
                let speed = if fast { SAM_WALK_FX * 3 / 2 } else { SAM_WALK_FX };
                self.body.vx_fx = match dist {
                    d if d > 110 => self.facing * speed,
                    d if d < 56 => -self.facing * speed,
                    _ => 0,
                };
                if self.timer == 0 {
                    self.body.vx_fx = 0;
                    let next = SAM_PATTERN[fast as usize][self.step as usize % SAM_PATTERN[0].len()];
                    self.step = self.step.wrapping_add(1);
                    match next {
                        BossState::Throw => {
                            self.shots_left = if fast { 3 } else { 1 };
                            self.go(BossState::Throw, 22);
                        }
                        BossState::Windup => self.go(BossState::Windup, pick(54, 38)),
                        _ => self.go(BossState::Crash, pick(44, 32)),
                    }
                }
            }
            BossState::Throw => {
                self.face(&player);
                if self.timer == 0 {
                    self.throw_cucumber(ctx);
                    self.shots_left = self.shots_left.saturating_sub(1);
                    if self.shots_left > 0 {
                        self.timer = 14;
                    } else {
                        self.go(BossState::Walk, ctx.difficulty.fire_interval(pick(64, 40)));
                    }
                }
            }
            BossState::Windup => {
                self.face(&player);
                self.body.vx_fx = 0;
                if self.timer == 0 {
                    let r = self.rect();
                    let speed = if fast { 6 * FX } else { 5 * FX };
                    let x = r.centre_x() + self.facing * (SAM_W / 2 + 12);
                    shot::spawn(ctx.shots, ShotKind::MegaCucumber, x, r.bottom() - 12, self.facing * speed, 0);
                    self.body.vx_fx = self.facing * SAM_LUNGE_FX;
                    events.insert(Events::SWING);
                    self.go(BossState::Swing, 18);
                }
            }
            BossState::Swing => {
                let r = self.rect();
                let wall = (self.facing < 0 && r.x <= self.arena.0 + 4) || (self.facing > 0 && r.right() >= self.arena.1 - 4);
                if self.timer == 0 || wall {
                    self.body.vx_fx = 0;
                    if fast {
                        // Overloaded, the swing ends in a stomp that sends shockwaves out.
                        let y = r.bottom() - 6;
                        for dir in [-1, 1] {
                            shot::spawn(ctx.shots, ShotKind::Shockwave, r.centre_x() + dir * 16, y, dir * 2 * FX, 0);
                        }
                        events.insert(Events::SLAM);
                    }
                    self.go(BossState::Winded, pick(80, 56));
                }
            }
            BossState::Winded => {
                self.body.vx_fx = 0;
                if self.timer == 0 {
                    self.go(BossState::Walk, pick(50, 30));
                }
            }
            BossState::Crash => {
                self.face(&player);
                self.body.vx_fx = 0;
                if self.timer == 0 {
                    self.go(BossState::Barrage, 100);
                }
            }
            BossState::Barrage => {
                self.body.vx_fx = 0;
                let every = if fast { 7 } else { 12 };
                if self.timer % every == 0 {
                    let x = self.arena.0 + 12 + ctx.rng.below((SCREEN_W - 24) as u32) as i32;
                    shot::spawn(ctx.shots, ShotKind::Candle, x, 18, 0, FX / 2);
                }
                if self.timer % 32 == 16 {
                    self.face(&player);
                    let r = self.rect();
                    let (x, y) = (r.centre_x() + self.facing * 8, r.y + 10);
                    for vy in [-FX / 2, 0, FX / 2] {
                        shot::spawn(ctx.shots, ShotKind::Orb, x, y, self.facing * 2 * FX, vy + FX / 4);
                    }
                }
                if self.timer == 0 {
                    self.go(BossState::Walk, pick(60, 36));
                }
            }
            _ => {}
        }
    }

    /// A cucumber lobbed to come down where the wassie stands now.
    fn throw_cucumber(&mut self, ctx: &mut BossCtx) {
        let r = self.rect();
        let (x, y) = (r.centre_x() + self.facing * 10, r.y + 14);
        let (tx, ty) = (ctx.player.centre_x(), ctx.player.centre_y());
        let n = THROW_FRAMES;
        let vx = ((tx - x) * FX / n).clamp(-3 * FX, 3 * FX);
        let vy = (ty - y) * FX / n - shot::GRAVITY_FX_SHOT * n / 2;
        shot::spawn(ctx.shots, ShotKind::Cucumber, x, y, vx, vy);
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
            shot::spawn(ctx.shots, ShotKind::Missile, x, 4, 0, 0);
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
        assert_eq!(a.boss.hp, BOSS_HP - 4, "a stunned mech takes double");
        assert!(!a.boss.harmful(), "touching a stalled mech is safe");
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
        assert_eq!(a.shots_of(ShotKind::Missile), 3);
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

    /// SAM in the Trenches' arena, with the wassie standing well to his left.
    fn sam() -> Arena {
        let level = levels::get(1);
        let mut boss = Boss::new(level);
        assert_eq!(boss.kind, BossKind::Sam);
        boss.wake();
        let x = boss.arena.0 + 30;
        Arena { boss, shots: [Shot::EMPTY; MAX_SHOTS], rng: Rng::new(7), player: Rect::new(x, 102, 10, 26) }
    }

    fn step_sam(a: &mut Arena) -> Events {
        a.boss.update(&mut BossCtx {
            level: levels::get(1),
            player: a.player,
            shots: &mut a.shots,
            rng: &mut a.rng,
            difficulty: Difficulty::Normal,
        })
    }

    fn sam_until(a: &mut Arena, state: BossState, limit: usize) -> Events {
        let mut events = Events::default();
        for _ in 0..limit {
            if a.boss.state == state {
                return events;
            }
            events.insert(step_sam(a));
        }
        panic!("SAM never reached {state:?}, stuck in {:?}", a.boss.state);
    }

    #[test]
    fn sam_is_bigger_than_the_wassie_and_throws_a_cucumber_in_an_arc() {
        let mut a = sam();
        assert!(a.boss.rect().h >= crate::player::PLAYER_H * 3 / 2);
        assert_eq!(a.boss.hp, SAM_HP);
        sam_until(&mut a, BossState::Throw, 800);
        sam_until(&mut a, BossState::Walk, 100);
        let c = a.shots.iter().find(|s| s.active && s.kind == ShotKind::Cucumber).expect("a cucumber");
        assert!(c.vx_fx < 0, "thrown at the wassie on his left");
        assert!(c.vy_fx < 0, "lobbed upwards first");
    }

    #[test]
    fn the_cucumber_hit_is_telegraphed_then_lunges_and_leaves_him_winded() {
        let mut a = sam();
        sam_until(&mut a, BossState::Windup, 1200);
        let tell = a.boss.timer;
        assert!(tell >= 30, "a clear warning: {tell} frames");
        let events = sam_until(&mut a, BossState::Swing, 200);
        assert!(events.contains(Events::SWING));
        assert_eq!(a.boss.contact(), Some((SWING_DAMAGE, true)), "running into him knocks hard");
        assert_eq!(a.shots.iter().filter(|s| s.active && s.kind == ShotKind::MegaCucumber).count(), 1);
        assert!(ShotKind::MegaCucumber.heavy());
        sam_until(&mut a, BossState::Winded, 100);
        assert!(!a.boss.harmful(), "winded, he is safe to touch");
        let mut events = Events::default();
        a.boss.damage(2, &mut events);
        assert_eq!(a.boss.hp, SAM_HP - 4, "and takes double damage");
    }

    #[test]
    fn market_crash_rains_candles_and_fires_energy() {
        let mut a = sam();
        sam_until(&mut a, BossState::Crash, 2000);
        sam_until(&mut a, BossState::Barrage, 100);
        for _ in 0..40 {
            step_sam(&mut a);
        }
        let candles = a.shots.iter().filter(|s| s.active && s.kind == ShotKind::Candle).count();
        let orbs = a.shots.iter().filter(|s| s.active && s.kind == ShotKind::Orb).count();
        assert!(candles >= 2 && orbs >= 3, "{candles} candles, {orbs} orbs");
    }

    #[test]
    fn at_half_health_sam_overloads_and_speeds_up() {
        let mut a = sam();
        sam_until(&mut a, BossState::Walk, 200);
        let mut events = Events::default();
        a.boss.damage(SAM_HP / 2, &mut events);
        assert_eq!(a.boss.phase(), 2);
        assert_eq!(a.boss.state, BossState::Overload);
        assert!(!a.boss.vulnerable(), "a moment of invulnerability while it overloads");
        assert!(events.contains(Events::SLAM));
        // Overloaded, the Cucumber Hit ends in shockwaves and the tell is shorter.
        sam_until(&mut a, BossState::Windup, 1500);
        assert!(a.boss.timer < 54);
        sam_until(&mut a, BossState::Winded, 300);
        assert_eq!(a.shots.iter().filter(|s| s.active && s.kind == ShotKind::Shockwave).count(), 2);
    }

    #[test]
    fn sam_goes_down_and_clears_the_level() {
        let mut a = sam();
        sam_until(&mut a, BossState::Walk, 200);
        let mut events = Events::default();
        a.boss.damage(SAM_HP / 2, &mut events);
        sam_until(&mut a, BossState::Walk, 100);
        assert!(a.boss.damage(SAM_HP, &mut events));
        assert!(events.contains(Events::BOSS_DOWN));
        let mut cleared = false;
        for _ in 0..SAM_DYING_FRAMES + 2 {
            cleared |= step_sam(&mut a).contains(Events::LEVEL_CLEAR);
        }
        assert!(cleared && a.boss.state == BossState::Defeated);
    }
}
