//! Level 1, the Badlands: a dystopian dusk over a ruined city, mossy cliffs of
//! purple brick and crystal-set flagstone, steel girders and energy spikes; the cyber bots that guard it, and
//! BERA 3000. The bots go on to guard the Solana Trenches too.

use agb::display::{
    Palette16, Rgb15,
    object::{Size, SpriteVram},
    tiled::RegularBackground,
};
use diamond_hands_logic::{Level, Tile, level::Theme};

use super::{Canvas, LazyPoses, POSE_BYTES, TileCache, drawing, ix, rgb};

/// Backdrop layers repeat every 256 px as they scroll.
pub const MAP_TILES: i32 = 32;
const MAP_PX: i32 = MAP_TILES * 8;
/// The sky is painted down to here; the city and terrain cover the rest.
const HORIZON: i32 = 136;
const SKY_ROWS: i32 = HORIZON / 8;

/// Backdrop (BG palette 0) indices. Index 0 is the backdrop colour.
mod sky_ix {
    pub const SKY_BANDS: i32 = 6;
    pub const SKY: [u8; 6] = [1, 2, 3, 4, 5, 6];
    pub const MOON_LIGHT: u8 = 7;
    pub const MOON: u8 = 8;
    pub const CRATER: u8 = 9;
    pub const STAR: u8 = 10;
    pub const TOWER: u8 = 11;
    pub const TOWER_DARK: u8 = 12;
    pub const TOWER_EDGE: u8 = 13;
    pub const WINDOW: u8 = 14;
    pub const WINDOW_HOT: u8 = 15;
}

type Colour = (u16, u16, u16);

/// Colour `i` of `n` steps along a three-stop gradient (top, middle, horizon).
const fn grad(stops: [Colour; 3], i: u16, n: u16) -> Rgb15 {
    let t = i * 2 * 16 / (n - 1);
    let (a, b, t) = if t < 16 { (stops[0], stops[1], t) } else { (stops[1], stops[2], t - 16) };
    rgb(
        (a.0 * (16 - t) + b.0 * t) / 16,
        (a.1 * (16 - t) + b.1 * t) / 16,
        (a.2 * (16 - t) + b.2 * t) / 16,
    )
}

/// One backdrop palette per stage of the level: a violet dusk, a deep night, and the
/// city burning under a blood moon.
pub static SKY_PALETTES: [Palette16; 3] = [
    sky_palette(
        [(5, 3, 16), (18, 8, 26), (31, 17, 24)],
        [(29, 28, 31), (22, 21, 28), (14, 12, 22)],
        (28, 26, 31),
        [(10, 6, 20), (5, 3, 12), (20, 10, 26), (14, 28, 31), (31, 8, 22)],
    ),
    sky_palette(
        [(1, 1, 6), (8, 4, 17), (17, 8, 25)],
        [(27, 28, 31), (19, 20, 27), (11, 11, 20)],
        (28, 28, 31),
        [(5, 3, 12), (2, 1, 6), (12, 6, 20), (14, 28, 31), (31, 8, 22)],
    ),
    sky_palette(
        [(6, 1, 5), (20, 3, 7), (31, 17, 6)],
        [(31, 22, 16), (28, 11, 9), (17, 4, 7)],
        (31, 20, 16),
        [(8, 2, 6), (4, 1, 3), (20, 6, 8), (31, 24, 8), (31, 6, 4)],
    ),
];

/// The backdrop palettes for a level's theme, one per stage.
pub fn sky_palettes(theme: Theme) -> &'static [Palette16] {
    match theme {
        Theme::Badlands => &SKY_PALETTES,
        Theme::Trenches => &super::trenches::SKY_PALETTES,
    }
}

const fn col(c: Colour) -> Rgb15 {
    rgb(c.0, c.1, c.2)
}

/// `moon` is lit, mid and shadow; `city` is tower, dark, edge, window, hot window.
/// Muted ([`super::BACKDROP_MUTE`]), backdrop colour and all, so the action stands
/// out.
const fn sky_palette(
    stops: [Colour; 3],
    moon: [Colour; 3],
    star: Colour,
    city: [Colour; 5],
) -> Palette16 {
    super::mute_palette(Palette16::new([
        grad(stops, 0, sky_ix::SKY_BANDS as u16),
        grad(stops, 0, sky_ix::SKY_BANDS as u16),
        grad(stops, 1, sky_ix::SKY_BANDS as u16),
        grad(stops, 2, sky_ix::SKY_BANDS as u16),
        grad(stops, 3, sky_ix::SKY_BANDS as u16),
        grad(stops, 4, sky_ix::SKY_BANDS as u16),
        grad(stops, 5, sky_ix::SKY_BANDS as u16),
        col(moon[0]),
        col(moon[1]),
        col(moon[2]),
        col(star),
        col(city[0]),
        col(city[1]),
        col(city[2]),
        col(city[3]),
        col(city[4]),
    ]), super::BACKDROP_MUTE, 0)
}

/// Terrain (BG palette 1) indices.
mod ground_ix {
    pub const OUTLINE: u8 = 1;
    pub const MOSS_LIGHT: u8 = 2;
    pub const MOSS: u8 = 3;
    pub const MOSS_DARK: u8 = 4;
    pub const ROCK_LIGHT: u8 = 5;
    pub const ROCK: u8 = 6;
    pub const ROCK_DARK: u8 = 7;
    pub const METAL_LIGHT: u8 = 8;
    pub const METAL: u8 = 9;
    pub const METAL_DARK: u8 = 10;
    pub const CRYSTAL_LIGHT: u8 = 11;
    pub const CRYSTAL: u8 = 12;
    pub const CRYSTAL_DARK: u8 = 13;
    /// The deep flagstones' face, a shade under the bricks'.
    pub const ROCK_DEEP: u8 = 14;
    pub const ENERGY: u8 = 15;
}

const GROUND_COLOURS: [(u16, u16, u16); 16] = [
    (0, 0, 0),
    (2, 1, 5),
    (24, 31, 10),
    (15, 27, 5),
    (6, 15, 3),
    (20, 9, 20),
    (15, 6, 16),
    (8, 3, 11),
    (24, 23, 27),
    (15, 14, 19),
    (7, 7, 11),
    (24, 30, 31),
    (8, 17, 31),
    (7, 5, 22),
    (12, 4, 14),
    (31, 6, 22),
];

/// Terrain palette for a theme and stage: as drawn at dusk, dimmer at night, and
/// red-lit in the last stretch.
pub fn ground_palette(stage: u8, fence: u8) -> Palette16 {
    tinted(&GROUND_COLOURS, stage, fence)
}

/// The back wall's palette ([`BACK_PALETTE`]): the terrain's, so props in front of the
/// wall keep their colours, with the rock tones swapped for the wall's muted ones.
pub fn back_palette(stage: u8, fence: u8) -> Palette16 {
    let mut colours = GROUND_COLOURS;
    for (i, c) in BACK_COLOURS {
        colours[i as usize] = c;
    }
    tinted(&colours, stage, fence)
}

/// `base` lit for a stage, with the fence nodes in their state.
fn tinted(base: &[(u16, u16, u16); 16], stage: u8, fence: u8) -> Palette16 {
    let (num, den, red) = [(1, 1, 0), (4, 5, 0), (3, 4, 4)][stage.min(2) as usize];
    let mut colours = [rgb(0, 0, 0); 16];
    for (i, (out, &(r, g, b))) in colours.iter_mut().zip(base.iter()).enumerate() {
        // Energy spikes and crystals keep their glow.
        let (num, den) = if matches!(i, 11..=13 | 15) { (1, 1) } else { (num, den) };
        *out = rgb((r * num / den + red).min(31), g * num / den, b * num / den);
    }
    // Fence nodes: dead and dark, a dim blink as a warning, or white-blue hot.
    colours[ground_ix::ENERGY as usize] = match fence {
        0 => rgb(5, 6, 9),
        1 => rgb(8, 14, 24),
        _ => rgb(22, 30, 31),
    };
    Palette16::new(colours)
}

/// 4x4 ordered-dither thresholds, 0..16.
const BAYER: [[i32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// Moon centre and radius; right of every menu line so text never sits on it.
const MOON: (i32, i32, i32) = (214, 44, 15);
/// Craters (centre, radius): dark bowls with a lit rim on the side away from the light.
const CRATERS: [(i32, i32, i32); 5] = [(208, 40, 4), (219, 52, 3), (221, 38, 2), (210, 53, 2), (216, 45, 2)];
/// Dark lunar seas.
const MARIA: [(i32, i32, i32); 3] = [(210, 46, 5), (218, 41, 4), (213, 35, 3)];
/// How far the moon's glow reaches past its edge.
const HALO: i32 = 14;

/// Static sky: a smooth dithered gradient, a shaded cratered moon with a soft glow, and
/// stars.
pub fn paint_sky(bg: &mut RegularBackground) {
    let pixel = |x: i32, y: i32| {
        let (mx, my, r) = MOON;
        let inside = |cx: i32, cy: i32, r: i32| (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r * r;
        let bayer = BAYER[(y % 4) as usize][(x % 4) as usize];
        if inside(mx, my, r) {
            // Lit from the upper left: -100..100 across the disc, dithered between tones.
            let s = -((x - mx) + (y - my)) * 100 / (r * 3 / 2) + (bayer - 8) * 4;
            let mut tone = if s > 40 { 2 } else if s > -35 { 1 } else { 0 };
            if MARIA.iter().any(|&(cx, cy, cr)| inside(cx, cy, cr)) {
                tone = (tone - 1 + (bayer < 5) as i32).max(0);
            }
            for &(cx, cy, cr) in &CRATERS {
                if inside(cx, cy, cr) {
                    tone = if inside(cx - 1, cy - 1, cr - 1) { 0 } else { 2 };
                }
            }
            return [sky_ix::CRATER, sky_ix::MOON, sky_ix::MOON_LIGHT][tone as usize];
        }
        let steps = (sky_ix::SKY_BANDS - 1) * 16;
        let base = y * steps / HORIZON;
        // Moonlight lifts the sky around the disc.
        let (dx, dy) = (x - mx, y - my);
        let reach = r + HALO;
        let d2 = dx * dx + dy * dy;
        let glow = if d2 < reach * reach { (reach * reach - d2) * 36 / (reach * reach) } else { 0 };
        let t = (base + glow).min(steps);
        let band = (t / 16 + (bayer < t % 16) as i32).min(sky_ix::SKY_BANDS - 1);
        if base < 40 && glow == 0 && (x * 37 + y * 101 + x * y * 7) % 89 == 0 {
            return sky_ix::STAR;
        }
        sky_ix::SKY[band as usize]
    };
    let mut cache = TileCache::new();
    for ty in 0..SKY_ROWS {
        for tx in 0..MAP_TILES {
            cache.paint(bg, (tx, ty), 0, &pixel);
        }
    }
    for ty in SKY_ROWS..20 {
        for tx in 0..MAP_TILES {
            cache.paint(bg, (tx, ty), 0, &|_, _| sky_ix::TOWER_DARK);
        }
    }
}

/// Ruined towers: (left x, width, roof y, broken roof, antenna).
const TOWERS: [(i32, i32, i32, bool, bool); 13] = [
    (0, 18, 88, false, false),
    (20, 12, 70, true, false),
    (34, 22, 96, false, false),
    (58, 10, 60, false, true),
    (70, 26, 84, true, false),
    (98, 14, 104, false, false),
    (114, 20, 66, false, true),
    (136, 8, 92, true, false),
    (146, 24, 78, true, false),
    (172, 16, 100, false, false),
    (190, 12, 62, true, true),
    (204, 26, 90, false, false),
    (232, 22, 74, true, false),
];

/// The tower in front at column `x`, and its roof height there.
fn tower_at(x: i32) -> Option<(i32, i32, i32, bool)> {
    let x = x.rem_euclid(MAP_PX);
    TOWERS.iter().rev().find(|t| (t.0..t.0 + t.1).contains(&x)).map(|&(x0, w, roof, broken, antenna)| {
        // Broken roofs are jagged, as if bitten off.
        let jag = if broken { ((x - x0) * 7 % 11 + (x - x0) % 3 * 2) / 2 } else { 0 };
        (x0, w, roof + jag, antenna)
    })
}

/// The ruined city skyline, scrolled slowly for parallax. Moonlight catches the right
/// edges; a few windows still burn; antennas blink on the tallest towers.
pub fn paint_city(bg: &mut RegularBackground) {
    let pixel = |x: i32, y: i32| {
        let Some((x0, w, roof, antenna)) = tower_at(x) else {
            return if y >= 128 { sky_ix::TOWER_DARK } else { 0 };
        };
        let col = x.rem_euclid(MAP_PX) - x0;
        if antenna && col == w / 2 && (roof - 14..roof).contains(&y) {
            return if y == roof - 14 { sky_ix::WINDOW_HOT } else { sky_ix::TOWER_EDGE };
        }
        if y < roof {
            return 0;
        }
        let hash = (x * 13 + y * 7 + x * y) % 17;
        if y == roof || col == w - 1 {
            sky_ix::TOWER_EDGE
        } else if col % 4 == 2 && (y - roof) % 6 == 3 && hash < 3 {
            if hash == 0 { sky_ix::WINDOW_HOT } else { sky_ix::WINDOW }
        } else if y > 120 || col == 0 {
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
    // Below the screen the towers' dark feet carry on, for when a tall level scrolls
    // the city up: one solid tile, set without painting it pixel by pixel.
    if let Some(id) = cache.intern([sky_ix::TOWER_DARK; 64]) {
        let effect = agb::display::tiled::TileEffect::new(false, false, 0);
        for ty in 20..32 {
            for tx in 0..MAP_TILES {
                bg.set_tile_dynamic16((tx, ty), &cache.tiles[id], effect);
            }
        }
    }
}

/// The back wall behind the caves, after `.claude/mockup/level1-tiles-sprites-sheet-
/// with-background.png`: from this row down, open cells show a dim arcade of round
/// arches on pillars instead of the black void, with brick above it.
const BACK_WALL_ROW: i32 = 20;
/// The rows of the floors the arcade stands on, the cavern's and the deep's (see
/// `logic/src/levels/badlands.rs`).
const ARCADE_FLOORS: [i32; 2] = [37, 56];
/// The arcade's height in cells, and its bays' width (a pillar to the next) in pixels.
const ARCADE_ROWS: i32 = 10;
const BAY: i32 = 64;
/// A back wall cell is its column in a bay and its row: 0 for the brick above the
/// arcade, then the arcade's rows from the top.
const BACK_CELLS: usize = (BAY as usize / 8) * (1 + ARCADE_ROWS as usize);
/// What stands in front of a back wall cell: nothing, or a prop (and its variant).
const BACK_VARIANTS: usize = 10;
/// BG palette slot for the back wall (the Trenches use it for their back room).
pub const BACK_PALETTE: u8 = 4;
/// The wall's tones, in the rock's palette indices: lit stone, brick, mortar and
/// shadow, and the dim brick seen through the arches.
const BACK_COLOURS: [(u8, (u16, u16, u16)); 4] = [
    (ground_ix::ROCK_LIGHT, (9, 4, 11)),
    (ground_ix::ROCK, (6, 2, 8)),
    (ground_ix::ROCK_DARK, (3, 1, 5)),
    (ground_ix::ROCK_DEEP, (4, 2, 6)),
];

/// The BG palette slot a [`terrain_key`]'s tile uses.
pub fn palette_slot(key: usize) -> u8 {
    if key >= BACK_BASE { BACK_PALETTE } else { 1 }
}

/// The back wall's [`terrain_key`] for cell `(tx, ty)` with a prop of `kind` (0 for
/// none) in front, or `None` if the wall doesn't show there.
fn back_key(kind: usize, tx: i32, ty: i32) -> Option<usize> {
    if ty < BACK_WALL_ROW {
        return None;
    }
    // Props keep the variant their own key would give them (see `terrain_pixels`).
    let variant = match kind {
        0 => 0,
        2 => 1 + (tx & 1) as usize,
        3 => 3,
        5 => 4 + (tx & 1) as usize,
        6 => 6 + ((tx & 3) >= 2) as usize,
        7 => 8 + (tx & 1) as usize,
        _ => return None,
    };
    let row = match ARCADE_FLOORS.iter().find(|&&f| f > ty) {
        Some(&floor) if floor - ty <= ARCADE_ROWS => (1 + ARCADE_ROWS - (floor - ty)) as usize,
        _ => 0,
    };
    let col = (tx & (BAY / 8 - 1)) as usize;
    Some(BACK_BASE + variant * BACK_CELLS + row * (BAY as usize / 8) + col)
}

/// Rows of brick: three-pixel courses over mortar, with the joints staggered.
fn brick(x: i32, y: i32, face: u8) -> u8 {
    let course = y.div_euclid(4);
    if y.rem_euclid(4) == 3 || (x + course.rem_euclid(2) * 8).rem_euclid(16) == 15 {
        ground_ix::ROCK_DARK
    } else {
        face
    }
}

/// A pixel of the arcade, `x` across a bay (a pillar at 0) and `y` down from its top
/// (`ARCADE_ROWS` cells tall, standing on the floor).
fn arcade_pixel(x: i32, y: i32) -> u8 {
    use ground_ix::*;
    let h = ARCADE_ROWS * 8;
    // The pillar: a capital flaring under the arch's springing, a shaft and a base.
    let d = (x - if x >= BAY / 2 { BAY } else { 0 }).abs();
    let (spring, base) = (36, h - 6);
    let half = match y {
        _ if y < spring => -1,
        _ if y < spring + 2 => 8,
        _ if y < spring + 4 => 6,
        _ if y < base => 4,
        _ if y < base + 3 => 6,
        _ => 7,
    };
    if d <= half {
        let band = y == spring + 1 || y == spring + 3 || y == base || y == base + 2;
        return if d == half || band {
            ROCK_DARK
        } else if x == BAY - half + 1 || x == BAY - half + 2 {
            ROCK_LIGHT
        } else {
            ROCK
        };
    }
    // A cornice over the arches.
    if y == 6 {
        return ROCK_LIGHT;
    }
    if y == 7 {
        return ROCK_DARK;
    }
    // The arch: a round head over the opening between the pillars.
    let (cx, r) = (BAY / 2, BAY / 2 - 6);
    let inside = if y < spring {
        let (dx, dy) = (x - cx, y - spring);
        dx * dx + dy * dy < r * r
    } else {
        (x - cx).abs() < r
    };
    if !inside {
        return brick(x, y, ROCK);
    }
    // Through the arch, the dark, and dim brick further in where the light reaches.
    if y >= 46 + (x - cx).abs() / 2 { brick(x + 4, y, ROCK_DEEP) } else { OUTLINE }
}

/// Number of distinct [`terrain_key`] values.
pub const TERRAIN_KEYS: usize = BACK_BASE + BACK_VARIANTS * BACK_CELLS;
/// Keys up to here are the rock's and the props' own; the back wall's follow.
const BACK_BASE: usize = 9 * 3 * 16 * 4 * 3;
/// The kind of a rock cell with a crystal set in it.
const GEM_KIND: usize = 8;

/// What a terrain cell looks like depends only on its kind, how deep it sits below the
/// surface, which sides are open, and its column mod 4 and row mod 3 (for speckles,
/// cracks, crystal veins and strata). Computing that key is cheap, so the renderer
/// builds each distinct tile once: the GBA has no divide instruction, and per-pixel
/// patterns are too slow to redo for every streamed cell.
pub fn terrain_key(level: &Level, tx: i32, ty: i32) -> usize {
    let mut c = level.char_at(tx, ty);
    // A false wall looks like rock, and so do pickups hidden in one: the wall stays whole.
    if c != b'.' && hidden_in_false_wall(level, tx, ty) {
        c = b'%';
    }
    let kind = match c {
        b'#' | b'%' => 1,
        b'=' => 2,
        b'^' => 3,
        b'r' => 4,
        b',' => 5,
        b'*' => 6,
        b'&' => 7,
        _ => 0,
    };
    if let Some(key) = back_key(kind, tx, ty) {
        return key;
    }
    if kind == 0 {
        return 0;
    }
    // Below the map counts as rock, so the screen's bottom edge gets no outline.
    let solid = |dx: i32, dy: i32| {
        let (x, y) = (tx + dx, ty + dy);
        if y >= level.height() {
            return true;
        }
        match level.char_at(x, y) {
            // The common cases first: this runs for every cell that scrolls in.
            b'.' => x < 0 || x >= level.width(),
            b'#' | b'%' | b'r' => true,
            _ => level.tile(x, y) == Tile::Solid || hidden_in_false_wall(level, x, y),
        }
    };
    let depth = if !solid(0, -1) {
        0
    } else if !solid(0, -2) {
        1
    } else {
        2
    };
    let open = |dx, dy| !solid(dx, dy) as usize;
    let sides = open(0, -1) | open(-1, 0) << 1 | open(1, 0) << 2 | open(0, 1) << 3;
    // Deep rock holds a crystal at one of two spots in some 32x24 blocks, scattered
    // by a hash of the block so they don't line up.
    let (bx, by) = (tx >> 2, ty.div_euclid(3));
    let kind = if kind == 1 && depth == 2 && sides == 0 {
        let at = |i: usize| GEMS[i].0 == (tx & 3) && GEMS[i].1 == ty.rem_euclid(3);
        let mut hash = (bx as u32).wrapping_mul(0x9E37_79B1) ^ (by as u32).wrapping_mul(0x85EB_CA77);
        hash ^= hash >> 15;
        let hash = hash.wrapping_mul(0x2C1B_3C6D) >> 28;
        if (at(0) && hash < 5) || (at(1) && (6..10).contains(&hash)) { GEM_KIND } else { kind }
    } else {
        kind
    };
    ((((kind * 3 + depth) * 16 + sides) * 4 + (tx & 3) as usize) * 3) + ty.rem_euclid(3) as usize
}

/// Whether cell `(tx, ty)` is a pickup tucked into a false wall, drawn as the wall.
fn hidden_in_false_wall(level: &Level, tx: i32, ty: i32) -> bool {
    let c = level.char_at(tx, ty);
    c != b'#' && c != b'%' && level.char_at(tx, ty - 1) == b'%' && level.char_at(tx, ty + 1) != b'.'
}

/// Scenery and props, drawn from 8x8 character grids.
const MOSS_ART: [&[u8; 8]; 8] = [
    b"........",
    b"........",
    b"........",
    b"...k....",
    b".k.km.k.",
    b"..kmk.mk",
    b".kmMmkmk",
    b"kmMMMmMk",
];
const SCRAP_ART: [&[u8; 8]; 8] = [
    b"........",
    b"........",
    b"........",
    b"........",
    b"....kk..",
    b"...kSgk.",
    b".kkkgk..",
    b"kSggkSgk",
];
const BOT_HEAD_ART: [&[u8; 8]; 8] = [
    b"........",
    b"........",
    b"...k....",
    b"..kSk...",
    b".kSSSSk.",
    b"kSckSgSk",
    b"kSSSggSk",
    b".kkkkkk.",
];
/// An electric fence's emitter: a steel pylon with a live node on top, where the
/// lightning curtain (sprites, see [`crate::art::Common::lightning`]) rises from. The
/// node and status light are the only things drawn in [`ground_ix::ENERGY`], so the
/// palette switches the current on and off.
const SPIKES_ART: [&[u8; 8]; 8] = [
    b"..keek..",
    b".keeeek.",
    b"..kSgk..",
    b"..kSgk..",
    b".kSegk..",
    b".kSSggk.",
    b"kSSSSSgk",
    b"gkgkgkgk",
];

fn stamp(px: &mut [u8; 64], art: &[&[u8; 8]; 8], flip: bool) {
    use ground_ix::*;
    for (y, row) in art.iter().enumerate() {
        for x in 0..8 {
            px[y * 8 + x] = match row[if flip { 7 - x } else { x }] {
                b'k' => OUTLINE,
                b'm' => MOSS,
                b'M' => MOSS_LIGHT,
                b'S' => METAL_LIGHT,
                b'g' => METAL_DARK,
                b'c' => CRYSTAL,
                b'e' => ENERGY,
                _ => 0,
            };
        }
    }
}

/// The moss on an exposed top, 32 px wide (`gx`), after the mockup. Its top row is
/// the walking line, so feet rest right on the lit grass; everything else hangs below
/// it: a lime band, then rounded leaves over the bricks, lit ones in front and dark
/// ones peeking out from behind. `k` outline, `H` lit, `M` moss, `d` dark and `.` the
/// rock showing through.
const MOSS_STRIP: [&[u8; 32]; 8] = [
    b"HHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH",
    b"MHMMHMMHMMMMMHMMMMMHMMHMMHMMMMMH",
    b"MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM",
    b"MMMMMMdddMMMMMMMMMMdddMMMMMMMMMM",
    b"dHMMMdddddHMMddHMMdddddHMMMddHMd",
    b"dMMMMddddkMMMkdMMMdkkkdMMMMdkMMk",
    b"kMMMMkkkk.kkk.kMMMk...kMMMMk.kk.",
    b".kkkk..........kkk.....kkkk.....",
];

/// Brick courses under the moss, after `.claude/mockup/level1-tiles.png`: each is the
/// pixel row below the surface it starts on, four rows tall over a row of mortar, and
/// the columns (mod 32) of its joints.
const COURSES: [(i32, [i32; 4]); 2] = [(6, [5, 14, 22, 29]), (11, [1, 10, 18, 26])];
/// The flagstones under the bricks: two rows of slabs every 24 px, eleven rows tall
/// over a row of mortar, with joints (mod 32) that lean right as they go down.
const SLAB_JOINTS: [[i32; 2]; 2] = [[0, 18], [9, 25]];
/// Crystals set in the flagstones: (column mod 4, row mod 3, x, y in the cell, big).
const GEMS: [(i32, i32, i32, i32, bool); 2] = [(1, 0, 4, 5, true), (3, 2, 6, 3, false)];

/// A stone's shading from where a pixel sits in it: `top`, `bottom`, `first` and
/// `last` say whether the pixel is on its top or bottom row or its first or last
/// column. Lit from the upper left, with rounded corners.
fn stone(top: bool, bottom: bool, first: bool, last: bool, light: u8, face: u8) -> u8 {
    use ground_ix::*;
    if (top || bottom) && (first || last) {
        OUTLINE
    } else if top || first {
        light
    } else if bottom || last {
        ROCK_DARK
    } else {
        face
    }
}

/// A pixel of the rock's interior at column `gx` (mod 32): two courses of rounded
/// bricks `py` rows under the surface, then big slanted flagstones (`gy` is the row
/// mod 24) studded with crystals.
fn rock_pixel(gx: i32, py: i32, gy: i32) -> u8 {
    use ground_ix::*;
    let wrap = |x: i32| x.rem_euclid(32);
    let speck = (gx * 7 + gy * 13 + gx * gy) % 29 == 0;
    if py < 16 {
        let Some(&(start, joints)) = COURSES.iter().find(|c| (c.0..c.0 + 4).contains(&py)) else {
            return OUTLINE;
        };
        if joints.contains(&gx) {
            return OUTLINE;
        }
        let r = py - start;
        let px = stone(r == 0, r == 3, joints.contains(&wrap(gx - 1)), joints.contains(&wrap(gx + 1)), ROCK_LIGHT, ROCK);
        return if px == ROCK && speck { ROCK_DARK } else { px };
    }
    let (row, yy) = (gy / 12, gy % 12);
    if yy == 11 {
        return OUTLINE;
    }
    let joints = SLAB_JOINTS[row as usize];
    let x = wrap(gx - yy / 3);
    if joints.contains(&x) {
        return OUTLINE;
    }
    let px = stone(yy == 0, yy == 10, joints.contains(&wrap(x - 1)), joints.contains(&wrap(x + 1)), ROCK, ROCK_DEEP);
    if px == ROCK_DEEP && speck { ROCK } else { px }
}

/// A pixel of a pillar of stacked blocks up an exposed side, `x` 0..3 across it and
/// `gy` the row mod 24.
fn pillar_pixel(x: i32, w: i32, gy: i32) -> u8 {
    use ground_ix::*;
    let s = gy % 6;
    if s == 5 {
        OUTLINE
    } else {
        stone(s == 0, s == 4, x == 0, x == w - 1, ROCK_LIGHT, ROCK)
    }
}

/// Pixels for a [`terrain_key`], after the tiles mockup in `.claude/mockup/`. Rock:
/// bright lime moss on exposed tops, hanging over the edge in leafy lobes; two
/// courses of rounded purple bricks under it, then big slanted flagstones set with
/// glowing blue crystals; pillars of stacked blocks up exposed sides, shadowed
/// undersides and rounded outer corners. Crystal-studded boulders, steel girders,
/// energy spikes, and scenery.
pub fn terrain_pixels(key: usize) -> [u8; 64] {
    use ground_ix::*;
    let row3 = (key % 3) as i32;
    let col4 = (key / 3 % 4) as i32;
    let sides = key / 12 % 16;
    let depth = (key / 192 % 3) as i32;
    let kind = key / 576;
    let (up, left, right, down) = (sides & 1 != 0, sides & 2 != 0, sides & 4 != 0, sides & 8 != 0);
    let mut px = [0u8; 64];
    if key >= BACK_BASE {
        let (variant, cell) = ((key - BACK_BASE) / BACK_CELLS, (key - BACK_BASE) % BACK_CELLS);
        let (row, col) = ((cell / (BAY as usize / 8)) as i32, (cell % (BAY as usize / 8)) as i32);
        for (i, p) in px.iter_mut().enumerate() {
            let (x, y) = (col * 8 + (i % 8) as i32, (i / 8) as i32);
            *p = if row == 0 { brick(x, y, ground_ix::ROCK) } else { arcade_pixel(x, (row - 1) * 8 + y) };
        }
        // The prop in front, as its own key draws it.
        let prop = match variant {
            0 => None,
            1 | 2 => Some((2, variant - 1)),
            3 => Some((3, 0)),
            4 | 5 => Some((5, variant - 4)),
            6 | 7 => Some((6, (variant - 6) * 2)),
            _ => Some((7, variant - 8)),
        };
        if let Some((kind, col4)) = prop {
            for (p, q) in px.iter_mut().zip(terrain_pixels(kind * 576 + col4 * 3)) {
                if q != 0 {
                    *p = q;
                }
            }
        }
        return px;
    }
    match kind {
        1 | GEM_KIND => {
            for (i, p) in px.iter_mut().enumerate() {
                let (x, y) = ((i % 8) as i32, (i / 8) as i32);
                let (gx, gy) = (col4 * 8 + x, row3 * 8 + y);
                let py = if depth < 2 { depth * 8 + y } else { 16 + y };
                let moss = if !up {
                    None
                } else {
                    match MOSS_STRIP[y as usize][gx as usize] {
                        b'.' => None,
                        b'k' => Some(OUTLINE),
                        b'H' => Some(MOSS_LIGHT),
                        b'M' => Some(MOSS),
                        _ => Some(MOSS_DARK),
                    }
                };
                *p = if let Some(m) = moss {
                    if (left && x == 0) || (right && x == 7) { OUTLINE } else { m }
                } else if down && y == 7 {
                    OUTLINE
                } else if (left && x == 0) || (right && x == 7) {
                    OUTLINE
                } else if left && right {
                    pillar_pixel(x - 1, 6, gy)
                } else if left && x <= 3 {
                    pillar_pixel(x - 1, 3, gy)
                } else if right && x >= 4 {
                    pillar_pixel(x - 4, 3, gy)
                } else if (left && x == 4) || (right && x == 3) {
                    OUTLINE
                } else if down && !up && y >= 2 {
                    // Ceilings and overhangs end in a course of bricks.
                    if y == 2 { OUTLINE } else { rock_pixel(gx, COURSES[0].0 + y - 3, gy) }
                } else if down && y == 6 {
                    ROCK_DARK
                } else {
                    rock_pixel(gx, py, gy)
                };
            }
            // A crystal glinting in the flagstones.
            if kind == GEM_KIND {
                for &(c, r, gx, gy, big) in &GEMS {
                    if (c, r) != (col4, row3) {
                        continue;
                    }
                    for (dx, dy) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1), (-2, 0), (2, 0), (0, -2), (0, 2), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
                        let d = (dx as i32).abs() + (dy as i32).abs();
                        let colour = match (big, d) {
                            (true, 0) => CRYSTAL_LIGHT,
                            (true, 1) => if dx + dy < 0 { CRYSTAL_LIGHT } else { CRYSTAL },
                            (true, _) => CRYSTAL_DARK,
                            (false, 0) => CRYSTAL_LIGHT,
                            (false, 1) => CRYSTAL,
                            (false, _) if d == 2 && dx != 0 && dy != 0 => CRYSTAL_DARK,
                            _ => continue,
                        };
                        px[((gy + dy) * 8 + gx + dx) as usize] = colour;
                    }
                }
            }
            round_corners(&mut px, up, left, right, down);
        }
        4 => {
            // A loose flagstone studded with crystal facets catching the light.
            for (i, p) in px.iter_mut().enumerate() {
                let (x, y) = ((i % 8) as i32, (i / 8) as i32);
                let edge = (up && y == 0) || (left && x == 0) || (right && x == 7) || (down && y == 7);
                *p = if edge {
                    OUTLINE
                } else if (2..=4).contains(&x) && (2..=4).contains(&y) && (x - 3).abs() + (y - 3).abs() <= 1 {
                    if (x, y) == (3, 3) || x + y < 6 { CRYSTAL_LIGHT } else { CRYSTAL }
                } else if (x - 3).abs() + (y - 3).abs() == 2 {
                    CRYSTAL_DARK
                } else {
                    stone((up && y == 1) || (!up && y == 0), (down && y == 6) || (!down && y == 7), (left && x == 1) || (!left && x == 0), (right && x == 6) || (!right && x == 7), ROCK_LIGHT, ROCK_DEEP)
                };
            }
            round_corners(&mut px, up, left, right, down);
        }
        2 => {
            // A steel girder: a lit top bar with bolted plates, a bracket underneath.
            for x in 0..8 {
                px[x] = OUTLINE;
                px[8 + x] = METAL_LIGHT;
                px[16 + x] = METAL;
                px[24 + x] = METAL_DARK;
                px[32 + x] = OUTLINE;
            }
            if col4 % 2 == 1 {
                // A plate joint and its bolt.
                px[8 + 7] = METAL;
                px[16 + 7] = OUTLINE;
                px[24 + 7] = OUTLINE;
                px[16 + 5] = METAL_LIGHT;
            }
            for y in 5..8 {
                let x = if col4 % 2 == 0 { y - 4 } else { 11 - y };
                let shadow = if col4 % 2 == 0 { x + 1 } else { x - 1 };
                px[y * 8 + x] = METAL;
                px[y * 8 + shadow] = OUTLINE;
            }
        }
        3 => stamp(&mut px, &SPIKES_ART, false),
        5 => stamp(&mut px, &MOSS_ART, col4 % 2 == 1),
        6 => stamp(&mut px, &SCRAP_ART, col4 >= 2),
        7 => stamp(&mut px, &BOT_HEAD_ART, col4 % 2 == 1),
        _ => {}
    }
    px
}

/// Rounds off the outer corners between open sides.
fn round_corners(px: &mut [u8; 64], up: bool, left: bool, right: bool, down: bool) {
    for (open_a, open_b, corner, inner) in [
        (up, left, [(0, 0), (1, 0), (0, 1)], (1, 1)),
        (up, right, [(7, 0), (6, 0), (7, 1)], (6, 1)),
        (down, left, [(0, 7), (1, 7), (0, 6)], (1, 6)),
        (down, right, [(7, 7), (6, 7), (7, 6)], (6, 6)),
    ] {
        if open_a && open_b {
            for (x, y) in corner {
                px[y * 8 + x] = 0;
            }
            px[inner.1 * 8 + inner.0] = ground_ix::OUTLINE;
        }
    }
}

/// The Badlands bots, their shots, and the boss.
pub struct Villains {
    /// The Sentry bot's poses, 32x32, facing right: see [`SentryPose`].
    pub sentry: LazyPoses<SENTRY_POSES>,
    /// Crouched, then lunging.
    pub crawler: [SpriteVram; 2],
    /// Rotor frames.
    pub drone: [SpriteVram; 2],
    pub mine: SpriteVram,
    pub plasma: SpriteVram,
    pub bomb: SpriteVram,
    pub shockwave: SpriteVram,
    pub missile: SpriteVram,
    /// Sparks circling something dazed: a stalled boss, a knocked-back wassie.
    pub star: SpriteVram,
}

/// The Sentry bot's poses in `game/gfx/sentry.tiles`, cut from
/// `.claude/skills/diamond-hands-gba/lvl1-bot-sprite-sheet.png` by
/// `scripts/make-bots.py`: a red one-eyed bot with a steel fist and an arm cannon.
/// Each is 32x32 facing right, feet on the bottom row, legs centred on column
/// [`SENTRY_ANCHOR`]. In the script's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SentryPose {
    Walk0,
    Walk1,
    Walk2,
    Walk3,
    Walk4,
    Walk5,
    /// Cannon raised.
    Aim,
    /// Cannon raised, its muzzle glowing pink.
    Charge,
    Hit,
    /// Beaten: rocking back, on its knees, face down.
    Stagger,
    Kneel,
    Down,
}

impl SentryPose {
    pub const WALK: [SentryPose; 6] =
        [SentryPose::Walk0, SentryPose::Walk1, SentryPose::Walk2, SentryPose::Walk3, SentryPose::Walk4, SentryPose::Walk5];
}

pub const SENTRY_POSES: usize = 12;
/// The column of a pose the hitbox's centre sits on (facing right).
pub const SENTRY_ANCHOR: i32 = 13;
static SENTRY_TILES: &[u8; SENTRY_POSES * POSE_BYTES] = include_bytes!("../../gfx/sentry.tiles");
static SENTRY_PALETTE: Palette16 = super::gfx_palette(include_bytes!("../../gfx/sentry.pal"));

const CRAWLER0: [&str; 13] = [
    "................",
    "................",
    "......kkkkkk....",
    ".....kooooook...",
    "....kooyyyoook..",
    "...kkoooooooook.",
    "..kiwkoooooGGk..",
    "..kiikooooGGGk..",
    "...kkkkkkkkkk...",
    "..kSk.kSk.kSk.k.",
    ".kSk.kSk..kSk.kS",
    "kSk..kSk...kSk.k",
    "kk..kk......kk..",
];
const CRAWLER1: [&str; 12] = [
    "................",
    "................",
    "................",
    "................",
    ".......kkkkkk...",
    "kkk..kkooooook..",
    "kiwkkooyyyoooook",
    "kiikkoooooooGGk.",
    ".kkkkkkkkkkkkk..",
    "kSSk.kSk..kSk.k.",
    ".kSkkSk...kSk.kS",
    "..kkkk.....kkk.k",
];
const DRONE0: [&str; 13] = [
    "kSSSSSkkSSSSSk..",
    "......kk........",
    ".....kggk.......",
    "...kkvvvvkk.....",
    "..kvCvvvvvVk....",
    ".kvvkkkkvvVVk...",
    ".kvkiiwikvVVk...",
    ".kvkiiiikvVVk...",
    ".kvvkkkkvvVVk...",
    "..kVvvvvvVVk....",
    "...kkkVVkkk.....",
    "....kS..Sk......",
    "...kS....Sk.....",
];
const DRONE1: [&str; 13] = [
    "...kSSkkSSk.....",
    "......kk........",
    ".....kggk.......",
    "...kkvvvvkk.....",
    "..kvCvvvvvVk....",
    ".kvvkkkkvvVVk...",
    ".kvkiwiikvVVk...",
    ".kvkiiiikvVVk...",
    ".kvvkkkkvvVVk...",
    "..kVvvvvvVVk....",
    "...kkkVVkkk.....",
    "....kS..Sk......",
    "....kS..Sk......",
];
const MINE: [&str; 12] = [
    "....k..k....",
    "...kSkkSk...",
    ".kkgggggGkk.",
    "kSggGrrGggSk",
    ".kgGrwwrGGk.",
    "kSgGrwwrGgSk",
    ".kggGrrGGGk.",
    "kSggggGGGgSk",
    ".kkgGGGGGkk.",
    "...kSkkSk...",
    "....k..k....",
    "............",
];
const PLASMA: [&str; 4] = [
    ".krrk.",
    "krwwrk",
    "krwwrk",
    ".krrk.",
];
const BOMB: [&str; 8] = [
    "..kk....",
    ".kSk....",
    "kvvvk...",
    "kvCvk...",
    "kvvvk...",
    ".kkk....",
    "..y.....",
    "........",
];
const MISSILE: [&str; 12] = [
    "..k...",
    ".kSk..",
    ".kgk..",
    "kgggk.",
    "kgrgk.",
    "kgggk.",
    "kgggk.",
    "kSgSk.",
    "kk.kk.",
    ".y.y..",
    ".o.o..",
    "..o...",
];

/// Energy wave thrown along the ground by BERA 3000's slam.
const SHOCKWAVE: [&str; 12] = [
    "....i.....",
    "...iwi..i.",
    "..iwbi.iwi",
    ".iwbbiibwi",
    ".iwbbbbbi.",
    "iwbbibbbi.",
    "iwbiibbbii",
    "iwibbbibbi",
    "iwbbbbbbbi",
    "iibbbibbbi",
    ".iiiiiiii.",
    "..........",
];

impl Villains {
    pub fn new() -> Self {
        Self {
            sentry: LazyPoses::new(SENTRY_TILES, &SENTRY_PALETTE),
            crawler: [
                drawing(Size::S16x16, 0, 16 - CRAWLER0.len() as i32, &CRAWLER0),
                drawing(Size::S16x16, 0, 16 - CRAWLER1.len() as i32, &CRAWLER1),
            ],
            drone: [drawing(Size::S16x16, 0, 1, &DRONE0), drawing(Size::S16x16, 0, 1, &DRONE1)],
            mine: drawing(Size::S16x16, 2, 2, &MINE),
            plasma: drawing(Size::S8x8, 1, 2, &PLASMA),
            bomb: drawing(Size::S8x8, 0, 0, &BOMB),
            shockwave: drawing(Size::S16x16, 3, 4, &SHOCKWAVE),
            missile: drawing(Size::S8x16, 1, 2, &MISSILE),
            star: drawing(Size::S8x8, 1, 1, &["..y..", ".yoy.", "yowoy", ".yoy.", "..y.."]),
        }
    }
}

/// BERA 3000, facing left, after `.claude/skills/diamond-hands-gba/bera-mockup.png`: a
/// hulking bear mech in red armour, with a gold visor, a vented steel snout, a steel
/// strap over the far shoulder, a star core in the chest, hazard stripes, big steel
/// gauntlets with magenta lights, and gold-capped boots.
///
/// The standing mech is built from parts so it can breathe without spending VRAM on
/// whole extra frames: [`Bera::legs`] stays planted while [`Bera::torso`] and
/// [`Bera::head`] rise and fall on top of it. Every part is drawn in the coordinates
/// of one 64x64 frame with the feet on row 63, and placed at its `BERA_*_AT` corner
/// (mirror it with [`flip_x`] when facing right).
pub struct Bera {
    pub head: SpriteVram,
    /// The head with the visor gone dark, while stalled.
    pub head_dead: SpriteVram,
    pub torso: SpriteVram,
    /// The torso with the near gauntlet raised and aimed.
    pub torso_aim: SpriteVram,
    pub legs: SpriteVram,
    /// The star in the chest core, normal and flaring on each breath in. Left off
    /// (dark) while stalled.
    pub core: [SpriteVram; 2],
    /// A puff of smoke venting off a stalled mech.
    pub smoke: SpriteVram,
    /// Hunched and charging, 64x32 with the feet on row 31; [`Bera::head`] goes on top
    /// at [`BERA_CHARGE_HEAD_AT`].
    pub charge: SpriteVram,
    /// Her head, facing left, for the dialogue box: the same sprite as [`Bera::head`].
    pub portrait: SpriteVram,
}

/// Top-left of the 32x32 head, 64x32 torso, 64x32 legs and 8x8 core in the 64x64 frame,
/// at rest.
pub const BERA_HEAD_AT: (i32, i32) = (17, 0);
pub const BERA_TORSO_AT: (i32, i32) = (0, 22);
pub const BERA_LEGS_AT: (i32, i32) = (0, 32);
pub const BERA_CORE_AT: (i32, i32) = (25, 34);
/// Top-left of the head within the 64x32 charge sprite.
pub const BERA_CHARGE_HEAD_AT: (i32, i32) = (0, -1);

/// X of a part `w` wide at `x` in a 64-wide frame, once the frame is mirrored.
pub const fn flip_x(x: i32, w: i32, flip: bool) -> i32 {
    if flip { 64 - x - w } else { x }
}

fn red(c: &mut Canvas, cx: i32, cy: i32, rx: i32, ry: i32) {
    c.ball(cx, cy, rx, ry, ix::RED, ix::RED_DARK);
}

fn red_pod(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32) {
    c.pod(x, y, w, h, ix::ORANGE, ix::RED, ix::RED_DARK);
}

fn steel_pod(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32) {
    c.pod(x, y, w, h, ix::STEEL_LIGHT, ix::STEEL, ix::STEEL_DARK);
}

/// A dark steel joint or fist.
fn dark_pod(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32) {
    c.pod(x, y, w, h, ix::STEEL, ix::STEEL_DARK, ix::STEEL_DARK);
}

/// Two bold hazard stripes slanting up to the right from (x, y), `len` wide.
fn hazard_stripes(c: &mut Canvas, x: i32, y: i32, len: i32, under: Option<u8>) {
    for i in [0, 4] {
        for d in 0..len {
            c.rect(x + d, y + i - d / 2, 2, 2, ix::GOLD);
            if let Some(colour) = under {
                c.px(x + d, y + 2 + i - d / 2, colour);
            }
        }
    }
}

/// A steel gauntlet with a magenta light panel on its outer face at `light_x`.
fn gauntlet(c: &mut Canvas, x: i32, y: i32, w: i32, h: i32, light_x: i32) {
    steel_pod(c, x, y, w, h);
    c.rect(light_x, y + 3, 3, 8, ix::LASER);
    c.rect(light_x + 1, y + 4, 1, 2, ix::LASER_LIGHT);
    c.px(light_x + 1, y + 8, ix::LASER_LIGHT);
    c.rect(x, y + 6, w, 1, ix::STEEL_DARK);
}

/// A dark steel fist with gold knuckles under a gauntlet.
fn fist(c: &mut Canvas, x: i32, y: i32, w: i32) {
    dark_pod(c, x, y, w, 5);
    for k in [2, 5, 8] {
        c.px(x + k, y + 1, ix::GOLD);
    }
}

impl Bera {
    pub fn new() -> Self {
        let head = bera_head(true);
        Self {
            portrait: head.clone(),
            head,
            head_dead: bera_head(false),
            torso: bera_torso(false),
            torso_aim: bera_torso(true),
            legs: bera_legs(),
            core: [bera_core(false), bera_core(true)],
            smoke: drawing(Size::S8x8, 1, 1, &["..gg..", ".gSSg.", "gSSSgg", "gSggGg", ".gGGg.", "..gg.."]),
            charge: bera_charging(),
        }
    }
}

/// Round ears with steel linings, the armoured dome with the visor band, the vented
/// snout with its magenta lamp, and the steel strap down to the far shoulder.
fn bera_head(lit: bool) -> SpriteVram {
    let (x, y) = BERA_HEAD_AT;
    let mut c = Canvas::at(Size::S32x32, x, y);
    for ex in [22, 40] {
        red(&mut c, ex, 6, 5, 5);
        c.ellipse(ex, 6, 3, 3, ix::OUTLINE);
        c.ellipse(ex, 6, 2, 2, ix::STEEL);
        c.px(ex - 1, 5, ix::STEEL_LIGHT);
        c.px(ex + 1, 7, ix::STEEL_DARK);
        c.px(ex + 2, 6, ix::STEEL_DARK);
        c.px(ex, 8, ix::STEEL_DARK);
    }
    // Jaw, then the dome with a highlight on its upper left.
    red(&mut c, 32, 18, 12, 6);
    red(&mut c, 31, 13, 12, 10);
    c.rect(25, 4, 6, 1, ix::ORANGE);
    c.rect(23, 5, 3, 1, ix::ORANGE);
    c.rect(22, 6, 1, 3, ix::ORANGE);
    c.line(36, 17, 40, 19, ix::RED_DARK);
    // Visor.
    c.rect(20, 11, 18, 5, ix::OUTLINE);
    if lit {
        c.rect(21, 12, 16, 2, ix::GOLD);
        c.rect(21, 14, 16, 1, ix::ORANGE);
        c.rect(23, 12, 4, 1, ix::WHITE);
    } else {
        c.rect(21, 12, 16, 3, ix::STEEL_DARK);
        c.rect(21, 12, 16, 1, ix::STEEL);
    }
    // Snout: a vented steel muzzle pointing ahead.
    steel_pod(&mut c, 17, 17, 14, 8);
    for vx in [20, 23, 26] {
        c.rect(vx, 19, 1, 4, ix::OUTLINE);
        c.px(vx + 1, 19, ix::STEEL_LIGHT);
    }
    let (lamp, glint) = if lit { (ix::LASER, ix::LASER_LIGHT) } else { (ix::STEEL_DARK, ix::STEEL) };
    c.rect(29, 20, 2, 2, lamp);
    c.px(29, 20, glint);
    // Steel strap from the neck over the far shoulder.
    let strap = [ix::OUTLINE, ix::STEEL_LIGHT, ix::STEEL_LIGHT, ix::STEEL, ix::STEEL, ix::STEEL_DARK, ix::OUTLINE];
    for (i, colour) in (0..).zip(strap) {
        c.line(31, 21 + i, 44, 15 + i, colour);
    }
    c.line(44, 15, 47, 18, ix::OUTLINE);
    c.line(44, 21, 47, 18, ix::OUTLINE);
    c.line(44, 16, 46, 18, ix::STEEL);
    c.line(44, 20, 46, 18, ix::STEEL_DARK);
    c.rect(44, 17, 2, 3, ix::STEEL);
    c.vram()
}

/// The far gauntlet, steel joints, barrel chest with the empty core socket, the striped
/// far shoulder pad, the near shoulder and the near gauntlet, hanging or aimed.
fn bera_torso(aim: bool) -> SpriteVram {
    let (x, y) = BERA_TORSO_AT;
    let mut c = Canvas::at(Size::S64x32, x, y);
    gauntlet(&mut c, 47, 34, 13, 16, 55);
    fist(&mut c, 48, 49, 11);
    dark_pod(&mut c, 13, 30, 9, 10);
    dark_pod(&mut c, 42, 30, 9, 10);
    // Belly, then the chest over it.
    red(&mut c, 32, 39, 12, 6);
    red(&mut c, 32, 33, 16, 10);
    c.ellipse(32, 40, 11, 4, ix::RED);
    c.ellipse(33, 41, 11, 4, ix::RED_DARK);
    c.ellipse(32, 40, 10, 3, ix::RED);
    c.line(32, 24, 32, 28, ix::RED_DARK);
    c.rect(23, 25, 7, 1, ix::ORANGE);
    c.rect(20, 26, 4, 1, ix::ORANGE);
    c.rect(19, 27, 1, 3, ix::ORANGE);
    red(&mut c, 49, 28, 7, 6);
    hazard_stripes(&mut c, 45, 28, 7, Some(ix::ORANGE));
    // Core socket: the core sprite puts the star in it.
    c.ellipse(28, 37, 6, 6, ix::OUTLINE);
    c.ellipse(28, 37, 5, 5, ix::STEEL_DARK);
    c.ellipse(27, 36, 4, 4, ix::STEEL);
    c.ellipse(28, 37, 3, 3, ix::STEEL_DARK);
    // Near shoulder.
    red(&mut c, 16, 29, 7, 6);
    c.rect(13, 25, 5, 1, ix::ORANGE);
    c.rect(11, 26, 2, 2, ix::ORANGE);
    if aim {
        // Gauntlet raised level, its light the muzzle.
        steel_pod(&mut c, -1, 30, 16, 10);
        c.rect(0, 32, 2, 6, ix::LASER);
        c.px(0, 33, ix::LASER_LIGHT);
        c.px(0, 36, ix::LASER_LIGHT);
        c.rect(1, 35, 13, 1, ix::STEEL_DARK);
        c.ellipse(14, 40, 3, 2, ix::OUTLINE);
        c.ellipse(14, 40, 2, 1, ix::STEEL_DARK);
    } else {
        gauntlet(&mut c, 3, 34, 14, 16, 4);
        fist(&mut c, 4, 49, 12);
    }
    c.vram()
}

/// Steel hips, round red thighs and boots with gold toe caps.
fn bera_legs() -> SpriteVram {
    let (x, y) = BERA_LEGS_AT;
    let mut c = Canvas::at(Size::S64x32, x, y);
    c.block(22, 42, 20, 6, ix::STEEL_DARK, ix::OUTLINE);
    c.rect(23, 43, 18, 1, ix::STEEL);
    for x in [13, 38] {
        red(&mut c, x + 7, 50, 6, 5);
        c.rect(x + 3, 47, 4, 1, ix::ORANGE);
        c.px(x + 2, 48, ix::ORANGE);
        red_pod(&mut c, x, 54, 14, 10);
        c.block(x + 1, 56, 12, 5, ix::GOLD, ix::OUTLINE);
        c.rect(x + 2, 59, 10, 1, ix::ORANGE);
        c.px(x + 2, 57, ix::WHITE);
        c.rect(x + 5, 58, 4, 1, ix::OUTLINE);
        c.rect(x + 2, 55, 9, 1, ix::ORANGE);
        c.rect(x, 63, 14, 1, ix::OUTLINE);
    }
    c.vram()
}

/// The four-pointed star in the chest core; `hot` when flaring.
fn bera_core(hot: bool) -> SpriteVram {
    let (x, y) = BERA_CORE_AT;
    let mut c = Canvas::at(Size::S8x8, x, y);
    let (inner, outer) = if hot { (ix::WHITE, ix::GOLD) } else { (ix::GOLD, ix::ORANGE) };
    let (cx, cy) = (28, 37);
    for d in -3..=3 {
        c.px(cx + d, cy, outer);
        c.px(cx, cy + d, outer);
    }
    for d in -2..=2 {
        c.px(cx + d, cy, inner);
        c.px(cx, cy + d, inner);
    }
    c.rect(cx - 1, cy - 1, 3, 3, inner);
    c.px(cx, cy, ix::WHITE);
    c.vram()
}

/// Hunched low behind the head, the near gauntlet planted like a forepaw, thrusters
/// streaking out behind.
fn bera_charging() -> SpriteVram {
    let mut c = Canvas::new(Size::S64x32);
    for (y, len) in [(6, 9), (11, 12), (16, 8)] {
        c.rect(64 - len, y, len, 2, ix::ORANGE);
        c.rect(64 - len + 3, y, len - 4, 1, ix::GOLD);
    }
    // Rear thigh and boot.
    red_pod(&mut c, 40, 21, 12, 8);
    red_pod(&mut c, 42, 24, 14, 8);
    c.block(43, 25, 12, 4, ix::GOLD, ix::OUTLINE);
    c.rect(46, 26, 6, 2, ix::OUTLINE);
    // Hunched back and the far shoulder pad.
    red(&mut c, 36, 14, 17, 10);
    red(&mut c, 44, 7, 7, 5);
    hazard_stripes(&mut c, 40, 7, 6, None);
    // Near gauntlet.
    steel_pod(&mut c, 8, 19, 16, 13);
    c.rect(9, 22, 3, 7, ix::LASER);
    c.px(10, 23, ix::LASER_LIGHT);
    c.px(10, 27, ix::LASER_LIGHT);
    c.rect(8, 31, 16, 1, ix::OUTLINE);
    c.rect(9, 30, 14, 1, ix::STEEL_DARK);
    for k in [13, 16, 19] {
        c.px(k, 30, ix::GOLD);
    }
    c.vram()
}
