//! Everything placed on the map besides terrain: villains, pickups and checkpoints.
//! Villains are small state machines that only run while near the camera.

use crate::{
    FX, TILE,
    block::{BlockLook, Map},
    level::{Level, Terrain},
    physics::{Body, Rect},
    player::GRAVITY_FX,
    shot::{self, MAX_SHOTS, Shot, ShotKind},
    world::Difficulty,
};

/// At most 256, for the [`ActorSet`] masks a piercing beam keeps of the actors it has
/// hit and a save keeps of those taken. Every `World` holds this many, on the heap.
pub const MAX_ACTORS: usize = 192;
const _: () = assert!(MAX_ACTORS <= ActorSet::CAPACITY);

/// One bit per actor slot: what a run has taken, or the actors a piercing beam has hit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActorSet(pub [u128; 2]);

impl ActorSet {
    pub const EMPTY: ActorSet = ActorSet([0; 2]);
    pub const CAPACITY: usize = 256;

    pub const fn contains(&self, i: usize) -> bool {
        self.0[i / 128] & (1 << (i % 128)) != 0
    }

    pub fn insert(&mut self, i: usize) {
        self.0[i / 128] |= 1 << (i % 128);
    }
}

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
    /// The jetpack, lying where an explorable level hides it.
    Jetpack,
    /// The elevator's access terminal: off until the wassie presses Up at it.
    Terminal,
    /// Casino croupier bot: deals a fan of three cards.
    Dealer,
    /// Armoured security bot on the upper floors: closes in and fires bursts.
    Guard,
    /// A clay dinosaur loose in the MagicEden zoo: ambles about, roars, then charges.
    /// Once the power is back the fences hold it: it goes home and calms down.
    Dino,
    /// Someone to talk to; what they say is the level's [`Level::npcs`] entry, by
    /// their order among the people on the map.
    Npc,
    /// The Wormhole portal: stepping in ends the level.
    Portal,
    /// A gate that stays shut until the power is back.
    Gate,
    /// A blockade bot at the Wormhole terminal's door: armoured, it can't be beaten.
    Blockade,
    /// Someone locked up by a boss: speaks only once its jailer is beaten, then hands
    /// over a reward (Smolsnip, in the Big Crab Bot's lair).
    Prisoner,
    /// A crab bot on the beach and in the tunnels under it: scuttles sideways, raises its
    /// claws, then snaps forward.
    Crab,
    /// A block of sand, a crate or a clod of soil that the wassie can shoot apart: solid
    /// terrain until it breaks (see [`crate::block`]).
    Block(BlockLook),
    /// A Bad Cop bot on Ethereum City's beat: walks behind a riot shield that stops bolts
    /// from the front, and lobs canisters at the wassie. Throwing, it lowers the shield.
    Cop,
    /// A green emerald: puts a charge back in the plasma shield.
    Emerald,
    /// CL207, in the Wormhole building's Ethereum City office: someone to talk to, who
    /// hands over the plasma shield once.
    Mentor,
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
            b'J' => ActorKind::Jetpack,
            b'E' => ActorKind::Terminal,
            b'd' => ActorKind::Dealer,
            b'g' => ActorKind::Guard,
            b'x' => ActorKind::Dino,
            b'e' | b'c' => ActorKind::Npc,
            b'w' => ActorKind::Portal,
            b'h' => ActorKind::Gate,
            b'm' => ActorKind::Blockade,
            b'a' => ActorKind::Crab,
            b'f' => ActorKind::Prisoner,
            b'n' | b'u' | b'y' => ActorKind::Block(BlockLook::from_marker(c)?),
            b'z' => ActorKind::Cop,
            b'i' => ActorKind::Emerald,
            b'I' => ActorKind::Mentor,
            _ => return None,
        })
    }

    pub fn size(self) -> (i32, i32) {
        match self {
            ActorKind::Diamond => (8, 8),
            ActorKind::Heal => (10, 10),
            ActorKind::Checkpoint => (8, 24),
            ActorKind::Sentry => (16, 26),
            ActorKind::Crawler => (16, 14),
            ActorKind::Drone => (16, 10),
            ActorKind::Mine => (12, 12),
            ActorKind::Coin => (8, 8),
            ActorKind::Part => (10, 12),
            ActorKind::Jetpack => (12, 16),
            ActorKind::Terminal => (12, 20),
            ActorKind::Dealer => (14, 22),
            ActorKind::Guard => (16, 26),
            ActorKind::Dino => (20, 24),
            ActorKind::Npc => (14, 28),
            ActorKind::Portal => (16, 52),
            ActorKind::Gate => (16, 32),
            ActorKind::Blockade => (16, 26),
            ActorKind::Crab => (22, 16),
            ActorKind::Prisoner => (20, 46),
            ActorKind::Block(_) => (16, 16),
            ActorKind::Cop => (16, 26),
            ActorKind::Emerald => (10, 10),
            ActorKind::Mentor => (14, 28),
        }
    }

    /// What a beaten bot leaves behind when it doesn't just blow apart.
    pub fn wreck(self) -> Option<crate::fx::FxKind> {
        match self {
            ActorKind::Sentry => Some(crate::fx::FxKind::Wreck),
            ActorKind::Guard => Some(crate::fx::FxKind::NinjaWreck),
            ActorKind::Crab => Some(crate::fx::FxKind::CrabWreck),
            ActorKind::Cop => Some(crate::fx::FxKind::CopWreck),
            _ => None,
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
                | ActorKind::Dino
                | ActorKind::Blockade
                | ActorKind::Crab
                | ActorKind::Cop
        )
    }

    /// Gold coins a bot pays out when beaten in a casino level.
    pub fn bounty(self) -> u32 {
        match self {
            ActorKind::Guard => 3,
            ActorKind::Sentry | ActorKind::Dealer | ActorKind::Cop => 2,
            ActorKind::Crawler | ActorKind::Drone | ActorKind::Mine | ActorKind::Crab => 1,
            _ => 0,
        }
    }

    pub fn max_hp(self) -> i32 {
        match self {
            ActorKind::Guard => 6,
            ActorKind::Dino | ActorKind::Cop => 4,
            ActorKind::Block(look) => look.hp(),
            ActorKind::Sentry | ActorKind::Dealer | ActorKind::Crab => 3,
            ActorKind::Crawler | ActorKind::Drone => 2,
            _ => 1,
        }
    }

    pub fn contact_damage(self) -> i32 {
        match self {
            ActorKind::Sentry
            | ActorKind::Crawler
            | ActorKind::Guard
            | ActorKind::Dino
            | ActorKind::Blockade
            | ActorKind::Crab
            | ActorKind::Cop => 2,
            _ => 1,
        }
    }

    /// Checkpoints stay lit, terminals on and gates open through a respawn, and a
    /// saved run brings them back that way.
    pub fn keeps_state(self) -> bool {
        matches!(self, ActorKind::Checkpoint | ActorKind::Terminal | ActorKind::Gate | ActorKind::Prisoner | ActorKind::Mentor)
    }

    /// Someone to talk to: their lines are the level's [`Level::npcs`], in map order.
    pub fn is_person(self) -> bool {
        matches!(self, ActorKind::Npc | ActorKind::Prisoner | ActorKind::Mentor)
    }

    /// Picked up by touching it.
    pub fn is_pickup(self) -> bool {
        matches!(self, ActorKind::Diamond | ActorKind::Heal | ActorKind::Coin | ActorKind::Part | ActorKind::Jetpack | ActorKind::Emerald)
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
                | ActorKind::Jetpack
                | ActorKind::Emerald
        )
    }
}

/// Where a sentry's cannon fires from: ahead of its centre, and up from its feet.
pub const SENTRY_MUZZLE: (i32, i32) = (17, 16);
/// Where a guard's gun fires from: ahead of its centre, and up from its feet.
pub const GUARD_MUZZLE: (i32, i32) = (18, 16);
/// Frames between the shots of a guard's burst.
pub const GUARD_BURST_GAP: u16 = 8;
/// Frames a guard holds its aim after the last shot of a burst.
pub const GUARD_RECOIL_FRAMES: u16 = 10;
/// Frames a sentry stands rocking back after a shot.
pub const SENTRY_RECOIL_FRAMES: u16 = 14;

/// Crawler and sentry behaviour states.
pub mod state {
    pub const IDLE: u8 = 0;
    pub const AIM: u8 = 1;
    pub const LUNGE: u8 = 2;
    pub const RETURN: u8 = 3;
    /// A sentry rocking back from its shot, cannon still up.
    pub const RECOIL: u8 = 4;
    /// Checkpoint flag that has been touched, a terminal switched on, or a gate open.
    pub const LIT: u8 = 1;
    /// A dino the fences hold again: home and harmless.
    pub const CALM: u8 = 5;
}

/// How long a dino roars before it charges, and how long the charge lasts.
pub const DINO_ROAR_FRAMES: u16 = 30;
pub const DINO_CHARGE_FRAMES: u16 = 36;
/// Where a blockade bot fires from, and how often.
pub const BLOCKADE_MUZZLE: (i32, i32) = (17, 16);
const BLOCKADE_EVERY: u16 = 90;
/// A crab bot: how long it holds its claws up before it snaps (the tell), how long the
/// snap lasts, and how long it backs off after.
pub const CRAB_TELL_FRAMES: u16 = 22;
pub const CRAB_SNAP_FRAMES: u16 = 16;
pub const CRAB_BACK_FRAMES: u16 = 24;
/// How near (across) and how level (up and down) the wassie must be to set it off.
const CRAB_REACH: (i32, i32) = (64, 18);
/// A Bad Cop: how long it winds up a throw (the tell: the shield comes down), how long
/// it stays open after letting go, and how far it sees across and up or down.
pub const COP_TELL_FRAMES: u16 = 26;
pub const COP_THROW_FRAMES: u16 = 18;
const COP_SIGHT: (i32, i32) = (120, 48);
/// Frames a canister is in the air before it comes down where the wassie stood.
const CANISTER_FRAMES: i32 = 40;
/// Where a cop's hand lets go of a canister: ahead of its centre, and up from its feet.
pub const COP_HAND: (i32, i32) = (6, 26);

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
        let body = if let ActorKind::Block(_) = kind {
            // The marker is the block's bottom-left tile.
            Body {
                x_fx: tx * TILE * FX,
                y_fx: ((ty + 1) * TILE - h) * FX,
                w,
                h,
                ..Body::default()
            }
        } else if kind.flies() && kind != ActorKind::Checkpoint {
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

    /// Takes damage; returns true when this hit finished it. A blockade bot's armour
    /// takes the hit and only flashes.
    pub fn damage(&mut self, amount: i32) -> bool {
        if self.kind == ActorKind::Blockade {
            self.flash = 6;
            return false;
        }
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
        self.alive && self.awake && self.kind.is_enemy() && !self.calm()
    }

    /// A dino the fences hold again.
    pub fn calm(&self) -> bool {
        self.kind == ActorKind::Dino && self.state == state::CALM
    }
}

/// What an enemy can see and touch during its update.
pub struct Ctx<'a> {
    pub level: &'a Level,
    /// The level with the blocks the wassie has broken: what villains walk on.
    pub map: Map<'a>,
    pub player: Rect,
    pub shots: &'a mut [Shot; MAX_SHOTS],
    pub difficulty: Difficulty,
    pub camera_x: i32,
    /// A terminal has brought the power back: fences hold, gates open.
    pub power: bool,
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
            ActorKind::Dino => self.dino(ctx),
            ActorKind::Blockade => self.blockade(ctx),
            ActorKind::Crab => self.crab(ctx),
            ActorKind::Cop => self.cop(ctx),
            ActorKind::Gate => {
                if ctx.power {
                    self.state = state::LIT;
                }
            }
            ActorKind::Npc | ActorKind::Prisoner | ActorKind::Portal | ActorKind::Mentor => self.face_player(&ctx.player),
            ActorKind::Diamond
            | ActorKind::Heal
            | ActorKind::Checkpoint
            | ActorKind::Coin
            | ActorKind::Part
            | ActorKind::Jetpack
            | ActorKind::Terminal
            | ActorKind::Emerald
            | ActorKind::Block(_) => {}
        }
    }

    /// Whether a Bad Cop's riot shield is up and facing a shot heading `vx_fx`: it is
    /// raised except while the cop throws, and only covers its front.
    pub fn shield_stops(&self, vx_fx: i32) -> bool {
        self.kind == ActorKind::Cop && !matches!(self.state, state::AIM | state::RECOIL) && vx_fx * self.facing < 0
    }

    fn fall(&mut self, level: &impl Terrain) -> crate::physics::Contacts {
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
                self.fall(&ctx.map);
                if self.timer == 0 {
                    let r = self.rect();
                    shot::spawn(
                        ctx.shots,
                        ShotKind::Bullet,
                        r.centre_x() + self.facing * SENTRY_MUZZLE.0,
                        r.bottom() - SENTRY_MUZZLE.1,
                        self.facing * 3 * FX / 2,
                        0,
                    );
                    self.state = state::RECOIL;
                    self.timer = SENTRY_RECOIL_FRAMES;
                }
            }
            state::RECOIL => {
                self.body.vx_fx = 0;
                self.fall(&ctx.map);
                if self.timer == 0 {
                    self.state = state::IDLE;
                    self.timer = ctx.difficulty.fire_interval(110) - SENTRY_RECOIL_FRAMES;
                }
            }
            _ => {
                self.body.vx_fx = self.facing * FX / 2;
                let contacts = self.fall(&ctx.map);
                let r = self.rect();
                let front = if self.facing > 0 { r.right() } else { r.x - 1 };
                let ledge = contacts.ground && !ctx.map.ground_ahead(front, r.bottom());
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
        let contacts = self.fall(&ctx.map);
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
        let contacts = self.fall(&ctx.map);
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
                self.fall(&ctx.map);
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
                let contacts = self.fall(&ctx.map);
                let r = self.rect();
                let front = if self.facing > 0 { r.right() } else { r.x - 1 };
                let ledge = contacts.ground && !ctx.map.ground_ahead(front, r.bottom());
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

    /// Ambles back and forth; when the wassie is level and near, stops to roar (the
    /// tell), then charges until a wall, a ledge or the time is up. With the power back
    /// it is calm: home, and harmless.
    fn dino(&mut self, ctx: &mut Ctx) {
        if ctx.power {
            if self.state != state::CALM {
                let (x, y) = self.home;
                self.body.x_fx = x * FX;
                self.body.y_fx = y * FX;
                self.state = state::CALM;
            }
            self.body.vx_fx = 0;
            self.fall(&ctx.map);
            return;
        }
        match self.state {
            state::AIM => {
                self.body.vx_fx = 0;
                if self.timer == 0 {
                    self.state = state::LUNGE;
                    self.timer = DINO_CHARGE_FRAMES;
                }
            }
            state::LUNGE => {
                self.body.vx_fx = self.facing * 9 * FX / 4;
                if self.timer == 0 {
                    self.state = state::IDLE;
                    self.timer = 70;
                }
            }
            _ => {
                self.body.vx_fx = self.facing * FX / 2;
                if self.timer == 0 && self.in_line(&ctx.player, 80, 20) {
                    self.face_player(&ctx.player);
                    self.state = state::AIM;
                    self.timer = DINO_ROAR_FRAMES;
                }
            }
        }
        let contacts = self.fall(&ctx.map);
        let r = self.rect();
        let front = if self.facing > 0 { r.right() } else { r.x - 1 };
        let ledge = contacts.ground && !ctx.map.ground_ahead(front, r.bottom());
        if contacts.wall() || ledge {
            if ledge {
                self.body.x_fx -= self.body.vx_fx;
            }
            self.facing = -self.facing;
            if self.state == state::LUNGE {
                self.state = state::IDLE;
                self.timer = 70;
            }
        }
    }

    /// Scuttles sideways along its stretch of sand, turning at walls and ledges. When
    /// the wassie is level and near, it stops and raises its claws (the tell), snaps
    /// forward, then backs off before it can snap again. It never walks off a ledge.
    fn crab(&mut self, ctx: &mut Ctx) {
        match self.state {
            state::AIM => {
                self.body.vx_fx = 0;
                if self.timer == 0 {
                    self.state = state::LUNGE;
                    self.timer = CRAB_SNAP_FRAMES;
                }
            }
            state::LUNGE => {
                self.body.vx_fx = self.facing * 2 * FX;
                if self.timer == 0 {
                    self.state = state::RETURN;
                    self.timer = CRAB_BACK_FRAMES;
                }
            }
            state::RETURN => {
                self.body.vx_fx = -self.facing * FX / 2;
                if self.timer == 0 {
                    self.state = state::IDLE;
                    self.timer = 50;
                }
            }
            _ => {
                self.body.vx_fx = self.facing * 3 * FX / 5;
                if self.timer == 0 && self.in_line(&ctx.player, CRAB_REACH.0, CRAB_REACH.1) {
                    self.face_player(&ctx.player);
                    self.state = state::AIM;
                    self.timer = CRAB_TELL_FRAMES;
                }
            }
        }
        let contacts = self.fall(&ctx.map);
        let r = self.rect();
        // The edge it is heading for: forward, or behind it while it backs off.
        let heading = if self.body.vx_fx < 0 { -1 } else { 1 };
        let front = if heading > 0 { r.right() } else { r.x - 1 };
        let ledge = contacts.ground && self.body.vx_fx != 0 && !ctx.map.ground_ahead(front, r.bottom());
        if ledge {
            self.body.x_fx -= self.body.vx_fx;
        }
        if contacts.wall() || ledge {
            match self.state {
                state::IDLE => self.facing = -self.facing,
                state::LUNGE => {
                    self.state = state::RETURN;
                    self.timer = CRAB_BACK_FRAMES;
                }
                _ => {}
            }
        }
    }

    /// Walks its beat behind the riot shield, turning at walls and ledges. When the
    /// wassie is in sight, it stops, faces it and winds up (the tell: the shield comes
    /// down), lobs a canister to come down where the wassie stands, and stays open a
    /// moment before the shield goes back up.
    fn cop(&mut self, ctx: &mut Ctx) {
        match self.state {
            state::AIM => {
                self.body.vx_fx = 0;
                if self.timer == 0 {
                    let r = self.rect();
                    let (x, y) = (r.centre_x() + self.facing * COP_HAND.0, r.bottom() - COP_HAND.1);
                    let (tx, ty) = (ctx.player.centre_x(), ctx.player.centre_y());
                    let n = CANISTER_FRAMES;
                    let vx = ((tx - x) * FX / n).clamp(-5 * FX / 2, 5 * FX / 2);
                    let vy = (ty - y) * FX / n - shot::GRAVITY_FX_SHOT * n / 2;
                    shot::spawn(ctx.shots, ShotKind::Canister, x, y, vx, vy);
                    self.state = state::RECOIL;
                    self.timer = COP_THROW_FRAMES;
                }
            }
            state::RECOIL => {
                self.body.vx_fx = 0;
                if self.timer == 0 {
                    self.state = state::IDLE;
                    self.timer = ctx.difficulty.fire_interval(120);
                }
            }
            _ => {
                self.body.vx_fx = self.facing * FX / 2;
                if self.timer == 0 && self.in_line(&ctx.player, COP_SIGHT.0, COP_SIGHT.1) {
                    self.face_player(&ctx.player);
                    self.state = state::AIM;
                    self.timer = COP_TELL_FRAMES;
                }
            }
        }
        let contacts = self.fall(&ctx.map);
        let r = self.rect();
        let front = if self.facing > 0 { r.right() } else { r.x - 1 };
        let ledge = contacts.ground && self.body.vx_fx != 0 && !ctx.map.ground_ahead(front, r.bottom());
        if ledge {
            self.body.x_fx -= self.body.vx_fx;
        }
        if (contacts.wall() || ledge) && self.state == state::IDLE {
            self.facing = -self.facing;
        }
    }

    /// Holds its post at the door, facing the wassie, and fires a round now and then.
    fn blockade(&mut self, ctx: &mut Ctx) {
        self.body.vx_fx = 0;
        self.fall(&ctx.map);
        self.face_player(&ctx.player);
        if self.timer == 0 && self.in_line(&ctx.player, 120, 24) {
            let r = self.rect();
            shot::spawn(
                ctx.shots,
                ShotKind::Bullet,
                r.centre_x() + self.facing * BLOCKADE_MUZZLE.0,
                r.bottom() - BLOCKADE_MUZZLE.1,
                self.facing * 3 * FX / 2,
                0,
            );
            self.timer = ctx.difficulty.fire_interval(BLOCKADE_EVERY);
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
                    shot::spawn(
                        ctx.shots,
                        ShotKind::Bullet,
                        r.centre_x() + self.facing * GUARD_MUZZLE.0,
                        r.bottom() - GUARD_MUZZLE.1,
                        self.facing * 2 * FX,
                        0,
                    );
                    self.count = self.count.saturating_sub(1);
                    self.timer = GUARD_BURST_GAP;
                    if self.count == 0 {
                        self.state = state::RECOIL;
                        self.timer = GUARD_RECOIL_FRAMES;
                    }
                }
            }
            state::RECOIL => {
                self.body.vx_fx = 0;
                if self.timer == 0 {
                    self.state = state::IDLE;
                    self.timer = ctx.difficulty.fire_interval(100) - GUARD_RECOIL_FRAMES;
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
        let contacts = self.fall(&ctx.map);
        let r = self.rect();
        let front = if self.facing > 0 { r.right() } else { r.x - 1 };
        let ledge = contacts.ground && !ctx.map.ground_ahead(front, r.bottom());
        if ledge {
            // Never walk off the floor it guards.
            self.body.x_fx -= self.facing * FX;
        }
    }
}
