//! Projectiles: the wassie's blaster bolts and charged beams, and everything the bad
//! guys throw back.

use crate::{FX, level::Level, level::Tile, physics::Rect};

pub const MAX_SHOTS: usize = 24;
/// Gravity on a thrown cucumber, per frame.
pub const GRAVITY_FX_SHOT: i32 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ShotKind {
    /// Tap B: a quick laser bolt.
    Bolt,
    /// Hold B and release: a fat piercing laser.
    Beam,
    /// Red plasma rounds from sentries and the mech.
    Bullet,
    /// EMP bombs dropped by drones; they fall.
    Bomb,
    /// Runs along the ground from the mech's slam.
    Shockwave,
    /// Missiles raining down in the mech's last phase.
    Missile,
    /// Playing cards a Dealer deals.
    Card,
    /// SAM's thrown cucumber: it arcs under gravity.
    Cucumber,
    /// CUCUMBER HIT: SAM's giant cucumber, fast and flat, knocking the wassie flying.
    MegaCucumber,
    /// MARKET CRASH: red candles dropping out of the chart.
    Candle,
    /// MARKET CRASH: magenta energy off SAM's exchange implants.
    Orb,
}

impl ShotKind {
    pub fn from_player(self) -> bool {
        matches!(self, ShotKind::Bolt | ShotKind::Beam)
    }

    /// Hitbox size; the wassie's lasers are given heading right (see [`Shot::size`]).
    pub fn size(self) -> (i32, i32) {
        match self {
            ShotKind::Bolt => (8, 4),
            ShotKind::Beam => (24, 8),
            ShotKind::Bullet => (6, 4),
            ShotKind::Bomb => (8, 8),
            ShotKind::Shockwave => (10, 12),
            ShotKind::Missile => (6, 12),
            ShotKind::Card => (6, 6),
            ShotKind::Cucumber => (10, 6),
            ShotKind::MegaCucumber => (24, 10),
            ShotKind::Candle => (6, 14),
            ShotKind::Orb => (6, 6),
        }
    }

    pub fn damage(self) -> i32 {
        match self {
            ShotKind::Bolt | ShotKind::Bullet | ShotKind::Bomb | ShotKind::Card | ShotKind::Orb => 1,
            ShotKind::Shockwave | ShotKind::Missile | ShotKind::Cucumber | ShotKind::Candle => 2,
            ShotKind::Beam | ShotKind::MegaCucumber => 3,
        }
    }

    /// Frames before the shot fizzles.
    fn life(self) -> u16 {
        match self {
            ShotKind::Bolt => 45,
            ShotKind::Beam => 50,
            _ => 240,
        }
    }

    /// Knocks the wassie back much harder than an ordinary hit.
    pub fn heavy(self) -> bool {
        self == ShotKind::MegaCucumber
    }

    /// Falls under gravity: per-frame increase in downward speed, and its limit.
    fn gravity(self) -> Option<(i32, i32)> {
        match self {
            ShotKind::Missile | ShotKind::Bomb => Some((24, 3 * FX)),
            ShotKind::Cucumber => Some((GRAVITY_FX_SHOT, 4 * FX)),
            ShotKind::Candle => Some((10, 2 * FX)),
            _ => None,
        }
    }

    /// Passes through enemies instead of stopping at the first.
    pub fn pierces(self) -> bool {
        self == ShotKind::Beam
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shot {
    pub kind: ShotKind,
    pub x_fx: i32,
    pub y_fx: i32,
    pub vx_fx: i32,
    pub vy_fx: i32,
    pub life: u16,
    pub active: bool,
    /// Actors already hit by a piercing beam (bit = actor index), so it hits each once.
    pub hits: u128,
    pub hit_boss: bool,
}

impl Shot {
    pub const EMPTY: Shot = Shot {
        kind: ShotKind::Bolt,
        x_fx: 0,
        y_fx: 0,
        vx_fx: 0,
        vy_fx: 0,
        life: 0,
        active: false,
        hits: 0,
        hit_boss: false,
    };

    /// The hitbox follows the heading: the wassie's lasers stand on end when fired up or
    /// down, and are square on the diagonals.
    pub fn size(kind: ShotKind, vx_fx: i32, vy_fx: i32) -> (i32, i32) {
        let (w, h) = kind.size();
        if !kind.from_player() || vy_fx == 0 {
            (w, h)
        } else if vx_fx == 0 {
            (h, w)
        } else {
            ((w + h) / 2, (w + h) / 2)
        }
    }

    pub fn rect(&self) -> Rect {
        let (w, h) = Shot::size(self.kind, self.vx_fx, self.vy_fx);
        Rect::new(self.x_fx >> 8, self.y_fx >> 8, w, h)
    }

    /// Moves one frame; fizzles on running out of life or into rock.
    pub fn update(&mut self, level: &Level) {
        if let Some((g, max)) = self.kind.gravity() {
            self.vy_fx = (self.vy_fx + g).min(max);
        }
        self.x_fx += self.vx_fx;
        self.y_fx += self.vy_fx;
        self.life = self.life.saturating_sub(1);
        let r = self.rect();
        let blocked = level.tile_at_px(r.centre_x(), r.centre_y()) == Tile::Solid;
        let below_map = r.y > level.px_height();
        if self.life == 0 || blocked || below_map {
            self.active = false;
        }
    }
}

/// Fires a shot centred on `(cx, cy)`. Returns false when the pool is full.
pub fn spawn(
    pool: &mut [Shot; MAX_SHOTS],
    kind: ShotKind,
    cx: i32,
    cy: i32,
    vx_fx: i32,
    vy_fx: i32,
) -> bool {
    let Some(slot) = pool.iter_mut().find(|s| !s.active) else {
        return false;
    };
    let (w, h) = Shot::size(kind, vx_fx, vy_fx);
    *slot = Shot {
        kind,
        x_fx: (cx - w / 2) * FX,
        y_fx: (cy - h / 2) * FX,
        vx_fx,
        vy_fx,
        life: kind.life(),
        active: true,
        hits: 0,
        hit_boss: false,
    };
    true
}
