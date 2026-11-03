//! Level 1, the Badlands: a dystopian dusk over a ruined city, mossy cliffs with
//! crystal veins, steel girders and energy spikes; the cyber bots that guard it, and
//! URSA-9000.

use agb::display::{
    Palette16, Rgb15,
    object::{Size, SpriteVram},
    tiled::RegularBackground,
};
use diamond_hands_logic::{Level, Tile};

use super::{Canvas, TileCache, drawing, ix, rgb};

/// Backdrop layers repeat every 256 px as they scroll.
pub const MAP_TILES: i32 = 32;
const MAP_PX: i32 = MAP_TILES * 8;
/// The sky is painted down to here; the city and terrain cover the rest.
const HORIZON: i32 = 136;
const SKY_ROWS: i32 = HORIZON / 8;

/// Backdrop (BG palette 0) indices. Index 0 is the backdrop colour.
mod sky_ix {
    pub const SKY: [u8; 4] = [1, 2, 3, 4];
    pub const MOON: u8 = 5;
    pub const STAR: u8 = 6;
    pub const TOWER: u8 = 7;
    pub const TOWER_DARK: u8 = 8;
    pub const TOWER_EDGE: u8 = 9;
    pub const WINDOW: u8 = 10;
    pub const WINDOW_HOT: u8 = 11;
    pub const CRATER: u8 = 12;
}

/// One backdrop palette per stage of the level: a violet dusk, a deep night, and the
/// city burning under a blood moon.
pub static SKY_PALETTES: [Palette16; 3] = [
    sky_palette([
        rgb(8, 5, 20),
        rgb(16, 8, 26),
        rgb(26, 10, 24),
        rgb(31, 16, 24),
        rgb(26, 24, 30),
        rgb(28, 26, 31),
        rgb(10, 6, 20),
        rgb(5, 3, 12),
        rgb(20, 10, 26),
        rgb(14, 28, 31),
        rgb(31, 8, 22),
        rgb(19, 17, 25),
    ]),
    sky_palette([
        rgb(2, 2, 8),
        rgb(5, 3, 14),
        rgb(10, 5, 20),
        rgb(16, 7, 24),
        rgb(24, 24, 28),
        rgb(28, 28, 31),
        rgb(5, 3, 12),
        rgb(2, 1, 6),
        rgb(12, 6, 20),
        rgb(14, 28, 31),
        rgb(31, 8, 22),
        rgb(16, 16, 22),
    ]),
    sky_palette([
        rgb(8, 1, 6),
        rgb(16, 2, 8),
        rgb(26, 6, 6),
        rgb(31, 14, 4),
        rgb(31, 20, 14),
        rgb(31, 20, 16),
        rgb(8, 2, 6),
        rgb(4, 1, 3),
        rgb(20, 6, 8),
        rgb(31, 24, 8),
        rgb(31, 6, 4),
        rgb(24, 12, 10),
    ]),
];

const fn sky_palette(c: [Rgb15; 12]) -> Palette16 {
    let black = rgb(0, 0, 0);
    Palette16::new([
        c[0], c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], c[8], c[9], c[10], c[11], black,
        black, black,
    ])
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

/// Terrain palette for a stage: as drawn at dusk, dimmer at night, and red-lit while
/// the city burns.
pub fn ground_palette(stage: u8) -> Palette16 {
    let (num, den, red) = [(1, 1, 0), (4, 5, 0), (3, 4, 4)][stage.min(2) as usize];
    let mut colours = [rgb(0, 0, 0); 16];
    for (i, (out, &(r, g, b))) in colours.iter_mut().zip(GROUND_COLOURS.iter()).enumerate() {
        // Energy spikes and crystals keep their glow.
        let (num, den) = if matches!(i, 11 | 12 | 15) { (1, 1) } else { (num, den) };
        *out = rgb((r * num / den + red).min(31), g * num / den, b * num / den);
    }
    Palette16::new(colours)
}

/// 4x4 ordered-dither thresholds, 0..16.
const BAYER: [[i32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// Moon centre and radius; right of every menu line so text never sits on it.
const MOON: (i32, i32, i32) = (212, 46, 12);
const CRATERS: [(i32, i32, i32); 3] = [(207, 42, 3), (216, 51, 3), (214, 38, 2)];

/// Static sky: a dithered four-band gradient, the cratered moon, and stars.
pub fn paint_sky(bg: &mut RegularBackground) {
    let pixel = |x: i32, y: i32| {
        let (mx, my, r) = MOON;
        let inside = |cx: i32, cy: i32, r: i32| (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r * r;
        if inside(mx, my, r) {
            return if CRATERS.iter().any(|&(cx, cy, cr)| inside(cx, cy, cr)) || inside(mx + 4, my + 4, r - 1) && !inside(mx, my, r - 1) {
                sky_ix::CRATER
            } else {
                sky_ix::MOON
            };
        }
        let t = y * 4 * 16 / HORIZON;
        let band = (t / 16 + (BAYER[(y % 4) as usize][(x % 4) as usize] < t % 16) as i32).min(3);
        if band <= 1 && (x * 37 + y * 101 + x * y * 7) % 89 == 0 {
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
}

/// Number of distinct [`terrain_key`] values.
pub const TERRAIN_KEYS: usize = 8 * 3 * 16 * 4 * 3;

/// What a terrain cell looks like depends only on its kind, how deep it sits below the
/// surface, which sides are open, and its column mod 4 and row mod 3 (for speckles,
/// cracks, crystal veins and strata). Computing that key is cheap, so the renderer
/// builds each distinct tile once: the GBA has no divide instruction, and per-pixel
/// patterns are too slow to redo for every streamed cell.
pub fn terrain_key(level: &Level, tx: i32, ty: i32) -> usize {
    let kind = match level.char_at(tx, ty) {
        b'#' => 1,
        b'=' => 2,
        b'^' => 3,
        b'r' => 4,
        b',' => 5,
        b'*' => 6,
        b'&' => 7,
        _ => return 0,
    };
    // Below the map counts as rock, so the screen's bottom edge gets no outline.
    let solid = |dx: i32, dy: i32| ty + dy >= level.height() || level.tile(tx + dx, ty + dy) == Tile::Solid;
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
/// Magenta energy flames over an emitter strip.
const SPIKES_ART: [&[u8; 8]; 8] = [
    b"..e.....",
    b"..e...e.",
    b".ewe..e.",
    b".ewe.ewe",
    b"ewwe.ewe",
    b"ewwweewe",
    b"SSSSSSSS",
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
        3 => stamp(&mut px, &SPIKES_ART, col4 % 2 == 1),
        5 => stamp(&mut px, &MOSS_ART, col4 % 2 == 1),
        6 => stamp(&mut px, &SCRAP_ART, col4 >= 2),
        7 => stamp(&mut px, &BOT_HEAD_ART, col4 % 2 == 1),
        _ => {}
    }
    px
}

/// The Badlands bots, their shots, and the boss.
pub struct Villains {
    /// Walk frames, then aiming.
    pub sentry: [SpriteVram; 3],
    /// Crouched, then lunging.
    pub crawler: [SpriteVram; 2],
    /// Rotor frames.
    pub drone: [SpriteVram; 2],
    pub mine: SpriteVram,
    pub plasma: SpriteVram,
    pub bomb: SpriteVram,
    pub shockwave: SpriteVram,
    pub missile: SpriteVram,
    pub ursa: Ursa,
}

/// Sentry bot, facing left: red armour, red visor, a steel arm cannon. The body is
/// drawn above the legs; the hitbox is the bottom 22 rows of a 16x32 sprite.
const SENTRY_BODY: [&str; 12] = [
    "....kkkkkkk.....",
    "...krrrrrrRk....",
    "..krkkkkkrRRk...",
    "..kkrwrrkrRRk...",
    "..krRRRRkrRRk...",
    "...kkkkkkkkk....",
    "..kSSkrrrrkSk...",
    "kkSggkrryrkgSk..",
    "kiSggkrrrrkgGk..",
    "kkSggkRRRRkgGk..",
    "..kkkkRRRRkkk...",
    "...kgGGGGGGk....",
];
const SENTRY_LEGS0: [&str; 6] = [
    "...krRk.krRk....",
    "...kRRk.kRRk....",
    "..kgGk...kgGk...",
    "..krrk...krrk...",
    ".kRRRk...kRRRk..",
    ".kkkkk...kkkkk..",
];
const SENTRY_LEGS1: [&str; 6] = [
    "..krRk...krRk...",
    "..kRRk...kRRk...",
    "..kgGk..kgGk....",
    ".krrk...krrk....",
    ".kRRRk.kRRRk....",
    ".kkkkk.kkkkk....",
];
const SENTRY_AIM: [&str; 3] = [
    "kkSggk",
    "ywSggk",
    "kkSggk",
];
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

/// Energy wave thrown along the ground by URSA-9000's slam.
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
            sentry: [sentry(&SENTRY_LEGS0, false), sentry(&SENTRY_LEGS1, false), sentry(&SENTRY_LEGS0, true)],
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
            ursa: Ursa::new(),
        }
    }
}

fn sentry(legs: &[&str], aim: bool) -> SpriteVram {
    let mut c = Canvas::new(Size::S16x32);
    let top = 32 - legs.len() as i32 - SENTRY_BODY.len() as i32;
    c.art(0, top, &SENTRY_BODY);
    if aim {
        c.art(0, top + 7, &SENTRY_AIM);
    }
    c.art(0, 32 - legs.len() as i32, legs);
    c.vram()
}

/// URSA-9000, facing left: a bear-shaped war mech in red armour with steel limbs, a
/// glowing visor, an arm cannon and back thrusters. Standing poses are 64x64 with feet
/// on row 63; the charge is 64x32 with feet on row 31.
pub struct Ursa {
    pub stand: SpriteVram,
    pub aim: SpriteVram,
    /// Laid over `stand` when stalled: a dead visor and venting smoke. Placed at
    /// [`DIZZY_AT`] in the standing sprite (mirror it when flipped).
    pub dizzy: SpriteVram,
    pub charge: SpriteVram,
    /// Sparks circling a stalled mech.
    pub star: SpriteVram,
    /// Her helmet, facing left, for the dialogue box.
    pub portrait: SpriteVram,
}

impl Ursa {
    fn new() -> Self {
        Self {
            stand: ursa_standing(false, false),
            aim: ursa_standing(true, false),
            dizzy: {
                let mut c = Canvas::new(Size::S32x16);
                let (x, y) = DIZZY_AT;
                // The visor, unlit, in overlay coordinates.
                c.rect(31 - 9 - x, 18 - 3 - y, 16, 4, ix::OUTLINE);
                c.rect(31 - 8 - x, 18 - 2 - y, 14, 2, ix::STEEL);
                c.rect(31 - 7 - x, 18 - 2 - y, 4, 1, ix::STEEL_DARK);
                c.rect(31 + 1 - x, 18 - 2 - y, 4, 1, ix::STEEL_DARK);
                for (sx, sy, r) in [(40, 7, 2), (44, 6, 2), (36, 6, 1)] {
                    c.ellipse(sx - x, sy - y, r, r, ix::STEEL);
                }
                c.vram()
            },
            charge: ursa_charging(),
            star: drawing(Size::S8x8, 1, 1, &["..y..", ".yoy.", "yowoy", ".yoy.", "..y.."]),
            portrait: {
                let mut c = Canvas::new(Size::S32x32);
                ursa_head(&mut c, 18, 18, true);
                c.vram()
            },
        }
    }
}

/// Bear-head helmet centred on `(cx, cy)`: round ears, armoured dome, visor slit, snout grill.
fn ursa_head(c: &mut Canvas, cx: i32, cy: i32, visor_lit: bool) {
    for ex in [cx - 10, cx + 9] {
        c.blob(ex, cy - 8, 4, 4, ix::RED);
        c.ellipse(ex, cy - 8, 2, 2, ix::STEEL);
    }
    c.blob(cx, cy, 12, 10, ix::RED);
    c.ellipse(cx + 2, cy - 4, 8, 4, ix::ORANGE);
    c.ellipse(cx + 2, cy - 3, 9, 4, ix::RED);
    // Visor slit.
    c.rect(cx - 9, cy - 3, 16, 4, ix::OUTLINE);
    let (lit, glow) = if visor_lit { (ix::GOLD, ix::ORANGE) } else { (ix::STEEL_DARK, ix::STEEL) };
    c.rect(cx - 8, cy - 2, 14, 2, glow);
    c.rect(cx - 7, cy - 2, 4, 1, lit);
    c.rect(cx + 1, cy - 2, 4, 1, lit);
    // Snout plate with a vent grill, pointing left.
    c.block(cx - 16, cy + 2, 12, 8, ix::STEEL_LIGHT, ix::OUTLINE);
    for x in [cx - 14, cx - 11, cx - 8] {
        c.rect(x, cy + 4, 1, 4, ix::STEEL_DARK);
    }
    c.rect(cx - 17, cy + 3, 3, 3, ix::OUTLINE);
}

/// Top-left of the stalled overlay within the 64x64 standing sprite.
pub const DIZZY_AT: (i32, i32) = (20, 4);

fn ursa_standing(aim: bool, stalled: bool) -> SpriteVram {
    let mut c = Canvas::new(Size::S64x64);
    // Back thrusters.
    c.block(48, 26, 8, 18, ix::STEEL, ix::OUTLINE);
    c.rect(49, 27, 2, 16, ix::STEEL_LIGHT);
    c.block(47, 43, 10, 4, ix::STEEL_DARK, ix::OUTLINE);
    // Piston legs and heavy feet.
    for x in [20, 35] {
        c.block(x + 2, 48, 7, 10, ix::STEEL, ix::OUTLINE);
        c.rect(x + 3, 49, 2, 8, ix::STEEL_LIGHT);
        c.block(x, 56, 12, 8, ix::RED, ix::OUTLINE);
        c.rect(x + 1, 57, 10, 2, ix::ORANGE);
        c.rect(x, 62, 12, 2, ix::OUTLINE);
    }
    // Armoured torso with a reactor core.
    c.blob(33, 38, 16, 12, ix::RED);
    c.ellipse(36, 32, 11, 4, ix::ORANGE);
    c.ellipse(36, 34, 12, 4, ix::RED);
    c.rect(19, 44, 28, 2, ix::RED_DARK);
    c.blob(32, 40, 5, 5, ix::STEEL_DARK);
    c.ellipse(32, 40, 3, 3, if stalled { ix::STEEL } else { ix::GOLD });
    c.ellipse(31, 39, 1, 1, ix::WHITE);
    // Warning stripes on the belt.
    for x in (21..45).step_by(4) {
        c.rect(x, 46, 2, 2, ix::GOLD);
    }
    // Arms: the left one is a cannon, raised and glowing when aiming.
    if aim {
        c.block(4, 32, 22, 9, ix::STEEL, ix::OUTLINE);
        c.rect(5, 33, 20, 2, ix::STEEL_LIGHT);
        c.block(0, 30, 7, 13, ix::STEEL_DARK, ix::OUTLINE);
        c.rect(1, 34, 3, 5, ix::ORANGE);
        c.rect(1, 35, 2, 3, ix::GOLD);
    } else {
        c.block(12, 34, 9, 18, ix::STEEL, ix::OUTLINE);
        c.rect(13, 35, 2, 16, ix::STEEL_LIGHT);
        c.block(10, 48, 12, 9, ix::STEEL_DARK, ix::OUTLINE);
        c.rect(11, 50, 3, 5, ix::ORANGE);
    }
    c.block(44, 34, 8, 14, ix::STEEL, ix::OUTLINE);
    c.block(43, 46, 10, 8, ix::RED, ix::OUTLINE);
    ursa_head(&mut c, 31, 18, !stalled);
    if stalled {
        // Vented smoke curling off the armour.
        for (x, y, r) in [(40, 6, 2), (44, 3, 2), (36, 2, 1)] {
            c.ellipse(x, y, r, r, ix::STEEL);
        }
    }
    c.vram()
}

fn ursa_charging() -> SpriteVram {
    let mut c = Canvas::new(Size::S64x32);
    // Thruster flames streaming out behind.
    for (y, len) in [(12, 10), (16, 13), (20, 9)] {
        c.rect(64 - len, y, len, 3, ix::ORANGE);
        c.rect(64 - len + 3, y + 1, len - 4, 1, ix::GOLD);
    }
    c.block(46, 10, 8, 14, ix::STEEL, ix::OUTLINE);
    // Treads.
    c.block(14, 24, 36, 8, ix::STEEL_DARK, ix::OUTLINE);
    for x in (16..48).step_by(5) {
        c.rect(x, 26, 2, 4, ix::STEEL);
    }
    // Hunched body and lowered head.
    c.blob(32, 16, 18, 9, ix::RED);
    c.ellipse(34, 11, 13, 3, ix::ORANGE);
    c.ellipse(34, 13, 14, 3, ix::RED);
    c.rect(16, 21, 32, 2, ix::RED_DARK);
    ursa_head(&mut c, 17, 15, true);
    c.vram()
}
