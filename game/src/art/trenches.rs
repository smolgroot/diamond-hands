//! Level 2, the Solana Trenches: the memecoin casino. A neon Vegas strip under a giant
//! gold coin, the gaming floor (felt tables, slot banks, trading screens), and the
//! exchange suites upstairs (server racks, the vault, red-lit screens).
//!
//! Like the Badlands, the terrain is built from 8x8 cells keyed by what a cell is and
//! what is around it ([`terrain_key`]), so each distinct cell is drawn once. Pictures
//! bigger than a cell (see `level::DECOR`) key each of their cells by its place in the
//! picture. Animation is all palette: the marquee bulbs chase by swapping two colours,
//! the electric floor switches with the fences, and the screens go red when the market
//! crashes.

use agb::display::{Palette16, Rgb15, tiled::RegularBackground};
use diamond_hands_logic::{Level, Tile, level};

use super::{TileCache, rgb};
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
    /// The electric floor, switched with the fence cycle.
    pub const ENERGY: u8 = 15;
}

type Colour = (u16, u16, u16);

/// Each area's terrain colours, indices 1..=12: the street, the casino floor, upstairs.
const GROUND: [[Colour; 12]; 3] = [
    // Outline, gold (light, mid, dark), steel light, slate, slate dark, pipe light,
    // pipe, pink, cyan, screen.
    [(3, 1, 7), (31, 30, 18), (29, 22, 6), (17, 10, 3), (24, 22, 28), (7, 9, 16), (3, 4, 9), (6, 22, 16), (3, 12, 11), (31, 6, 22), (6, 28, 31), (4, 2, 10)],
    [(3, 1, 6), (31, 30, 20), (30, 23, 7), (18, 11, 3), (25, 22, 29), (8, 8, 17), (3, 3, 9), (6, 22, 15), (2, 12, 10), (31, 7, 22), (8, 26, 31), (5, 2, 11)],
    [(3, 0, 3), (31, 27, 16), (28, 18, 5), (15, 8, 3), (24, 18, 24), (9, 4, 10), (4, 1, 5), (8, 20, 13), (3, 10, 8), (31, 4, 12), (10, 22, 31), (4, 1, 6)],
];

/// The casino's terrain palette for an area (`stage`), the fence cycle, the marquee's
/// chase step and whether the market is crashing (the screens flash red).
pub fn ground_palette(stage: u8, fence: u8, chase: bool, crash: bool) -> Palette16 {
    let mut colours = [rgb(0, 0, 0); 16];
    for (out, &(r, g, b)) in colours[1..13].iter_mut().zip(GROUND[stage.min(2) as usize].iter()) {
        *out = rgb(r, g, b);
    }
    let (on, off) = (rgb(31, 29, 14), rgb(14, 7, 4));
    colours[gx::BULB_A as usize] = if chase { on } else { off };
    colours[gx::BULB_B as usize] = if chase { off } else { on };
    colours[gx::ENERGY as usize] = match fence {
        0 => rgb(6, 4, 10),
        1 => rgb(16, 8, 14),
        _ => rgb(31, 8, 24),
    };
    if crash {
        colours[gx::SCREEN as usize] = rgb(18, 1, 4);
        colours[gx::FELT_LIGHT as usize] = rgb(31, 6, 8);
    }
    Palette16::new(colours)
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

const fn sky(top: Colour, bottom: Colour, coin: [Colour; 3], star: Colour, city: [Colour; 5]) -> Palette16 {
    Palette16::new([
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
    ])
}

/// The strip at night under a giant gold coin. Indoors, on the casino floor and up in
/// the suites, there is no sky: the coin and the stars take the back wall's colour, and
/// the skyline becomes dim columns and lit panels on the wall.
pub static SKY_PALETTES: [Palette16; 3] = [
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
}

/// Plain terrain kinds take keys below [`DECOR_BASE`]; pictures above it.
const DECOR_BASE: usize = 8 * 3 * 16 * 4 * 3;
/// Cells a picture may have: up to 8 across and 8 down.
const DECOR_CELLS: usize = 64;
/// Number of distinct [`terrain_key`] values.
pub const TERRAIN_KEYS: usize = DECOR_BASE + level::DECOR.len() * DECOR_CELLS;

/// Map characters that look like the casino's walls.
fn looks_solid(c: u8) -> bool {
    matches!(c, b'#' | b'%')
}

/// What a cell of the casino looks like: see the module docs.
pub fn terrain_key(level: &Level, tx: i32, ty: i32) -> usize {
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
    let kind = match c {
        b'#' => 1,
        b'=' => 2,
        b'^' => 3,
        b'T' => 4,
        b'|' => 5,
        b'%' => 6,
        _ => return 0,
    };
    // Past the map's top and bottom counts as wall, so ceilings and the floor get no
    // trim along the screen's edges.
    let solid = |dx: i32, dy: i32| {
        let (x, y) = (tx + dx, ty + dy);
        y < 0 || y >= level.height() || x < 0 || x >= level.width() || looks_solid(level.char_at(x, y))
    };
    let (depth, sides) = match kind {
        1 | 6 => {
            let depth = if !solid(0, -1) { 0 } else if !solid(0, -2) { 1 } else { 2 };
            let open = |dx, dy| !solid(dx, dy) as usize;
            (depth, open(0, -1) | open(-1, 0) << 1 | open(1, 0) << 2 | open(0, 1) << 3)
        }
        // Tables: which ends are open. Pedestals: which half, and whether it's the foot.
        4 => (0, (level.char_at(tx - 1, ty) != b'T') as usize | ((level.char_at(tx + 1, ty) != b'T') as usize) << 1),
        5 => (0, (level.char_at(tx - 1, ty) == b'|') as usize | (level.is_floor(tx, ty + 1) as usize) << 1),
        _ => (0, 0),
    };
    ((((kind * 3 + depth) * 16 + sides) * 4 + (tx & 3) as usize) * 3) + ty.rem_euclid(3) as usize
}

/// Pixels for a [`terrain_key`].
pub fn terrain_pixels(key: usize) -> [u8; 64] {
    let mut px = [0u8; 64];
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
    let row3 = (key % 3) as i32;
    let col4 = (key / 3 % 4) as i32;
    let sides = key / 12 % 16;
    let depth = key / 192 % 3;
    let kind = key / 576;
    for (n, p) in px.iter_mut().enumerate() {
        let (x, y) = ((n % 8) as i32, (n / 8) as i32);
        *p = match kind {
            1 | 6 => wall(x, y, sides, depth, col4, row3, kind == 6),
            2 => ledge(x, y),
            3 => grid(x, y),
            4 => table(x, y, sides & 1 != 0, sides & 2 != 0, col4),
            5 => pedestal(x, y, sides & 1 != 0, sides & 2 != 0),
            _ => 0,
        };
    }
    px
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

/// The electric floor: gold posts with a live grid strung between them.
fn grid(x: i32, y: i32) -> u8 {
    use gx::*;
    match (x, y) {
        (0 | 1, 0..=5) => if x == 0 { GOLD_LIGHT } else { GOLD_DARK },
        (_, 1 | 3 | 5) => ENERGY,
        (_, 6) => GOLD,
        (_, 7) => if x % 2 == 0 { GOLD_DARK } else { OUTLINE },
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
