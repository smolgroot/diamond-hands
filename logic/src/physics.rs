//! Axis-aligned boxes moving through the tile map, one axis at a time.
//!
//! Speeds stay below one tile per frame, so checking the tiles under the leading edge
//! after each move is enough to never tunnel.

use crate::{FX, TILE, level::Level, level::Tile};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    pub fn centre_x(&self) -> i32 {
        self.x + self.w / 2
    }

    pub fn centre_y(&self) -> i32 {
        self.y + self.h / 2
    }

    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    /// Shrunk by `n` on every side.
    pub fn inset(&self, n: i32) -> Rect {
        Rect::new(self.x + n, self.y + n, self.w - 2 * n, self.h - 2 * n)
    }
}

/// What a move ran into.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Contacts {
    pub ground: bool,
    pub ceiling: bool,
    pub wall_left: bool,
    pub wall_right: bool,
    /// Touching cactus spikes after the move.
    pub spike: bool,
}

impl Contacts {
    pub fn wall(&self) -> bool {
        self.wall_left || self.wall_right
    }
}

/// A moving box: position is the top-left corner in fixed point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Body {
    pub x_fx: i32,
    pub y_fx: i32,
    pub vx_fx: i32,
    pub vy_fx: i32,
    pub w: i32,
    pub h: i32,
}

impl Body {
    /// A body whose feet stand on the bottom edge of tile `(tx, ty)`, centred on it.
    pub fn standing_on_tile(tx: i32, ty: i32, w: i32, h: i32) -> Self {
        let x = tx * TILE + TILE / 2 - w / 2;
        let y = (ty + 1) * TILE - h;
        Self {
            x_fx: x * FX,
            y_fx: y * FX,
            w,
            h,
            ..Self::default()
        }
    }

    pub fn x(&self) -> i32 {
        self.x_fx >> 8
    }

    pub fn y(&self) -> i32 {
        self.y_fx >> 8
    }

    pub fn rect(&self) -> Rect {
        Rect::new(self.x(), self.y(), self.w, self.h)
    }

    pub fn bottom(&self) -> i32 {
        self.y() + self.h
    }

    /// Moves by the velocity, stopping at walls, floors, ceilings and the tops of
    /// one-way planks. Velocity on a blocked axis is zeroed.
    pub fn step(&mut self, level: &Level) -> Contacts {
        let mut contacts = Contacts::default();
        self.move_x(level, &mut contacts);
        self.move_y(level, &mut contacts);
        let r = self.rect().inset(1);
        contacts.spike = tiles_in(&r).any(|(tx, ty)| level.tile(tx, ty) == Tile::Spike);
        contacts
    }

    fn move_x(&mut self, level: &Level, contacts: &mut Contacts) {
        if self.vx_fx == 0 {
            return;
        }
        self.x_fx += self.vx_fx;
        let r = self.rect();
        let rows = r.y.div_euclid(TILE)..=(r.bottom() - 1).div_euclid(TILE);
        if self.vx_fx > 0 {
            let tx = (r.right() - 1).div_euclid(TILE);
            if rows.clone().any(|ty| level.tile(tx, ty) == Tile::Solid) {
                self.x_fx = (tx * TILE - self.w) * FX;
                self.vx_fx = 0;
                contacts.wall_right = true;
            }
        } else {
            let tx = r.x.div_euclid(TILE);
            if rows.clone().any(|ty| level.tile(tx, ty) == Tile::Solid) {
                self.x_fx = (tx + 1) * TILE * FX;
                self.vx_fx = 0;
                contacts.wall_left = true;
            }
        }
    }

    fn move_y(&mut self, level: &Level, contacts: &mut Contacts) {
        let old_bottom = self.bottom();
        self.y_fx += self.vy_fx;
        let r = self.rect();
        let cols = r.x.div_euclid(TILE)..=(r.right() - 1).div_euclid(TILE);
        if self.vy_fx >= 0 {
            let floor_in_row = |ty: i32| {
                cols.clone().any(|tx| match level.tile(tx, ty) {
                    Tile::Solid => true,
                    Tile::OneWay => old_bottom <= ty * TILE,
                    _ => false,
                })
            };
            // Sunk into a floor tile, or feet resting exactly on one: a body standing
            // still only sinks a fraction of a pixel per frame under gravity.
            let ty = (r.bottom() - 1).div_euclid(TILE);
            let floor = if floor_in_row(ty) {
                Some(ty)
            } else if r.bottom().rem_euclid(TILE) == 0 && floor_in_row(ty + 1) {
                Some(ty + 1)
            } else {
                None
            };
            if let Some(ty) = floor {
                self.y_fx = (ty * TILE - self.h) * FX;
                self.vy_fx = 0;
                contacts.ground = true;
            }
        } else {
            let ty = r.y.div_euclid(TILE);
            if cols.clone().any(|tx| level.tile(tx, ty) == Tile::Solid) {
                self.y_fx = (ty + 1) * TILE * FX;
                self.vy_fx = 0;
                contacts.ceiling = true;
            }
        }
    }

    /// Whether there is floor directly under the body (without moving it).
    pub fn on_floor(&self, level: &Level) -> bool {
        let r = self.rect();
        let ty = r.bottom().div_euclid(TILE);
        (r.x.div_euclid(TILE)..=(r.right() - 1).div_euclid(TILE))
            .any(|tx| level.is_floor(tx, ty) && r.bottom() == ty * TILE)
    }
}

/// Tile coordinates covered by `r`.
pub fn tiles_in(r: &Rect) -> impl Iterator<Item = (i32, i32)> {
    let cols = r.x.div_euclid(TILE)..=(r.right() - 1).div_euclid(TILE);
    let rows = r.y.div_euclid(TILE)..=(r.bottom() - 1).div_euclid(TILE);
    rows.flat_map(move |ty| cols.clone().map(move |tx| (tx, ty)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::Theme;

    const ROOM: Level = Level {
        name: "TEST",
        subtitle: "",
        theme: Theme::Canyon,
        rows: &[
            "..........",
            "..........",
            "......#...",
            "..........",
            "...===....",
            "..........",
            "......^...",
            "##########",
        ],
    };

    fn body(x: i32, y: i32) -> Body {
        Body {
            x_fx: x * FX,
            y_fx: y * FX,
            w: 6,
            h: 8,
            ..Body::default()
        }
    }

    #[test]
    fn falls_onto_solid_ground() {
        let mut b = body(2, 40);
        let mut landed = false;
        for _ in 0..20 {
            b.vy_fx = 3 * FX;
            landed |= b.step(&ROOM).ground;
        }
        assert!(landed);
        assert_eq!(b.bottom(), 56);
        assert!(b.on_floor(&ROOM));
    }

    #[test]
    fn one_way_planks_hold_from_above_only() {
        let mut b = body(26, 20);
        b.vy_fx = 2 * FX;
        while !b.step(&ROOM).ground {
            b.vy_fx = 2 * FX;
        }
        assert_eq!(b.bottom(), 32, "landed on the plank");

        let mut b = body(26, 40);
        for _ in 0..10 {
            b.vy_fx = -2 * FX;
            assert!(!b.step(&ROOM).ceiling, "jumps up through the plank");
        }
        assert!(b.y() < 32);
    }

    #[test]
    fn walls_and_ceilings_stop_motion() {
        let mut b = body(30, 16);
        b.vx_fx = 3 * FX;
        let mut hit = false;
        for _ in 0..5 {
            b.vx_fx = 3 * FX;
            hit |= b.step(&ROOM).wall_right;
        }
        assert!(hit);
        assert_eq!(b.rect().right(), 48);

        let mut b = body(48, 30);
        let mut bonk = false;
        for _ in 0..5 {
            b.vy_fx = -3 * FX;
            bonk |= b.step(&ROOM).ceiling;
        }
        assert!(bonk);
        assert_eq!(b.y(), 24);
    }

    #[test]
    fn map_edges_are_walls_and_spikes_are_reported() {
        let mut b = body(1, 48);
        b.vx_fx = -2 * FX;
        assert!(b.step(&ROOM).wall_left);
        assert_eq!(b.x(), 0);

        let b2 = body(46, 48);
        let mut b2 = b2;
        assert!(b2.step(&ROOM).spike);
    }
}
