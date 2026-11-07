//! Everything placed on the map besides terrain: villains, pickups and checkpoints.
//! Villains are small state machines that only run while near the camera.

use crate::{
    FX, TILE,
    level::Level,
    physics::{Body, Rect},
    player::GRAVITY_FX,
    shot::{self, MAX_SHOTS, Shot, ShotKind},
    world::Difficulty,
};

/// At most 128, for the `u128` mask a piercing beam keeps of the actors it has hit.
/// Every `World` holds this many, and worlds are built on the GBA's small IWRAM stack,
/// so keep it no bigger than the levels need.
pub const MAX_ACTORS: usize = 96;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ActorKind {
    Diamond,
    Heal,
    Checkpoint,
    /// Walks ledges, turns at the edge, fires when the wassie is in line.
    Sentry,
    /// Spider-bot: crouches, revs up, then lunges.
    Crawler,
    /// Hovers overhead and drops EMP bombs.
    Drone,
    /// Rolls in from the right, bouncing.
    Mine,
    /// Gold coin, the casino's currency.
    Coin,
    /// A plasma cell for CL207's shield.
    Part,
    /// The elevator's access terminal: off until the wassie presses Up at it.
    Terminal,
    /// Casino croupier bot: deals a fan of three cards.
    Dealer,
    /// Armoured security bot on the upper floors: closes in and fires bursts.
    Guard,
}

impl ActorKind {
    pub fn from_marker(c: u8) -> Option<ActorKind> {
        Some(match c {
            b'o' => ActorKind::Diamond,
            b'+' => ActorKind::Heal,
            b'C' => ActorKind::Checkpoint,
            b'b' => ActorKind::Sentry,
            b's' => ActorKind::Crawler,
            b'v' => ActorKind::Drone,
            b't' => ActorKind::Mine,
            b'$' => ActorKind::Coin,
            b'p' => ActorKind::Part,
            b'E' => ActorKind::Terminal,
            b'd' => ActorKind::Dealer,
            b'g' => ActorKind::Guard,
            _ => return None,
        })
    }

    pub fn size(self) -> (i32, i32) {
        match self {
            ActorKind::Diamond => (8, 8),
            ActorKind::Heal => (10, 10),
            ActorKind::Checkpoint => (8, 24),
            ActorKind::Sentry => (14, 22),
            ActorKind::Crawler => (16, 14),
            ActorKind::Drone => (16, 10),
            ActorKind::Mine => (12, 12),
            ActorKind::Coin => (8, 8),
            ActorKind::Part => (10, 12),
            ActorKind::Terminal => (12, 20),
            ActorKind::Dealer => (14, 22),
            ActorKind::Guard => (16, 26),
        }
    }

    pub fn is_enemy(self) -> bool {
        matches!(
            self,
            ActorKind::Sentry
                | ActorKind::Crawler
                | ActorKind::Drone
                | ActorKind::Mine
                | ActorKind::Dealer
                | ActorKind::Guard
        )
    }

    /// Gold coins a bot pays out when beaten in a casino level.
    pub fn bounty(self) -> u32 {
        match self {
            ActorKind::Guard => 3,
            ActorKind::Sentry | ActorKind::Dealer => 2,
            ActorKind::Crawler | ActorKind::Drone | ActorKind::Mine => 1,
            _ => 0,
        }
    }

    pub fn max_hp(self) -> i32 {
        match self {
            ActorKind::Guard => 6,
            ActorKind::Sentry | ActorKind::Dealer => 3,
            ActorKind::Crawler | ActorKind::Drone => 2,
            _ => 1,
        }
    }

    pub fn contact_damage(self) -> i32 {
        match self {
            ActorKind::Sentry | ActorKind::Crawler | ActorKind::Guard => 2,
            _ => 1,
        }
    }

    /// Checkpoints stay lit and terminals stay on through a respawn.
    fn keeps_state(self) -> bool {
        matches!(self, ActorKind::Checkpoint | ActorKind::Terminal)
    }

    /// Picked up by touching it.
    pub fn is_pickup(self) -> bool {
        matches!(self, ActorKind::Diamond | ActorKind::Heal | ActorKind::Coin | ActorKind::Part)
    }

    /// Floats instead of falling.
    fn flies(self) -> bool {
        matches!(
            self,
            ActorKind::Drone
                | ActorKind::Diamond
                | ActorKind::Heal
                | ActorKind::Checkpoint
                | ActorKind::Coin
                | ActorKind::Part
        )
    }
}

/// Crawler and sentry behaviour states.
pub mod state {
    pub const IDLE: u8 = 0;
    pub const AIM: u8 = 1;
    pub const LUNGE: u8 = 2;
    pub const RETURN: u8 = 3;
    /// Checkpoint flag that has been touched, or a terminal switched on.
    pub const LIT: u8 = 1;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Actor {
    pub kind: ActorKind,
    pub body: Body,
    /// Spawn position (top-left, pixels), where a respawn puts it back.
    pub home: (i32, i32),
    pub hp: i32,
    pub alive: bool,
    /// Shot down or collected: stays gone after a respawn. Actors that merely left
    /// (a mine rolling off screen) come back.
    pub killed: bool,
    pub awake: bool,
    pub facing: i32,
    pub state: u8,
    pub timer: u16,
    /// Frames left flashing white after a hit.
    pub flash: u8,
    /// Shots left in a burst.
    pub count: u8,
}

impl Actor {
    pub const EMPTY: Actor = Actor {
        kind: ActorKind::Diamond,
        body: Body {
            x_fx: 0,
            y_fx: 0,
            vx_fx: 0,
            vy_fx: 0,
            w: 0,
            h: 0,
        },
        home: (0, 0),
        hp: 0,
        alive: false,
        killed: false,
        awake: false,
        facing: -1,
        state: 0,
        timer: 0,
        flash: 0,
        count: 0,
    };

    /// Places `kind` for a marker at tile `(tx, ty)`: walkers stand on the tile's
    /// bottom edge, fliers and pickups are centred in it.
    pub fn spawn(kind: ActorKind, tx: i32, ty: i32) -> Actor {
        let (w, h) = kind.size();
        let body = if kind.flies() && kind != ActorKind::Checkpoint {
            let x = tx * TILE + TILE / 2 - w / 2;
            let y = ty * TILE + TILE / 2 - h / 2;
            Body {
                x_fx: x * FX,
                y_fx: y * FX,
                w,
                h,
                ..Body::default()
            }
        } else {
            Body::standing_on_tile(tx, ty, w, h)
        };
        Actor {
            kind,
            body,
            home: (body.x(), body.y()),
            hp: kind.max_hp(),
            alive: true,
            ..Actor::EMPTY
        }
    }

    pub fn rect(&self) -> Rect {
        self.body.rect()
    }

    /// Back to the spawn point, asleep, at full health, unless it was killed or
    /// collected. Used when the player respawns. Checkpoints keep their lit state.
    pub fn reset(&mut self) {
        if self.killed {
            return;
        }
        let (x, y) = self.home;
        let (w, h) = self.kind.size();
        self.body = Body {
            x_fx: x * FX,
            y_fx: y * FX,
            w,
            h,
            ..Body::default()
        };
        self.hp = self.kind.max_hp();
        self.alive = true;
        self.awake = false;
        self.facing = -1;
        self.timer = 0;
        self.flash = 0;
        self.count = 0;
        if !self.kind.keeps_state() {
            self.state = state::IDLE;
        }
    }

    /// Takes damage; returns true when this hit finished it.
    pub fn damage(&mut self, amount: i32) -> bool {
        self.hp -= amount;
        self.flash = 6;
        if self.hp <= 0 {
            self.alive = false;
            self.killed = true;
            true
        } else {
            false
        }
    }

    /// Whether the wassie can hurt or be hurt by this actor right now.
    pub fn is_threat(&self) -> bool {
        self.alive && self.awake && self.kind.is_enemy()
    }
}

/// What an enemy can see and touch during its update.
pub struct Ctx<'a> {
    pub level: &'a Level,
    pub player: Rect,
    pub shots: &'a mut [Shot; MAX_SHOTS],
    pub difficulty: Difficulty,
    pub camera_x: i32,
}

impl Actor {
    pub fn update(&mut self, ctx: &mut Ctx) {
        self.flash = self.flash.saturating_sub(1);
        self.timer = self.timer.saturating_sub(1);
        match self.kind {
            ActorKind::Sentry => self.sentry(ctx),
            ActorKind::Crawler => self.crawler(ctx),
            ActorKind::Drone => self.drone(ctx),
            ActorKind::Mine => self.mine(ctx),
            ActorKind::Dealer => self.dealer(ctx),
            ActorKind::Guard => self.guard(ctx),
            ActorKind::Diamond
            | ActorKind::Heal
            | ActorKind::Checkpoint
            | ActorKind::Coin
            | ActorKind::Part
            | ActorKind::Terminal => {}
        }
    }

    fn fall(&mut self, level: &Level) -> crate::physics::Contacts {
        self.body.vy_fx = (self.body.vy_fx + GRAVITY_FX).min(4 * FX);
        self.body.step(level)
    }

    /// Whether the player is roughly level with this actor and within `range` pixels.
    fn in_line(&self, player: &Rect, range: i32, slack: i32) -> bool {
        let r = self.rect();
        (player.centre_x() - r.centre_x()).abs() <= range
            && (player.centre_y() - r.centre_y()).abs() <= slack
    }

    fn face_player(&mut self, player: &Rect) {
        self.facing = if player.centre_x() < self.rect().centre_x() { -1 } else { 1 };
    }

    fn sentry(&mut self, ctx: &mut Ctx) {
        match self.state {
            state::AIM => {
                self.body.vx_fx = 0;
                self.fall(ctx.level);
                if self.timer == 0 {
                    let r = self.rect();
                    let muzzle = r.centre_x() + self.facing * 10;
                    shot::spawn(ctx.shots, ShotKind::Bullet, muzzle, r.y + 9, self.facing * 3 * FX / 2, 0);
                    self.state = state::IDLE;
                    self.timer = ctx.difficulty.fire_interval(110);
                }
            }
            _ => {
                self.body.vx_fx = self.facing * FX / 2;
                let contacts = self.fall(ctx.level);
                let r = self.rect();
                let front = if self.facing > 0 { r.right() } else { r.x - 1 };
                let ledge = contacts.ground
                    && !ctx.level.is_floor(front.div_euclid(TILE), r.bottom().div_euclid(TILE));
                if contacts.wall() || ledge {
                    self.facing = -self.facing;
                }
                if self.timer == 0 && self.in_line(&ctx.player, 120, 20) {
                    self.face_player(&ctx.player);
                    self.state = state::AIM;
                    self.timer = 20;
                }
            }
        }
    }

    fn crawler(&mut self, ctx: &mut Ctx) {
        match self.state {
            state::IDLE => {
                self.body.vx_fx = 0;
                if self.timer == 0 && self.in_line(&ctx.player, 56, 24) {
                    self.face_player(&ctx.player);
                    self.state = state::AIM;
                    self.timer = 24;
                }
            }
            state::AIM if self.timer == 0 => {
                self.state = state::LUNGE;
                self.timer = 18;
            }
            state::AIM => {}
            state::LUNGE => {
                self.body.vx_fx = self.facing * 5 * FX / 2;
                if self.timer == 0 {
                    self.state = state::RETURN;
                }
            }
            _ => {
                let dx = self.home.0 - self.body.x();
                self.body.vx_fx = dx.signum() * FX / 2;
                if dx == 0 {
                    self.state = state::IDLE;
                    self.timer = 60;
                }
            }
        }
        let contacts = self.fall(ctx.level);
        if contacts.wall() && self.state == state::LUNGE {
            self.state = state::RETURN;
        }
    }

    fn drone(&mut self, ctx: &mut Ctx) {
        let r = self.rect();
        let dx = ctx.player.centre_x() - r.centre_x();
        self.facing = if dx < 0 { -1 } else { 1 };
        self.body.vx_fx = (dx * 8).clamp(-3 * FX / 4, 3 * FX / 4);
        // A slow triangle-wave bob around the spawn height.
        self.state = self.state.wrapping_add(1);
        let phase = (self.state as i32 % 64 - 32).abs() - 16;
        self.body.x_fx += self.body.vx_fx;
        self.body.y_fx = (self.home.1 + phase / 2) * FX;
        if self.timer == 0 && dx.abs() < 24 {
            shot::spawn(ctx.shots, ShotKind::Bomb, r.centre_x(), r.bottom() + 4, 0, FX);
            self.timer = ctx.difficulty.fire_interval(150);
        }
    }

    fn mine(&mut self, ctx: &mut Ctx) {
        self.body.vx_fx = -FX;
        let contacts = self.fall(ctx.level);
        if contacts.ground {
            self.body.vy_fx = -2 * FX;
        }
        if contacts.wall() || self.body.x() < ctx.camera_x - 32 {
            self.alive = false;
        }
    }

    /// Paces a short beat by the tables; when the wassie is level and in range, winds
    /// up, then deals three cards in a fan.
    fn dealer(&mut self, ctx: &mut Ctx) {
        match self.state {
            state::AIM => {
                self.body.vx_fx = 0;
                self.fall(ctx.level);
                if self.timer == 0 {
                    let r = self.rect();
                    let x = r.centre_x() + self.facing * 8;
                    for vy in [-FX / 2, 0, FX / 2] {
                        shot::spawn(ctx.shots, ShotKind::Card, x, r.y + 10, self.facing * 2 * FX, vy);
                    }
                    self.state = state::IDLE;
                    self.timer = ctx.difficulty.fire_interval(130);
                }
            }
            _ => {
                let off = self.body.x() - self.home.0;
                if (off >= 20 && self.facing > 0) || (off <= -20 && self.facing < 0) {
                    self.facing = -self.facing;
                }
                self.body.vx_fx = self.facing * FX / 3;
                let contacts = self.fall(ctx.level);
                let r = self.rect();
                let front = if self.facing > 0 { r.right() } else { r.x - 1 };
                let ledge = contacts.ground
                    && !ctx.level.is_floor(front.div_euclid(TILE), r.bottom().div_euclid(TILE));
                if contacts.wall() || ledge {
                    self.facing = -self.facing;
                }
                if self.timer == 0 && self.in_line(&ctx.player, 128, 40) {
                    self.face_player(&ctx.player);
                    self.state = state::AIM;
                    self.timer = 28;
                }
            }
        }
    }

    /// Walks up to the wassie and stops at arm's length, then fires a three-round
    /// burst after a tell.
    fn guard(&mut self, ctx: &mut Ctx) {
        match self.state {
            state::AIM => {
                self.body.vx_fx = 0;
                if self.timer == 0 {
                    self.state = state::LUNGE;
                    self.count = 3;
                }
            }
            state::LUNGE => {
                self.body.vx_fx = 0;
                if self.timer == 0 {
                    let r = self.rect();
                    let muzzle = r.centre_x() + self.facing * 11;
                    shot::spawn(ctx.shots, ShotKind::Bullet, muzzle, r.y + 11, self.facing * 2 * FX, 0);
                    self.count = self.count.saturating_sub(1);
                    self.timer = 8;
                    if self.count == 0 {
                        self.state = state::IDLE;
                        self.timer = ctx.difficulty.fire_interval(100);
                    }
                }
            }
            _ => {
                let dx = ctx.player.centre_x() - self.rect().centre_x();
                let near = self.in_line(&ctx.player, 150, 30);
                if near {
                    self.face_player(&ctx.player);
                }
                self.body.vx_fx = if near && dx.abs() > 56 { self.facing * 3 * FX / 5 } else { 0 };
                if self.timer == 0 && self.in_line(&ctx.player, 120, 24) {
                    self.state = state::AIM;
                    self.timer = 24;
                }
            }
        }
        let contacts = self.fall(ctx.level);
        let r = self.rect();
        let front = if self.facing > 0 { r.right() } else { r.x - 1 };
        let ledge = contacts.ground && !ctx.level.is_floor(front.div_euclid(TILE), r.bottom().div_euclid(TILE));
        if ledge {
            // Never walk off the floor it guards.
            self.body.x_fx -= self.facing * FX;
        }
    }
}
