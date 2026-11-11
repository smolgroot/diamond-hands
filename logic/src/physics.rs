//! Axis-aligned boxes moving through the tile map, one axis at a time.
//!
//! Speeds stay below one tile per frame, so checking the tiles under the leading edge
//! after each move is enough to never tunnel.
//!
//! Slopes are floors under a body's centre only: where the centre column crosses a
//! slope, the body stands on its floor whatever its edges overhang. Walking up one, the
//! leading foot sinks a few pixels into the rock the ramp is cut from, so walls are only
//! looked for above that. Walking down one, a body that was standing is pulled down
//! onto it rather than stepping off into the air.

use crate::{FX, TILE, level::Level, level::Tile};

/// How far over the feet a slope's floor may be and still be stepped up onto, in
/// pixels: more than a frame's climb at the fastest walk or lunge.
const SLOPE_CLIMB: i32 = 6;
/// How far a standing body is pulled down onto the floor below it, walking down a
/// slope or off its foot.
const SLOPE_SNAP: i32 = 4;
/// On a slope, how many pixels up from the feet are not checked for walls. Less than
/// a tile, so a step a tile high is still a wall.
const SLOPE_STEP: i32 = 7;

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
    /// Touching energy spikes after the move.
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
        let standing = self.vy_fx >= 0 && self.standing(level);
        self.move_x(level, &mut contacts);
        self.move_y(level, &mut contacts, standing);
        let r = self.rect().inset(1);
        contacts.spike = tiles_in(&r).any(|(tx, ty)| level.tile(tx, ty) == Tile::Spike);
        contacts
    }

    fn move_x(&mut self, level: &Level, contacts: &mut Contacts) {
        if self.vx_fx == 0 {
            return;
        }
        // Coming off the top of a slope, the feet may still be a pixel into the floor
        // ahead: the slope counts until the move is done.
        let was_on_slope = level.slope_floor(self.rect().centre_x(), self.bottom()).is_some();
        self.x_fx += self.vx_fx;
        let r = self.rect();
        let on_slope = was_on_slope || level.slope_floor(r.centre_x(), r.bottom()).is_some();
        let low = if on_slope { (r.bottom() - 1 - SLOPE_STEP).max(r.y) } else { r.bottom() - 1 };
        let rows = r.y.div_euclid(TILE)..=low.div_euclid(TILE);
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

    fn move_y(&mut self, level: &Level, contacts: &mut Contacts, standing: bool) {
        let old_bottom = self.bottom();
        self.y_fx += self.vy_fx;
        let r = self.rect();
        let cols = r.x.div_euclid(TILE)..=(r.right() - 1).div_euclid(TILE);
        if self.vy_fx >= 0 {
            let reach = if standing { SLOPE_SNAP } else { 0 };
            let floor = if let Some(y) = level.slope_floor(r.centre_x(), r.bottom()) {
                // Over a slope, only its floor counts.
                (y >= old_bottom.min(r.bottom()) - SLOPE_CLIMB && y <= r.bottom() + reach).then_some(y)
            } else {
                let floor_in_row = |ty: i32| {
                    cols.clone().any(|tx| match level.tile(tx, ty) {
                        Tile::Solid => true,
                        Tile::OneWay => old_bottom <= ty * TILE,
                        _ => false,
                    })
                };
                // Sunk into a floor tile, or feet resting exactly on one: a body
                // standing still only sinks a fraction of a pixel per frame under
                // gravity. Off the foot of a slope, the floor may be a little lower.
                let ty = (r.bottom() - 1).div_euclid(TILE);
                let next = (r.bottom() + TILE - 1).div_euclid(TILE);
                if floor_in_row(ty) {
                    Some(ty * TILE)
                } else if next * TILE - r.bottom() <= reach && floor_in_row(next) {
                    Some(next * TILE)
                } else {
                    None
                }
            };
            if let Some(y) = floor {
                self.y_fx = (y - self.h) * FX;
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
            .any(|tx| matches!(level.tile(tx, ty), Tile::Solid | Tile::OneWay) && r.bottom() == ty * TILE)
    }

    /// Whether the body stands on something: a slope's floor under its centre, to the
    /// pixel, or else flat floor directly under it.
    pub fn standing(&self, level: &Level) -> bool {
        let r = self.rect();
        match level.slope_floor(r.centre_x(), r.bottom()) {
            Some(y) => (y - r.bottom()).abs() <= 1,
            None => self.on_floor(level),
        }
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

    const ROOM: Level = Level {
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
        ..Level::BASE
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

    /// A steep hill (feet at 72, up to 48 and down again), a gentle one (up to 56 and
    /// down), then a step a tile high.
    const HILLS: Level = Level {
        rows: &[
            r"........................................",
            r"........................................",
            r"........................................",
            r"........................................",
            r"........................................",
            r"........................................",
            r"........./##\...........................",
            r"......../####\........{{##}}............",
            r"......./######\.....{{######}}.....#....",
            r"########################################",
        ],
        ..Level::BASE
    };

    /// Walks a `w` by 26 body right from column 2 at 1.5 px a frame under gravity, for
    /// `frames`. Returns each frame's feet and whether it was on the ground.
    fn walk(w: i32, frames: usize) -> (Body, [(i32, bool); 300]) {
        let mut b = Body::standing_on_tile(2, 8, w, 26);
        let mut log = [(0, false); 300];
        for entry in log.iter_mut().take(frames) {
            b.vx_fx = 3 * FX / 2;
            b.vy_fx = (b.vy_fx + 64).min(5 * FX);
            let c = b.step(&HILLS);
            *entry = (b.bottom(), c.ground);
        }
        (b, log)
    }

    #[test]
    fn walks_up_and_down_slopes_without_leaving_the_ground() {
        for w in [10, 16] {
            let (b, log) = walk(w, 300);
            assert!(log.iter().all(|&(_, ground)| ground), "width {w}: left the ground on a slope");
            let highest = log.iter().map(|&(feet, _)| feet).min().unwrap();
            assert_eq!(highest, 48, "width {w}: topped the steep hill");
            let steps = log.windows(2).map(|p| (p[1].0 - p[0].0).abs()).max().unwrap();
            assert!(steps <= 2, "width {w}: the feet jumped {steps} px in a frame");
            // Over the gentle hill and stopped by the step after it.
            let gentle = log.iter().skip(80).map(|&(feet, _)| feet).min().unwrap();
            assert_eq!(gentle, 56, "width {w}: topped the gentle hill");
            assert_eq!(b.rect().right(), 35 * TILE, "width {w}: a tile-high step is a wall");
            assert_eq!(b.bottom(), 72);
        }
    }

    #[test]
    fn lands_on_a_slope_under_its_centre() {
        // Dropped over the steep hill's left face, centred on column 8.
        let mut b = Body::standing_on_tile(8, 0, 10, 26);
        let mut landed = false;
        for _ in 0..60 {
            b.vy_fx = (b.vy_fx + 64).min(5 * FX);
            landed |= b.step(&HILLS).ground;
        }
        assert!(landed);
        let x = b.rect().centre_x();
        let floor = 8 * TILE - crate::level::Slope::Up.height(x.rem_euclid(TILE));
        assert_eq!(b.bottom(), floor, "standing on the slope's floor under its centre");
        assert!(b.standing(&HILLS));
    }

    #[test]
    fn rock_under_a_slope_is_solid_to_shots() {
        // Column 12 is the steep hill's down slope in row 6: solid below its floor.
        assert!(!HILLS.solid_px(12 * TILE + 6, 6 * TILE + 1), "air over the slope");
        assert!(HILLS.solid_px(12 * TILE + 1, 6 * TILE + 7), "rock under it");
        assert!(HILLS.solid_px(10 * TILE, 6 * TILE), "and the hill's top");
    }

    #[test]
    fn gentle_slopes_pair_up() {
        use crate::level::Slope::*;
        assert_eq!(HILLS.tile(20, 8), Tile::Slope(UpLow));
        assert_eq!(HILLS.tile(21, 8), Tile::Slope(UpHigh));
        assert_eq!(HILLS.tile(26, 7), Tile::Slope(DownHigh));
        assert_eq!(HILLS.tile(27, 7), Tile::Slope(DownLow));
        // The halves meet without a step.
        assert_eq!(UpLow.height(7), UpHigh.height(0));
        assert_eq!(DownHigh.height(7), DownLow.height(0));
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
