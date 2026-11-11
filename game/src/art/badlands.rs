//! Level 1, the Badlands: a dystopian dusk over a ruined city, mossy cliffs with
//! crystal veins, steel girders and energy spikes; the cyber bots that guard it, and
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
    pub const SCRAP: u8 = 14;
    pub const ENERGY: u8 = 15;
}

const GROUND_COLOURS: [(u16, u16, u16); 16] = [
    (0, 0, 0),
    (3, 2, 7),
    (16, 28, 10),
    (8, 20, 8),
    (4, 11, 6),
    (16, 11, 22),
    (10, 6, 16),
    (5, 3, 9),
    (18, 26, 24),
    (8, 17, 16),
    (3, 8, 9),
    (22, 29, 31),
    (8, 17, 31),
    (4, 7, 20),
    (26, 24, 28),
    (31, 6, 22),
];

/// Terrain palette for a theme and stage: as drawn at dusk, dimmer at night, and
/// red-lit in the last stretch.
pub fn ground_palette(stage: u8, fence: u8) -> Palette16 {
    let (num, den, red) = [(1, 1, 0), (4, 5, 0), (3, 4, 4)][stage.min(2) as usize];
    let base = &GROUND_COLOURS;
    let mut colours = [rgb(0, 0, 0); 16];
    for (i, (out, &(r, g, b))) in colours.iter_mut().zip(base.iter()).enumerate() {
        // Energy spikes and crystals keep their glow.
        let (num, den) = if matches!(i, 11 | 12 | 15) { (1, 1) } else { (num, den) };
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

/// Number of distinct [`terrain_key`] values.
pub const TERRAIN_KEYS: usize = 8 * 3 * 16 * 4 * 3;

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
        _ => return 0,
    };
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
                b'w' => SCRAP,
                _ => 0,
            };
        }
    }
}

/// Pixels for a [`terrain_key`]. Rock: moss on exposed tops with drips over the edge,
/// purple strata and cracks that darken with depth, blue crystal veins, lit right faces,
/// shaded left faces, shadowed undersides and rounded outer corners. Crystal-studded
/// boulders, steel girders, energy spikes, and scenery.
pub fn terrain_pixels(key: usize) -> [u8; 64] {
    use ground_ix::*;
    let row3 = (key % 3) as i32;
    let col4 = (key / 3 % 4) as i32;
    let sides = key / 12 % 16;
    let depth = key / 192 % 3;
    let kind = key / 576;
    let (up, left, right, down) = (sides & 1 != 0, sides & 2 != 0, sides & 4 != 0, sides & 8 != 0);
    let mut px = [0u8; 64];
    match kind {
        1 | 4 => {
            let boulder = kind == 4;
            for (i, p) in px.iter_mut().enumerate() {
                let (x, y) = ((i % 8) as i32, (i / 8) as i32);
                // Row within the 24px strata cycle, and a sparse speckle pattern.
                let gy = row3 * 8 + y;
                let speck = ((col4 * 8 + x) * 7 + gy * 13 + x * y) % 23 == 0;
                let crack = col4 == 1 && row3 == 2 && matches!((x, y), (2, 1) | (3, 2) | (3, 3) | (4, 4) | (4, 5) | (5, 6));
                let vein = depth > 0 && col4 == 3 && row3 == 1;
                let edge = (up && y == 0) || (left && x == 0) || (right && x == 7) || (down && y == 7);
                *p = if boulder {
                    // A rock studded with crystal facets catching the light.
                    if edge {
                        OUTLINE
                    } else if (1..=3).contains(&x) && (1..=3).contains(&y) {
                        if x + y <= 3 { CRYSTAL_LIGHT } else { CRYSTAL }
                    } else if (5..=6).contains(&x) && y == 5 {
                        CRYSTAL_DARK
                    } else if (x + y) > 9 || speck {
                        ROCK
                    } else {
                        ROCK_LIGHT
                    }
                } else if up && y == 0 {
                    OUTLINE
                } else if up && y == 1 {
                    MOSS_LIGHT
                } else if up && y <= 3 {
                    if speck { MOSS_LIGHT } else { MOSS }
                } else if up && y == 4 && (x + col4) % 3 != 0 {
                    MOSS_DARK
                } else if up && y == 5 && (x + col4) % 3 == 1 {
                    // Moss dripping over the rock.
                    MOSS_DARK
                } else if left && x == 0 {
                    OUTLINE
                } else if left && x == 1 {
                    ROCK_DARK
                } else if right && x == 7 {
                    OUTLINE
                } else if right && x == 6 {
                    ROCK_LIGHT
                } else if down && y == 7 {
                    OUTLINE
                } else if down && y == 6 {
                    ROCK_DARK
                } else if crack {
                    OUTLINE
                } else if vein && matches!((x, y), (3, 3) | (4, 3) | (4, 4) | (5, 4)) {
                    if (x, y) == (3, 3) { CRYSTAL_LIGHT } else { CRYSTAL }
                } else if vein && matches!((x, y), (3, 4) | (5, 5)) {
                    CRYSTAL_DARK
                } else if depth == 2 {
                    // Deep rock: dark, with faint veins.
                    if (gy + x) % 7 == 0 || speck { ROCK } else { ROCK_DARK }
                } else if gy % 6 == 0 || speck {
                    ROCK_DARK
                } else if (gy + 3) % 6 == 0 && (col4 + x / 4) % 2 == 0 {
                    ROCK_LIGHT
                } else {
                    ROCK
                };
            }
            // Round off outer corners.
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
                    px[inner.1 * 8 + inner.0] = OUTLINE;
                }
            }
        }
        2 => {
            // A steel girder: rivets on the face, a truss underneath.
            for x in 0..8 {
                px[x] = OUTLINE;
                px[8 + x] = METAL_LIGHT;
                px[16 + x] = METAL;
                px[24 + x] = METAL_DARK;
                px[32 + x] = OUTLINE;
            }
            px[16 + 3] = METAL_LIGHT;
            px[24 + 3] = OUTLINE;
            for y in 5..8 {
                let x = if col4 % 2 == 0 { y - 4 } else { 11 - y };
                px[y * 8 + x] = METAL_DARK;
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
