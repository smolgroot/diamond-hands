//! Levels are hand-drawn ASCII maps, one character per 8x8 tile, read in place.
//!
//! Terrain: `#` solid rock, `r` boulder (solid), `=` one-way plank (solid from above
//! only), `^` cactus spikes. Scenery, which is air for collision: `,` dry grass, `*`
//! pebbles, `&` cow skull. Anything else is air. Markers, which are air too:
//!
//! | char | meaning |
//! | ---- | ------- |
//! | `P`  | player start (feet on the tile below) |
//! | `C`  | checkpoint flag |
//! | `o`  | sat |
//! | `+`  | cold-wallet heal |
//! | `b`  | Bandit Bear |
//! | `s`  | rattlesnake |
//! | `v`  | FUD vulture |
//! | `t`  | tumbleweed |
//! | `X`  | left edge of the boss arena (any row) |
//! | `B`  | the boss |

use crate::TILE;

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
            b'=' => Tile::OneWay,
            b'^' => Tile::Spike,
            _ => Tile::Empty,
        }
    }
}

/// Which backdrop and tileset the renderer uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Canyon,
}

#[derive(Debug)]
pub struct Level {
    pub name: &'static str,
    pub subtitle: &'static str,
    pub theme: Theme,
    pub rows: &'static [&'static str],
}

/// Arena width in tiles: exactly one screen.
pub const ARENA_TILES: i32 = crate::SCREEN_W / TILE;

impl Level {
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

    /// Every marker character with its tile position, in row-major order. Terrain and
    /// scenery are not markers.
    pub fn markers(&self) -> impl Iterator<Item = (u8, i32, i32)> + '_ {
        self.rows.iter().enumerate().flat_map(|(ty, row)| {
            row.bytes()
                .enumerate()
                .filter(|&(_, c)| !matches!(c, b'.' | b'#' | b'r' | b'=' | b'^' | b',' | b'*' | b'&'))
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

    /// Left edge of the boss arena, in pixels.
    pub fn arena_px(&self) -> i32 {
        self.find(b'X').map_or(self.px_width() - crate::SCREEN_W, |(x, _)| x * TILE)
    }
}
