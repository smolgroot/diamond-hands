//! Pixel art drawn in code at boot: small ASCII drawings and shape primitives become
//! sprites and background tiles.

pub mod arrival;
pub mod badlands;
pub mod casino;
pub mod menu;
pub mod trenches;
pub mod ui;
pub mod wassie;

use alloc::{collections::BTreeMap, vec::Vec};

use agb::{
    ExternalAllocator,
    display::{
        Palette16, Rgb15,
        object::{DynamicSprite16, Size, SpriteVram},
        tiled::{DynamicTile16, RegularBackground, TileEffect},
    },
};

pub const fn rgb(r: u16, g: u16, b: u16) -> Rgb15 {
    Rgb15::new(r | (g << 5) | (b << 10))
}

/// Sprite palette indices for the bots, pickups, effects and HUD. The wassie has its
/// own palette (see `wassie.rs`). Colours follow the cover and the gameplay reference:
/// red and orange armour, steel, electric blue diamonds, violet, and magenta laser.
pub mod ix {
    pub const CLEAR: u8 = 0;
    pub const OUTLINE: u8 = 1;
    pub const WHITE: u8 = 2;
    pub const RED: u8 = 3;
    pub const RED_DARK: u8 = 4;
    pub const ORANGE: u8 = 5;
    pub const GOLD: u8 = 6;
    pub const STEEL_LIGHT: u8 = 7;
    pub const STEEL: u8 = 8;
    pub const STEEL_DARK: u8 = 9;
    pub const BLUE: u8 = 10;
    pub const CYAN: u8 = 11;
    pub const PURPLE: u8 = 12;
    pub const PURPLE_DARK: u8 = 13;
    pub const LASER: u8 = 14;
    pub const LASER_LIGHT: u8 = 15;
}

pub static SPRITE_PALETTE: Palette16 = Palette16::new([
    rgb(0, 0, 0),
    rgb(2, 2, 6),
    rgb(31, 31, 31),
    rgb(28, 5, 6),
    rgb(15, 2, 4),
    rgb(31, 15, 2),
    rgb(31, 26, 8),
    rgb(24, 27, 31),
    rgb(12, 15, 23),
    rgb(6, 7, 13),
    rgb(6, 14, 31),
    rgb(14, 28, 31),
    rgb(17, 6, 24),
    rgb(8, 3, 13),
    rgb(31, 6, 22),
    rgb(31, 19, 29),
]);

/// Characters used in the ASCII sprite drawings. `.` and anything unlisted is left
/// transparent.
const SPRITE_KEY: &[(u8, u8)] = &[
    (b'k', ix::OUTLINE),
    (b'w', ix::WHITE),
    (b'r', ix::RED),
    (b'R', ix::RED_DARK),
    (b'o', ix::ORANGE),
    (b'y', ix::GOLD),
    (b'S', ix::STEEL_LIGHT),
    (b'g', ix::STEEL),
    (b'G', ix::STEEL_DARK),
    (b'b', ix::BLUE),
    (b'i', ix::CYAN),
    (b'v', ix::PURPLE),
    (b'V', ix::PURPLE_DARK),
    (b'c', ix::LASER),
    (b'C', ix::LASER_LIGHT),
];

/// A sprite being drawn. The pixel buffer lives in EWRAM: agb's IWRAM allocator
/// reaches up into the stacks at the top of IWRAM, and a 64x64 buffer placed there was
/// overwritten by interrupt pushes before it reached VRAM.
pub struct Canvas {
    sprite: DynamicSprite16<ExternalAllocator>,
    w: i32,
    h: i32,
    /// Drawing coordinates of the sprite's top-left pixel; see [`Canvas::at`].
    origin: (i32, i32),
}

impl Canvas {
    pub fn new(size: Size) -> Self {
        let mut sprite = DynamicSprite16::new_in(size, ExternalAllocator);
        sprite.clear(ix::CLEAR as usize);
        let (w, h) = size.to_width_height();
        Self {
            sprite,
            w: w as i32,
            h: h as i32,
            origin: (0, 0),
        }
    }

    /// A canvas for one part of a bigger drawing, whose top-left sits at (x, y) in that
    /// drawing: everything is drawn in the drawing's coordinates and clipped to the part.
    pub fn at(size: Size, x: i32, y: i32) -> Self {
        let mut c = Self::new(size);
        c.origin = (x, y);
        c
    }

    pub fn px(&mut self, x: i32, y: i32, colour: u8) {
        let (x, y) = (x - self.origin.0, y - self.origin.1);
        if (0..self.w).contains(&x) && (0..self.h).contains(&y) {
            self.sprite.set_pixel(x as usize, y as usize, colour);
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, colour: u8) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.px(xx, yy, colour);
            }
        }
    }

    /// Filled rectangle with a 1px outline.
    pub fn block(&mut self, x: i32, y: i32, w: i32, h: i32, fill: u8, outline: u8) {
        self.rect(x, y, w, h, outline);
        self.rect(x + 1, y + 1, w - 2, h - 2, fill);
    }

    /// Filled ellipse centred at (cx, cy) with radii (rx, ry).
    pub fn ellipse(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, colour: u8) {
        for y in cy - ry..=cy + ry {
            for x in cx - rx..=cx + rx {
                let (dx, dy) = (x - cx, y - cy);
                if dx * dx * ry * ry + dy * dy * rx * rx <= rx * rx * ry * ry {
                    self.px(x, y, colour);
                }
            }
        }
    }

    /// Ellipse with a 1px outline.
    pub fn blob(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, fill: u8) {
        self.ellipse(cx, cy, rx + 1, ry + 1, ix::OUTLINE);
        self.ellipse(cx, cy, rx, ry, fill);
    }

    /// Cel-shaded ellipse lit from the upper left: an outline, then a `base` body with
    /// a `shade` rim on the lower right.
    pub fn ball(&mut self, cx: i32, cy: i32, rx: i32, ry: i32, base: u8, shade: u8) {
        self.ellipse(cx, cy, rx + 1, ry + 1, ix::OUTLINE);
        self.ellipse(cx, cy, rx, ry, shade);
        let (ox, oy) = ((rx / 5).max(1), (ry / 5).max(1));
        self.ellipse(cx - ox, cy - oy, rx - ox, ry - oy, base);
    }

    /// Cel-shaded box with rounded corners: an outline with the corners clipped, a
    /// `base` body, a `lit` edge on the top and left and a `shade` rim on the bottom
    /// and right.
    #[allow(clippy::too_many_arguments)]
    pub fn pod(&mut self, x: i32, y: i32, w: i32, h: i32, lit: u8, base: u8, shade: u8) {
        self.rect(x + 1, y, w - 2, h, ix::OUTLINE);
        self.rect(x, y + 1, w, h - 2, ix::OUTLINE);
        self.rect(x + 1, y + 1, w - 2, h - 2, shade);
        self.rect(x + 1, y + 1, w - 3, h - 3, base);
        self.rect(x + 2, y + 1, w - 5, 1, lit);
        self.rect(x + 1, y + 2, 1, h - 5, lit);
    }

    /// 1px line from (x0, y0) to (x1, y1), both ends included.
    pub fn line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32, colour: u8) {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = ((x1 - x0).signum(), (y1 - y0).signum());
        let mut err = dx + dy;
        loop {
            self.px(x0, y0, colour);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Draws `rows` of [`SPRITE_KEY`] characters with the top-left at (x, y).
    pub fn art(&mut self, x: i32, y: i32, rows: &[&str]) {
        self.art_with(SPRITE_KEY, x, y, rows);
    }

    /// Like [`Canvas::art`] with another character key, for sprites on their own palette.
    pub fn art_with(&mut self, key: &[(u8, u8)], x: i32, y: i32, rows: &[&str]) {
        for (dy, row) in rows.iter().enumerate() {
            for (dx, ch) in row.bytes().enumerate() {
                if let Some(&(_, colour)) = key.iter().find(|(c, _)| *c == ch) {
                    self.px(x + dx as i32, y + dy as i32, colour);
                }
            }
        }
    }

    /// Draws `rows` turned a quarter turn clockwise (what pointed right points down),
    /// with the top-left of the turned drawing at (x, y).
    pub fn art_turned_cw(&mut self, key: &[(u8, u8)], x: i32, y: i32, rows: &[&str]) {
        let height = rows.len() as i32;
        for (row_ix, row) in rows.iter().enumerate() {
            for (col, ch) in row.bytes().enumerate() {
                if let Some(&(_, colour)) = key.iter().find(|(c, _)| *c == ch) {
                    self.px(x + height - 1 - row_ix as i32, y + col as i32, colour);
                }
            }
        }
    }

    /// Draws `rows` turned a quarter turn anticlockwise (what pointed right points up),
    /// with the top-left of the turned drawing at (x, y).
    pub fn art_turned(&mut self, key: &[(u8, u8)], x: i32, y: i32, rows: &[&str]) {
        let width = rows.iter().map(|r| r.len()).max().unwrap_or(0) as i32;
        for (row_ix, row) in rows.iter().enumerate() {
            for (col, ch) in row.bytes().enumerate() {
                if let Some(&(_, colour)) = key.iter().find(|(c, _)| *c == ch) {
                    self.px(x + row_ix as i32, y + width - 1 - col as i32, colour);
                }
            }
        }
    }

    pub fn vram(self) -> SpriteVram {
        self.sprite.to_vram(&SPRITE_PALETTE)
    }

    pub fn vram_with(self, palette: &'static Palette16) -> SpriteVram {
        self.sprite.to_vram(palette)
    }
}

/// A sprite from one ASCII drawing placed at (x, y).
pub fn drawing(size: Size, x: i32, y: i32, rows: &[&str]) -> SpriteVram {
    let mut c = Canvas::new(size);
    c.art(x, y, rows);
    c.vram()
}

/// Sprites shared by every level: pickups, HUD, laser and effects, menu cursor.
pub struct Common {
    /// Blue diamond and its glinting frame.
    pub diamond: [SpriteVram; 2],
    pub heal: SpriteVram,
    /// Life bar: the two 4px end caps, and the seven 8px body cells for each HP
    /// (0..=MAX_HP). Cells that look the same share one sprite.
    pub life_caps: [SpriteVram; 2],
    pub life: [[SpriteVram; LIFE_CELLS]; 9],
    /// Boss bar cells: index = pixels filled (0..=8).
    pub boss_cell: [SpriteVram; 9],
    /// Laser bolt heading right, up, and up-right (flip for the other headings).
    pub bolt: SpriteVram,
    pub bolt_up: SpriteVram,
    pub bolt_diagonal: SpriteVram,
    pub beam: SpriteVram,
    pub beam_up: SpriteVram,
    pub beam_diagonal: SpriteVram,
    /// Two-frame muzzle flash.
    pub muzzle: [SpriteVram; 2],
    /// Charging glow, small then full.
    pub charge: [SpriteVram; 2],
    pub cursor: SpriteVram,
    /// Checkpoint beacon, dark then lit.
    pub flag: [SpriteVram; 2],
    /// "!" bubble over a bot about to attack.
    pub alert: SpriteVram,
    pub impact: [SpriteVram; 3],
    pub explosion: [SpriteVram; 3],
    pub glint: [SpriteVram; 2],
    pub sparks: [SpriteVram; 2],
    /// Jetpack exhaust, laser pink: a glowing blob, then sparks.
    pub puff: [SpriteVram; 2],
    /// Muzzle ring for a forward shot facing right, wide then narrowing.
    pub muzzle_ring: [SpriteVram; 2],
    /// Dust behind a running heel, billowing then breaking up.
    pub dust: [SpriteVram; 3],
    /// Half of a take-off or landing burst, kicking right; flip for the left half.
    pub kick: [SpriteVram; 3],
    /// Wind streaks over a hard fall.
    pub speed_lines: SpriteVram,
}

/// Cream and tan for the dust, after the sprite sheet's dust clouds.
static DUST_PALETTE: Palette16 = Palette16::new([
    rgb(0, 0, 0),
    rgb(31, 29, 24),
    rgb(27, 23, 17),
    rgb(19, 15, 11),
    rgb(10, 7, 6),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
]);
/// Highlight, cream, tan shade, dark outline.
const DUST_KEY: &[(u8, u8)] = &[(b'w', 1), (b'o', 2), (b'd', 3), (b'k', 4)];

const DUST0: [&str; 6] = [
    "..ww....",
    ".wwoo...",
    "wwooodd.",
    "woooddd.",
    ".kdddkk.",
    "..kkk...",
];
const DUST1: [&str; 7] = [
    "...ww...",
    "..wwoo..",
    ".woooo..",
    "wwood.ww",
    "woddkwod",
    ".kkk.kdk",
    "......k.",
];
const DUST2: [&str; 6] = [
    "..w.....",
    ".wo...w.",
    "..d..wo.",
    "......d.",
    ".w......",
    "..d.....",
];
/// Kick-up, the right half: grit sprays out low and wide from under the feet.
const KICK0: [&str; 8] = [
    "................",
    "................",
    "................",
    "...w............",
    "..woo..w........",
    "wwoodd.wo.......",
    "woooddwwod......",
    "kkdddkkkdk......",
];
const KICK1: [&str; 8] = [
    "................",
    "..........w.....",
    ".......w...o....",
    "....w...o.......",
    "...wo.ww...ww...",
    "..wod.woo.wood..",
    ".wodd.kodd.kdk..",
    "..kk...kk.......",
];
const KICK2: [&str; 8] = [
    "................",
    "............w...",
    "..........o.....",
    "........w.....o.",
    "...........d....",
    ".....w..........",
    "...........d..w.",
    "................",
];

fn dust(size: Size, x: i32, y: i32, rows: &[&str]) -> SpriteVram {
    let mut c = Canvas::new(size);
    c.art_with(DUST_KEY, x, y, rows);
    c.vram_with(&DUST_PALETTE)
}

/// An upright ring of laser light round the muzzle, 8x16: magenta rim, pink band,
/// white-hot centre. `rx`, `ry` in half pixels so it can sit centred in the sprite.
fn muzzle_ring(rx: i32, ry: i32) -> SpriteVram {
    let mut c = Canvas::new(Size::S8x16);
    for y in 0..16 {
        for x in 0..8 {
            // Distance from the centre (4, 8) on a 0..=64 scale.
            let (dx, dy) = (2 * x + 1 - 8, 2 * y + 1 - 16);
            let d = 64 * dx * dx / (rx * rx) + 64 * dy * dy / (ry * ry);
            let colour = match d {
                0..=12 => ix::WHITE,
                13..=30 => continue,
                31..=44 => ix::LASER_LIGHT,
                45..=64 => ix::LASER,
                _ => continue,
            };
            c.px(x, y, colour);
        }
    }
    c.vram()
}

const DIAMOND0: [&str; 8] = [
    "..kkkkkk..",
    ".kiwiibbk.",
    "kiiwbbbbVk",
    "kbbbbbbVVk",
    ".kbbbbVVk.",
    "..kbbVVk..",
    "...kbVk...",
    "....kk....",
];
const DIAMOND1: [&str; 8] = [
    "..kkkkkk..",
    ".kibiibbk.",
    "kiibbbwbVk",
    "kbbbbbwwVk",
    ".kbbbbVVk.",
    "..kbbVVk..",
    "...kbVk...",
    "....kk....",
];
const MUZZLE0: [&str; 8] = [
    "..w.....",
    ".wCw..w.",
    "wCwCwwCw",
    ".wCwwwCw",
    "wCwCwwCw",
    ".wCw..w.",
    "..w.....",
    "........",
];
const MUZZLE1: [&str; 8] = [
    ".c......",
    "c.c..c..",
    ".cC.cCc.",
    "..cwwwCc",
    ".cC.cCc.",
    "c.c..c..",
    ".c......",
    "........",
];
const ALERT: [&str; 11] = [
    ".kkkkk.",
    "kwwwwwk",
    "kwwkwwk",
    "kwwkwwk",
    "kwwkwwk",
    "kwwwwwk",
    "kwwkwwk",
    "kwwwwwk",
    ".kkkkk.",
    "..kk...",
    "..k....",
];
const IMPACT0: [&str; 7] = [
    "...w...",
    "..wCw..",
    ".wCwCw.",
    "wCwwwCw",
    ".wCwCw.",
    "..wCw..",
    "...w...",
];
const IMPACT1: [&str; 7] = [
    "...c...",
    "..c.c..",
    ".c...c.",
    "c..w..c",
    ".c...c.",
    "..c.c..",
    "...c...",
];
const IMPACT2: [&str; 7] = [
    "c.....c",
    ".......",
    "...C...",
    "..C.C..",
    "...C...",
    ".......",
    "c.....c",
];
const BOOM0: [&str; 14] = [
    "......kk......",
    "....kkwwkk....",
    "...kwwwwwwk...",
    "..kwwyyyywwk..",
    ".kwwyyyyyywwk.",
    ".kwyyyooyyywk.",
    "kwwyyooooyywwk",
    "kwwyyooooyywwk",
    ".kwyyyooyyywk.",
    ".kwwyyyyyywwk.",
    "..kwwyyyywwk..",
    "...kwwwwwwk...",
    "....kkwwkk....",
    "......kk......",
];
const BOOM1: [&str; 14] = [
    "...o....y.....",
    ".....oo...o...",
    "..oyyyyyy.....",
    ".oyyoooooyo..y",
    "..yoorrrroyo..",
    "yoorr.k.rroyo.",
    ".yor.k.k.royo.",
    ".yor..k..ryo..",
    "oyorr...rroy..",
    "..yoorrrooyo..",
    "...oyyyyyo..o.",
    "o....oo....y..",
    "....y....o....",
    "..............",
];
const BOOM2: [&str; 14] = [
    "..g.......k...",
    "......G.......",
    ".k..RR...R..g.",
    "....R..R...R..",
    "..R...k....R..",
    ".R..G...g...R.",
    "...k.......R..",
    ".R.....k......",
    "..R..g...R..G.",
    "....R...R.....",
    ".g....RR...k..",
    "......k.......",
    "...k......G...",
    "..............",
];
const GLINT0: [&str; 7] = [
    "...w...",
    "...i...",
    "..iwi..",
    "wiwwwiw",
    "..iwi..",
    "...i...",
    "...w...",
];
const GLINT1: [&str; 7] = [
    "i.....i",
    ".......",
    "...w...",
    "..w.w..",
    "...w...",
    ".......",
    "i.....i",
];
const SPARK0: [&str; 7] = [
    "y.....y",
    ".y...y.",
    "..ywy..",
    "..www..",
    "..ywy..",
    ".y...y.",
    "y.....y",
];
/// Jetpack exhaust: laser energy, not smoke. A glowing blob, then sparks.
const PUFF0: [&str; 5] = [
    ".cCc.",
    "cCwCc",
    "CwwwC",
    "cCwCc",
    ".cCc.",
];
const PUFF1: [&str; 5] = [
    "c...c",
    "..C..",
    ".CwC.",
    "..C..",
    "c...c",
];
const SPARK1: [&str; 7] = [
    "o.....o",
    ".......",
    "...y...",
    "..y.y..",
    "...y...",
    ".......",
    "o.....o",
];

/// Laser bolt heading right: white-hot core, pink edge, a few trailing sparks, and a
/// ring of light round the head, as in the sprite sheet.
const BOLT: [&str; 8] = [
    "..........cCCc..",
    "......c...cwwC..",
    "..c.cCCCCCcwwwC.",
    ".cCCwwwwwwwwwwwC",
    ".cCCwwwwwwwwwwwC",
    "..c.cCCCCCcwwwC.",
    "......c...cwwC..",
    "..........cCCc..",
];

/// Draws a small effect drawing centred in an 8x8 or 16x16 sprite.
fn centred(size: Size, rows: &[&str]) -> SpriteVram {
    let (w, h) = size.to_width_height();
    let x = (w as i32 - rows[0].len() as i32) / 2;
    let y = (h as i32 - rows.len() as i32) / 2;
    drawing(size, x, y, rows)
}

fn turned(size: Size, rows: &[&str]) -> SpriteVram {
    let mut c = Canvas::new(size);
    c.art_turned(SPRITE_KEY, 0, 0, rows);
    c.vram()
}

impl Common {
    pub fn new() -> Self {
        Self {
            diamond: [centred(Size::S16x16, &DIAMOND0), centred(Size::S16x16, &DIAMOND1)],
            heal: cold_wallet(),
            life_caps: life_caps(),
            life: life_bars(),
            boss_cell: core::array::from_fn(|n| boss_cell(n as i32)),
            bolt: drawing(Size::S16x8, 0, 0, &BOLT),
            bolt_up: turned(Size::S8x16, &BOLT),
            bolt_diagonal: laser_diagonal(Size::S16x16, 11, 3),
            beam: beam(),
            beam_diagonal: laser_diagonal(Size::S32x32, 22, 7),
            beam_up: {
                let mut c = Canvas::new(Size::S8x32);
                for y in 0..32 {
                    c.rect(0, y, 8, 1, ix::LASER);
                    c.rect(1, y, 6, 1, ix::LASER_LIGHT);
                    c.rect(3, y, 2, 1, ix::WHITE);
                }
                c.vram()
            },
            muzzle: [drawing(Size::S8x8, 0, 0, &MUZZLE0), drawing(Size::S8x8, 0, 0, &MUZZLE1)],
            charge: [
                drawing(Size::S8x8, 2, 2, &[".c.", "cCc", ".c."]),
                drawing(
                    Size::S8x8,
                    0,
                    0,
                    &["..c..c..", ".cCccCc.", "cCwwwwCc", ".cwwwwc.", ".cwwwwc.", "cCwwwwCc", ".cCccCc.", "..c..c.."],
                ),
            ],
            cursor: cursor(),
            flag: [flag(false), flag(true)],
            alert: drawing(Size::S8x16, 0, 0, &ALERT),
            impact: [&IMPACT0[..], &IMPACT1, &IMPACT2].map(|r| centred(Size::S8x8, r)),
            explosion: [&BOOM0[..], &BOOM1, &BOOM2].map(|r| centred(Size::S16x16, r)),
            glint: [centred(Size::S8x8, &GLINT0), centred(Size::S8x8, &GLINT1)],
            sparks: [centred(Size::S8x8, &SPARK0), centred(Size::S8x8, &SPARK1)],
            puff: [centred(Size::S8x8, &PUFF0), centred(Size::S8x8, &PUFF1)],
            muzzle_ring: [muzzle_ring(8, 16), muzzle_ring(6, 11)],
            dust: [dust(Size::S8x8, 0, 2, &DUST0), dust(Size::S8x8, 0, 1, &DUST1), dust(Size::S8x8, 0, 2, &DUST2)],
            kick: [&KICK0, &KICK1, &KICK2].map(|r| dust(Size::S16x8, 0, 0, r)),
            speed_lines: drawing(
                Size::S8x16,
                0,
                0,
                &[
                    ".S....S.", ".S....S.", ".S..g.S.", ".g..g.S.", "....g.g.", "....S...", ".S..S...",
                    ".S..g..S", ".g.....S", ".......g", "....S...", "....g...",
                ],
            ),
        }
    }
}

/// Cold wallet: a steel hardware wallet with a glowing screen. Heals.
fn cold_wallet() -> SpriteVram {
    drawing(
        Size::S16x16,
        2,
        2,
        &[
            "..kkkkkkkk..",
            ".kSSSSSSSSk.",
            "kSkkkkkkkkgk",
            "kSkiibiibkgk",
            "kSkiwwwwbkgk",
            "kSkibwwbbkgk",
            "kSkkkkkkkkgk",
            "kSgcgggggcgk",
            "kggggggggGk.",
            ".kkkkkkkkk..",
        ],
    )
}

/// Green, yellow and red candle bodies for the life bar.
static LIFE_PALETTE: Palette16 = Palette16::new([
    rgb(0, 0, 0),
    rgb(2, 2, 6),
    rgb(31, 31, 31),
    rgb(4, 24, 9),
    rgb(2, 12, 5),
    rgb(30, 25, 4),
    rgb(15, 12, 2),
    rgb(28, 5, 6),
    rgb(15, 2, 4),
    rgb(7, 7, 11),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
]);

/// Body cells in the life bar, 8px of fill each.
pub const LIFE_CELLS: usize = 7;

/// Life bar cells for every HP.
fn life_bars() -> [[SpriteVram; LIFE_CELLS]; 9] {
    let mut made: Vec<((i32, i32), SpriteVram)> = Vec::new();
    core::array::from_fn(|hp| {
        core::array::from_fn(|cell| {
            let hp = hp as i32;
            let shown = (life_fill(hp) - cell as i32 * 8).clamp(0, 8);
            // An empty cell is the same whatever the colour.
            let key = (shown, if shown > 0 { life_tier(hp) } else { 0 });
            if let Some((_, sprite)) = made.iter().find(|(k, _)| *k == key) {
                return sprite.clone();
            }
            let sprite = life_cell(shown, key.1);
            made.push((key, sprite.clone()));
            sprite
        })
    })
}

/// Pixels of fill at `hp`, over the 56px interior.
fn life_fill(hp: i32) -> i32 {
    hp * 56 / diamond_hands_logic::player::MAX_HP
}

/// Green when nearly full (3), yellow half way (2), red when critical (1).
fn life_tier(hp: i32) -> i32 {
    match hp * 4 / diamond_hands_logic::player::MAX_HP {
        3.. => 3,
        2 => 2,
        _ => 1,
    }
}

/// The life bar is a horizontal candlestick, 64px wide: a 4px wick cap at each end and
/// seven 8px cells of body between them, filled in proportion to HP. A cell holds
/// `shown` pixels of fill.
fn life_cell(shown: i32, tier: i32) -> SpriteVram {
    // Body colours: lit, shade, dark track.
    let (lit, shade) = match tier {
        3 => (3, 4),
        2 => (5, 6),
        _ => (7, 8),
    };
    let mut c = Canvas::new(Size::S8x8);
    for y in 0..8 {
        c.rect(0, y, 8, 1, if y == 0 || y == 7 { 1 } else { 9 });
    }
    for x in 0..shown {
        for y in 1..7 {
            c.px(x, y, if y >= 5 { shade } else { lit });
        }
        c.px(x, 1, 2);
    }
    c.vram_with(&LIFE_PALETTE)
}

/// The wicks: thin stalks, and the body's outline closing the track.
fn life_caps() -> [SpriteVram; 2] {
    [0, 1].map(|right| {
        let mut c = Canvas::new(Size::S8x8);
        // The 4px cap: three of stalk and one of body wall, mirrored on the right.
        let (stalk, wall) = if right == 0 { (0..3, 3) } else { (1..4, 0) };
        for x in stalk {
            c.rect(x, 3, 1, 2, 9);
        }
        c.rect(wall, 0, 1, 8, 1);
        c.rect(wall, 1, 1, 6, 9);
        c.vram_with(&LIFE_PALETTE)
    })
}

fn boss_cell(filled: i32) -> SpriteVram {
    let mut c = Canvas::new(Size::S8x8);
    c.rect(0, 1, 8, 6, ix::OUTLINE);
    c.rect(0, 2, filled, 4, ix::RED);
    c.rect(0, 2, filled, 1, ix::ORANGE);
    c.vram()
}

/// Charged laser: a fat bar with a white-hot core, 24x8 hitbox in a 32x8 sprite.
fn beam() -> SpriteVram {
    let mut c = Canvas::new(Size::S32x8);
    c.rect(0, 0, 24, 8, ix::LASER);
    c.rect(1, 1, 23, 6, ix::LASER_LIGHT);
    c.rect(2, 3, 22, 2, ix::WHITE);
    c.rect(24, 1, 2, 6, ix::LASER);
    c.rect(26, 2, 2, 4, ix::LASER);
    c.px(29, 1, ix::LASER_LIGHT);
    c.px(30, 5, ix::LASER);
    c.vram()
}

/// A laser heading up-right, centred in the sprite. `t` runs along it, `s` across;
/// within `half` of the axis it has a white-hot core, a light-pink band and a magenta
/// edge. The head comes to a point; the tail breaks up into a few sparks.
fn laser_diagonal(size: Size, len: i32, half: i32) -> SpriteVram {
    let mut c = Canvas::new(size);
    let (w, h) = size.to_width_height();
    let (cx, cy) = (w as i32 / 2, h as i32 / 2);
    let core = half / 3;
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let t = (x - cx) - (y - cy);
            let s = ((x - cx) + (y - cy)).abs();
            if t > len || s > half.min(len - t) {
                continue;
            }
            let colour = match s {
                _ if t < -len => continue,
                _ if t < -len + 4 => {
                    if s == 0 && t.rem_euclid(3) == 0 { ix::LASER } else { continue }
                }
                s if s <= core => ix::WHITE,
                s if s < half => ix::LASER_LIGHT,
                _ => ix::LASER,
            };
            c.px(x, y, colour);
        }
    }
    c.vram()
}

/// Checkpoint beacon: a steel pole with a dark banner that lights up with the
/// wassie's grin once touched.
fn flag(lit: bool) -> SpriteVram {
    let (cloth, face) = if lit { (ix::PURPLE_DARK, ix::CYAN) } else { (ix::STEEL_DARK, ix::STEEL) };
    let mut c = Canvas::new(Size::S16x32);
    c.rect(3, 4, 2, 28, ix::OUTLINE);
    c.rect(3, 4, 1, 28, ix::STEEL_LIGHT);
    c.block(2, 1, 4, 4, if lit { ix::CYAN } else { ix::STEEL }, ix::OUTLINE);
    c.block(5, 5, 11, 10, cloth, ix::OUTLINE);
    // A small smiling face.
    c.px(8, 8, face);
    c.px(12, 8, face);
    c.px(7, 10, face);
    c.px(13, 10, face);
    c.rect(8, 11, 5, 1, face);
    c.vram()
}

/// Menu cursor: a small gold arrow.
fn cursor() -> SpriteVram {
    let mut c = Canvas::new(Size::S8x16);
    for x in 0..5 {
        let half = 5 - x;
        c.rect(x + 1, 6 - half + 1, 1, 2 * half - 1, ix::OUTLINE);
        c.rect(x, 6 - half, 1, 2 * half - 1, ix::GOLD);
    }
    c.vram()
}

/// Interns 8x8 tiles by their pixels, so identical cells share VRAM.
pub struct TileCache {
    ids: BTreeMap<[u8; 64], usize>,
    pub tiles: Vec<DynamicTile16>,
}

impl TileCache {
    pub fn new() -> Self {
        Self {
            ids: BTreeMap::new(),
            tiles: Vec::new(),
        }
    }

    /// The tile with these pixels, created on first use. `None` when fully transparent.
    pub fn intern(&mut self, pixels: [u8; 64]) -> Option<usize> {
        if pixels.iter().all(|&p| p == 0) {
            return None;
        }
        let tiles = &mut self.tiles;
        Some(*self.ids.entry(pixels).or_insert_with(|| {
            let mut tile = DynamicTile16::new();
            // A whole row of the tile per word, a nibble a pixel, leftmost lowest.
            for (word, row) in tile.data_mut().iter_mut().zip(pixels.chunks_exact(8)) {
                *word = row.iter().rev().fold(0, |acc, &p| acc << 4 | u32::from(p));
            }
            tiles.push(tile);
            tiles.len() - 1
        }))
    }

    /// Fills cell `(tx, ty)` of `bg` with `pixel(x, y)` in screen coordinates, using
    /// background palette `palette`. Fully transparent cells are left empty.
    pub fn paint(
        &mut self,
        bg: &mut RegularBackground,
        (tx, ty): (i32, i32),
        palette: u8,
        pixel: &impl Fn(i32, i32) -> u8,
    ) {
        let mut pixels = [0; 64];
        for (i, p) in pixels.iter_mut().enumerate() {
            *p = pixel(tx * 8 + (i % 8) as i32, ty * 8 + (i / 8) as i32);
        }
        if let Some(id) = self.intern(pixels) {
            bg.set_tile_dynamic16((tx, ty), &self.tiles[id], TileEffect::new(false, false, palette));
        }
    }
}
