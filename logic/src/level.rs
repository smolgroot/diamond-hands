//! Levels are hand-drawn ASCII maps, one character per 8x8 tile, read in place.
//!
//! Terrain: `#` solid rock, `r` crystal boulder (solid), `=` one-way steel girder
//! (solid from above only), `T` one-way casino table top, `^` an electric fence's
//! emitter: its lightning curtain runs from there up to the rock above (see
//! [`Level::fence_hit`]). Scenery,
//! which is air for collision: `,` moss, `*` scrap, `&` a dead bot's head, `|` a table's
//! pedestal, `%` a false wall that looks like rock (a secret passage). Big pictures are
//! scenery too, drawn across a block of their letter: see [`DECOR`]. Anything else is
//! air. A column of rock from the top of the map to the bottom is a sealed wall: the
//! camera stays on one side of it, and only an elevator crosses it. Markers, which are
//! air too:
//!
//! | char | meaning |
//! | ---- | ------- |
//! | `P`  | player start (feet on the tile below) |
//! | `C`  | checkpoint flag |
//! | `o`  | diamond |
//! | `+`  | cold-wallet heal |
//! | `b`  | Sentry bot |
//! | `s`  | Crawler bot |
//! | `v`  | Drone |
//! | `t`  | rolling Mine (a casino chip in the Trenches) |
//! | `d`  | Dealer bot: throws a fan of cards |
//! | `g`  | Guard bot: armoured, fires bursts |
//! | `$`  | gold coin |
//! | `p`  | plasma cell, a part for CL207's shield |
//! | `E`  | the elevator's access terminal |
//! | `L`  | the elevator car's left edge, where it leaves from (feet row) |
//! | `l`  | where the elevator arrives (feet row) |
//! | `X`  | left edge of the boss arena (any row) |
//! | `B`  | the boss |

use crate::{TILE, talk::Line};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Empty,
    Solid,
    OneWay,
    Spike,
}

impl Tile {
    pub fn from_char(c: u8) -> Tile {
        match c {
            b'#' | b'r' => Tile::Solid,
            b'=' | b'T' => Tile::OneWay,
            b'^' => Tile::Spike,
            _ => Tile::Empty,
        }
    }
}

/// Which backdrop and tileset the renderer uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Badlands,
    /// A neon casino strip, its gaming floor and the exchange suites above it.
    Trenches,
}

/// Who waits in the arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossKind {
    /// BERA 3000, the bear war mech.
    Bera,
    /// SAM, the cyborg exchange founder with a cucumber.
    Sam,
}

/// Scenery pictures bigger than a tile: the letter fills the picture's whole block in
/// the map, `(letter, width, height)` in tiles. A wider or taller block repeats it.
pub const DECOR: [(u8, i32, i32); 13] = [
    (b'S', 2, 3),  // slot machine
    (b'M', 4, 3),  // trading screen
    (b'N', 6, 2),  // CASINO sign
    (b'F', 4, 2),  // the exchange's sign
    (b'U', 4, 2),  // pump pill sign
    (b'Q', 3, 2),  // SOL sign
    (b'K', 2, 1),  // pile of gold coins
    (b'D', 4, 5),  // casino doors
    (b'Z', 2, 4),  // server rack
    (b'V', 4, 4),  // vault door
    (b'R', 3, 1),  // roulette wheel on a table
    (b'W', 3, 2),  // VIP sign
    (b'A', 2, 3),  // neon playing card
];

/// The size of the picture `c` draws, if it is a picture.
pub fn decor_size(c: u8) -> Option<(i32, i32)> {
    DECOR.iter().find(|d| d.0 == c).map(|&(_, w, h)| (w, h))
}

/// Characters that are terrain or scenery, never markers.
fn is_scenery(c: u8) -> bool {
    matches!(c, b'.' | b'#' | b'r' | b'=' | b'T' | b'^' | b',' | b'*' | b'&' | b'|' | b'%') || decor_size(c).is_some()
}

#[derive(Debug)]
pub struct Level {
    pub name: &'static str,
    pub subtitle: &'static str,
    pub theme: Theme,
    /// Shown over the boss's health bar.
    pub boss_name: &'static str,
    /// What the boss says when the wassie walks into the arena; empty for none.
    pub boss_talk: &'static [Line],
    pub boss: BossKind,
    /// A call from CL207 when a run starts the level fresh; empty for none.
    pub call: &'static [Line],
    /// A casino level: bots pay gold coins for bounties, and the penguin's exchange
    /// waits after the boss.
    pub coins: bool,
    /// Columns where the second and third stage of the backdrop start; empty to split
    /// the level in thirds.
    pub stages: &'static [i32],
    /// The first bit of the save's shield-part mask this level's parts use.
    pub parts_base: u8,
    pub rows: &'static [&'static str],
}

/// Arena width in tiles: exactly one screen.
pub const ARENA_TILES: i32 = crate::SCREEN_W / TILE;

impl Level {
    /// A level with no map or extras, to build test levels from.
    pub const BASE: Level = Level {
        name: "TEST",
        subtitle: "",
        theme: Theme::Badlands,
        boss_name: "",
        boss_talk: &[],
        boss: BossKind::Bera,
        call: &[],
        coins: false,
        stages: &[],
        parts_base: 0,
        rows: &[],
    };

    pub fn width(&self) -> i32 {
        self.rows[0].len() as i32
    }

    pub fn height(&self) -> i32 {
        self.rows.len() as i32
    }

    pub fn px_width(&self) -> i32 {
        self.width() * TILE
    }

    pub fn px_height(&self) -> i32 {
        self.height() * TILE
    }

    /// The map character at a tile, or `.` outside the map.
    pub fn char_at(&self, tx: i32, ty: i32) -> u8 {
        if tx < 0 || ty < 0 || tx >= self.width() || ty >= self.height() {
            return b'.';
        }
        self.rows[ty as usize].as_bytes()[tx as usize]
    }

    /// Collision at a tile. The left and right edges of the map are walls; above and
    /// below it is open air, so falling out of the bottom is a pit.
    pub fn tile(&self, tx: i32, ty: i32) -> Tile {
        if tx < 0 || tx >= self.width() {
            return Tile::Solid;
        }
        Tile::from_char(self.char_at(tx, ty))
    }

    /// Tile at a pixel position.
    pub fn tile_at_px(&self, x: i32, y: i32) -> Tile {
        self.tile(x.div_euclid(TILE), y.div_euclid(TILE))
    }

    pub fn is_floor(&self, tx: i32, ty: i32) -> bool {
        matches!(self.tile(tx, ty), Tile::Solid | Tile::OneWay)
    }

    /// The top row of the lightning curtain over a fence emitter at `(tx, ty)`: it rises
    /// through air, girders and tables until rock or the top of the map.
    pub fn fence_top(&self, tx: i32, ty: i32) -> i32 {
        let mut top = ty;
        while top > 0 && self.tile(tx, top - 1) != Tile::Solid {
            top -= 1;
        }
        top
    }

    /// The fence whose curtain overlaps the pixel rect `(x, y, w, h)`, as the left and
    /// right pixel edges of its run of emitters. A curtain fills its column from the
    /// emitter up to the rock above, so a point is inside one when, looking down from
    /// it, an emitter comes before any rock.
    pub fn fence_hit(&self, x: i32, y: i32, w: i32, h: i32) -> Option<(i32, i32)> {
        let bottom = self.height();
        if y + h <= 0 {
            return None;
        }
        for tx in x.div_euclid(TILE)..=(x + w - 1).div_euclid(TILE) {
            let mut ty = y.div_euclid(TILE).max(0);
            while ty < bottom {
                match self.tile(tx, ty) {
                    Tile::Solid => break,
                    Tile::Spike => {
                        let (mut l, mut r) = (tx, tx);
                        while self.tile(l - 1, ty) == Tile::Spike {
                            l -= 1;
                        }
                        while self.tile(r + 1, ty) == Tile::Spike {
                            r += 1;
                        }
                        return Some((l * TILE, (r + 1) * TILE));
                    }
                    _ => ty += 1,
                }
            }
        }
        None
    }

    /// Every marker character with its tile position, in row-major order. Terrain and
    /// scenery are not markers.
    pub fn markers(&self) -> impl Iterator<Item = (u8, i32, i32)> + '_ {
        self.rows.iter().enumerate().flat_map(|(ty, row)| {
            row.bytes()
                .enumerate()
                .filter(|&(_, c)| !is_scenery(c))
                .map(move |(tx, c)| (c, tx as i32, ty as i32))
        })
    }

    /// The first `marker` in row-major order. A plain byte search: starting a level
    /// runs several of these, and on the GBA they must fit in a frame.
    pub fn find(&self, marker: u8) -> Option<(i32, i32)> {
        self.rows.iter().enumerate().find_map(|(ty, row)| {
            row.bytes()
                .position(|c| c == marker)
                .map(|tx| (tx as i32, ty as i32))
        })
    }

    /// The backdrop stage (0, 1 or 2) at level pixel `x`.
    pub fn stage_at(&self, x: i32) -> u8 {
        if self.stages.is_empty() {
            return (x * 3 / self.px_width().max(1)).clamp(0, 2) as u8;
        }
        self.stages.iter().filter(|&&col| x >= col * TILE).count().min(2) as u8
    }

    /// Whether column `tx` is rock from the top of the map to the bottom: a sealed wall.
    pub fn sealed(&self, tx: i32) -> bool {
        (0..self.height()).all(|ty| self.char_at(tx, ty) == b'#')
    }

    /// Left edge of the boss arena, in pixels.
    pub fn arena_px(&self) -> i32 {
        self.find(b'X').map_or(self.px_width() - crate::SCREEN_W, |(x, _)| x * TILE)
    }
}
