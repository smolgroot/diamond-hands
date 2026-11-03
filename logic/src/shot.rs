//! Projectiles: the wassie's blaster bolts and charged beams, and everything the bad
//! guys throw back.

use crate::{FX, level::Level, level::Tile, physics::Rect};

pub const MAX_SHOTS: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ShotKind {
    /// Tap B: a quick laser bolt.
    Bolt,
    /// Hold B and release: a fat piercing laser.
    Beam,
    /// Bandit and sheriff revolver rounds.
    Bullet,
    /// Dropped by FUD vultures; falls.
    Dust,
    /// Runs along the ground from the sheriff's slam.
    Shockwave,
    /// Red candles raining down in the sheriff's last phase.
    Candle,
}

impl ShotKind {
    pub fn from_player(self) -> bool {
        matches!(self, ShotKind::Bolt | ShotKind::Beam)
    }

    pub fn size(self) -> (i32, i32) {
        match self {
            ShotKind::Bolt => (8, 4),
            ShotKind::Beam => (24, 8),
            ShotKind::Bullet => (6, 4),
            ShotKind::Dust => (8, 8),
            ShotKind::Shockwave => (10, 12),
            ShotKind::Candle => (6, 12),
        }
    }

    pub fn damage(self) -> i32 {
        match self {
            ShotKind::Bolt | ShotKind::Bullet | ShotKind::Dust => 1,
            ShotKind::Shockwave | ShotKind::Candle => 2,
            ShotKind::Beam => 3,
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

    pub fn rect(&self) -> Rect {
        let (w, h) = self.kind.size();
        Rect::new(self.x_fx >> 8, self.y_fx >> 8, w, h)
    }

    /// Moves one frame; fizzles on running out of life or into rock.
    pub fn update(&mut self, level: &Level) {
        if self.kind == ShotKind::Candle || self.kind == ShotKind::Dust {
            self.vy_fx = (self.vy_fx + 24).min(3 * FX);
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
    let (w, h) = kind.size();
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
