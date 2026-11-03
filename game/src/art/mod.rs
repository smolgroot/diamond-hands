//! Pixel art drawn in code at boot: small ASCII drawings and shape primitives become
//! sprites and background tiles.

pub mod canyon;
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

/// Sprite palette indices, shared by every sprite.
pub mod ix {
    pub const CLEAR: u8 = 0;
    pub const OUTLINE: u8 = 1;
    pub const WHITE: u8 = 2;
    pub const TEAL: u8 = 3;
    pub const TEAL_DARK: u8 = 4;
    pub const FUR: u8 = 5;
    pub const LASER: u8 = 6;
    pub const LASER_LIGHT: u8 = 7;
    pub const GOLD: u8 = 8;
    pub const GOLD_DARK: u8 = 9;
    pub const RED: u8 = 10;
    pub const RED_DARK: u8 = 11;
    pub const BONE: u8 = 12;
    pub const BONE_DARK: u8 = 13;
    pub const STEEL: u8 = 14;
    pub const FUR_DARK: u8 = 15;
}

pub static SPRITE_PALETTE: Palette16 = Palette16::new([
    rgb(0, 0, 0),
    rgb(2, 2, 4),
    rgb(31, 31, 31),
    rgb(6, 23, 18),
    rgb(3, 14, 11),
    rgb(20, 11, 5),
    rgb(30, 5, 22),
    rgb(31, 19, 29),
    rgb(31, 25, 6),
    rgb(25, 14, 0),
    rgb(29, 4, 9),
    rgb(17, 2, 4),
    rgb(26, 22, 17),
    rgb(17, 13, 10),
    rgb(11, 14, 20),
    rgb(10, 5, 2),
]);

/// Characters used in the ASCII sprite drawings. `.` and anything unlisted is left
/// transparent.
const SPRITE_KEY: &[(u8, u8)] = &[
    (b'k', ix::OUTLINE),
    (b'w', ix::WHITE),
    (b't', ix::TEAL),
    (b'd', ix::TEAL_DARK),
    (b'h', ix::FUR),
    (b'H', ix::FUR_DARK),
    (b'c', ix::LASER),
    (b'C', ix::LASER_LIGHT),
    (b'y', ix::GOLD),
    (b'Y', ix::GOLD_DARK),
    (b'r', ix::RED),
    (b'R', ix::RED_DARK),
    (b'l', ix::BONE),
    (b'L', ix::BONE_DARK),
    (b'g', ix::STEEL),
];

/// A sprite being drawn. The pixel buffer lives in EWRAM: agb's IWRAM allocator
/// reaches up into the stacks at the top of IWRAM, and a 64x64 buffer placed there was
/// overwritten by interrupt pushes before it reached VRAM.
pub struct Canvas {
    sprite: DynamicSprite16<ExternalAllocator>,
    w: i32,
    h: i32,
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
        }
    }

    pub fn px(&mut self, x: i32, y: i32, colour: u8) {
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

/// Sprites shared by every level: pickups, HUD, shots, menu cursor.
pub struct Common {
    pub sat: [SpriteVram; 2],
    pub heal: SpriteVram,
    /// Life bar quarters: index = segments filled (0..=4).
    pub life: [SpriteVram; 5],
    /// Boss bar cells: index = pixels filled (0..=8).
    pub boss_cell: [SpriteVram; 9],
    pub bolt: SpriteVram,
    pub beam: SpriteVram,
    pub muzzle: SpriteVram,
    /// Charging glow, small then full.
    pub charge: [SpriteVram; 2],
    pub cursor: SpriteVram,
    pub flag: [SpriteVram; 2],
}

impl Common {
    pub fn new() -> Self {
        Self {
            sat: [sat(5), sat(2)],
            heal: cold_wallet(),
            life: core::array::from_fn(|n| life_quarter(n as i32)),
            boss_cell: core::array::from_fn(|n| boss_cell(n as i32)),
            bolt: drawing(Size::S8x8, 0, 2, &["kccccCCk", "cCCCCwwC", "kccccCCk"]),
            beam: beam(),
            muzzle: drawing(
                Size::S8x8,
                0,
                0,
                &["..w.....", ".wCw..w.", "wCwCwwCw", ".wCCCCw.", "wCwCwwCw", ".wCw..w.", "..w....."],
            ),
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
        }
    }
}

/// Sat-coin with a ₿ mark; `rx` narrows it for the spin. Drawn in a 16x16 sprite
/// centred on the 8x8 hitbox.
fn sat(rx: i32) -> SpriteVram {
    let mut c = Canvas::new(Size::S16x16);
    c.ellipse(8, 8, rx + 1, 6, ix::GOLD_DARK);
    c.ellipse(8, 8, rx, 5, ix::GOLD);
    if rx > 3 {
        c.rect(7, 5, 1, 7, ix::GOLD_DARK);
        c.rect(8, 5, 2, 1, ix::GOLD_DARK);
        c.rect(8, 8, 2, 1, ix::GOLD_DARK);
        c.rect(8, 11, 2, 1, ix::GOLD_DARK);
        c.px(10, 6, ix::GOLD_DARK);
        c.px(10, 9, ix::GOLD_DARK);
        c.px(10, 10, ix::GOLD_DARK);
    }
    c.vram()
}

/// Cold wallet: a steel hardware wallet with a glowing screen. Heals.
fn cold_wallet() -> SpriteVram {
    drawing(
        Size::S16x16,
        2,
        2,
        &[
            "..kkkkkkkk..",
            ".kggggggggk.",
            "kgkkkkkkkkgk",
            "kgkCCcCCckgk",
            "kgkCrrrrCkgk",
            "kgkCrrrrckgk",
            "kgkkkkkkkkgk",
            "kggwggggwggk",
            "kgggggggggk.",
            ".kkkkkkkkk..",
        ],
    )
}

/// Four life segments, `filled` of them lit.
fn life_quarter(filled: i32) -> SpriteVram {
    let mut c = Canvas::new(Size::S32x8);
    for i in 0..4 {
        let x = i * 8;
        c.block(x, 0, 8, 8, ix::RED_DARK, ix::OUTLINE);
        if i < filled {
            c.rect(x + 1, 1, 6, 6, ix::RED);
            c.rect(x + 1, 1, 6, 2, ix::WHITE);
            c.rect(x + 1, 2, 6, 1, ix::RED);
            c.px(x + 2, 2, ix::WHITE);
        }
    }
    c.vram()
}

fn boss_cell(filled: i32) -> SpriteVram {
    let mut c = Canvas::new(Size::S8x8);
    c.rect(0, 1, 8, 6, ix::OUTLINE);
    c.rect(0, 2, filled, 4, ix::GOLD);
    c.rect(0, 2, filled, 1, ix::WHITE);
    c.vram()
}

/// Charged laser: a fat glowing bar, 24x8 hitbox in a 32x8 sprite.
fn beam() -> SpriteVram {
    let mut c = Canvas::new(Size::S32x8);
    c.rect(0, 0, 24, 8, ix::LASER);
    c.rect(1, 1, 23, 6, ix::LASER_LIGHT);
    c.rect(2, 3, 22, 2, ix::WHITE);
    c.rect(24, 1, 2, 6, ix::LASER);
    c.rect(26, 2, 2, 4, ix::LASER);
    c.vram()
}

/// Checkpoint flag on a pole; grey ₿ until touched, then gold.
fn flag(lit: bool) -> SpriteVram {
    let (fill, mark) = if lit { (ix::GOLD, ix::GOLD_DARK) } else { (ix::BONE_DARK, ix::BONE) };
    let mut c = Canvas::new(Size::S16x32);
    c.rect(3, 4, 2, 28, ix::OUTLINE);
    c.rect(3, 4, 1, 28, ix::STEEL);
    c.block(2, 2, 4, 3, ix::GOLD, ix::OUTLINE);
    c.block(5, 5, 11, 9, fill, ix::OUTLINE);
    // A ₿: stem with strokes through it, and the two bowls.
    c.rect(9, 6, 1, 7, mark);
    c.rect(10, 7, 2, 1, mark);
    c.rect(10, 9, 2, 1, mark);
    c.rect(10, 11, 2, 1, mark);
    c.px(12, 8, mark);
    c.px(12, 10, mark);
    c.px(10, 6, mark);
    c.px(10, 12, mark);
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
            for (i, &p) in pixels.iter().enumerate() {
                tile.set_pixel(i % 8, i / 8, p);
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

/// Scales a colour's channels by `num / den`, for darker times of day.
pub const fn shade(c: (u16, u16, u16), num: u16, den: u16) -> Rgb15 {
    rgb(c.0 * num / den, c.1 * num / den, c.2 * num / den)
}
