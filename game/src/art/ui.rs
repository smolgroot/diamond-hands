//! Screens drawn on their own background layer: the dialogue box and the world map.
//! The map uses the sprite palette's colours (see [`super::ix`]) as background palette
//! [`UI_PALETTE`]; the dialogue box has its own metal and neon in [`DIALOG_PALETTE`]. Both are
//! painted once at boot from flat fills and repeating patterns, so most of their cells
//! share tiles.

use alloc::{vec, vec::Vec};

use agb::display::{
    Palette16, WIDTH,
    object::{Size, SpriteVram},
    tiled::{DynamicTile16, RegularBackground, TileEffect},
};
use diamond_hands_logic::{levels::MAP, talk};

use super::{Canvas, TileCache, ix, rgb};
use crate::font::{self, Font};

/// Background palette slot for these layers.
pub const UI_PALETTE: u8 = 2;
/// Background palette slot for the dialogue box's colours.
pub const DIALOG_SLOT: u8 = 3;

/// The dialogue box's colours: gunmetal and night purple, lit by magenta neon, with a
/// hazard-yellow nameplate.
pub static DIALOG_PALETTE: Palette16 = Palette16::new([
    rgb(0, 0, 0),
    rgb(3, 1, 6),
    rgb(7, 3, 13),
    rgb(5, 2, 10),
    rgb(9, 6, 15),
    rgb(15, 11, 21),
    rgb(23, 19, 28),
    rgb(31, 5, 21),
    rgb(31, 21, 29),
    rgb(29, 28, 25),
    rgb(31, 25, 5),
    rgb(25, 14, 2),
    rgb(20, 4, 17),
    rgb(14, 2, 13),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
]);

/// [`DIALOG_PALETTE`] indices.
mod pal {
    pub const CLEAR: u8 = 0;
    /// Outlines, the text's shadow and the name.
    pub const INK: u8 = 1;
    /// The text panel, and every other row of it a shade darker: scanlines.
    pub const PANEL: u8 = 2;
    pub const SCAN: u8 = 3;
    pub const STEEL_DARK: u8 = 4;
    pub const STEEL: u8 = 5;
    pub const STEEL_LIGHT: u8 = 6;
    pub const NEON: u8 = 7;
    pub const NEON_LIGHT: u8 = 8;
    pub const TEXT: u8 = 9;
    pub const PLATE: u8 = 10;
    pub const PLATE_DARK: u8 = 11;
    /// Behind the portrait, scanlined like the panel.
    pub const SCREEN: u8 = 12;
    pub const SCREEN_SCAN: u8 = 13;
}

/// The dialogue box spans the top of the screen, clear of everyone standing below.
pub const BOX_HEIGHT: i32 = 56;
/// The box's map holds two copies, 256 rows apart: the portrait frame on the left
/// (the wassie talking) and on the right (anyone else). The renderer scrolls to one.
pub const BOX_SCROLL_RIGHT: i32 = 256;

/// The portrait's metal frame, 44x44 with chamfered corners, and the 32x32 portrait
/// inside it.
const FRAME_SIZE: i32 = 44;
const FRAME_TOP: i32 = 4;
const PORTRAIT_INSET: i32 = 6;
/// The text panel's top and bottom; the nameplate and a vented header sit on its rim.
const PANEL_TOP: i32 = 8;
const PANEL_BOTTOM: i32 = 54;

/// Where things go with the portrait on one side: left for the wassie, right for
/// everyone else.
#[derive(Clone, Copy)]
struct Layout {
    frame_x: i32,
    panel: (i32, i32),
    text_x: i32,
}

const fn layout(right: bool) -> Layout {
    if right {
        Layout { frame_x: WIDTH - 4 - FRAME_SIZE, panel: (2, 190), text_x: 8 }
    } else {
        Layout { frame_x: 4, panel: (50, WIDTH - 2), text_x: 56 }
    }
}

/// Where a 32x32 portrait sits on screen with the frame on the right or the left.
pub const fn portrait_pos(right: bool) -> (i32, i32) {
    (layout(right).frame_x + PORTRAIT_INSET, FRAME_TOP + PORTRAIT_INSET)
}

/// The first row of text, and the step between rows: the big font is 10 pixels tall,
/// 11 with its shadow.
const ROWS_Y: i32 = 16;
const ROW_STEP: i32 = 11;
/// The name's top, inside the nameplate.
const NAME_Y: i32 = 4;
/// Big-font glyph advance.
const ADVANCE: i32 = 8;

/// A glowing neon tube, 3 pixels thick, from `(x, y)` running `len` pixels across or
/// down; `(along, across)` is the pixel's position on it. Rounded ends.
fn tube(along: i32, across: i32, len: i32) -> Option<u8> {
    if !(0..len).contains(&along) || !(0..3).contains(&across) {
        return None;
    }
    let end = along == 0 || along == len - 1;
    match (across, end) {
        (1, true) => Some(pal::NEON),
        (_, true) => None,
        (1, false) => Some(pal::NEON_LIGHT),
        _ => Some(pal::NEON),
    }
}

/// The portrait frame at `(fx, fy)` from its corner: a dark rim, a bevel lit from the
/// top left, gunmetal with rivets, an inner lip, then a scanlined magenta screen.
fn frame_pixel(fx: i32, fy: i32) -> Option<u8> {
    let s = FRAME_SIZE;
    if !(0..s).contains(&fx) || !(0..s).contains(&fy) {
        return None;
    }
    let (cx, cy) = (fx.min(s - 1 - fx), fy.min(s - 1 - fy));
    if cx + cy < 4 {
        return None;
    }
    let depth = cx.min(cy).min(cx + cy - 4);
    Some(match depth {
        0 => pal::INK,
        1 if fx + fy < s => pal::STEEL_LIGHT,
        1 => pal::STEEL_DARK,
        2 if (cx, cy) == (2, 9) || (cx, cy) == (9, 2) => pal::STEEL_LIGHT,
        2 => pal::STEEL,
        3 => pal::STEEL_DARK,
        4 => pal::INK,
        _ if fy & 1 == 1 => pal::SCREEN_SCAN,
        _ => pal::SCREEN,
    })
}

/// The text panel at `(px, py)` from its corner, `w` wide: a bevelled metal rim with a
/// vent grille along the bottom, around dark scanlined glass.
fn panel_pixel(px: i32, py: i32, w: i32) -> Option<u8> {
    let h = PANEL_BOTTOM - PANEL_TOP;
    if !(0..w).contains(&px) || !(0..h).contains(&py) {
        return None;
    }
    let (cx, cy) = (px.min(w - 1 - px), py.min(h - 1 - py));
    if cx + cy < 3 {
        return None;
    }
    let depth = cx.min(cy).min(cx + cy - 3);
    let lit = if cy <= cx { py < h / 2 } else { px < w / 2 };
    let grille = py >= h - 3 && (px - w / 2).abs() < 24;
    Some(match depth {
        0 => pal::INK,
        1 | 2 if grille => {
            if px & 1 == 0 { pal::INK } else { pal::STEEL_DARK }
        }
        1 if lit => pal::STEEL_LIGHT,
        1 => pal::STEEL_DARK,
        2 => pal::STEEL,
        3 => pal::INK,
        _ if py & 1 == 1 => pal::SCAN,
        _ => pal::PANEL,
    })
}

/// The raised header on the panel's top rim, at its far end, with magenta hazard
/// stripes. Its ends slope out towards the panel.
fn header_pixel(x: i32, y: i32, (_, x1): (i32, i32)) -> Option<u8> {
    const TOP: i32 = 2;
    if !(TOP..PANEL_TOP).contains(&y) {
        return None;
    }
    let slope = PANEL_TOP - y;
    let (left, right) = (x1 - 72 + slope, x1 - 8 - slope);
    if !(left..right).contains(&x) {
        return None;
    }
    Some(match () {
        _ if y == TOP || x == left || x == right - 1 => pal::INK,
        _ if y == TOP + 1 => pal::STEEL_LIGHT,
        _ if x >= right - 34 && x < right - 4 && y > TOP + 1 && (x + y) & 7 < 4 => pal::NEON,
        _ => pal::STEEL,
    })
}

/// The box's colour at (x, y) on screen, with the portrait frame on the right side or
/// the left.
fn box_pixel(x: i32, y: i32, right: bool) -> u8 {
    let l = layout(right);
    let (x0, x1) = l.panel;
    let fy = y - FRAME_TOP;
    // Neon tubes: on the frame's outer side and under it, and on the panel's far end
    // and bottom rim.
    let frame_side = if right { l.frame_x + FRAME_SIZE } else { l.frame_x - 3 };
    let panel_side = if right { x0 - 1 } else { x1 - 2 };
    let tubes = [
        tube(y - 18, x - frame_side, 16),
        tube(x - (l.frame_x + 12), y - (FRAME_TOP + FRAME_SIZE + 1), 20),
        tube(y - 22, x - panel_side, 16),
        tube(x - (x1 - 44), y - (PANEL_BOTTOM - 2), 24),
    ];
    tubes
        .into_iter()
        .flatten()
        .next()
        .or_else(|| header_pixel(x, y, l.panel))
        .or_else(|| frame_pixel(x - l.frame_x, fy))
        .or_else(|| panel_pixel(x - x0, y - PANEL_TOP, x1 - x0))
        .unwrap_or(pal::CLEAR)
}

/// The metal frame and scanlined panel, painted once at boot in both copies.
pub fn paint_dialog_box(bg: &mut RegularBackground) {
    let pixel = |x: i32, y: i32| box_pixel(x, y % BOX_SCROLL_RIGHT, y >= BOX_SCROLL_RIGHT);
    let mut cache = TileCache::new();
    for base in [0, BOX_SCROLL_RIGHT / 8] {
        for ty in base..base + BOX_HEIGHT / 8 {
            for tx in 0..32 {
                cache.paint(bg, (tx, ty), DIALOG_SLOT, &pixel);
            }
        }
    }
}

/// The cells holding the panel's changing parts: the nameplate, the text and the
/// prompt. The same tiles sit at a different column in each copy of the box: right
/// of the frame on the left, and left of it on the right.
const TEXT_COLS: i32 = 24;
const TEXT_ROWS: i32 = BOX_HEIGHT / 8;
const TEXT_W: usize = TEXT_COLS as usize * 8;
const TEXT_H: usize = TEXT_ROWS as usize * 8;

const fn first_col(right: bool) -> i32 {
    if right { 0 } else { WIDTH / 8 - TEXT_COLS }
}

/// One page of dialogue as shown: who speaks, the rows typed so far, and the bob of
/// the continue arrow once the line is done.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Page<'a> {
    pub name: &'a str,
    pub rows: [&'a str; talk::MAX_ROWS],
    pub arrow: Option<i32>,
}

/// The dialogue's text, painted into tiles of the box's own background rather than
/// drawn as objects: there is no room left in sprite VRAM for a second font, and it
/// leaves the GBA's 128 objects to the scene. Repainting stamps the page into a copy
/// of the box's pixels and writes only the tiles that changed, so typing a character
/// touches a tile or two.
pub struct DialogText {
    /// One tile per cell, row by row, shown in both copies of the box.
    tiles: Vec<DynamicTile16>,
    /// The box's own pixels under the cells, frame on the left then on the right.
    base: [Vec<u8>; 2],
    /// The pixels being composed, and those in the tiles now.
    next: Vec<u8>,
    shown: Vec<u8>,
    /// The page in `next`, and which side it was painted for.
    last: Option<(bool, Page<'static>)>,
}

impl DialogText {
    pub fn new(bg: &mut RegularBackground) -> Self {
        let mut tiles = Vec::new();
        for ty in 0..TEXT_ROWS {
            for tx in 0..TEXT_COLS {
                let tile = DynamicTile16::new().fill_with(pal::CLEAR);
                for right in [false, true] {
                    let base = if right { BOX_SCROLL_RIGHT / 8 } else { 0 };
                    let effect = TileEffect::new(false, false, DIALOG_SLOT);
                    bg.set_tile_dynamic16((first_col(right) + tx, base + ty), &tile, effect);
                }
                tiles.push(tile);
            }
        }
        let base = [false, true].map(|right| {
            let x0 = first_col(right) * 8;
            let mut pixels = vec![0; TEXT_W * TEXT_H];
            for (i, p) in pixels.iter_mut().enumerate() {
                let (x, y) = ((i % TEXT_W) as i32, (i / TEXT_W) as i32);
                *p = box_pixel(x0 + x, y, right);
            }
            pixels
        });
        Self {
            tiles,
            base,
            next: vec![0; TEXT_W * TEXT_H],
            // Matches no colour, so the first paint writes every tile.
            shown: vec![u8::MAX; TEXT_W * TEXT_H],
            last: None,
        }
    }

    fn put(&mut self, x: i32, y: i32, colour: u8) {
        if (0..TEXT_W as i32).contains(&x) && (0..TEXT_H as i32).contains(&y) {
            self.next[y as usize * TEXT_W + x as usize] = colour;
        }
    }

    /// Stamps `text` in the big font (3x5 doubled) at `(x, y)` in cell pixels, with a
    /// drop shadow if given.
    fn stamp(&mut self, text: &str, x: i32, y: i32, colour: u8, shadow: Option<u8>) {
        let layers = [shadow.map(|s| (1, s)), Some((0, colour))];
        for (offset, colour) in layers.into_iter().flatten() {
            for (i, byte) in text.bytes().enumerate() {
                let Some(rows) = font::glyph(byte) else {
                    continue;
                };
                let gx = x + i as i32 * ADVANCE + offset;
                for (dy, bits) in rows.iter().enumerate() {
                    for dx in 0..3 {
                        if bits & (0b100 >> dx) == 0 {
                            continue;
                        }
                        let (px, py) = (gx + dx * 2, y + offset + dy as i32 * 2);
                        for (sx, sy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                            self.put(px + sx, py + sy, colour);
                        }
                    }
                }
            }
        }
    }

    /// The speaker's name on a hazard-yellow plate across the panel's top rim, its
    /// right end slanted, trailed by three chevrons.
    fn nameplate(&mut self, name: &str, text_x: i32) {
        let (x0, x1) = (text_x - 4, text_x + Font::width(name, true) + 2);
        for y in 2..16 {
            let right = x1 + (15 - y) / 2;
            for x in x0..=right {
                let colour = match () {
                    _ if y == 2 || y == 15 || x == x0 || x == right => pal::INK,
                    _ if y == 14 => pal::PLATE_DARK,
                    _ => pal::PLATE,
                };
                self.put(x, y, colour);
            }
            if (3..9).contains(&y) {
                for k in 0..3 {
                    let bx = right + 3 + k * 5;
                    for x in bx..bx + 3 {
                        self.put(x, y, if k == 2 { pal::PLATE_DARK } else { pal::PLATE });
                    }
                }
            }
        }
        self.stamp(name, text_x, NAME_Y, pal::INK, None);
    }

    /// A small downward arrow at the end of the last row: more to come.
    fn arrow(&mut self, x: i32, y: i32) {
        for r in 0..4 {
            for dx in r..7 - r {
                self.put(x + dx + 1, y + r + 1, pal::INK);
            }
        }
        for r in 0..4 {
            for dx in r..7 - r {
                self.put(x + dx, y + r, if r == 0 { pal::NEON_LIGHT } else { pal::NEON });
            }
        }
    }

    /// Paints `page`, with the frame on the right side or the left. Typing on from the
    /// last page stamps only the new characters, and a bob redraws only the arrow; a
    /// new line or speaker repaints everything. Either way, only the tiles whose pixels
    /// changed are written.
    pub fn paint(&mut self, right: bool, page: &Page<'static>) {
        let text_x = layout(right).text_x - first_col(right) * 8;
        let follows = self.last.as_ref().is_some_and(|(was_right, old)| {
            *was_right == right
                && old.name == page.name
                && old.rows.iter().zip(&page.rows).all(|(o, n)| n.starts_with(o))
        });
        let mut dirty = Dirty::NONE;
        if let (true, Some((_, old))) = (follows, self.last) {
            for (i, (o, n)) in old.rows.iter().zip(&page.rows).enumerate() {
                if n.len() > o.len() {
                    let (x, y) = (text_x + o.len() as i32 * ADVANCE, row_y(i));
                    self.stamp(&n[o.len()..], x, y, pal::TEXT, Some(pal::INK));
                    dirty.add(x, y, text_x + n.len() as i32 * ADVANCE, y + ROW_STEP);
                }
            }
            if old.arrow != page.arrow {
                let (x, y) = arrow_pos(text_x);
                for py in y..y + 6 {
                    let r = py as usize * TEXT_W + x as usize..py as usize * TEXT_W + x as usize + 8;
                    self.next[r.clone()].copy_from_slice(&self.base[right as usize][r]);
                }
                if let Some(bob) = page.arrow {
                    self.arrow(x, y + bob);
                }
                dirty.add(x, y, x + 8, y + 6);
            }
        } else {
            self.next.copy_from_slice(&self.base[right as usize]);
            self.nameplate(page.name, text_x);
            for (i, row) in page.rows.iter().enumerate() {
                self.stamp(row, text_x, row_y(i), pal::TEXT, Some(pal::INK));
            }
            if let Some(bob) = page.arrow {
                let (x, y) = arrow_pos(text_x);
                self.arrow(x, y + bob);
            }
            dirty = Dirty::ALL;
        }
        self.last = Some((right, *page));
        self.flush(dirty);
    }

    /// Writes the tiles under `dirty` whose pixels changed.
    fn flush(&mut self, dirty: Dirty) {
        let cols = TEXT_COLS as usize;
        for ty in dirty.y0.max(0) / 8..(dirty.y1.min(TEXT_H as i32) + 7) / 8 {
            for tx in dirty.x0.max(0) / 8..(dirty.x1.min(TEXT_W as i32) + 7) / 8 {
                let (cx, cy) = (tx as usize * 8, ty as usize * 8);
                let rows = (cy..cy + 8).map(|y| y * TEXT_W + cx..y * TEXT_W + cx + 8);
                if rows.clone().all(|r| self.next[r.clone()] == self.shown[r]) {
                    continue;
                }
                let tile = &mut self.tiles[ty as usize * cols + tx as usize];
                // A whole row of the tile per word, a nibble a pixel, leftmost lowest.
                for (word, r) in tile.data_mut().iter_mut().zip(rows) {
                    let pixels = &self.next[r.clone()];
                    *word = pixels.iter().rev().fold(0, |acc, &p| acc << 4 | u32::from(p));
                    self.shown[r.clone()].copy_from_slice(pixels);
                }
            }
        }
    }
}

/// The top of row `i` of text.
const fn row_y(i: usize) -> i32 {
    ROWS_Y + i as i32 * ROW_STEP
}

/// Where the continue arrow sits, before its bob: the end of the last row.
const fn arrow_pos(text_x: i32) -> (i32, i32) {
    let x = text_x + (talk::ROW_CHARS as i32 - 1) * ADVANCE;
    (x, row_y(talk::MAX_ROWS - 1) + 3)
}

/// A rectangle of cell pixels to flush, `x1` and `y1` exclusive.
#[derive(Clone, Copy)]
struct Dirty {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl Dirty {
    const NONE: Self = Self { x0: i32::MAX, y0: i32::MAX, x1: i32::MIN, y1: i32::MIN };
    const ALL: Self = Self { x0: 0, y0: 0, x1: TEXT_W as i32, y1: TEXT_H as i32 };

    /// Grows to cover `(x0, y0)..(x1, y1)` and a pixel more right and down for shadows.
    fn add(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        *self = Self {
            x0: self.x0.min(x0),
            y0: self.y0.min(y0),
            x1: self.x1.max(x1 + 1),
            y1: self.y1.max(y1 + 1),
        };
    }
}

/// One island on the world map: a union of ellipses `(cx, cy, rx, ry)` with a top
/// colour, a pattern colour, a lit rim and a cliff face below. Textures use bit masks:
/// the GBA has no divide instruction, and this runs for every pixel at boot.
struct Island {
    blobs: &'static [(i32, i32, i32, i32)],
    top: u8,
    pattern: u8,
    rim: u8,
    cliff: u8,
    /// Whether `pattern` shows at a pixel of the top.
    texture: fn(i32, i32) -> bool,
}

/// Height of the cliff face under each island, in pixels.
const CLIFF: i32 = 4;

static ISLANDS: [Island; 3] = [
    // The Badlands: orange mesas speckled with red rock.
    Island {
        blobs: &[(42, 106, 34, 20), (62, 94, 20, 14), (22, 98, 16, 12)],
        top: ix::ORANGE,
        pattern: ix::RED,
        rim: ix::GOLD,
        cliff: ix::RED_DARK,
        texture: |x, y| (x + 3 * y) & 15 == 0 && y & 1 == 0,
    },
    // Solana Trenches: a violet strip lined with neon.
    Island {
        blobs: &[(120, 62, 34, 16), (100, 70, 16, 10), (142, 70, 16, 10)],
        top: ix::PURPLE,
        pattern: ix::LASER,
        rim: ix::LASER_LIGHT,
        cliff: ix::STEEL_DARK,
        texture: |x, y| y & 7 == 0 && x & 3 != 0,
    },
    // Ethereum City: blue blocks with lit windows.
    Island {
        blobs: &[(196, 94, 30, 22), (212, 76, 16, 12), (178, 112, 14, 8)],
        top: ix::BLUE,
        pattern: ix::CYAN,
        rim: ix::CYAN,
        cliff: ix::STEEL,
        texture: |x, y| x & 7 < 2 && y & 7 < 2,
    },
];

impl Island {
    fn bounds(&self) -> (i32, i32, i32, i32) {
        self.blobs.iter().fold((i32::MAX, i32::MAX, i32::MIN, i32::MIN), |(x0, y0, x1, y1), &(cx, cy, rx, ry)| {
            (x0.min(cx - rx), y0.min(cy - ry), x1.max(cx + rx), y1.max(cy + ry))
        })
    }

    fn inside(&self, x: i32, y: i32) -> bool {
        self.blobs.iter().any(|&(cx, cy, rx, ry)| {
            let (dx, dy) = (x - cx, y - cy);
            dx * dx * ry * ry + dy * dy * rx * rx <= rx * rx * ry * ry
        })
    }
}

/// Map height in pixels.
const MAP_H: i32 = 160;

/// Which island covers each map pixel (index + 1, or 0 for sea), worked out once so
/// the outline and cliff tests are lookups rather than more ellipse maths.
struct Land(alloc::vec::Vec<u8>);

impl Land {
    fn new() -> Self {
        let mut land = vec![0; (WIDTH * MAP_H) as usize];
        for (i, island) in ISLANDS.iter().enumerate() {
            let (x0, y0, x1, y1) = island.bounds();
            for y in y0.max(0)..=y1.min(MAP_H - 1) {
                for x in x0.max(0)..=x1.min(WIDTH - 1) {
                    if island.inside(x, y) {
                        land[(y * WIDTH + x) as usize] = i as u8 + 1;
                    }
                }
            }
        }
        Self(land)
    }

    fn at(&self, x: i32, y: i32) -> u8 {
        if (0..WIDTH).contains(&x) && (0..MAP_H).contains(&y) {
            self.0[(y * WIDTH + x) as usize]
        } else {
            0
        }
    }

    /// The land colour at a pixel, or `None` for sea. Each top is outlined, lit along
    /// its upper rim, and stands on a cliff face.
    fn pixel(&self, x: i32, y: i32) -> Option<u8> {
        let here = self.at(x, y);
        if here != 0 {
            let island = &ISLANDS[here as usize - 1];
            let edge = [(-1, 0), (1, 0), (0, -1), (0, 1)]
                .iter()
                .any(|&(dx, dy)| self.at(x + dx, y + dy) != here);
            return Some(match () {
                _ if edge => ix::OUTLINE,
                _ if self.at(x, y - 2) != here => island.rim,
                _ if (island.texture)(x, y) => island.pattern,
                _ => island.top,
            });
        }
        // The cliff hangs below the top's lower edge.
        let (above, owner) = (1..=CLIFF).find_map(|d| {
            let owner = self.at(x, y - d);
            (owner != 0).then_some((d, owner))
        })?;
        let side = self.at(x - 1, y - above) != owner || self.at(x + 1, y - above) != owner;
        let cliff = ISLANDS[owner as usize - 1].cliff;
        Some(if above == CLIFF || side { ix::OUTLINE } else { cliff })
    }
}

/// Gold dashes along the road between neighbouring places.
fn road(x: i32, y: i32) -> bool {
    MAP.windows(2).any(|pair| {
        let (ax, ay, bx, by) = (pair[0].x, pair[0].y, pair[1].x, pair[1].y);
        // Cheap box test first; it also keeps the products below in range.
        if x < ax.min(bx) - 2 || x > ax.max(bx) + 2 || y < ay.min(by) - 2 || y > ay.max(by) + 2 {
            return false;
        }
        let (dx, dy) = (bx - ax, by - ay);
        let (px, py) = (x - ax, y - ay);
        let along = px * dx + py * dy;
        let len_sq = dx * dx + dy * dy;
        if along < 0 || along > len_sq {
            return false;
        }
        // Within about a pixel of the line: |cross| / len <= 1, squared to stay integer.
        let cross = px * dy - py * dx;
        cross * cross * 4 <= len_sq * 3 && (along * 12 / len_sq) % 2 == 0
    })
}

/// The world map: islands for each place on a violet sea under a faint cyber grid,
/// joined by a dashed gold road.
pub fn paint_map(bg: &mut RegularBackground) {
    let land = Land::new();
    let sea = |x: i32, y: i32| if x & 15 == 8 || y & 15 == 8 { ix::PURPLE } else { ix::PURPLE_DARK };
    let pixel = |x: i32, y: i32| {
        if road(x, y) {
            return ix::GOLD;
        }
        land.pixel(x, y).unwrap_or_else(|| sea(x, y))
    };
    // Boxes around everything that isn't open sea: each island with its cliff, and
    // each stretch of road. Cells clear of all of them are painted as plain sea.
    let mut boxes = [(0, 0, 0, 0); ISLANDS.len() + MAP.len() - 1];
    for (b, island) in boxes.iter_mut().zip(&ISLANDS) {
        let (x0, y0, x1, y1) = island.bounds();
        *b = (x0 - 1, y0 - 1, x1 + 1, y1 + CLIFF + 1);
    }
    for (b, pair) in boxes[ISLANDS.len()..].iter_mut().zip(MAP.windows(2)) {
        let (a, z) = (&pair[0], &pair[1]);
        *b = (a.x.min(z.x) - 2, a.y.min(z.y) - 2, a.x.max(z.x) + 2, a.y.max(z.y) + 2);
    }
    let mut cache = TileCache::new();
    for ty in 0..20 {
        for tx in 0..30 {
            let (x, y) = (tx * 8, ty * 8);
            let busy = boxes
                .iter()
                .any(|&(x0, y0, x1, y1)| x + 7 >= x0 && x <= x1 && y + 7 >= y0 && y <= y1);
            if busy {
                cache.paint(bg, (tx, ty), UI_PALETTE, &pixel);
            } else {
                cache.paint(bg, (tx, ty), UI_PALETTE, &sea);
            }
        }
    }
}

/// Marker for a place whose level is still being built: a steel beacon with a "?".
pub fn soon_marker() -> SpriteVram {
    let mut c = Canvas::new(Size::S16x16);
    c.art(
        3,
        2,
        &[
            "...kkkk...",
            "..kSSSgk..",
            ".kSkkkkgk.",
            "kSgkSSkggk",
            "kggggkkggk",
            "kgggkkgggk",
            ".kggggggk.",
            "..kgkkgk..",
            "...kggk...",
            "....kk....",
        ],
    );
    c.vram()
}
