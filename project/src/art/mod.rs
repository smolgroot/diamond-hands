//! Pixel art drawn in code at boot: small ASCII drawings and shape primitives become
//! sprites and background tiles.

pub mod arrival;
pub mod badlands;
pub mod casino;
pub mod menu;
pub mod trenches;
pub mod studio;
pub mod ui;
pub mod wassie;

use alloc::vec::Vec;

use agb::display::{
        Palette16, Rgb15,
        object::{Sprite, SpriteVram},
        tiled::{DynamicTile16, RegularBackground, TileEffect},
    };

pub const fn rgb(r: u16, g: u16, b: u16) -> Rgb15 {
    Rgb15::new(r | (g << 5) | (b << 10))
}

/// How far the backdrops (sky and city) are muted, saturation and brightness in
/// percent, so the wassie, the bots and the ground they stand on stand out from them.
pub const BACKDROP_MUTE: (u16, u16) = (50, 68);
/// And the scenery drawn on the terrain layer (signs, slot machines, screens): less,
/// as it is nearer and still has to read.
pub const SCENERY_MUTE: (u16, u16) = (70, 78);

/// `c` with its saturation cut to `sat` percent (towards its own grey) and its
/// brightness to `bright` percent.
pub const fn mute(c: Rgb15, sat: u16, bright: u16) -> Rgb15 {
    const fn mix(x: u16, grey: u16, sat: u16, bright: u16) -> u16 {
        (x * sat + grey * (100 - sat)) / 100 * bright / 100
    }
    let v = c.0;
    let (r, g, b) = (v & 31, (v >> 5) & 31, (v >> 10) & 31);
    let grey = (r * 77 + g * 150 + b * 29) / 256;
    rgb(mix(r, grey, sat, bright), mix(g, grey, sat, bright), mix(b, grey, sat, bright))
}

/// Every colour of `palette` from index `first` on muted as [`mute`]. Index 0 is
/// transparent, except in background palette 0, where it is the backdrop colour.
pub const fn mute_palette(palette: Palette16, (sat, bright): (u16, u16), first: usize) -> Palette16 {
    let mut out = palette;
    let mut i = first;
    while i < 16 {
        out.update_colour(i, mute(out.colour(i), sat, bright));
        i += 1;
    }
    out
}

/// Sprite palette indices for the bots, pickups, effects and HUD. The wassie has its
/// own palette (see `wassie.rs`). Colours follow the cover and the gameplay reference:
/// red and orange armour, steel, electric blue diamonds, violet, and magenta laser.
pub mod ix {
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

/// A sheet-drawn bot's 32x32 poses (see `scripts/make-bots.py`), each loaded into
/// sprite VRAM from ROM when something on screen shows it and let go once nothing has
/// for a frame: a whole set would take 6 KB or more, and only a few poses are ever on
/// screen at once.
pub struct LazyPoses<const N: usize> {
    /// `N` studio sprite frames, the poses.
    frames: &'static [Sprite],
    loaded: core::cell::RefCell<[Option<SpriteVram>; N]>,
    /// Poses shown since the last [`LazyPoses::sweep`], one bit each.
    used: core::cell::Cell<u32>,
}

impl<const N: usize> LazyPoses<N> {
    pub fn new(frames: &'static [Sprite]) -> Self {
        assert_eq!(frames.len(), N, "a pose sheet of the wrong size");
        Self {
            frames,
            loaded: core::cell::RefCell::new(core::array::from_fn(|_| None)),
            used: core::cell::Cell::new(0),
        }
    }

    /// Pose `i`, loading it if it isn't already.
    pub fn sprite(&self, i: usize) -> SpriteVram {
        self.used.set(self.used.get() | 1 << i);
        self.loaded.borrow_mut()[i]
            .get_or_insert_with(|| SpriteVram::from(&self.frames[i]))
            .clone()
    }

    /// Lets go of the poses nothing showed since the last sweep. Call once a frame,
    /// before drawing.
    pub fn sweep(&self) {
        let used = self.used.replace(0);
        for (i, slot) in self.loaded.borrow_mut().iter_mut().enumerate() {
            if used & 1 << i == 0 {
                *slot = None;
            }
        }
    }
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
    /// An electric fence's lightning curtain, stacked up a tile column: 32px and 8px
    /// pieces, four crackles of each. Every piece starts and ends on the centre line,
    /// so any of them stack seamlessly.
    pub lightning: [SpriteVram; LIGHTNING_FRAMES],
    pub lightning_short: [SpriteVram; LIGHTNING_FRAMES],
    /// A spark flickering over an emitter about to go live.
    pub fence_spark: SpriteVram,
}

pub const LIGHTNING_FRAMES: usize = 4;

impl Common {
    /// The pickups, shots, effects and HUD pieces, loaded from the studio's sprite
    /// sheets (`studio/sprites/`, drawn in `assets/sprites/`).
    pub fn new() -> Self {
        use studio::*;
        Self {
            diamond: pickups::diamond(),
            heal: pickups::heal(),
            life_caps: hud_8x8::life_caps(),
            life: [
                hud_8x8::life_0(),
                hud_8x8::life_1(),
                hud_8x8::life_2(),
                hud_8x8::life_3(),
                hud_8x8::life_4(),
                hud_8x8::life_5(),
                hud_8x8::life_6(),
                hud_8x8::life_7(),
                hud_8x8::life_8(),
            ],
            boss_cell: hud_8x8_2::boss_cell(),
            bolt: laser_16x8::bolt(),
            bolt_up: laser_8x16::bolt_up(),
            bolt_diagonal: laser_16x16::bolt_diagonal(),
            beam: laser_32x8::beam(),
            beam_diagonal: laser_32x32::beam_diagonal(),
            beam_up: laser_8x32::beam_up(),
            muzzle: effects_8x8::muzzle(),
            charge: effects_8x8::charge(),
            cursor: prompts::cursor(),
            flag: flag::flag(),
            alert: prompts::alert(),
            impact: effects_8x8::impact(),
            explosion: effects_16x16::explosion(),
            glint: effects_8x8::glint(),
            sparks: effects_8x8::sparks(),
            puff: effects_8x8::puff(),
            muzzle_ring: effects_8x16::muzzle_ring(),
            dust: dust_8x8::dust(),
            kick: dust_16x8::kick(),
            lightning: fence_8x32::lightning(),
            lightning_short: fence_8x8::lightning_short(),
            fence_spark: fence_8x8::fence_spark(),
            speed_lines: effects_8x16::speed_lines(),
        }
    }
}

/// Body cells in the life bar, 8px of fill each.
pub const LIFE_CELLS: usize = 7;

/// Scanlines left before the next vblank, counted from anywhere in the frame. The main
/// loop starts just after a vblank, so a frame on time starts with about 228.
pub fn lines_to_vblank() -> i32 {
    const VCOUNT: *const u16 = 0x0400_0006 as *const u16;
    // SAFETY: VCOUNT is a read-only hardware register, always mapped.
    let line = i32::from(unsafe { VCOUNT.read_volatile() });
    if line >= 160 { 388 - line } else { 160 - line }
}

/// Whether the frame can still afford work that could wait for a later one. The margin
/// leaves room for the rest of the frame, and for cartridge and EWRAM wait states.
pub fn has_spare_time() -> bool {
    const RESERVED_LINES: i32 = 96;
    lines_to_vblank() > RESERVED_LINES
}

/// Euclidean `n / 3` and `n % 3`. The ARM7 has no divide instruction, and terrain asks
/// for these for every cell, so the usual range is done with a multiply.
pub fn div_rem_3(n: i32) -> (i32, i32) {
    if (0..0x1_0000).contains(&n) {
        let quotient = ((n as u32 * 43_691) >> 17) as i32;
        (quotient, n - quotient * 3)
    } else {
        (n.div_euclid(3), n.rem_euclid(3))
    }
}

/// Interns 8x8 tiles by their pixels, so identical cells share VRAM.
pub struct TileCache {
    /// Each tile's place in `tiles`, sorted by a hash of its pixels; a hash match is
    /// checked against the tile itself. Eight bytes a tile in one block: the heap is
    /// tight. A level's terrain has hundreds of tiles, agb copies a background's tile
    /// map whenever it changes, and the pause menu and the world map need large blocks
    /// beside them, so a map keyed by the pixels left it too fragmented to give them.
    ids: Vec<(u32, u16)>,
    pub tiles: Vec<DynamicTile16>,
}

/// FNV-1a over a tile's packed words.
fn tile_hash(words: &[u32; 8]) -> u32 {
    words.iter().fold(0x811c_9dc5, |h, &w| (h ^ w).wrapping_mul(0x0100_0193))
}

impl TileCache {
    pub fn new() -> Self {
        Self {
            ids: Vec::new(),
            tiles: Vec::new(),
        }
    }

    /// The tile with these pixels, created on first use. `None` when fully transparent.
    pub fn intern(&mut self, pixels: [u8; 64]) -> Option<usize> {
        // A whole row of the tile per word, a nibble a pixel, leftmost lowest.
        let mut packed = [0u32; 8];
        for (word, row) in packed.iter_mut().zip(pixels.chunks_exact(8)) {
            *word = row.iter().rev().fold(0, |acc, &p| acc << 4 | u32::from(p & 15));
        }
        if packed == [0; 8] {
            return None;
        }
        let hash = tile_hash(&packed);
        let first = self.ids.partition_point(|&(h, _)| h < hash);
        let same = self.ids[first..].iter().take_while(|&&(h, _)| h == hash);
        if let Some(&(_, id)) = same.clone().find(|&&(_, id)| self.tiles[id as usize].data() == packed) {
            return Some(id as usize);
        }
        let mut tile = DynamicTile16::new();
        tile.data_mut().copy_from_slice(&packed);
        self.tiles.push(tile);
        let id = (self.tiles.len() - 1) as u16;
        self.ids.insert(first + same.count(), (hash, id));
        Some(id as usize)
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
