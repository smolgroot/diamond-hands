//! Level 1, the Canyon: sky and mesa backdrop, desert terrain tiles, the villains and
//! Sheriff Ursa.

use agb::display::{
    Palette16, Rgb15,
    object::{Size, SpriteVram},
    tiled::RegularBackground,
};
use diamond_hands_logic::{Level, Tile};

use super::{Canvas, TileCache, drawing, ix, rgb, shade};

/// Backdrop layers repeat every 256 px as they scroll.
pub const MAP_TILES: i32 = 32;
const MAP_PX: i32 = MAP_TILES * 8;
/// The backdrop is painted down to here; terrain covers the rest.
const HORIZON: i32 = 136;
const SKY_ROWS: i32 = HORIZON / 8;

/// Backdrop (BG palette 0) indices. Index 0 is the backdrop colour.
mod sky_ix {
    pub const SKY: [u8; 4] = [1, 2, 3, 4];
    pub const SUN: u8 = 5;
    pub const STAR: u8 = 6;
    pub const MESA: u8 = 7;
    pub const MESA_DARK: u8 = 8;
    pub const MESA_LIT: u8 = 9;
}

/// One backdrop palette per time of day: high noon, sunset, and a moonlit night. Stars
/// share the top sky colour except at night.
pub static SKY_PALETTES: [Palette16; 3] = [
    sky_palette([
        rgb(12, 20, 31),
        rgb(17, 24, 31),
        rgb(23, 27, 30),
        rgb(30, 28, 22),
        rgb(31, 31, 22),
        rgb(12, 20, 31),
        rgb(23, 12, 6),
        rgb(17, 8, 4),
        rgb(29, 18, 9),
    ]),
    sky_palette([
        rgb(13, 7, 19),
        rgb(24, 9, 15),
        rgb(31, 15, 7),
        rgb(31, 24, 10),
        rgb(31, 30, 17),
        rgb(13, 7, 19),
        rgb(12, 4, 6),
        rgb(8, 2, 4),
        rgb(21, 8, 6),
    ]),
    sky_palette([
        rgb(1, 1, 6),
        rgb(3, 3, 10),
        rgb(6, 5, 15),
        rgb(11, 7, 17),
        rgb(28, 28, 23),
        rgb(28, 28, 31),
        rgb(5, 3, 8),
        rgb(3, 2, 5),
        rgb(9, 6, 12),
    ]),
];

const fn sky_palette(c: [Rgb15; 9]) -> Palette16 {
    let black = rgb(0, 0, 0);
    Palette16::new([
        c[0], c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7], c[8], black, black, black, black,
        black, black,
    ])
}

/// Terrain (BG palette 1) indices.
mod ground_ix {
    pub const OUTLINE: u8 = 1;
    pub const SAND_LIGHT: u8 = 2;
    pub const SAND: u8 = 3;
    pub const SAND_DARK: u8 = 4;
    pub const ROCK_LIGHT: u8 = 5;
    pub const ROCK: u8 = 6;
    pub const ROCK_DARK: u8 = 7;
    pub const WOOD_LIGHT: u8 = 8;
    pub const WOOD: u8 = 9;
    pub const WOOD_DARK: u8 = 10;
    pub const CACTUS_LIGHT: u8 = 11;
    pub const CACTUS: u8 = 12;
    pub const CACTUS_DARK: u8 = 13;
    pub const SPINE: u8 = 14;
}

const GROUND_COLOURS: [(u16, u16, u16); 15] = [
    (0, 0, 0),
    (6, 3, 2),
    (31, 26, 16),
    (28, 20, 11),
    (22, 14, 7),
    (27, 14, 8),
    (21, 9, 5),
    (13, 5, 3),
    (27, 19, 10),
    (20, 12, 5),
    (12, 7, 3),
    (16, 26, 10),
    (8, 19, 7),
    (4, 11, 5),
    (30, 30, 24),
];

/// Terrain palette for a time of day: full sun, a warm dusk, a dim night.
pub fn ground_palette(stage: u8) -> Palette16 {
    let (num, den) = [(1, 1), (4, 5), (1, 2)][stage.min(2) as usize];
    let mut colours = [rgb(0, 0, 0); 16];
    for (out, &c) in colours.iter_mut().zip(GROUND_COLOURS.iter()) {
        *out = shade(c, num, den);
    }
    Palette16::new(colours)
}

/// 4x4 ordered-dither thresholds, 0..16.
const BAYER: [[i32; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// Centre and radius; right of every menu line so text never sits on it.
const SUN: (i32, i32, i32) = (208, 50, 11);

/// Static sky: a dithered four-band gradient, the sun (the moon at night) and stars.
pub fn paint_sky(bg: &mut RegularBackground) {
    let pixel = |x: i32, y: i32| {
        let (sx, sy, r) = SUN;
        let (dx, dy) = (x - sx, y - sy);
        if dx * dx + dy * dy <= r * r {
            return sky_ix::SUN;
        }
        let t = y * 4 * 16 / HORIZON;
        let band = (t / 16 + (BAYER[(y % 4) as usize][(x % 4) as usize] < t % 16) as i32).min(3);
        if band == 0 && (x * 37 + y * 101 + x * y * 7) % 97 == 0 {
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
    // Below the horizon only terrain or pits show; fill with the darkest sky band so a
    // pit looks like a drop into shadow.
    for ty in SKY_ROWS..20 {
        for tx in 0..MAP_TILES {
            cache.paint(bg, (tx, ty), 0, &|_, _| sky_ix::MESA_DARK);
        }
    }
}

/// Flat-topped mesas and buttes: (centre x, half the top's width, top y).
const MESAS: [(i32, i32, i32); 5] = [
    (26, 13, 86),
    (70, 4, 100),
    (134, 22, 72),
    (196, 7, 92),
    (230, 3, 104),
];

/// Skyline height at `x`: rolling foothills with mesas rising out of them.
fn skyline(x: i32) -> (i32, i32) {
    let x = x.rem_euclid(MAP_PX);
    let wave = (x % 64 - 32).abs();
    let hills = 112 + wave / 4;
    let mut top = hills;
    for (cx, half, peak) in MESAS {
        let d = (x - cx).abs().min(MAP_PX - (x - cx).abs());
        let flare = (d - half).max(0);
        top = top.min(peak + flare * flare / 3 + flare);
    }
    (top, hills)
}

/// Mesas, scrolled slowly for parallax. Strata stripe the cliffs; the sunward (right)
/// faces catch the light and the left faces fall into shadow.
pub fn paint_mesas(bg: &mut RegularBackground) {
    // One skyline lookup per column instead of three per pixel.
    let table: [(i32, i32); MAP_PX as usize + 2] = core::array::from_fn(|i| skyline(i as i32 - 1));
    let at = |x: i32| table[(x + 1) as usize];
    let pixel = |x: i32, y: i32| {
        let (top, hills) = at(x);
        if y < top {
            return 0;
        }
        if y == top || at(x + 1).0 > y {
            sky_ix::MESA_LIT
        } else if at(x - 1).0 > y + 1 || (y < hills && (y + (x / 24) % 2) % 6 == 0) {
            sky_ix::MESA_DARK
        } else {
            sky_ix::MESA
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
/// cracks and strata). Computing that key is cheap, so the renderer builds each
/// distinct tile once: the GBA has no divide instruction, and per-pixel patterns are
/// too slow to redo for every streamed cell.
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

/// Scenery and small props, drawn from 8x8 character grids.
const GRASS_ART: [&[u8; 8]; 8] = [
    b"........",
    b"........",
    b"........",
    b"...k....",
    b".k.ky.k.",
    b"..kyk.yk",
    b".kykykyk",
    b"kyyyyyyk",
];
const PEBBLES_ART: [&[u8; 8]; 8] = [
    b"........",
    b"........",
    b"........",
    b"........",
    b"........",
    b"....kk..",
    b".kk.klsk",
    b"klskklsk",
];
const SKULL_ART: [&[u8; 8]; 8] = [
    b"........",
    b"........",
    b"k......k",
    b"kk.ww.kk",
    b".kwwwwk.",
    b"..wkkw..",
    b"..wwww..",
    b"...ww...",
];
const CACTUS_ART: [&[u8; 8]; 8] = [
    b"...K....",
    b"..KgK.K.",
    b"K.KgKKgK",
    b"gKKgKgK.",
    b"KgggggK.",
    b".KgGgK..",
    b".KgGgK.w",
    b"wKgGgKw.",
];

fn stamp(px: &mut [u8; 64], art: &[&[u8; 8]; 8], flip: bool) {
    use ground_ix::*;
    for (y, row) in art.iter().enumerate() {
        for x in 0..8 {
            px[y * 8 + x] = match row[if flip { 7 - x } else { x }] {
                b'k' => OUTLINE,
                b'K' => CACTUS_DARK,
                b'y' => WOOD_LIGHT,
                b'l' => ROCK_LIGHT,
                b's' => ROCK,
                b'w' => SPINE,
                b'g' => CACTUS,
                b'G' => CACTUS_LIGHT,
                _ => 0,
            };
        }
    }
}

/// Pixels for a [`terrain_key`]. Rock: a sandy crust on exposed tops, strata and
/// cracks below that darken with depth, lit right faces, shaded left faces, shadowed
/// undersides and rounded outer corners. Boulders, wooden planks, cactus, and scenery.
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
                *p = if boulder {
                    // Weathered boulder: light on top-left, shaded bottom-right.
                    if (up && y == 0) || (left && x == 0) || (right && x == 7) || (down && y == 7) {
                        OUTLINE
                    } else if (x + y) < 5 && (up || left) {
                        SAND_LIGHT
                    } else if (x + y) > 9 || speck {
                        ROCK
                    } else {
                        ROCK_LIGHT
                    }
                } else if up && y == 0 {
                    OUTLINE
                } else if up && y <= 2 {
                    SAND_LIGHT
                } else if up && y <= 4 {
                    if speck { SAND_DARK } else { SAND }
                } else if up && y == 5 {
                    SAND_DARK
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
            for x in 0..8 {
                px[x] = OUTLINE;
                px[8 + x] = WOOD_LIGHT;
                px[16 + x] = WOOD;
                px[24 + x] = WOOD_DARK;
                px[32 + x] = OUTLINE;
            }
            px[16 + 1] = OUTLINE;
            px[16 + 6] = OUTLINE;
            if col4 % 2 == 0 {
                // A bracket under every other board.
                for y in 5..8 {
                    px[y * 8 + 3] = WOOD_DARK;
                    px[y * 8 + 4] = OUTLINE;
                }
            }
        }
        3 => stamp(&mut px, &CACTUS_ART, col4 % 2 == 1),
        5 => stamp(&mut px, &GRASS_ART, col4 % 2 == 1),
        6 => stamp(&mut px, &PEBBLES_ART, col4 >= 2),
        7 => stamp(&mut px, &SKULL_ART, false),
        _ => {}
    }
    px
}

/// The Canyon's villains, their shots, and the boss.
pub struct Villains {
    pub bandit: [SpriteVram; 3],
    pub snake: [SpriteVram; 2],
    pub vulture: [SpriteVram; 2],
    pub tumbleweed: SpriteVram,
    pub bullet: SpriteVram,
    pub dust: SpriteVram,
    pub shockwave: SpriteVram,
    pub candle: SpriteVram,
    pub ursa: Ursa,
}

/// Bandit Bear, facing left, feet on row 31. Hat, bandana, revolver.
const BANDIT: [&str; 18] = [
    "....kkkkkkk.....",
    "...kHHHHHHHk....",
    "...kHHyyyHHk....",
    ".kkkkkkkkkkkkk..",
    "..khhhhhhhhhk...",
    "..khkhhhkhhhk...",
    ".kllhhhhhhhhk...",
    "kkllkhhhhhhhk...",
    ".klllhhhhhhk....",
    "..krrrrrrrrk....",
    "...krrrrrrk.....",
    "..khhhhhhhhhk...",
    "kgghhllllhhhk...",
    "kgkhhllllhhhk...",
    ".kkhhllllhhhk...",
    "..khhhllllhhk...",
    "..khhhhhhhhhk...",
    "...khhhhhhhk....",
];
const BANDIT_LEGS: [[&str; 4]; 2] = [
    ["...khhk.khhk....", "...khhk.khhk....", "...kHHk.kHHk....", "..kkkkk.kkkkk..."],
    ["..khhk...khhk...", "..khhk...khhk...", "..kHHk...kHHk...", ".kkkkk...kkkkk.."],
];
/// The gun arm raised for a shot, over rows 11..14.
const BANDIT_AIM: [&str; 3] = ["gggkhhhhhhhhk...", "kgkkhllllhhhk...", ".k.khllllhhhk..."];

const SNAKE: [[&str; 9]; 2] = [
    [
        "...kkk..........",
        "..kyyyk.........",
        ".kkyyyyk........",
        "rkyyyyyk........",
        "..kkyyk..kkkk...",
        "...kyyyykyYyyk..",
        "..kyYyyyyyyYyk.k",
        "...kyyyyyYyykkLk",
        "....kkkkkkkkkLk.",
    ],
    [
        "................",
        "................",
        "..kkk...........",
        ".kyyykkk........",
        "rkyyyyyykkkkk...",
        "..kkyyyyyyYyyk..",
        "....kkyYyyyyYyk.",
        "...kyyyyYyyykkLk",
        "...kkkkkkkkkkLk.",
    ],
];

const VULTURE: [[&str; 10]; 2] = [
    [
        "..kk........kk..",
        ".kLLk......kLLk.",
        ".kLLLk....kLLLk.",
        "..kLLLkkkkLLLk..",
        "...kLLkkkkLLk...",
        "kkkrrkkkkkkk....",
        "Yyrrrkllkkkkk...",
        "kkkrrkkkkkkkkk..",
        "....kkkkkkkkk...",
        ".....kk..kk.....",
    ],
    [
        "................",
        "................",
        "................",
        "....kkkkkkkk....",
        "kkkrrkkkkkkkk...",
        "Yyrrrkllkkkkk...",
        "kkkrrkkkkkkkkk..",
        "..kLLLkkkkkLLLk.",
        ".kLLLk.kk.kLLLk.",
        "kLLk........kLLk",
    ],
];

const TUMBLEWEED: [&str; 12] = [
    "....kkkkk...",
    "..kkHLHHLkk.",
    ".kHLkHHkLHk.",
    "kHkLHLLHLkHk",
    "kLHHkHHkHHLk",
    "kHLHLkkLHLHk",
    "kHHkLHHLkHHk",
    "kLHHkHHHLkLk",
    ".kHLHkHLkHk.",
    ".kkHLHHLHkk.",
    "..kkHLLHkk..",
    "....kkkk....",
];

const SHOCKWAVE: [&str; 12] = [
    "....L.....",
    "...LlL..L.",
    "..LllL.LlL",
    ".LlllLLllL",
    ".LllllllL.",
    "LlllLlllL.",
    "LllLLlllLL",
    "LlLlllLllL",
    "LllllllllL",
    "LLlllLlllL",
    ".LLLLLLLL.",
    "..........",
];

impl Villains {
    pub fn new() -> Self {
        Self {
            bandit: [bandit(0, false), bandit(1, false), bandit(0, true)],
            snake: SNAKE.map(|rows| drawing(Size::S16x16, 0, 16 - rows.len() as i32, &rows)),
            vulture: VULTURE.map(|rows| drawing(Size::S16x16, 0, 3, &rows)),
            tumbleweed: drawing(Size::S16x16, 2, 2, &TUMBLEWEED),
            bullet: drawing(Size::S8x8, 1, 2, &[".kkkk.", "kyyyRk", "kYYYRk", ".kkkk."]),
            dust: drawing(
                Size::S8x8,
                0,
                0,
                &["..LL....", ".LllL.L.", "LllllLlL", "LlLlllL.", ".LllllL.", "LlLLlllL", ".L..LL..", "........"],
            ),
            shockwave: drawing(Size::S16x16, 3, 4, &SHOCKWAVE),
            candle: drawing(
                Size::S8x16,
                1,
                0,
                &[
                    "..k...", "..k...", "kkkkkk", "krrrRk", "krwrRk", "krwrRk", "krrrRk", "krrrRk",
                    "krrrRk", "krrrRk", "krrrRk", "kkkkkk", "..k...", "..k...",
                ],
            ),
            ursa: Ursa::new(),
        }
    }
}

fn bandit(step: usize, aim: bool) -> SpriteVram {
    let mut c = Canvas::new(Size::S16x32);
    let top = 32 - BANDIT.len() as i32 - 4;
    c.art(0, top, &BANDIT);
    if aim {
        c.art(0, top + 11, &BANDIT_AIM);
    }
    c.art(0, 28, &BANDIT_LEGS[step]);
    c.vram()
}

/// Sheriff Ursa, facing left. Standing poses are 64x64 with feet on row 63; the charge
/// is 64x32 with feet on row 31.
pub struct Ursa {
    pub stand: SpriteVram,
    pub aim: SpriteVram,
    pub dizzy: SpriteVram,
    pub charge: SpriteVram,
    /// Little stars circling a stunned sheriff.
    pub star: SpriteVram,
}

impl Ursa {
    fn new() -> Self {
        Self {
            stand: ursa_standing(false, false),
            aim: ursa_standing(true, false),
            dizzy: ursa_standing(false, true),
            charge: ursa_charging(),
            star: drawing(Size::S8x8, 1, 1, &["..y..", ".yyy.", "yywyy", ".yyy.", ".y.y."]),
        }
    }
}

/// Big sheriff's hat, brim at `brim_y`, crown centred on `cx`.
fn sheriff_hat(c: &mut Canvas, cx: i32, brim_y: i32) {
    c.block(cx - 19, brim_y, 38, 4, ix::FUR_DARK, ix::OUTLINE);
    c.block(cx - 11, brim_y - 9, 22, 10, ix::FUR_DARK, ix::OUTLINE);
    c.rect(cx - 10, brim_y - 2, 20, 2, ix::GOLD_DARK);
    // Brim curls up at the ends.
    c.rect(cx - 20, brim_y - 2, 3, 3, ix::OUTLINE);
    c.rect(cx + 17, brim_y - 2, 3, 3, ix::OUTLINE);
}

/// Five-point-ish sheriff's star.
fn badge(c: &mut Canvas, x: i32, y: i32) {
    c.art(x, y, &["...k...", "..kyk..", "kkkyykk", "kyyyyyk", ".kyyyk.", "kyykyyk", "kk...kk"]);
}

fn ursa_standing(aim: bool, dizzy: bool) -> SpriteVram {
    let mut c = Canvas::new(Size::S64x64);
    // Legs and paws.
    c.block(19, 50, 11, 14, ix::FUR, ix::OUTLINE);
    c.block(34, 50, 11, 14, ix::FUR, ix::OUTLINE);
    c.rect(19, 61, 11, 3, ix::OUTLINE);
    c.rect(34, 61, 11, 3, ix::OUTLINE);
    // Body, belly and badge.
    c.blob(32, 40, 15, 13, ix::FUR);
    c.ellipse(31, 43, 9, 9, ix::BONE);
    c.rect(22, 47, 20, 2, ix::FUR_DARK);
    c.rect(29, 47, 4, 2, ix::GOLD);
    badge(&mut c, 34, 33);
    // Arms: the left one points the revolver when aiming.
    if aim {
        c.block(6, 34, 18, 7, ix::FUR, ix::OUTLINE);
        c.block(0, 31, 10, 5, ix::STEEL, ix::OUTLINE);
        c.rect(7, 35, 3, 4, ix::OUTLINE);
        c.px(2, 32, ix::WHITE);
    } else {
        c.block(15, 34, 7, 16, ix::FUR, ix::OUTLINE);
        c.block(13, 46, 6, 8, ix::STEEL, ix::OUTLINE);
    }
    c.block(43, 34, 7, 15, ix::FUR, ix::OUTLINE);
    // Head, ears, muzzle.
    c.blob(21, 12, 3, 3, ix::FUR);
    c.blob(40, 12, 3, 3, ix::FUR);
    c.blob(30, 20, 12, 10, ix::FUR);
    c.blob(23, 24, 7, 5, ix::BONE);
    c.rect(17, 21, 4, 3, ix::OUTLINE);
    c.rect(19, 27, 7, 1, ix::OUTLINE);
    if dizzy {
        for (x, y) in [(25, 16), (34, 16)] {
            c.px(x, y, ix::OUTLINE);
            c.px(x + 2, y, ix::OUTLINE);
            c.px(x + 1, y + 1, ix::OUTLINE);
            c.px(x, y + 2, ix::OUTLINE);
            c.px(x + 2, y + 2, ix::OUTLINE);
        }
    } else {
        // Glaring eyes under heavy brows.
        c.rect(24, 17, 3, 2, ix::OUTLINE);
        c.rect(33, 17, 3, 2, ix::OUTLINE);
        c.px(25, 17, ix::WHITE);
        c.px(34, 17, ix::WHITE);
        c.rect(23, 15, 5, 1, ix::OUTLINE);
        c.rect(32, 14, 5, 1, ix::OUTLINE);
    }
    // Bandana knotted at the neck.
    c.rect(19, 29, 24, 3, ix::RED);
    c.rect(19, 31, 24, 1, ix::RED_DARK);
    c.rect(38, 31, 4, 3, ix::RED);
    sheriff_hat(&mut c, 30, 9);
    c.vram()
}

fn ursa_charging() -> SpriteVram {
    let mut c = Canvas::new(Size::S64x32);
    for x in [16, 26, 38, 48] {
        c.block(x, 22, 8, 10, ix::FUR, ix::OUTLINE);
        c.rect(x, 29, 8, 3, ix::OUTLINE);
    }
    c.blob(36, 16, 22, 9, ix::FUR);
    c.rect(18, 22, 36, 2, ix::FUR_DARK);
    c.blob(14, 16, 10, 8, ix::FUR);
    c.blob(7, 19, 6, 4, ix::BONE);
    c.rect(1, 17, 3, 3, ix::OUTLINE);
    c.rect(10, 13, 3, 2, ix::OUTLINE);
    c.rect(9, 11, 5, 1, ix::OUTLINE);
    c.rect(14, 22, 12, 3, ix::RED);
    // Hat pushed back by the wind.
    c.block(16, 3, 20, 3, ix::FUR_DARK, ix::OUTLINE);
    c.block(21, 0, 12, 4, ix::FUR_DARK, ix::OUTLINE);
    // Dust kicked up behind.
    for (x, y, r) in [(60, 26, 3), (56, 29, 2), (62, 20, 2)] {
        c.ellipse(x, y, r, r, ix::BONE_DARK);
    }
    c.vram()
}
