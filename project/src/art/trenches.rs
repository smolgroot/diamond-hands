//! Level 2, the Solana Trenches: the memecoin casino. A neon Vegas strip under a giant
//! gold coin, the gaming floor (felt tables, slot banks, trading screens), the exchange
//! suites and SAM's rooftop up the tower, and below ground the back room (slot banks and
//! crypto ATMs) and the sewers (brickwork, outflow pipes, graffiti).
//!
//! Each stratum of the map (see `Level::strata`) has its own terrain palette, chosen
//! per cell, so a shaft between two shows both in their own light: the street storey
//! in [`TERRAIN_SLOTS`]`[0]` (switched with the stage), the back room in `[1]` and the
//! sewers in `[2]`. Scenery pictures use a muted copy of their stratum's palette
//! ([`SCENERY_SLOTS`]), so they sit back behind the wassie and the bots. The sewers
//! draw rock as brick and girders as rusty catwalks.
//!
//! Like the Badlands, the terrain is built from 8x8 cells keyed by what a cell is and
//! what is around it ([`terrain_key`]), so each distinct cell is drawn once. Pictures
//! bigger than a cell (see `level::DECOR`) key each of their cells by its place in the
//! picture. Animation is all palette: the marquee bulbs chase by swapping two colours,
//! the fence emitters switch with the fence cycle, and the screens go red when the market
//! crashes.

use agb::display::{Palette16, Rgb15, tiled::RegularBackground};
use diamond_hands_logic::{
    Level, TILE, Tile,
    level::{self, Slope},
};

use super::{SCENERY_MUTE, TileCache, mute_palette, rgb};
use crate::font;

/// Terrain (BG palette 1) indices for the casino.
mod gx {
    pub const OUTLINE: u8 = 1;
    pub const GOLD_LIGHT: u8 = 2;
    pub const GOLD: u8 = 3;
    pub const GOLD_DARK: u8 = 4;
    pub const WALL_LIGHT: u8 = 5;
    pub const WALL: u8 = 6;
    pub const WALL_DARK: u8 = 7;
    pub const FELT_LIGHT: u8 = 8;
    pub const FELT: u8 = 9;
    pub const PINK: u8 = 10;
    pub const CYAN: u8 = 11;
    pub const SCREEN: u8 = 12;
    /// Marquee bulbs: the two sets swap colours to chase.
    pub const BULB_A: u8 = 13;
    pub const BULB_B: u8 = 14;
    /// Fence emitter nodes, switched with the fence cycle.
    pub const ENERGY: u8 = 15;
}

type Colour = (u16, u16, u16);

/// Background palette slots for each stratum's terrain, and for its scenery.
pub const TERRAIN_SLOTS: [u8; 3] = [1, 4, 5];
pub const SCENERY_SLOTS: [u8; 3] = [6, 7, 8];

/// Each area's terrain colours, indices 1..=12: the street, the casino floor, the
/// exchange's lobby, and the tower top.
const GROUND: [[Colour; 12]; 4] = [
    // Outline, gold (light, mid, dark), steel light, slate, slate dark, pipe light,
    // pipe, pink, cyan, screen.
    [(3, 1, 7), (31, 30, 18), (29, 22, 6), (17, 10, 3), (24, 22, 28), (7, 9, 16), (3, 4, 9), (6, 22, 16), (3, 12, 11), (31, 6, 22), (6, 28, 31), (4, 2, 10)],
    [(3, 1, 6), (31, 30, 20), (30, 23, 7), (18, 11, 3), (25, 22, 29), (8, 8, 17), (3, 3, 9), (6, 22, 15), (2, 12, 10), (31, 7, 22), (8, 26, 31), (5, 2, 11)],
    [(3, 0, 3), (31, 27, 16), (28, 18, 5), (15, 8, 3), (24, 18, 24), (9, 4, 10), (4, 1, 5), (8, 20, 13), (3, 10, 8), (31, 4, 12), (10, 22, 31), (4, 1, 6)],
    [(3, 0, 4), (31, 28, 17), (29, 19, 6), (15, 8, 3), (26, 21, 28), (10, 6, 13), (4, 2, 6), (8, 22, 15), (3, 11, 9), (31, 5, 15), (10, 25, 31), (4, 1, 7)],
];

/// The back room under the casino: plum walls, old gold, green felt, hot pink.
const BACK_ROOM: [Colour; 12] = [
    (2, 0, 4), (30, 26, 14), (26, 18, 5), (14, 7, 2), (21, 15, 25), (9, 4, 13), (3, 1, 6), (5, 21, 12), (2, 10, 7), (31, 4, 18), (6, 24, 28), (4, 1, 8),
];

/// The sewers' own palette, after `.claude/mockup/level2-sewers-tiles-sprites-sheet.png`
/// (see [`sw`]): mossy green-grey brick, a steel rail with hazard orange dashes, rust,
/// toxic slime, the dark teal of the vaults behind
/// (`level2-sewers-background-tiles-sprites-sheet.png`). Indices 1..=13 (14 is spare);
/// 15 is the fences' energy.
const SEWERS: [Colour; 14] = [
    (1, 2, 3),
    (17, 20, 17),
    (11, 14, 12),
    (6, 9, 8),
    (9, 20, 5),
    (15, 31, 8),
    (27, 28, 27),
    (14, 16, 17),
    (31, 16, 3),
    (19, 10, 5),
    (4, 10, 10),
    (2, 6, 7),
    (1, 4, 5),
    (31, 27, 10),
];

/// Sewer palette indices. The fence pylon (drawn with [`gx`]'s golds, 2-4) comes out in
/// concrete.
mod sw {
    pub const OUTLINE: u8 = 1;
    pub const STONE_LIGHT: u8 = 2;
    pub const STONE: u8 = 3;
    pub const STONE_DARK: u8 = 4;
    pub const MOSS: u8 = 5;
    pub const SLIME: u8 = 6;
    pub const STEEL_LIGHT: u8 = 7;
    pub const STEEL: u8 = 8;
    pub const HAZARD: u8 = 9;
    pub const RUST: u8 = 10;
    /// The vaults behind, lit to dark.
    pub const BACK_LIGHT: u8 = 11;
    pub const BACK: u8 = 12;
    pub const BACK_DARK: u8 = 13;
}

/// The terrain palette for stratum `stratum` (0 the street storey, then the back room
/// and the sewers): its area's light (`stage`, on the street storey), the fence cycle,
/// the marquee's chase step and whether the market is crashing (the screens flash red).
pub fn stratum_palette(stratum: usize, stage: u8, fence: u8, chase: bool, crash: bool) -> Palette16 {
    let mut colours = [rgb(0, 0, 0); 16];
    let area: &[Colour] = match stratum {
        0 => &GROUND[(stage as usize).min(GROUND.len() - 1)],
        1 => &BACK_ROOM,
        _ => &SEWERS,
    };
    for (out, &(r, g, b)) in colours[1..].iter_mut().zip(area.iter()) {
        *out = rgb(r, g, b);
    }
    // Marquee bulbs up top. The sewers have no marquee: their slots are their own.
    if stratum < 2 {
        let (on, off) = (rgb(31, 29, 14), rgb(14, 7, 4));
        colours[gx::BULB_A as usize] = if chase { on } else { off };
        colours[gx::BULB_B as usize] = if chase { off } else { on };
    }
    colours[gx::ENERGY as usize] = match fence {
        0 => rgb(6, 4, 10),
        1 => rgb(8, 14, 24),
        _ => rgb(22, 30, 31),
    };
    if crash && stratum == 0 {
        colours[gx::SCREEN as usize] = rgb(18, 1, 4);
        colours[gx::FELT_LIGHT as usize] = rgb(31, 6, 8);
    }
    Palette16::new(colours)
}

/// The muted copy of a stratum's palette that its scenery is drawn in.
pub fn scenery_palette(terrain: &Palette16) -> Palette16 {
    mute_palette(terrain.clone(), SCENERY_MUTE, 1)
}

/// The palette slot for terrain cell `key` in row `ty`: its stratum's, muted for
/// scenery.
pub fn palette_slot(level: &Level, ty: i32, key: usize) -> u8 {
    let stratum = level.stratum(ty).min(TERRAIN_SLOTS.len() - 1);
    // The sewers' pictures sit on the back wall, drawn into the same tiles, so they
    // keep its palette; their art is dim enough on its own.
    if key >= DECOR_BASE && stratum < 2 { SCENERY_SLOTS[stratum] } else { TERRAIN_SLOTS[stratum] }
}

/// Backdrop (BG palette 0): the same slots as the Badlands' sky (bands 1-6, the "moon"
/// 7-9, stars 10, towers 11-13, windows 14-15).
mod sky_ix {
    pub const BANDS: i32 = 6;
    pub const SKY: [u8; 6] = [1, 2, 3, 4, 5, 6];
    pub const COIN_LIGHT: u8 = 7;
    pub const COIN: u8 = 8;
    pub const COIN_DARK: u8 = 9;
    pub const STAR: u8 = 10;
    pub const TOWER: u8 = 11;
    pub const TOWER_DARK: u8 = 12;
    pub const TOWER_EDGE: u8 = 13;
    pub const WINDOW: u8 = 14;
    pub const WINDOW_HOT: u8 = 15;
}

const fn lerp(a: Colour, b: Colour, t: u16, n: u16) -> Rgb15 {
    rgb((a.0 * (n - t) + b.0 * t) / n, (a.1 * (n - t) + b.1 * t) / n, (a.2 * (n - t) + b.2 * t) / n)
}

/// Muted ([`super::BACKDROP_MUTE`]), so the action stands out from it.
const fn sky(top: Colour, bottom: Colour, coin: [Colour; 3], star: Colour, city: [Colour; 5]) -> Palette16 {
    mute_palette(Palette16::new([
        lerp(top, bottom, 0, 5),
        lerp(top, bottom, 0, 5),
        lerp(top, bottom, 1, 5),
        lerp(top, bottom, 2, 5),
        lerp(top, bottom, 3, 5),
        lerp(top, bottom, 4, 5),
        lerp(top, bottom, 5, 5),
        rgb(coin[0].0, coin[0].1, coin[0].2),
        rgb(coin[1].0, coin[1].1, coin[1].2),
        rgb(coin[2].0, coin[2].1, coin[2].2),
        rgb(star.0, star.1, star.2),
        rgb(city[0].0, city[0].1, city[0].2),
        rgb(city[1].0, city[1].1, city[1].2),
        rgb(city[2].0, city[2].1, city[2].2),
        rgb(city[3].0, city[3].1, city[3].2),
        rgb(city[4].0, city[4].1, city[4].2),
    ]), super::BACKDROP_MUTE, 0)
}

/// The strip at night under a giant gold coin. Indoors, on the casino floor and in the
/// exchange's lobby, there is no sky: the coin and the stars take the back wall's
/// colour, and the skyline becomes dim columns and lit panels on the wall. Up the tower
/// the night is back, deeper, over the suites' windows and the rooftop.
pub static SKY_PALETTES: [Palette16; 4] = [
    sky((2, 0, 8), (24, 4, 20), [(31, 30, 16), (29, 22, 6), (18, 11, 3)], (28, 24, 31), [
        (8, 3, 16),
        (4, 1, 9),
        (24, 6, 26),
        (8, 30, 28),
        (31, 12, 26),
    ]),
    sky((5, 1, 9), (9, 2, 11), [(6, 1, 10), (6, 1, 10), (6, 1, 10)], (6, 1, 10), [
        (8, 2, 12),
        (4, 1, 7),
        (15, 5, 17),
        (26, 19, 7),
        (31, 10, 22),
    ]),
    sky((4, 0, 3), (10, 1, 5), [(5, 0, 4), (5, 0, 4), (5, 0, 4)], (5, 0, 4), [
        (8, 1, 5),
        (4, 0, 3),
        (16, 3, 8),
        (28, 8, 9),
        (31, 20, 10),
    ]),
    sky((1, 0, 6), (17, 3, 16), [(31, 30, 16), (29, 22, 6), (18, 11, 3)], (26, 22, 31), [
        (6, 2, 13),
        (3, 1, 7),
        (20, 5, 22),
        (8, 26, 26),
        (31, 10, 22),
    ]),
];

const BAYER: [[i32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
const HORIZON: i32 = 136;
const MAP_TILES: i32 = 32;
const MAP_PX: i32 = MAP_TILES * 8;
/// The giant coin over the strip: centre and radius, clear of the title text.
const COIN: (i32, i32, i32) = (206, 40, 17);

/// The sky: a dithered gradient, stars, and a giant gold coin with an S stamped on it.
pub fn paint_sky(bg: &mut RegularBackground) {
    let pixel = |x: i32, y: i32| {
        let (cx, cy, r) = COIN;
        let (dx, dy) = (x - cx, y - cy);
        let d2 = dx * dx + dy * dy;
        let bayer = BAYER[(y % 4) as usize][(x % 4) as usize];
        if d2 <= r * r {
            let rim = d2 >= (r - 2) * (r - 2);
            let inner = d2 <= (r - 4) * (r - 4) && d2 >= (r - 5) * (r - 5);
            // An S across the face: three bars joined at alternate ends.
            let (sx, sy) = (dx + 6, dy + 8);
            let s_mark = (0..12).contains(&sx)
                && (0..16).contains(&sy)
                && (matches!(sy, 0..=2 | 7..=9 | 14..=16)
                    || (sy < 7 && sx < 3)
                    || (sy > 9 && sx > 8));
            let lit = -(dx + dy) * 8 / r + (bayer - 8) / 4 > 2;
            return match () {
                _ if rim => sky_ix::COIN_DARK,
                _ if inner || s_mark => sky_ix::COIN_DARK,
                _ if lit => sky_ix::COIN_LIGHT,
                _ => sky_ix::COIN,
            };
        }
        let steps = (sky_ix::BANDS - 1) * 16;
        let t = (y * steps / HORIZON).min(steps);
        let band = (t / 16 + (bayer < t % 16) as i32).min(sky_ix::BANDS - 1);
        if y < 48 && (x * 37 + y * 101 + x * y * 7) % 131 == 0 {
            return sky_ix::STAR;
        }
        sky_ix::SKY[band as usize]
    };
    let pixel = |x: i32, y: i32| if y >= HORIZON { sky_ix::TOWER_DARK } else { pixel(x, y) };
    let mut cache = TileCache::new();
    for ty in 0..20 {
        for tx in 0..MAP_TILES {
            cache.paint(bg, (tx, ty), 0, &pixel);
        }
    }
    fill_below(bg, &mut cache);
}

/// Below the painted screen, the dark at the towers' feet carries on, for when the
/// camera goes down below the street and the backdrop drifts up: one tile, set without
/// painting it pixel by pixel.
fn fill_below(bg: &mut RegularBackground, cache: &mut TileCache) {
    if let Some(id) = cache.intern([sky_ix::TOWER_DARK; 64]) {
        let effect = agb::display::tiled::TileEffect::new(false, false, 0);
        for ty in 20..32 {
            for tx in 0..MAP_TILES {
                bg.set_tile_dynamic16((tx, ty), &cache.tiles[id], effect);
            }
        }
    }
}

/// Casino hotel towers: (left x, width, roof y, kind). Kinds: 0 a plain slab, 1 a
/// stepped crown, 2 a billboard on stilts, 3 a needle with a beacon.
const TOWERS: [(i32, i32, i32, u8); 11] = [
    (0, 22, 80, 1),
    (24, 18, 96, 2),
    (44, 14, 62, 3),
    (60, 26, 88, 0),
    (88, 20, 70, 1),
    (110, 24, 100, 2),
    (136, 12, 58, 3),
    (150, 28, 84, 1),
    (180, 16, 104, 0),
    (198, 26, 74, 2),
    (226, 30, 92, 1),
];

/// The skyline, scrolled slowly behind the terrain: slab hotels with stepped crowns,
/// lit window grids, billboards and beacon needles.
pub fn paint_city(bg: &mut RegularBackground) {
    let pixel = |x: i32, y: i32| {
        let xm = x.rem_euclid(MAP_PX);
        let Some(&(x0, w, roof, kind)) = TOWERS.iter().rev().find(|t| (t.0..t.0 + t.1).contains(&xm)) else {
            return if y >= 128 { sky_ix::TOWER_DARK } else { 0 };
        };
        let col = xm - x0;
        let top = match kind {
            // Steps in from each side, two floors at a time.
            1 => roof + (4 - (col.min(w - 1 - col) / 3).min(4)) * 4,
            3 if (w / 2 - 1..=w / 2).contains(&col) => roof - 18,
            3 => roof,
            _ => roof,
        };
        if kind == 3 && y == roof - 18 && col == w / 2 {
            return sky_ix::WINDOW_HOT;
        }
        if y < top {
            return 0;
        }
        if kind == 2 && (roof..roof + 14).contains(&y) {
            // The billboard: an edge frame round a hot face with a stripe.
            let edge = y == roof || y == roof + 13 || col == 0 || col == w - 1;
            return if edge {
                sky_ix::TOWER_EDGE
            } else if (y - roof) % 5 == 2 {
                sky_ix::WINDOW
            } else {
                sky_ix::WINDOW_HOT
            };
        }
        if kind == 2 && (roof + 14..roof + 20).contains(&y) {
            return if col % 6 == 2 { sky_ix::TOWER_EDGE } else { 0 };
        }
        // Windows on a grid fixed to the tiles, lit by window rather than by pixel, so
        // cells repeat: background VRAM has no room for a unique tile per cell.
        let lit = ((xm / 4) * 7 + (y / 8) * 3 + kind as i32) % 5;
        if y == top || col == w - 1 {
            sky_ix::TOWER_EDGE
        } else if xm % 4 == 1 && y % 8 == 3 && lit < 2 && y > top + 2 {
            if lit == 0 && (xm / 4) % 3 == 0 { sky_ix::WINDOW_HOT } else { sky_ix::WINDOW }
        } else if y > 122 || col == 0 {
            sky_ix::TOWER_DARK
        } else {
            sky_ix::TOWER
        }
    };
    let mut cache = TileCache::new();
    for ty in 0..20 {
        for tx in 0..MAP_TILES {
            cache.paint(bg, (tx, ty), 0, &pixel);
        }
    }
    fill_below(bg, &mut cache);
}

// Terrain keys. 0 is an empty cell. Then each look (the casino's, the sewers') has its
// own run of keys: walls by false or not, depth, open sides and variant; one girder;
// one fence emitter; tables by open ends and column; pedestals; slopes by shape and
// variant; and the rock under a slope, by the slope's shape and variant. Pictures come
// last and look the same everywhere: their stratum's palette colours them.
/// Variants of a cell by its place on the map: four columns by three rows.
const VARIANTS: usize = 12;
const WALL_KEYS: usize = 2 * 3 * 16 * VARIANTS;
const LEDGE: usize = WALL_KEYS;
const FENCE: usize = LEDGE + 1;
const TABLE: usize = FENCE + 1;
const PEDESTAL: usize = TABLE + 16;
const SLOPES: usize = PEDESTAL + 4;
const UNDER_SLOPE: usize = SLOPES + Slope::ALL.len() * VARIANTS;
const LOOK_KEYS: usize = UNDER_SLOPE + Slope::ALL.len() * VARIANTS;
/// The casino's look (on the street storey and in the back room) and the sewers'.
const LOOKS: usize = 2;
const DECOR_BASE: usize = 1 + LOOKS * LOOK_KEYS;
/// Cells a picture may have: up to 8 across and 8 down.
const DECOR_CELLS: usize = 64;
/// Number of [`terrain_key`] values that aren't drawn over the sewers' back wall. Those
/// that are come after, one run of these per back wall cell, and are sparse: only a few
/// of them are on the map.
pub const TERRAIN_KEYS: usize = DECOR_BASE + level::DECOR.len() * DECOR_CELLS;

/// The sewers' back wall, after `.claude/mockup/level2-sewers-background-tiles-sprites-
/// sheet.png`: vaults on pillars along the corridor, a round grate in each bay, a ledge,
/// and dark water at the foot, all in dim teal. It fills the corridor from its ceiling
/// down to the floor at this row (see `logic/src/levels/trenches.rs`); above, in the
/// shafts, it is plain brick, and below the floor (the sunken channel) water.
const SEWER_FLOOR: i32 = 80;
/// The corridor's height in cells, and a bay's width (a pillar to the next) in pixels.
const VAULT_ROWS: i32 = 14;
const BAY: i32 = 64;
const BAY_CELLS: i32 = BAY / 8;

/// The key of `own` (a cell's key on its own, 0 for nothing) drawn over the back wall
/// at `(tx, ty)`.
fn back_key(own: usize, tx: i32, ty: i32) -> usize {
    let row = (ty - (SEWER_FLOOR - VAULT_ROWS)).clamp(-1, VAULT_ROWS) + 1;
    let cell = row * BAY_CELLS + (tx & (BAY_CELLS - 1));
    own + TERRAIN_KEYS * (1 + cell as usize)
}

/// A pixel of the back wall: `row` is 0 for the brick above the corridor, then the
/// corridor's rows, then the water under the floor; `x` across a bay (a pillar at 0).
fn back_pixel(x: i32, row: i32, y: i32) -> u8 {
    use sw::*;
    let brick = |x: i32, y: i32| {
        let course = y.div_euclid(4);
        if y.rem_euclid(4) == 3 || (x + course.rem_euclid(2) * 4).rem_euclid(8) == 7 { OUTLINE } else { BACK_DARK }
    };
    if row == 0 {
        return brick(x, y);
    }
    let water = |x: i32, y: i32| {
        // Dark water, and the pillars and grates catching in it in broken streaks.
        let d = (x - if x >= BAY / 2 { BAY } else { 0 }).abs();
        let streak = y % 3 == 1 && ((d <= 3 && (x + y) % 5 != 0) || ((x - BAY / 2).abs() <= 5 - y / 6 && (x * 3 + y) % 4 != 0));
        if streak { if y % 6 == 1 { BACK_LIGHT } else { BACK } } else { BACK_DARK }
    };
    let y = (row - 1) * 8 + y;
    if row > VAULT_ROWS {
        return water(x, y);
    }
    // Top to bottom: the vaults, the pillars' capitals, the grates, a ledge, brick, water.
    let (spring, ledge, wet) = (38, 84, 100);
    if y >= wet {
        return if y == wet { OUTLINE } else { water(x, y - wet) };
    }
    let d = (x - if x >= BAY / 2 { BAY } else { 0 }).abs();
    if y >= ledge {
        return match y - ledge {
            0 => BACK_LIGHT,
            1 | 2 => BACK,
            3 => OUTLINE,
            _ => brick(x, y),
        };
    }
    // The pillars: a capital under the vault's springing, a ring halfway down.
    let half = match y {
        _ if y < spring => -1,
        _ if y < spring + 3 => 6,
        _ if (spring + 24..spring + 27).contains(&y) => 5,
        _ => 4,
    };
    if d <= half {
        return if d == half || y == spring + 2 || y == spring + 26 {
            OUTLINE
        } else if y == spring || y == spring + 24 || x == BAY - half + 1 {
            BACK_LIGHT
        } else {
            BACK
        };
    }
    // The vault's ribs, springing from the capitals and meeting over the bay.
    let (cx, cy) = (BAY / 2, spring + 4);
    let (dx, dy) = (x - cx, y - cy);
    let r2 = dx * dx + dy * dy;
    let (inner, outer) = (BAY / 2 - 3, BAY / 2 + 2);
    if y < cy && r2 < outer * outer && r2 >= inner * inner {
        // Stones along the rib, its inner edge catching the light.
        let seam = (dx * 5 + dy * 7).rem_euclid(23) == 0;
        return if r2 < (inner + 1) * (inner + 1) { BACK_LIGHT } else if seam { OUTLINE } else { BACK };
    }
    if r2 >= outer * outer || y < cy - inner {
        return brick(x, y);
    }
    // Under the vault: the dark recess, faint brick low down, and a round grate.
    let (gx, gy, gr) = (BAY / 2, spring + 22, 9);
    let (ex, ey) = (x - gx, y - gy);
    let g2 = ex * ex + ey * ey;
    if g2 <= gr * gr {
        return if g2 > (gr - 2) * (gr - 2) {
            if ex + ey < 0 { BACK_LIGHT } else { BACK }
        } else if ex.rem_euclid(3) == 0 {
            BACK
        } else {
            OUTLINE
        };
    }
    if y > spring + 34 && brick(x, y) == OUTLINE { OUTLINE } else { BACK_DARK }
}

/// Map characters that look like the casino's walls.
fn looks_solid(c: u8) -> bool {
    matches!(c, b'#' | b'%')
}

/// Whether map character `c` is a slope.
fn is_slope(c: u8) -> bool {
    matches!(c, b'/' | b'\\' | b'{' | b'}')
}

/// The shape of the slope at `(tx, ty)`, if it is one.
fn slope_at(level: &Level, tx: i32, ty: i32) -> Option<Slope> {
    match level.tile(tx, ty) {
        Tile::Slope(shape) => Some(shape),
        _ => None,
    }
}

/// What a cell of the casino looks like: see the module docs. In the sewers, a cell
/// with nothing in it, or with something that doesn't fill it (a picture, a catwalk, a
/// fence, a slope), is drawn over the back wall: its key is then [`back_key`]'s.
pub fn terrain_key(level: &Level, tx: i32, ty: i32) -> usize {
    let key = own_key(level, tx, ty);
    if level.stratum(ty) < 2 {
        return key;
    }
    let off = key.wrapping_sub(1 + LOOK_KEYS);
    let open = key == 0 || key >= DECOR_BASE || matches!(off, LEDGE | FENCE) || (SLOPES..UNDER_SLOPE).contains(&off);
    if open { back_key(key, tx, ty) } else { key }
}

/// A cell's key on its own, without the back wall.
fn own_key(level: &Level, tx: i32, ty: i32) -> usize {
    let mut c = level.char_at(tx, ty);
    // Pickups hidden in a false wall are drawn behind it, so the wall stays whole.
    if !looks_solid(c) && level.char_at(tx, ty - 1) == b'%' && level.char_at(tx, ty + 1) != b'.' {
        c = b'%';
    }
    if let Some(i) = level::DECOR.iter().position(|d| d.0 == c) {
        let (_, w, h) = level::DECOR[i];
        let mut x0 = tx;
        while level.char_at(x0 - 1, ty) == c {
            x0 -= 1;
        }
        let mut y0 = ty;
        while level.char_at(tx, y0 - 1) == c {
            y0 -= 1;
        }
        let (cx, cy) = ((tx - x0) % w, (ty - y0) % h);
        return DECOR_BASE + i * DECOR_CELLS + (cy * 8 + cx) as usize;
    }
    let look = (level.stratum(ty) >= 2) as usize;
    let base = 1 + look * LOOK_KEYS;
    // Background VRAM is tight: the casino's walls keep their twelve variants, the
    // sewers' bricks two (their courses repeat every cell anyway), slopes one.
    let variant = if look == 1 { ((tx & 1) * 3) as usize } else { ((tx & 3) * 3 + ty.rem_euclid(3)) as usize };
    match c {
        b'#' | b'%' => {
            if let Some(shape) = slope_at(level, tx, ty - 1) {
                return base + UNDER_SLOPE + shape as usize * VARIANTS;
            }
            // Past the map's top and bottom counts as wall, so ceilings and the floor
            // get no trim along the screen's edges; a slope is rock under its floor.
            let solid = |dx: i32, dy: i32| {
                let (x, y) = (tx + dx, ty + dy);
                if y < 0 || y >= level.height() || x < 0 || x >= level.width() {
                    return true;
                }
                let c = level.char_at(x, y);
                looks_solid(c) || is_slope(c)
            };
            let depth = if !solid(0, -1) { 0 } else if !solid(0, -2) { 1 } else { 2 };
            let open = |dx, dy| !solid(dx, dy) as usize;
            let sides = open(0, -1) | open(-1, 0) << 1 | open(1, 0) << 2 | open(0, 1) << 3;
            base + (((c == b'%') as usize * 3 + depth) * 16 + sides) * VARIANTS + variant
        }
        b'=' => base + LEDGE,
        b'^' => base + FENCE,
        // Tables: which ends are open, and the column. Pedestals: which half, and
        // whether it's the foot.
        b'T' => {
            let ends = (level.char_at(tx - 1, ty) != b'T') as usize | ((level.char_at(tx + 1, ty) != b'T') as usize) << 1;
            base + TABLE + ends * 4 + (tx & 3) as usize
        }
        b'|' => base + PEDESTAL + ((level.char_at(tx - 1, ty) == b'|') as usize | (level.is_floor(tx, ty + 1) as usize) << 1),
        _ => match slope_at(level, tx, ty) {
            Some(shape) => base + SLOPES + shape as usize * VARIANTS,
            None => 0,
        },
    }
}

/// Pixels for a [`terrain_key`].
pub fn terrain_pixels(key: usize) -> [u8; 64] {
    let mut px = [0u8; 64];
    if key == 0 {
        return px;
    }
    if key >= TERRAIN_KEYS {
        // Something over the sewers' back wall: the wall, then what's in front.
        let (own, cell) = (key % TERRAIN_KEYS, (key / TERRAIN_KEYS - 1) as i32);
        let (row, col) = (cell / BAY_CELLS, cell % BAY_CELLS);
        for (n, p) in px.iter_mut().enumerate() {
            *p = back_pixel(col * 8 + (n % 8) as i32, row, (n / 8) as i32);
        }
        if own != 0 {
            for (p, q) in px.iter_mut().zip(terrain_pixels(own)) {
                if q != 0 {
                    *p = q;
                }
            }
        }
        return px;
    }
    if key >= DECOR_BASE {
        let i = (key - DECOR_BASE) / DECOR_CELLS;
        let cell = (key - DECOR_BASE) % DECOR_CELLS;
        let (cx, cy) = ((cell % 8) as i32, (cell / 8) as i32);
        let (letter, w, h) = level::DECOR[i];
        for (n, p) in px.iter_mut().enumerate() {
            let (x, y) = (cx * 8 + (n % 8) as i32, cy * 8 + (n / 8) as i32);
            *p = picture(letter, x, y, w * 8, h * 8);
        }
        return px;
    }
    let sewer = (key - 1) / LOOK_KEYS == 1;
    let off = (key - 1) % LOOK_KEYS;
    let variant = off % VARIANTS;
    let (col4, row3) = ((variant / 3) as i32, (variant % 3) as i32);
    for (n, p) in px.iter_mut().enumerate() {
        let (x, y) = ((n % 8) as i32, (n / 8) as i32);
        *p = match off {
            _ if off < WALL_KEYS => {
                let sides = off / VARIANTS % 16;
                let depth = off / (VARIANTS * 16) % 3;
                let cracked = off / (VARIANTS * 16 * 3) == 1;
                if sewer {
                    sewer_wall(x, y, sides, depth, col4, row3, cracked)
                } else {
                    wall(x, y, sides, depth, col4, row3, cracked)
                }
            }
            LEDGE if sewer => catwalk(x, y),
            LEDGE => ledge(x, y),
            FENCE => grid(x, y),
            _ if off < PEDESTAL => {
                let t = off - TABLE;
                table(x, y, t / 4 & 1 != 0, t / 4 & 2 != 0, (t % 4) as i32)
            }
            _ if off < SLOPES => {
                let t = off - PEDESTAL;
                pedestal(x, y, t & 1 != 0, t & 2 != 0)
            }
            _ if off < UNDER_SLOPE => slope(x, y, 0, Slope::ALL[(off - SLOPES) / VARIANTS], sewer),
            _ => slope(x, y, TILE, Slope::ALL[(off - UNDER_SLOPE) / VARIANTS], sewer),
        };
    }
    px
}

/// A slope cell (`below` 0), or the rock cell under one (`below` 8): the walkway's lit
/// edge running along the floor line, with its neon inset all the way along, then rock
/// as the wall's; in the sewers, a concrete wedge. One tile per shape: the fill doesn't
/// follow the wall's variants.
fn slope(x: i32, y: i32, below: i32, shape: Slope, sewer: bool) -> u8 {
    use gx::*;
    let d = y + below - (TILE - shape.height(x));
    if d < 0 {
        return 0;
    }
    if sewer {
        // The rail along the slope, then brick.
        return match d {
            0 => sw::STEEL_LIGHT,
            1 => sw::STEEL,
            2 => if x.rem_euclid(4) < 2 { sw::HAZARD } else { sw::OUTLINE },
            3 => sw::OUTLINE,
            _ => sewer_brick(x, y + below, false),
        };
    }
    match d {
        0 | 5 => OUTLINE,
        1 => GOLD_LIGHT,
        2 => WALL_LIGHT,
        3 => PINK,
        4 => WALL,
        _ => wall(x, y, 0, 1, 1, 0, false),
    }
}

/// Casino architecture, after the level 2 casino mockup: a pale steel walkway with
/// magenta neon set into it, over a dark slate sub-floor run through with green pipes
/// and riveted steel plates. Open sides get a neon edge, overhangs a gold trim. A false
/// wall shows a hairline crack.
fn wall(x: i32, y: i32, sides: usize, depth: usize, col4: i32, row3: i32, cracked: bool) -> u8 {
    use gx::*;
    let (up, left, right, down) = (sides & 1 != 0, sides & 2 != 0, sides & 4 != 0, sides & 8 != 0);
    let corner = |a: bool, b: bool, cx: i32, cy: i32| a && b && (x - cx).abs() + (y - cy).abs() <= 1;
    if corner(up, left, 0, 0) || corner(up, right, 7, 0) || corner(down, left, 0, 7) || corner(down, right, 7, 7) {
        return 0;
    }
    if up {
        // The walkway: a lit lip, steel, a neon insert every other cell, a dark seam.
        match y {
            0 => return OUTLINE,
            1 => return GOLD_LIGHT,
            2 | 3 if col4 % 2 == 1 && (1..=6).contains(&x) => return if y == 2 { WALL_LIGHT } else { PINK },
            2 | 3 => return WALL_LIGHT,
            4 => return if x == 7 && col4 % 2 == 0 { OUTLINE } else { WALL },
            5 => return OUTLINE,
            _ => {}
        }
    }
    if down {
        match y {
            7 => return OUTLINE,
            6 => return GOLD_DARK,
            _ => {}
        }
    }
    if left && x == 0 || right && x == 7 {
        return OUTLINE;
    }
    if left && x == 1 {
        return PINK;
    }
    if right && x == 6 {
        return CYAN;
    }
    if cracked && col4 == 2 && row3 == 1 && matches!((x, y), (3, 2) | (4, 3) | (4, 4) | (3, 5)) {
        return OUTLINE;
    }
    let gy = row3 * 8 + y;
    if depth == 2 {
        // Deep down: dark slate, a pipe along every third row of cells.
        return match (row3, y) {
            (1, 2) => OUTLINE,
            (1, 3) => FELT_LIGHT,
            (1, 4) => FELT,
            (1, 5) => OUTLINE,
            _ if (x + gy * 3 + col4 * 5) % 13 == 0 => WALL,
            _ => WALL_DARK,
        };
    }
    // Riveted steel plates on every fourth cell, pipes between.
    if col4 == 0 {
        return match (x, y) {
            (0, _) | (_, 0) => OUTLINE,
            (7, _) | (_, 7) => WALL_DARK,
            (2 | 5, 2 | 5) | (1, _) | (_, 1) => WALL_LIGHT,
            _ => WALL,
        };
    }
    match y {
        2 => OUTLINE,
        3 => FELT_LIGHT,
        4 => FELT,
        5 => OUTLINE,
        _ if (x + col4) % 7 == 0 => WALL_DARK,
        _ => WALL,
    }
}

/// A neon-edged balcony with a row of marquee bulbs under it.
fn ledge(x: i32, y: i32) -> u8 {
    use gx::*;
    match y {
        0 | 4 => OUTLINE,
        1 => CYAN,
        2 => WALL_LIGHT,
        3 => WALL,
        5 if x % 4 == 1 => BULB_A,
        5 if x % 4 == 3 => BULB_B,
        _ => 0,
    }
}

/// A sewer brick's pixel at `(gx, gy)` in the bricks' 32 by 24 pattern: 8 by 4 bricks,
/// every other course offset by half a brick, lit on top, some dark and some mossy;
/// darker deep down.
fn sewer_brick(gx: i32, gy: i32, deep: bool) -> u8 {
    use sw::*;
    let course = gy.div_euclid(4);
    let shift = course.rem_euclid(2) * 4;
    let (bx, by) = ((gx + shift).rem_euclid(8), gy.rem_euclid(4));
    if by == 3 || bx == 7 {
        return OUTLINE;
    }
    let brick = (gx + shift).div_euclid(8) * 5 + course * 3;
    let (light, face) = match brick.rem_euclid(7) {
        _ if deep => (STONE, STONE_DARK),
        0 | 4 => (STONE, STONE_DARK),
        _ => (STONE_LIGHT, STONE),
    };
    if brick.rem_euclid(7) == 2 && (bx + by * 2) % 3 == 0 {
        // Moss growing on the brick.
        return MOSS;
    }
    if by == 0 && bx < 6 || bx == 0 { light } else { face }
}

/// Moss drips hanging off the rail over the brick: (column in the 32 px pattern, length).
const SEWER_DRIPS: [(i32, i32); 4] = [(3, 3), (12, 2), (21, 4), (28, 2)];

/// Sewer brickwork, after the sewers mockup: a steel rail along exposed tops with a
/// row of hazard orange dashes and moss dripping from under it, wrapping round open
/// corners; courses of mossy green-grey brick, darker deep down. The bricks line up
/// across cells, as the variant gives each cell its place in a 32 by 24 pattern.
fn sewer_wall(x: i32, y: i32, sides: usize, depth: usize, col4: i32, row3: i32, cracked: bool) -> u8 {
    use sw::*;
    let (up, left, right, down) = (sides & 1 != 0, sides & 2 != 0, sides & 4 != 0, sides & 8 != 0);
    // Rounded outer corners show the dark of the back wall.
    let corner = |a: bool, b: bool, cx: i32, cy: i32| a && b && (x - cx).abs() + (y - cy).abs() <= 1;
    if corner(up, left, 0, 0) || corner(up, right, 7, 0) || corner(down, left, 0, 7) || corner(down, right, 7, 7) {
        return BACK_DARK;
    }
    let (gx, gy) = (col4 * 8 + x, row3 * 8 + y);
    if up {
        // The rail: its lit top is the walking line.
        match y {
            0 => return STEEL_LIGHT,
            1 => return if gx % 16 == 15 { STEEL } else { STEEL_LIGHT },
            2 => return STEEL,
            3 => return if gx.rem_euclid(4) < 2 { HAZARD } else { OUTLINE },
            4 => return OUTLINE,
            _ => {}
        }
    }
    // The rail wraps down an open side under it.
    if up && y <= 5 && (left && x <= 2 || right && x >= 5) {
        let d = if left { x } else { 7 - x };
        return match d {
            0 => if left { STEEL_LIGHT } else { STEEL },
            1 => STEEL,
            _ => OUTLINE,
        };
    }
    if down {
        match y {
            7 => return OUTLINE,
            6 => return STONE_DARK,
            _ => {}
        }
    }
    if left && x == 0 || right && x == 7 {
        return OUTLINE;
    }
    if left && x == 1 {
        return STONE_LIGHT;
    }
    if right && x == 6 {
        return STONE_DARK;
    }
    if cracked && col4 == 2 && row3 == 1 && matches!((x, y), (3, 2) | (4, 3) | (4, 4) | (3, 5)) {
        return OUTLINE;
    }
    if up {
        for &(dx, len) in &SEWER_DRIPS {
            if gx == dx && y < 5 + len {
                return if y == 4 + len { SLIME } else { MOSS };
            }
        }
    }
    sewer_brick(gx, gy, depth == 2)
}

/// The sewers' catwalk, after the mockup: a steel deck with hazard dashes along its
/// edge and rust under it, on brackets.
fn catwalk(x: i32, y: i32) -> u8 {
    use sw::*;
    match y {
        0 => STEEL_LIGHT,
        1 => STEEL,
        2 => if x % 4 < 2 { HAZARD } else { OUTLINE },
        3 => RUST,
        4 => OUTLINE,
        5 | 6 if x == 1 || x == 6 => if y == 5 { STEEL } else { OUTLINE },
        _ => 0,
    }
}

/// An electric fence's emitter: a gold pylon with a live node on top, where the
/// lightning curtain rises from.
fn grid(x: i32, y: i32) -> u8 {
    use gx::*;
    const ART: [&[u8; 8]; 8] = [
        b"..keek..",
        b".keeeek.",
        b"..kYyk..",
        b"..kYyk..",
        b".kYedk..",
        b".kYYyyk.",
        b"kYYYYYdk",
        b"dkdkdkdk",
    ];
    match ART[y as usize][x as usize] {
        b'k' => OUTLINE,
        b'Y' => GOLD_LIGHT,
        b'y' => GOLD,
        b'd' => GOLD_DARK,
        b'e' => ENERGY,
        _ => 0,
    }
}

/// A card table's top: a gold rim round green felt, rounded at its ends, with a card
/// printed on the felt here and there.
fn table(x: i32, y: i32, left_end: bool, right_end: bool, col4: i32) -> u8 {
    use gx::*;
    if (left_end && x == 0 || right_end && x == 7) && (y == 0 || y == 6) {
        return 0;
    }
    if left_end && x == 0 || right_end && x == 7 {
        return if y == 7 { 0 } else { OUTLINE };
    }
    match y {
        0 | 6 => OUTLINE,
        1 => GOLD_LIGHT,
        5 => GOLD_DARK,
        7 => 0,
        2..=4 if col4 == 1 && (2..=5).contains(&x) => if y == 2 || x == 2 || x == 5 { WALL_LIGHT } else { FELT_LIGHT },
        2 => FELT_LIGHT,
        _ => FELT,
    }
}

/// Half of a table's gold pedestal (the right half when `right`), flaring at the foot.
fn pedestal(x: i32, y: i32, right: bool, foot: bool) -> u8 {
    use gx::*;
    // Mirror so the pedestal's centre is between the two halves.
    let d = if right { x } else { 7 - x };
    let half = if foot && y >= 5 { 2 + (y - 4) } else { 2 };
    match d {
        _ if d > half => 0,
        _ if d == half => OUTLINE,
        _ if d == half - 1 => if right { GOLD_DARK } else { GOLD_LIGHT },
        _ => GOLD,
    }
}

/// Neon bulbs round the edge of a `w` by `h` sign, alternating sets so they chase.
fn bulbs(x: i32, y: i32, w: i32, h: i32) -> Option<u8> {
    let on_edge = x == 1 || y == 1 || x == w - 2 || y == h - 2;
    if !on_edge {
        return None;
    }
    let along = if y == 1 || y == h - 2 { x } else { y };
    match along % 4 {
        1 => Some(gx::BULB_A),
        3 => Some(gx::BULB_B),
        _ => Some(gx::OUTLINE),
    }
}

/// Whether `text` in the 5x7 dialogue font covers (x, y) when drawn from (tx, ty).
fn text_at(text: &str, tx: i32, ty: i32, x: i32, y: i32) -> bool {
    let (dx, dy) = (x - tx, y - ty);
    if dx < 0 || !(0..7).contains(&dy) {
        return false;
    }
    let (i, gx) = ((dx / 6) as usize, dx % 6);
    let Some(&byte) = text.as_bytes().get(i) else {
        return false;
    };
    gx < 5 && font::dialog_glyph(byte).is_some_and(|rows| rows[dy as usize] & (0b10000 >> gx) != 0)
}

/// A neon text sign: bulbs round a dark board, the text in `ink` with a gold shadow.
fn sign(text: &str, ink: u8, x: i32, y: i32, w: i32, h: i32) -> u8 {
    use gx::*;
    if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
        return OUTLINE;
    }
    if let Some(bulb) = bulbs(x, y, w, h) {
        return bulb;
    }
    let tx = (w - text.len() as i32 * 6 + 1) / 2;
    let ty = (h - 7) / 2;
    if text_at(text, tx, ty, x, y) {
        ink
    } else if text_at(text, tx + 1, ty + 1, x, y) {
        GOLD_DARK
    } else {
        SCREEN
    }
}

/// One pixel of a scenery picture, in the picture's own coordinates.
fn picture(letter: u8, x: i32, y: i32, w: i32, h: i32) -> u8 {
    use gx::*;
    match letter {
        b'S' => slot_machine(x, y),
        b'M' => trading_screen(x, y, w, h),
        b'N' => sign("CASINO", PINK, x, y, w, h),
        b'W' => sign("VIP", GOLD_LIGHT, x, y, w, h),
        b'F' => exchange_sign(x, y, w, h),
        b'U' => pump_pill(x, y, w, h),
        b'Q' => sol_sign(x, y, w, h),
        b'K' => coin_pile(x, y),
        b'D' => casino_doors(x, y, w, h),
        b'Z' => server_rack(x, y, w, h),
        b'V' => vault_door(x, y, w, h),
        b'R' => roulette(x, y, w),
        b'A' => card_sign(x, y, w, h),
        b'Y' => atm(x, y),
        b'H' => outflow(x, y),
        b'G' => graffiti(x, y, w),
        b'O' => vines(x, y),
        b'q' => pipework(x, y),
        b'k' => barrier(x, y),
        b'j' => ladder(x, y),
        _ => 0,
    }
}

/// A crypto ATM, 16x24: a cyan header with a gold coin, a lit screen, a keypad, and a
/// banknote sticking out of the cash slot.
fn atm(x: i32, y: i32) -> u8 {
    use gx::*;
    if !(1..=14).contains(&x) {
        return 0;
    }
    if y == 0 || y == 23 || x == 1 || x == 14 {
        return OUTLINE;
    }
    match y {
        1..=5 => {
            let (dx, dy) = (x * 2 - 15, (y - 3) * 2);
            let d = dx * dx + dy * dy;
            if d <= 12 {
                if (dx == -1 || dx == 1) && dy.abs() <= 2 { GOLD_DARK } else { GOLD_LIGHT }
            } else if d <= 20 {
                GOLD_DARK
            } else {
                CYAN
            }
        }
        6 | 13 => OUTLINE,
        7..=12 => {
            let line = (y == 8 && (4..=11).contains(&x)) || (y == 10 && (4..=8).contains(&x));
            if x == 2 || x == 13 { WALL_DARK } else if line { CYAN } else { SCREEN }
        }
        14..=18 => {
            if x == 2 { WALL_LIGHT } else if y % 2 == 1 && x % 2 == 0 && (4..=11).contains(&x) { WALL_LIGHT } else { WALL }
        }
        19 => if (4..=11).contains(&x) { OUTLINE } else { WALL },
        20 | 21 if (5..=10).contains(&x) => if x == 5 || x == 10 || y == 21 { FELT } else { FELT_LIGHT },
        22 => WALL_DARK,
        _ => if x == 2 { WALL_LIGHT } else { WALL },
    }
}

/// A sewer outflow, 24x24, after the mockup: a riveted rust ring round a dark pipe
/// mouth with slime glowing in it.
fn outflow(x: i32, y: i32) -> u8 {
    use sw::*;
    let (dx, dy) = (x - 12, y - 10);
    let d2 = dx * dx + dy * dy;
    if d2 <= 100 {
        return match () {
            _ if d2 > 90 => OUTLINE,
            _ if d2 > 56 => {
                // Rivets round the ring.
                let rivet = d2 > 66 && d2 < 82 && (dx * dx - dy * dy).rem_euclid(13) == 0;
                if rivet { STEEL_LIGHT } else if dx + dy < -5 { HAZARD } else { RUST }
            }
            _ if d2 > 46 => OUTLINE,
            _ if dy >= 3 => if dy == 3 { SLIME } else { MOSS },
            _ if dy >= 0 && (x + y) % 3 == 0 => MOSS,
            _ => OUTLINE,
        };
    }
    0
}

/// Graffiti, `w` by 16: NGMI sprayed in hazard orange with a dark shadow and drips,
/// over a slime green swoosh, on the brick under the back wall's ledge.
fn graffiti(x: i32, y: i32, w: i32) -> u8 {
    use sw::*;
    let tx = (w - 4 * 6 + 1) / 2;
    if text_at("NGMI", tx, 2, x, y) {
        return HAZARD;
    }
    let drip = [tx + 1, tx + 9, tx + 20].contains(&x) && (9..9 + (x % 3) + 2).contains(&y);
    if drip {
        return HAZARD;
    }
    if text_at("NGMI", tx + 1, 3, x, y) {
        return OUTLINE;
    }
    let wave = 11 + [0, 0, 1, 1, 1, 0, 0, -1, -1, -1][(x % 10) as usize];
    if y == wave && (2..w - 2).contains(&x) {
        return MOSS;
    }
    0
}

/// Vines, 16x16: a rope sagging between two hooks in the ceiling, leafy strands
/// hanging off it.
fn vines(x: i32, y: i32) -> u8 {
    use sw::*;
    let dx2 = x * 2 - 15;
    let rope = 1 + (4 * 64 - dx2 * dx2) / 64;
    if y == rope {
        return RUST;
    }
    if y == rope + 1 && x > 0 && x < 15 {
        return OUTLINE;
    }
    // Strands: (column, length).
    for (sx, len) in [(3, 11), (6, 8), (11, 14), (13, 9)] {
        let top = 1 + (4 * 64 - (sx * 2 - 15) * (sx * 2 - 15)) / 64 + 1;
        if y > top && y <= top + len {
            let k = y - top;
            let leaf = k % 3 == 1;
            if x == sx {
                return if leaf { SLIME } else { MOSS };
            }
            if leaf && x == sx + if k % 6 == 1 { 1 } else { -1 } {
                return MOSS;
            }
        }
    }
    0
}

/// Pipework on the wall, 32x24: a pipe along the wall with flanged joints, turning down
/// through an elbow into a downpipe that ends in a flange.
fn pipework(x: i32, y: i32) -> u8 {
    use sw::*;
    // The run: rows 4..=9; the downpipe: columns 21..=26, from the elbow down.
    let shade = |across: i32, len: i32| match across {
        0 | 5 => OUTLINE,
        1 => STONE_LIGHT,
        4 => STONE_DARK,
        _ => if len % 12 == 6 { STONE_DARK } else { STONE },
    };
    let flange = |across: i32| match across {
        -1 | 6 => OUTLINE,
        0 => STEEL_LIGHT,
        _ => STEEL,
    };
    let run = (4..=9).contains(&y) && x < 24;
    let down = (21..=26).contains(&x) && y >= 10;
    // The elbow: a quarter ring between them.
    let (ex, ey) = (x - 20, y - 10);
    let e2 = ex * ex + ey * ey;
    if x >= 24 && y <= 10 && ey <= 0 && ex >= 0 && e2 <= 6 * 6 + 1 {
        return match e2 {
            _ if e2 > 30 => OUTLINE,
            _ if e2 > 20 => STONE_LIGHT,
            _ if e2 > 4 => STONE,
            _ => STONE_DARK,
        };
    }
    if run {
        if x % 12 == 3 || x % 12 == 4 {
            return flange(y - 4);
        }
        return shade(y - 4, x);
    }
    if (3..=10).contains(&y) && (x % 12 == 3 || x % 12 == 4) && x < 24 {
        return OUTLINE;
    }
    if down {
        if y >= 21 || y == 15 || y == 16 {
            return flange(x - 21);
        }
        return shade(x - 21, y);
    }
    if (20..=27).contains(&x) && (y >= 21 || y == 15 || y == 16) {
        return OUTLINE;
    }
    0
}

/// A road barrier on the floor, 16x8: a hazard-striped board on two legs.
fn barrier(x: i32, y: i32) -> u8 {
    use sw::*;
    match y {
        1 | 5 if (1..=14).contains(&x) => OUTLINE,
        2..=4 if x == 1 || x == 14 => OUTLINE,
        2..=4 if (2..=13).contains(&x) => if (x + y).rem_euclid(6) < 3 { HAZARD } else { OUTLINE },
        6 | 7 if x == 3 || x == 12 => STEEL,
        6 | 7 if x == 2 || x == 4 || x == 11 || x == 13 => if y == 7 { OUTLINE } else { 0 },
        _ => 0,
    }
}

/// A steel ladder, 8 px wide and repeating down: two rails and a rung every 4 px.
fn ladder(x: i32, y: i32) -> u8 {
    use sw::*;
    match (x, y.rem_euclid(4)) {
        (0 | 7, _) => OUTLINE,
        (1, _) => STEEL_LIGHT,
        (6, _) => STEEL,
        (_, 1) => STEEL_LIGHT,
        (_, 2) => OUTLINE,
        _ => 0,
    }
}

/// A slot machine, 16x24, after the mockup: a pink dome with a light, a slate cabinet
/// with a diamond glowing on its screen, a coin tray, and a stool in front.
fn slot_machine(x: i32, y: i32) -> u8 {
    use gx::*;
    let body = (1..=14).contains(&x);
    match y {
        0 if (4..=11).contains(&x) => OUTLINE,
        1..=3 if (3..=12).contains(&x) => {
            if x == 3 || x == 12 { OUTLINE } else if y == 2 && (6..=9).contains(&x) { BULB_A } else { PINK }
        }
        4 if body => OUTLINE,
        5..=13 if body => {
            if x == 1 || x == 14 {
                OUTLINE
            } else if (6..=12).contains(&y) && (3..=12).contains(&x) {
                // The screen and the diamond on it.
                if y == 6 || y == 12 || x == 3 || x == 12 {
                    OUTLINE
                } else {
                    let (dx, dy) = ((x * 2 - 15).abs(), y - 7);
                    let gem = dx <= 2 * (4 - (dy - 1).max(0)) && dy >= 0 && (dy > 0 || dx <= 5);
                    if gem && dy <= 1 { CYAN } else if gem { WALL_LIGHT } else { PINK }
                }
            } else if x == 2 {
                WALL_LIGHT
            } else {
                WALL
            }
        }
        14 if body => GOLD,
        15 if body => OUTLINE,
        16 | 17 if (3..=12).contains(&x) => if y == 16 { WALL_DARK } else { OUTLINE },
        // The stool: a red cushion on a gold post.
        19 if (4..=11).contains(&x) => OUTLINE,
        20 if (4..=11).contains(&x) => if x == 4 || x == 11 { OUTLINE } else { PINK },
        21 if (4..=11).contains(&x) => OUTLINE,
        22 | 23 if (7..=8).contains(&x) => GOLD_DARK,
        _ => 0,
    }
}

/// A trading screen: a dark bezel round a chart of green and red candles and a cyan
/// price line diving at the end.
fn trading_screen(x: i32, y: i32, w: i32, h: i32) -> u8 {
    use gx::*;
    if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
        return OUTLINE;
    }
    if x == 1 || y == 1 || x == w - 2 || y == h - 2 {
        return WALL_DARK;
    }
    // A candle every four pixels: its body spans a band around a wandering price.
    let i = (x - 3) / 4;
    let price = h / 2 + [2, -1, -4, -2, -6, -3, 4, 7][(i as usize) % 8];
    let up = i % 3 != 2;
    let col = (x - 3) % 4;
    let (lo, hi) = (price - 3, price + 3);
    if col == 1 && (lo - 2..=hi + 2).contains(&y) && (3..h - 3).contains(&y) {
        let body = (lo..=hi).contains(&y);
        return if up { FELT_LIGHT } else if body { PINK } else { PINK };
    }
    if col == 0 && (lo..=hi).contains(&y) && (3..h - 3).contains(&y) {
        return if up { FELT } else { PINK };
    }
    if (x + y) % 7 == 0 && y < 5 {
        return WALL;
    }
    SCREEN
}

/// The exchange's sign: two slanted bars and its three letters, cyan on dark.
fn exchange_sign(x: i32, y: i32, w: i32, h: i32) -> u8 {
    use gx::*;
    if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
        return OUTLINE;
    }
    if x == 1 || y == 1 || x == w - 2 || y == h - 2 {
        return WALL_LIGHT;
    }
    for (row, len) in [(4, 6), (8, 5)] {
        let shift = 3 + (row - y).max(0);
        if (row..row + 3).contains(&y) && (shift..shift + len).contains(&x) {
            return CYAN;
        }
    }
    if text_at("FTX", 13, 4, x, y) {
        return CYAN;
    }
    if text_at("FTX", 14, 5, x, y) {
        return OUTLINE;
    }
    SCREEN
}

/// The pump pill: a big glowing capsule, half green, half white.
fn pump_pill(x: i32, y: i32, w: i32, h: i32) -> u8 {
    use gx::*;
    // A capsule along a gentle slope, as an ellipse in skewed coordinates.
    let (cx, cy) = (w / 2, h / 2);
    let (dx, dy) = (x - cx, y - cy + (x - cx) / 4);
    let inside = |rx: i32, ry: i32| dx * dx * ry * ry + dy * dy * rx * rx <= rx * rx * ry * ry;
    if !inside(14, 6) {
        return if inside(15, 7) { CYAN } else { 0 };
    }
    if !inside(13, 5) {
        return OUTLINE;
    }
    if dx.abs() <= 0 {
        return OUTLINE;
    }
    let shine = dy == -3 && dx.abs() > 2 && dx.abs() < 10;
    match () {
        _ if shine => GOLD_LIGHT,
        _ if dx < 0 => if dy > 2 { FELT } else { FELT_LIGHT },
        _ => if dy > 2 { WALL_LIGHT } else { GOLD_LIGHT },
    }
}

/// The SOL sign: three slanted bars, cyan to pink.
fn sol_sign(x: i32, y: i32, w: i32, h: i32) -> u8 {
    use gx::*;
    let _ = h;
    for (i, row) in [2, 7, 12].into_iter().enumerate() {
        if !(row..row + 3).contains(&y) {
            continue;
        }
        // The middle bar leans the other way.
        let lean = if i == 1 { -(y - row) } else { y - row };
        let (x0, x1) = (3 + lean, w - 3 + lean);
        if x == x0 - 1 || x == x1 {
            return OUTLINE;
        }
        if (x0..x1).contains(&x) {
            return [CYAN, WALL_LIGHT, PINK][i];
        }
    }
    0
}

/// A heap of gold coins, 16x8.
fn coin_pile(x: i32, y: i32) -> u8 {
    use gx::*;
    let coins = [(4, 5), (11, 5), (8, 2), (2, 6), (14, 6)];
    for &(cx, cy) in coins.iter().rev() {
        let (dx, dy) = (x - cx, (y - cy) * 2);
        let d = dx * dx + dy * dy;
        if d <= 9 {
            return if dy < 0 && dx < 1 { GOLD_LIGHT } else if dy > 1 { GOLD_DARK } else { GOLD };
        }
        if d <= 14 {
            return OUTLINE;
        }
    }
    0
}

/// The casino's front doors: a gold arch hung with bulbs over two glass doors.
fn casino_doors(x: i32, y: i32, w: i32, h: i32) -> u8 {
    use gx::*;
    if y < 8 {
        // The marquee over the door.
        if y == 0 || x == 0 || x == w - 1 {
            return OUTLINE;
        }
        if y == 7 {
            return GOLD_DARK;
        }
        if let Some(b) = bulbs(x, y, w, 8) {
            return b;
        }
        return if text_at("777", 7, 1, x, y) { PINK } else { GOLD };
    }
    if x <= 1 || x >= w - 2 {
        return if x == 0 || x == w - 1 { OUTLINE } else { GOLD };
    }
    if x == w / 2 || x == w / 2 - 1 {
        return OUTLINE;
    }
    if y == h - 1 {
        return GOLD_DARK;
    }
    let glint = (x + y) % 9 == 0 || (x + y) % 9 == 1;
    let handle = (x == w / 2 - 3 || x == w / 2 + 2) && (y - 8 == 14 || y - 8 == 15);
    match () {
        _ if handle => GOLD_LIGHT,
        _ if glint => CYAN,
        _ => SCREEN,
    }
}

/// A server rack, 16x32: a dark cabinet with rows of blinking lights and vents.
fn server_rack(x: i32, y: i32, w: i32, h: i32) -> u8 {
    use gx::*;
    if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
        return OUTLINE;
    }
    if (y - 2) % 6 == 0 {
        return WALL_DARK;
    }
    let slot = (y - 2) / 6;
    if (y - 2) % 6 == 2 && x >= 2 && x <= 7 {
        return match (x + slot) % 3 {
            0 => BULB_A,
            1 => FELT_LIGHT,
            _ => BULB_B,
        };
    }
    if (y - 2) % 6 == 4 && x >= 9 && x <= w - 3 && x % 2 == 0 {
        return OUTLINE;
    }
    if x == 1 { WALL_LIGHT } else { WALL }
}

/// The vault, 32x32: a round door with a gold wheel and bolts, coins spilling out.
fn vault_door(x: i32, y: i32, w: i32, h: i32) -> u8 {
    use gx::*;
    let (cx, cy) = (w / 2, h / 2);
    let (dx, dy) = (x - cx, y - cy);
    let d2 = dx * dx + dy * dy;
    if d2 > 15 * 15 {
        return if x == 0 || y == 0 || x == w - 1 || y == h - 1 { OUTLINE } else { WALL_DARK };
    }
    if d2 > 13 * 13 {
        return OUTLINE;
    }
    let bolt = [(0, -11), (11, 0), (0, 11), (-11, 0), (8, 8), (-8, -8), (8, -8), (-8, 8)]
        .iter()
        .any(|&(bx, by)| (dx - bx).abs() <= 1 && (dy - by).abs() <= 1);
    if bolt {
        return GOLD_LIGHT;
    }
    if d2 <= 2 * 2 {
        return GOLD_DARK;
    }
    let spoke = (dx == 0 || dy == 0 || dx == dy || dx == -dy) && d2 <= 7 * 7;
    if spoke {
        return GOLD;
    }
    if (6 * 6..=7 * 7).contains(&d2) {
        return GOLD;
    }
    if dx + dy < -6 { WALL_LIGHT } else { WALL }
}

/// A roulette wheel seen from the side, sitting on a table: gold rim, red and black
/// pockets, a spindle.
fn roulette(x: i32, y: i32, w: i32) -> u8 {
    use gx::*;
    let cx = w / 2;
    let dx = x - cx;
    let half = [4, 8, 10, 11, 11, 10, 8, 0][y as usize];
    if dx.abs() > half {
        return 0;
    }
    if dx.abs() == half || y == 6 {
        return OUTLINE;
    }
    if y == 0 || y == 1 {
        return if dx.abs() <= 1 { GOLD_LIGHT } else if y == 0 { 0 } else { OUTLINE };
    }
    if y == 2 || y == 5 {
        return GOLD;
    }
    if (dx + 20) % 2 == 0 { PINK } else { OUTLINE }
}

/// A neon playing card on the wall: a cyan frame round a dark card with a pink spade.
fn card_sign(x: i32, y: i32, w: i32, h: i32) -> u8 {
    use gx::*;
    if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
        return CYAN;
    }
    if x == 1 || y == 1 || x == w - 2 || y == h - 2 {
        return OUTLINE;
    }
    let (cx, cy) = (w / 2, h / 2 - 1);
    let (dx, dy) = (x - cx, y - cy);
    // A spade: a heart turned over, on a stem.
    let lobe = |ox: i32| (dx - ox) * (dx - ox) + (dy - 1) * (dy - 1) * 2 <= 8;
    let point = dy <= 0 && dy >= -5 && dx.abs() <= 5 + dy;
    let stem = dx.abs() <= (dy - 2).max(0) && (3..=5).contains(&dy);
    if lobe(-2) || lobe(2) || point || stem { PINK } else { SCREEN }
}

/// Whether actors at tile `(tx, ty)` are hidden behind a false wall.
pub fn behind_false_wall(level: &Level, tx: i32, ty: i32) -> bool {
    level.char_at(tx, ty - 1) == b'%' && level.tile(tx, ty) == Tile::Empty
}
