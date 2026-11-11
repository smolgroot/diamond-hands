//! The pause menu: three tabs (the controls, the wassie's bag, the diamond wallet) over
//! a description box, laid out like a handheld RPG's bag screen in the game's own
//! night-purple metal and neon.
//!
//! Background VRAM is tight, and this module is shaped by it:
//!
//! - All eight screenblocks (tile maps) are taken, so the menu borrows the dialogue
//!   box's layer, which is hidden while paused. That layer is 64 rows tall and the box
//!   uses only rows 0-6 and 32-38, so the menu goes in rows [`MENU_ROW`] onwards and
//!   the renderer scrolls there. It uses the sprite palette's colours (see
//!   [`super::ix`]) as background palette [`UI_PALETTE`].
//! - Only about 380 tiles are free with a level's terrain loaded, even with the
//!   dialogue text's tiles lent to the menu. So the menu is a tile-grid design: text
//!   is a fixed 8x8 cell per character and every row and icon sits on the grid. Each
//!   glyph in each colour is then one tile, shared wherever it appears, and a page
//!   takes well under 200 tiles.
//! - Each repaint composes the whole screen into packed 4bpp cells, compares them with
//!   what the tiles show, and re-tiles only the cells that changed, through a cache
//!   that shares identical cells. The cache counts the cells using each tile and frees
//!   a tile as soon as none do, so it holds just the page on screen.
//! - The menu only exists while the game is paused; the renderer
//!   [`MenuScreen::clear`]s it on resume, which frees its tiles.
//!
//! Animated parts (the selection arrow, the wassie, glints) are sprites drawn on top.

use alloc::{collections::BTreeMap, vec, vec::Vec};

use agb::display::tiled::{DynamicTile16, RegularBackground, TileEffect};
use diamond_hands_logic::{
    Game,
    controls::{self, BINDINGS},
    game::pause_menu,
    inventory::{ITEMS, Item},
};

use super::{SPRITE_KEY, ix, ui::UI_PALETTE};
use crate::font;

const W: i32 = 240;
const H: i32 = 160;
const COLS: usize = 30;
const ROWS: usize = 20;

/// The first row of the borrowed layer the menu is painted in, and the scroll that
/// shows it.
const MENU_ROW: i32 = 8;
pub const SCROLL_Y: i32 = MENU_ROW * 8;

/// Menu font: the dialogue's 5x7 glyphs, one 8x8 cell each.
const CELL: i32 = 8;

/// The tabs, in [`pause_menu`] order.
const TABS: [&str; pause_menu::TABS as usize] = ["CONTROLS", "BAG", "WALLET"];
const TAB_X: i32 = 12;
const TAB_W: i32 = 68;
const TAB_STEP: i32 = 74;
const TAB_TOP: i32 = 1;

/// The body panel and the description box below it, as (x, y, w, h).
const BODY: (i32, i32, i32, i32) = (2, 16, 236, 96);
const DESC: (i32, i32, i32, i32) = (2, 112, 236, 46);
/// The right edge text lines up to.
const RIGHT: i32 = 232;
/// Rows of text in the description box: a header, then two lines.
const DESC_HEAD_Y: i32 = 120;
const DESC_ROWS_Y: [i32; 2] = [136, 144];
const DESC_TEXT_X: i32 = 48;

/// The left pocket panel, where the wassie stands, and its feet on the pedestal.
const POCKET: (i32, i32, i32, i32) = (6, 20, 82, 88);
pub const WASSIE_FEET: (i32, i32) = (47, 96);

/// The bag's item list: first row of text, the step between rows, and where the
/// icons, names and equipped badges go. Five rows of a name over a slot fill the body
/// panel at one 16-pixel row each, all on the 8x8 grid. The badge sits on the slot's
/// line, so a long name (PLASMA CELLS) can run past it.
const LIST_Y: i32 = 24;
const LIST_STEP: i32 = 16;
const LIST_ICON_X: i32 = 104;
const LIST_NAME_X: i32 = 128;
const LIST_BADGE_X: i32 = 216;

/// The right-hand column of the wallet and the item card.
const SIDE_X: i32 = 104;
const CARD_X: i32 = 96;

/// The controls list: first row and the step between bindings (two rows each).
const CONTROLS_Y: i32 = 24;
const CONTROLS_STEP: i32 = 16;

/// The wallet's big diamond: centre x, top, half width, crown and pavilion heights.
const GEM: (i32, i32, i32, i32, i32) = (47, 40, 26, 12, 30);

/// What a page shows; a repaint happens only when this changes.
#[derive(Clone, Copy, PartialEq, Eq)]
struct PageKey {
    tab: u8,
    item: u8,
    inspecting: bool,
    diamonds: u32,
    fuel: u32,
    score: u32,
}

impl PageKey {
    fn of(game: &Game) -> Self {
        let m = game.menu;
        Self {
            tab: m.tab,
            item: m.item,
            inspecting: m.inspecting,
            diamonds: game.diamond_balance() ^ game.world.coins << 20,
            fuel: game.world.player.hover_fuel as u32,
            score: game.score(),
        }
    }
}

/// One 8x8 cell as a 4bpp tile stores it: a word per row, a nibble per pixel,
/// leftmost pixel lowest.
type Cell = [u32; 8];

/// The whole screen as packed cells, row by row, and every menu-font glyph's masks
/// (see [`glyph_masks`]) by character from [`FIRST_GLYPH`], worked out once per menu.
struct Pixels {
    cells: Vec<Cell>,
    glyphs: Vec<Option<(Cell, Cell)>>,
}

/// The backdrop's four cells: night navy with the faint purple grid line through the
/// last pixel column of even cells and the last pixel row of even cell rows.
const fn backdrop(tx_even: bool, ty_even: bool) -> Cell {
    let (navy, grid) = (splat(ix::OUTLINE), splat(ix::PURPLE_DARK));
    let column = if tx_even { (navy & !(0xF << 28)) | (grid & (0xF << 28)) } else { navy };
    let mut cell = [column; 8];
    if ty_even {
        cell[7] = grid;
    }
    cell
}
const BACKDROP: [Cell; 4] = [backdrop(true, true), backdrop(false, true), backdrop(true, false), backdrop(false, false)];

/// A cell's key in the tile cache: a hash of its pixels.
fn hash(cell: &Cell) -> u32 {
    cell.iter().fold(0x811C_9DC5, |h, &w| (h ^ w).wrapping_mul(0x0100_0193).rotate_left(5))
}

/// [`SPRITE_KEY`] as a lookup table by character; 0 for transparent.
const KEY_LUT: [u8; 128] = {
    let mut lut = [0; 128];
    let mut i = 0;
    while i < SPRITE_KEY.len() {
        lut[SPRITE_KEY[i].0 as usize] = SPRITE_KEY[i].1;
        i += 1;
    }
    lut
};

/// The first character the glyph table holds: the font has nothing below '!'.
const FIRST_GLYPH: u8 = b'!';

/// A colour repeated in every nibble of a word.
const fn splat(c: u8) -> u32 {
    c as u32 * 0x1111_1111
}

/// A menu-font glyph's lit pixels and its shadow's, as nibble masks over an 8x8 cell:
/// the glyph one pixel in from the left, the shadow one further right and down. The
/// cell's last row stays clear, so stacked rows of text keep a pixel apart.
fn glyph_masks(rows: &[u8; 7]) -> (Cell, Cell) {
    let mut glyph = [0; 8];
    let mut shadow = [0; 8];
    for (y, &bits) in rows.iter().enumerate() {
        for dx in 0..5 {
            if bits & (0b10000 >> dx) != 0 {
                glyph[y] |= 0xF << ((1 + dx) * 4);
                if y < 6 {
                    shadow[y + 1] |= 0xF << ((2 + dx) * 4);
                }
            }
        }
    }
    (glyph, shadow)
}

impl Pixels {
    fn px(&mut self, x: i32, y: i32, c: u8) {
        if !(0..W).contains(&x) || !(0..H).contains(&y) {
            return;
        }
        let row = &mut self.cells[(y >> 3) as usize * COLS + (x >> 3) as usize][(y & 7) as usize];
        let shift = (x & 7) as u32 * 4;
        *row = (*row & !(0xF << shift)) | u32::from(c) << shift;
    }

    /// A filled rectangle: per row, the odd pixels at each end, and whole cell rows a
    /// word at a time between them.
    fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, c: u8) {
        let (x0, x1) = (x.max(0), (x + w).min(W));
        if x0 >= x1 {
            return;
        }
        let word = splat(c);
        let head = ((x0 + 7) & !7).min(x1);
        let tail = (x1 & !7).max(head);
        for yy in y.max(0)..(y + h).min(H) {
            let (base, r) = ((yy >> 3) as usize * COLS, (yy & 7) as usize);
            for xx in (x0..head).chain(tail..x1) {
                let row = &mut self.cells[base + (xx >> 3) as usize][r];
                let shift = (xx & 7) as u32 * 4;
                *row = (*row & !(0xF << shift)) | word & (0xF << shift);
            }
            for tx in (head >> 3)..(tail >> 3) {
                self.cells[base + tx as usize][r] = word;
            }
        }
    }

    fn new() -> Self {
        Self {
            cells: vec![[0; 8]; COLS * ROWS],
            glyphs: (FIRST_GLYPH..128).map(|b| font::dialog_glyph(b).map(glyph_masks)).collect(),
        }
    }

    /// The backdrop over the whole screen.
    fn backdrop(&mut self) {
        for (i, cell) in self.cells.iter_mut().enumerate() {
            let (tx, ty) = (i % COLS, i / COLS);
            *cell = BACKDROP[(tx & 1) | (ty & 1) << 1];
        }
    }

    /// Text with a drop shadow, `scale` times the size, `advance` pixels a character.
    /// Returns the x just past it.
    fn text_with(&mut self, s: &str, x: i32, y: i32, c: u8, scale: i32, advance: i32) -> i32 {
        let on_grid = scale == 1 && advance == CELL && x & 7 == 0 && y & 7 == 0 && x >= 0 && y >= 0;
        if on_grid {
            // A cell a character: mask the shadow and the glyph straight into it.
            let (ink, shade) = (splat(c), splat(ix::OUTLINE));
            for (i, byte) in s.bytes().enumerate() {
                let tx = (x >> 3) as usize + i;
                let glyph = self.glyphs.get(byte.wrapping_sub(FIRST_GLYPH) as usize).copied();
                let Some(Some((glyph, shadow))) = glyph.filter(|_| tx < COLS) else {
                    continue;
                };
                let cell = &mut self.cells[(y >> 3) as usize * COLS + tx];
                for r in 0..8 {
                    cell[r] = (cell[r] & !shadow[r]) | (shade & shadow[r]);
                    cell[r] = (cell[r] & !glyph[r]) | (ink & glyph[r]);
                }
            }
            return x + s.len() as i32 * advance;
        }
        for (offset, colour) in [(1, ix::OUTLINE), (0, c)] {
            for (i, byte) in s.bytes().enumerate() {
                let Some(rows) = font::dialog_glyph(byte) else {
                    continue;
                };
                // The glyph sits a pixel in from the cell's left, its shadow inside it.
                let gx = x + i as i32 * advance + scale + offset;
                for (dy, bits) in rows.iter().enumerate() {
                    for dx in 0..5 {
                        if bits & (0b10000 >> dx) != 0 {
                            self.fill(gx + dx * scale, y + offset + dy as i32 * scale, scale, scale, colour);
                        }
                    }
                }
            }
        }
        x + s.len() as i32 * advance
    }

    /// Text on the grid, a cell a character.
    fn text(&mut self, s: &str, x: i32, y: i32, c: u8) -> i32 {
        self.text_with(s, x, y, c, 1, CELL)
    }

    /// Text ending at `right`.
    fn text_right(&mut self, s: &str, right: i32, y: i32, c: u8) {
        self.text(s, right - width(s), y, c);
    }

    /// An ASCII drawing in [`SPRITE_KEY`] characters, `scale` times the size.
    fn art(&mut self, rows: &[&str], x: i32, y: i32, scale: i32) {
        for (dy, row) in rows.iter().enumerate() {
            for (dx, ch) in row.bytes().enumerate() {
                let c = KEY_LUT[(ch & 0x7F) as usize];
                if c == 0 {
                    continue;
                }
                let (px, py) = (x + dx as i32 * scale, y + dy as i32 * scale);
                if scale == 1 {
                    self.px(px, py, c);
                } else {
                    self.fill(px, py, scale, scale, c);
                }
            }
        }
    }

    /// A riveted metal panel: dark rim, a bevel lit from the top left, a dark lip,
    /// then `inner`, with clipped corners.
    fn panel(&mut self, (x, y, w, h): (i32, i32, i32, i32), inner: u8) {
        // Each ring as four strips, so every pixel is written about once.
        let ring = |p: &mut Self, i: i32, top_left: u8, bottom_right: u8| {
            let (rx, ry, rw, rh) = (x + i, y + i, w - 2 * i, h - 2 * i);
            p.fill(rx, ry, rw, 1, top_left);
            p.fill(rx, ry + 1, 1, rh - 2, top_left);
            p.fill(rx, ry + rh - 1, rw, 1, bottom_right);
            p.fill(rx + rw - 1, ry + 1, 1, rh - 2, bottom_right);
        };
        ring(self, 0, ix::OUTLINE, ix::OUTLINE);
        ring(self, 1, ix::STEEL, ix::STEEL_DARK);
        ring(self, 2, ix::OUTLINE, ix::OUTLINE);
        self.fill(x + 3, y + 3, w - 6, h - 6, inner);
        // Clipped outer corners show the backdrop's navy.
        for (cx, cy) in [(x, y), (x + w - 1, y), (x, y + h - 1), (x + w - 1, y + h - 1)] {
            self.px(cx, cy, ix::OUTLINE);
        }
        for (rx, ry) in [(x + 2, y + 2), (x + w - 3, y + 2), (x + 2, y + h - 3), (x + w - 3, y + h - 3)] {
            self.px(rx, ry, ix::STEEL_LIGHT);
        }
    }

    /// Teal corner brackets just inside a box: industrial range-finder marks.
    fn brackets(&mut self, (x, y, w, h): (i32, i32, i32, i32), c: u8) {
        let (x1, y1) = (x + w - 1, y + h - 1);
        for (cx, cy, dx, dy) in [(x, y, 1, 1), (x1, y, -1, 1), (x, y1, 1, -1), (x1, y1, -1, -1)] {
            for i in 0..5 {
                self.px(cx + i * dx, cy, c);
                self.px(cx, cy + i * dy, c);
            }
        }
    }

    /// A glowing magenta tube along a row: lit core, rounded ends.
    fn neon(&mut self, x: i32, y: i32, len: i32) {
        self.fill(x + 1, y, len - 2, 3, ix::LASER);
        self.fill(x, y + 1, len, 1, ix::LASER);
        self.fill(x + 1, y + 1, len - 2, 1, ix::LASER_LIGHT);
    }
}

fn width(s: &str) -> i32 {
    s.len() as i32 * CELL
}

/// A tile in the menu's cache and how many cells use it.
struct Slot {
    tile: DynamicTile16,
    cell: Cell,
    /// Its key in `ids`.
    key: u32,
    users: u16,
}

/// The menu's tiles and pixels. Call [`MenuScreen::clear`] before dropping it, so the
/// layer lets go of its tiles.
pub struct MenuScreen {
    /// Each distinct cell's slot in `slots`, by [`hash`] (probing on to the next key
    /// when two cells collide).
    ids: BTreeMap<u32, usize>,
    /// Tiles by slot; `None` for a freed slot, listed in `free` for reuse.
    slots: Vec<Option<Slot>>,
    free: Vec<usize>,
    next: Pixels,
    /// The bag's page without its selection (see [`compose_base`]), kept while the bag
    /// is open so moving the selection redraws only what moves. Other pages are
    /// composed straight into `next`: the heap has no room to keep them all.
    bag_base: Option<Vec<Cell>>,
    /// The slot each menu cell shows now, [`NO_SLOT`] where the menu hasn't painted.
    shown: Vec<u16>,
    painted: Option<PageKey>,
    /// Cell rows of `next` still to be tiled, one bit each.
    pending: u32,
}

const NO_SLOT: u16 = u16::MAX;

/// Most tiles set per frame. A whole new page takes a few frames, wiping down the
/// screen, and the game keeps reading the buttons meanwhile.
const CELLS_PER_FRAME: usize = 160;

/// The cell rows the bag repaints when only the selected item changes: the item
/// names, and the description box's text.
const BAG_ROWS: [core::ops::RangeInclusive<usize>; 2] = [
    (LIST_Y / 8) as usize..=((LIST_Y + (ITEMS.len() as i32 - 1) * LIST_STEP) / 8) as usize,
    (DESC_HEAD_Y / 8) as usize..=(DESC_ROWS_Y[1] / 8) as usize,
];

impl MenuScreen {
    pub fn new() -> Self {
        Self {
            ids: BTreeMap::new(),
            slots: Vec::new(),
            free: Vec::new(),
            next: Pixels::new(),
            bag_base: None,
            shown: vec![NO_SLOT; COLS * ROWS],
            painted: None,
            pending: 0,
        }
    }

    /// Brings `bg` towards the game's menu state: composes the page when the state
    /// changes, then tiles up to [`CELLS_PER_FRAME`] changed cells, top row first.
    pub fn paint(&mut self, game: &Game, bg: &mut RegularBackground) {
        let key = PageKey::of(game);
        if self.painted != Some(key) {
            let was = self.painted.replace(key);
            let in_bag = key.tab == pause_menu::BAG && !key.inspecting;
            // The bag's selection moving touches only the names and the description.
            let only_selection = in_bag && was.is_some_and(|w| PageKey { item: key.item, ..w } == key);
            if only_selection {
                for r in BAG_ROWS.iter().flat_map(|r| r.clone()) {
                    self.pending |= 1 << r;
                }
            } else {
                self.pending = (1 << ROWS) - 1;
            }
            if !in_bag {
                self.bag_base = None;
                compose_base(&mut self.next, game);
            } else {
                match &self.bag_base {
                    Some(base) => self.next.cells.copy_from_slice(base),
                    None => {
                        compose_base(&mut self.next, game);
                        self.bag_base = Some(self.next.cells.clone());
                    }
                }
                bag_selection(&mut self.next, key.item as usize);
            }
        }
        let mut set = 0;
        while self.pending != 0 && set < CELLS_PER_FRAME {
            let r = self.pending.trailing_zeros() as usize;
            self.pending &= !(1 << r);
            set += self.flush(bg, r);
        }
    }

    /// Points every menu cell of `bg` at `blank` and drops the menu's tiles, which
    /// frees them once the frame on screen moves on.
    pub fn clear(&mut self, bg: &mut RegularBackground, blank: &DynamicTile16) {
        for i in 0..COLS * ROWS {
            let (tx, ty) = ((i % COLS) as i32, (i / COLS) as i32);
            bg.set_tile_dynamic16((tx, MENU_ROW + ty), blank, TileEffect::new(false, false, 0));
        }
        self.ids.clear();
        self.slots.clear();
        self.free.clear();
        self.shown.fill(NO_SLOT);
        self.painted = None;
        self.pending = 0;
    }

    /// Tiles the cells whose pixels changed. A tile no cell uses any more is freed
    /// straight away (its VRAM comes back once the frame on screen moves on), so the
    /// cache only ever holds the page being shown.
    /// Returns how many cells were set.
    fn flush(&mut self, bg: &mut RegularBackground, row: usize) -> usize {
        let effect = TileEffect::new(false, false, UI_PALETTE);
        let mut set = 0;
        for i in row * COLS..(row + 1) * COLS {
            let cell = self.next.cells[i];
            let old = (self.shown[i] != NO_SLOT).then_some(self.shown[i] as usize);
            if old.and_then(|s| self.slots[s].as_ref()).is_some_and(|s| s.cell == cell) {
                continue;
            }
            // Probe from the cell's hash to its slot, or to the first free key.
            let mut key = hash(&cell);
            let found = loop {
                match self.ids.get(&key) {
                    Some(&id) if self.slots[id].as_ref().is_some_and(|s| s.cell == cell) => break Some(id),
                    Some(_) => key = key.wrapping_add(1),
                    None => break None,
                }
            };
            let id = match found {
                Some(id) => id,
                None => {
                    let mut tile = DynamicTile16::new();
                    tile.data_mut().copy_from_slice(&cell);
                    let slot = Some(Slot { tile, cell, key, users: 0 });
                    let id = match self.free.pop() {
                        Some(free) => {
                            self.slots[free] = slot;
                            free
                        }
                        None => {
                            self.slots.push(slot);
                            self.slots.len() - 1
                        }
                    };
                    self.ids.insert(key, id);
                    id
                }
            };
            let (tx, ty) = ((i % COLS) as i32, (i / COLS) as i32);
            if let Some(slot) = self.slots[id].as_mut() {
                slot.users += 1;
                bg.set_tile_dynamic16((tx, MENU_ROW + ty), &slot.tile, effect);
                set += 1;
            }
            self.shown[i] = id as u16;
            // The cell's old tile loses a user, and goes once it has none.
            if let Some(old) = old
                && let Some(slot) = self.slots[old].as_mut()
            {
                slot.users -= 1;
                if slot.users == 0 {
                    self.ids.remove(&slot.key);
                    self.slots[old] = None;
                    self.free.push(old);
                }
            }
        }
        set
    }
}

/// Paints the menu for the game's state into `p`, all but the bag's selection (see
/// [`bag_selection`]).
fn compose_base(p: &mut Pixels, game: &Game) {
    p.backdrop();
    let menu = game.menu;
    p.panel(BODY, ix::PURPLE_DARK);
    tabs(p, menu.tab);
    p.panel(DESC, ix::OUTLINE);
    p.brackets((DESC.0 + 4, DESC.1 + 4, DESC.2 - 8, DESC.3 - 8), ix::CYAN);
    match menu.tab {
        pause_menu::CONTROLS => controls_page(p),
        pause_menu::WALLET => wallet_page(p, game),
        _ if menu.inspecting => item_card(p, game, menu.item as usize),
        _ => bag_page(p, game),
    }
}

/// The tab strip: the open tab lit with a neon top and joined to the body, the
/// others dark steel. Arrows at the ends say Left and Right switch.
fn tabs(p: &mut Pixels, open: u8) {
    p.text("<", 0, 8, ix::CYAN);
    p.text(">", W - CELL, 8, ix::CYAN);
    for (i, label) in TABS.iter().enumerate() {
        let x = TAB_X + i as i32 * TAB_STEP;
        let lit = i as u8 == open;
        let (fill, text) = if lit { (ix::PURPLE, ix::WHITE) } else { (ix::STEEL_DARK, ix::STEEL_LIGHT) };
        // Clipped top corners; the open tab runs down into the body panel's rim.
        let bottom = if lit { BODY.1 + 3 } else { BODY.1 };
        for y in TAB_TOP..bottom {
            let inset = (TAB_TOP + 3 - y).max(0);
            p.fill(x + inset, y, TAB_W - 2 * inset, 1, ix::OUTLINE);
            if y > TAB_TOP {
                let inner = inset + 1;
                p.fill(x + inner, y, TAB_W - 2 * inner, 1, fill);
            }
        }
        if lit {
            p.neon(x + 4, TAB_TOP + 1, TAB_W - 8);
        } else {
            p.fill(x + 4, TAB_TOP + 2, TAB_W - 8, 1, ix::STEEL);
        }
        // On the grid, as near the middle as it goes.
        let tx = (x + (TAB_W - width(label)) / 2 + 4) & !7;
        p.text(label, tx, 8, text);
    }
}

/// The description box's header (title on the left, button hint on the right), two
/// rows of text, and an icon at 2x to their left.
fn describe(p: &mut Pixels, icon: &[&str], title: &str, hint: &str, rows: [&str; 2]) {
    p.art(icon, 8, DESC_HEAD_Y, 2);
    p.text(title, DESC_TEXT_X, DESC_HEAD_Y, ix::WHITE);
    p.text_right(hint, RIGHT, DESC_HEAD_Y, ix::CYAN);
    p.fill(DESC_TEXT_X, DESC_HEAD_Y + 11, RIGHT - DESC_TEXT_X, 1, ix::PURPLE);
    for (row, y) in rows.iter().zip(DESC_ROWS_Y) {
        p.text(row, DESC_TEXT_X, y, ix::STEEL_LIGHT);
    }
}

/// Every binding on a band of its own: a badge for the button, the button's name,
/// what it does in play (white) and, below, in menus (teal).
fn controls_page(p: &mut Pixels) {
    for (i, b) in BINDINGS.iter().enumerate() {
        let y = CONTROLS_Y + i as i32 * CONTROLS_STEP;
        if i % 2 == 1 {
            p.fill(BODY.0 + 3, y, BODY.2 - 6, CONTROLS_STEP, ix::PURPLE);
        }
        p.art(badge(b.keys), 8, y + 2, 1);
        p.text(controls::label(b.keys), 24, y, ix::GOLD);
        p.text(b.play, 72, y, ix::WHITE);
        if !b.menu.is_empty() {
            let x = p.text("MENU:", 72, y + 8, ix::STEEL);
            p.text(b.menu, x + CELL, y + 8, ix::CYAN);
        }
    }
    describe(p, &GAMEPAD, "CONTROLS", "B: RESUME", ["THE GAME WAITS WHILE", "THIS MENU IS OPEN."]);
}

/// The bag, but for the selection: the wassie on a pedestal (a sprite, drawn by the
/// renderer) and the item list's icons, slots and equipped badges.
fn bag_page(p: &mut Pixels, game: &Game) {
    p.panel(POCKET, ix::OUTLINE);
    p.text("WASSIE", 24, 32, ix::CYAN);
    pedestal(p, WASSIE_FEET.0, WASSIE_FEET.1);

    for (i, info) in ITEMS.iter().enumerate() {
        let y = LIST_Y + i as i32 * LIST_STEP;
        p.art(icon(info.item), LIST_ICON_X, y, 1);
        p.text(info.kind, LIST_NAME_X, y + 8, ix::STEEL);
        if info.item.equipped(game) {
            p.art(&EQUIPPED_BADGE, LIST_BADGE_X, y + CELL, 1);
        }
    }
}

/// What moves with the bag's selection: the item names, the chosen one lit (the arrow
/// beside it is a sprite), and its description.
fn bag_selection(p: &mut Pixels, selected: usize) {
    for (i, info) in ITEMS.iter().enumerate() {
        let y = LIST_Y + i as i32 * LIST_STEP;
        let name = if i == selected { ix::WHITE } else { ix::STEEL_LIGHT };
        p.text(info.name, LIST_NAME_X, y, name);
    }
    let info = &ITEMS[selected];
    describe(p, icon(info.item), info.name, "A: INSPECT", info.about);
}

/// The inspected item's card: a big icon on a pedestal, its name large, its slot and
/// state, the detail text, and a live gauge if it has one.
fn item_card(p: &mut Pixels, game: &Game, selected: usize) {
    let info = &ITEMS[selected];
    p.panel(POCKET, ix::OUTLINE);
    p.brackets((POCKET.0 + 6, POCKET.1 + 6, POCKET.2 - 12, POCKET.3 - 12), ix::CYAN);
    p.art(icon(info.item), 24, 32, 3);
    pedestal(p, WASSIE_FEET.0, WASSIE_FEET.1 - 8);

    p.text_with(info.name, CARD_X, 24, ix::WHITE, 2, card_title_advance(info.name));
    let state = if info.item.equipped(game) { "EQUIPPED" } else { "NOT HELD" };
    let after = p.text(info.kind, CARD_X, 48, ix::GOLD);
    p.text(state, after + CELL, 48, ix::CYAN);
    for (i, row) in info.detail.iter().enumerate() {
        p.text(row, CARD_X, 64 + CELL * i as i32, ix::STEEL_LIGHT);
    }
    if let Some((value, max)) = info.gauge.reading(game) {
        let label = if info.item == Item::Cells { "CELLS" } else { "FUEL" };
        let end = p.text(label, CARD_X, 104 - CELL, ix::GOLD);
        let (bx, bw) = (end + CELL, RIGHT - end - CELL);
        let y = 104 - CELL + 1;
        p.fill(bx, y, bw, 6, ix::OUTLINE);
        p.fill(bx + 1, y + 1, bw - 2, 4, ix::STEEL_DARK);
        let lit = (bw - 2) * value as i32 / max.max(1) as i32;
        p.fill(bx + 1, y + 1, lit, 4, ix::CYAN);
        p.fill(bx + 1, y + 1, lit, 1, ix::WHITE);
    }
    describe(p, icon(info.item), info.name, "B: BACK", info.about);
}

/// The step between the letters of the card's name, drawn at 2x: 14 pixels, or closer
/// for a long name (PLASMA CELLS) so its last 10-pixel letter still ends by [`RIGHT`].
fn card_title_advance(name: &str) -> i32 {
    let gaps = (name.len() as i32 - 1).max(1);
    14.min((RIGHT - CARD_X - 10) / gaps)
}

/// The wallet: a big cut diamond, the balance in large digits, and where it stands.
/// Every number is the game's own: the HUD's count, the level's total, the save.
fn wallet_page(p: &mut Pixels, game: &Game) {
    let world = &game.world;
    p.panel(POCKET, ix::OUTLINE);
    gem(p);
    pedestal(p, WASSIE_FEET.0, WASSIE_FEET.1);

    p.text("BALANCE", SIDE_X, 24, ix::CYAN);
    let mut buf = [0; 10];
    let end = p.text_with(font::number(game.diamond_balance(), &mut buf), SIDE_X, 36, ix::WHITE, 2, 16);
    p.text("DIAMONDS", end + CELL, 44, ix::STEEL_LIGHT);

    let total = world.level.markers().filter(|&(c, _, _)| c == b'o').count() as u32;
    let mut a = [0; 10];
    let mut b = [0; 10];
    let mut found = [0; 21];
    let (n, t) = (font::number(world.diamonds, &mut a), font::number(total, &mut b));
    let len = n.len() + 1 + t.len();
    found[..n.len()].copy_from_slice(n.as_bytes());
    found[n.len()] = b'/';
    found[n.len() + 1..len].copy_from_slice(t.as_bytes());
    let found = core::str::from_utf8(&found[..len]).unwrap_or("");
    let mut c = [0; 10];
    let mut d = [0; 10];
    let mut e = [0; 10];
    let rows = [
        ("BANKED", font::number(game.wallet, &mut c)),
        ("FOUND HERE", found),
        ("GOLD COINS", font::number(world.coins + game.purse, &mut d)),
        ("SCORE", font::number(game.score(), &mut e)),
    ];
    for (i, (label, value)) in rows.into_iter().enumerate() {
        let y = 58 + 13 * i as i32;
        p.fill(SIDE_X, y + 10, RIGHT - SIDE_X, 1, ix::PURPLE);
        p.text(label, SIDE_X, y, ix::STEEL_LIGHT);
        p.text_right(value, RIGHT, y, ix::WHITE);
    }

    describe(p, &DIAMOND_ICON, "DIAMONDS", "B: RESUME", ["CLEARS BANK THEM. THE", "PENGUIN BUYS GOLD 1:8."]);
}

/// A round steel plinth under whatever stands on the pocket panel, with a magenta rim.
fn pedestal(p: &mut Pixels, cx: i32, feet: i32) {
    for dy in 0..6i32 {
        let half = 22 - (dy - 2).abs();
        let c = match dy {
            0 => ix::LASER_LIGHT,
            1 => ix::LASER,
            5 => ix::OUTLINE,
            _ => ix::STEEL_DARK,
        };
        p.fill(cx - half, feet + dy, half * 2, 1, c);
    }
}

/// The wallet's big cut diamond: a crown and a pavilion with lit, mid and shaded
/// facets, outlined.
fn gem(p: &mut Pixels) {
    let (cx, top, half, crown, pavilion) = GEM;
    let table = half * 5 / 9;
    for y in 0..crown + pavilion {
        let hw = if y < crown {
            table + (half - table) * y / crown
        } else {
            half * (pavilion - (y - crown)) / pavilion
        };
        for dx in -hw..=hw {
            let edge = dx.abs() >= hw - 1 || y == 0 || y == crown + pavilion - 1;
            let third = hw / 3;
            let c = match () {
                _ if edge => ix::OUTLINE,
                // Crown: a bright table between angled side facets, the girdle
                // catching the light along its bottom.
                _ if y == crown - 1 => ix::WHITE,
                _ if y < crown && dx.abs() < table - 1 && dx < -table / 3 => ix::WHITE,
                _ if y < crown && (dx.abs() < table - 1 || dx < 0) => ix::CYAN,
                _ if y < crown => ix::BLUE,
                // Pavilion: three facets narrowing to the point.
                _ if dx < -third => ix::CYAN,
                _ if dx <= third => ix::BLUE,
                _ => ix::PURPLE,
            };
            p.px(cx + dx, top + y, c);
        }
    }
}

/// Where the wallet diamond's glints go, for the renderer's sparkle sprites.
pub const GEM_GLINTS: [(i32, i32); 3] = [(GEM.0 - 12, GEM.1 + 5), (GEM.0 + 9, GEM.1 + 20), (GEM.0 - 4, GEM.1 + 32)];

/// Where the selection arrow points at item row `i`: the left of its icon.
pub fn item_cursor(i: u8) -> (i32, i32) {
    (LIST_ICON_X, LIST_Y + i as i32 * LIST_STEP + 4)
}

/// The bag's icon for each item: 16x16 drawings in [`SPRITE_KEY`] characters, after
/// the inventory mockup in the project skill (`inventory-mockup.png`): violet housings,
/// steel trim, gold caps, cyan and magenta lights.
fn icon(item: Item) -> &'static [&'static str] {
    match item {
        Item::Blaster => &BLASTER_ICON,
        Item::Jetpack => &JETPACK_ICON,
        Item::Shield => &SHIELD_ICON,
        Item::Scarf => &SCARF_ICON,
        Item::Cells => &CELL_ICON,
    }
}

/// The blaster side on: a violet slide with a cyan sight, a magenta energy strip
/// running to a cyan muzzle, steel trim and a grip raked back.
const BLASTER_ICON: [&str; 16] = [
    "................",
    "................",
    "...kkkk.........",
    "..kiiibkkkkkkkk.",
    ".kSvvvvvvCCCCCik",
    ".kgvvvvvvcccccik",
    ".kgVVVVVVVVVVVk.",
    ".kSgVGGGGGGGGVk.",
    "..kSgvVkkkkkkk..",
    "..kSgvVkyk......",
    ".kSgvVkkkk......",
    ".kSgvVk.........",
    "kSgvVk..........",
    "kgGGVk..........",
    "kkkkkk..........",
    "................",
];

/// Twin steel tanks with gold caps and rings either side of a violet housing, its
/// screen showing a cyan S, and cyan-hot nozzles.
const JETPACK_ICON: [&str; 16] = [
    "................",
    "................",
    ".kkkk......kkkk.",
    "kyyyokkkkkkoyyyk",
    "kgwSgkvvvvkgSwgk",
    "kgwSgkviiikgSwgk",
    "kvSgVkivvvkVgSvk",
    "kvSgVkviivkVgSvk",
    "kvSgVkVVVikVgSvk",
    "kVggVkiiiVkVggVk",
    "kyyyokVVVVkoyyyk",
    ".kVVkkGGGGkkVVk.",
    ".kiik.kkkk.kiik.",
    ".cwwc......cwwc.",
    "..ii........ii..",
    "................",
];

/// CL207's plasma shield, a steel belt seen from a little above: violet inside,
/// magenta lights either side of a buckle with a cyan shield on it.
const SHIELD_ICON: [&str; 16] = [
    "................",
    "................",
    "................",
    "....kkkkkkkk....",
    "..kkgSSSSSSgkk..",
    ".kSgVVVvvVVVgSk.",
    "kSgVkkkkkkkkVgSk",
    "kgSSkkkkkkkkSSGk",
    "kgCgSSSSSSSggCGk",
    "kgcgSkiiiikGgcGk",
    ".kggSkibbikGGGk.",
    "..kkSkkiikkGkk..",
    "....kgggggGk....",
    "....kkkkkkkk....",
    "................",
    "................",
];

/// A plasma cell, a part for the shield: steel caps with gold ends on a glass tube
/// of cyan plasma, a white swirl through it.
const CELL_ICON: [&str; 16] = [
    "......kkkk......",
    "...kkkyyyokkk...",
    "...kSSSSSSSgk...",
    "...kgggggggGk...",
    "....kkkkkkkk....",
    "....kbiwwibk....",
    "....kbwiiibk....",
    "....kbiwiibk....",
    "....kbiiwibk....",
    "....kbiiiwbk....",
    "....kbiwwibk....",
    "....kkkkkkkk....",
    "...kSSSSSSSgk...",
    "...kgggggggGk...",
    "...kkkyyyokkk...",
    "......kkkk......",
];

/// The wassie's red scarf: a twisted loop and two tails blowing out behind it.
const SCARF_ICON: [&str; 16] = [
    "................",
    ".......kkkkkkk..",
    "....kkrrrrrrRRk.",
    "..kkrrRRRRRRRRrk",
    "..krRkkkkkRRrrRk",
    ".krrRRRRrrrrrRRk",
    ".krrrrrrrrrrRRRk",
    "..kRrrrrrrRRRRk.",
    "...kRrRRkkRRRk..",
    "..kkkkkkRRRk....",
    ".krrrrrrrrRk....",
    "kRrrrrrrrrk.....",
    "kkkRrrrrrRk.....",
    "..kRkkkRrk......",
    "..kk...kk.......",
    "................",
];

const DIAMOND_ICON: [&str; 16] = [
    "................",
    "................",
    "....kkkkkkkk....",
    "...kiwwiiibbk...",
    "..kiwwiiiibbbk..",
    ".kiiiiiiiibbbbk.",
    ".kkkkkkkkkkkkkk.",
    "..kiiibbbbbvvk..",
    "...kiibbbbvvk...",
    "....kiibbbvk....",
    ".....kibbvk.....",
    "......kbvk......",
    ".......kk.......",
    "................",
    "................",
    "................",
];

const GAMEPAD: [&str; 16] = [
    "................",
    "................",
    "................",
    "................",
    "..kkkkkkkkkkkk..",
    ".kSSSSSSSSSSSSk.",
    "kSSkSSSSSSSScSSk",
    "kSkkkSSSSSSSSiSk",
    "kSSkSSSSSSScSSSk",
    "kSSSSSkkSkkSSSSk",
    ".kSSSkk..kkSSSk.",
    "..kkk......kkk..",
    "................",
    "................",
    "................",
    "................",
];

/// "E" for equipped, on a teal tag.
const EQUIPPED_BADGE: [&str; 8] = [
    ".kkkkkkkkkkkkk..",
    "kiiiiiiiiiiiiik.",
    "kiiwwwwwwwiiiik.",
    "kiiwiiiiiiiiiik.",
    "kiiwwwwwiiiiiik.",
    "kiiwiiiiiiiiiik.",
    "kiiwwwwwwwiiiik.",
    ".kkkkkkkkkkkkk..",
];

/// A badge for a button in the controls list, 12x12 or smaller.
fn badge(keys: u16) -> &'static [&'static str] {
    use diamond_hands_logic::keys as k;
    match keys {
        controls::DPAD => &DPAD_BADGE,
        k::A => &A_BADGE,
        k::B => &B_BADGE,
        k::START => &START_BADGE,
        _ => &SHOULDER_BADGE,
    }
}

const DPAD_BADGE: [&str; 12] = [
    "....kkkk....",
    "....kSSk....",
    "....kggk....",
    "kkkkkggkkkkk",
    "kSggggggggGk",
    "kSggggggggGk",
    "kkkkkggkkkkk",
    "....kggk....",
    "....kGGk....",
    "....kkkk....",
    "............",
    "............",
];

const A_BADGE: [&str; 12] = [
    "...kkkkk....",
    "..kCcccck...",
    ".kCcckccck..",
    "kCcckckccck.",
    "kcckcccckck.",
    "kcckkkkkcck.",
    "kcckcccckck.",
    ".kcckcckck..",
    "..kcccccRk..",
    "...kkkkk....",
    "............",
    "............",
];

const B_BADGE: [&str; 12] = [
    "...kkkkk....",
    "..kiiiiik...",
    ".kiikkkiiik.",
    "kiiikiikiik.",
    "kbiikkkiibk.",
    "kbiikiikibk.",
    "kbiikkkiibk.",
    ".kbbbbbbbk..",
    "..kbbbbbk...",
    "...kkkkk....",
    "............",
    "............",
];

const SHOULDER_BADGE: [&str; 12] = [
    "............",
    "............",
    "..kkkkkkkk..",
    ".kSSSSSSSSk.",
    "kSggggggggGk",
    "kgGGGGGGGGGk",
    "kkkkkkkkkkkk",
    "............",
    "............",
    "............",
    "............",
    "............",
];

const START_BADGE: [&str; 12] = [
    "............",
    "............",
    "............",
    ".kkkkkkkkkk.",
    "kSSSSSSSSSSk",
    "kggggggggggk",
    ".kkkkkkkkkk.",
    "............",
    "............",
    "............",
    "............",
    "............",
];
