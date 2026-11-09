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
//! | `J`  | the jetpack, in an explorable level that hides it |
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
const fn is_scenery(c: u8) -> bool {
    if matches!(c, b'.' | b'#' | b'r' | b'=' | b'T' | b'^' | b',' | b'*' | b'&' | b'|' | b'%') {
        return true;
    }
    let mut i = 0;
    while i < DECOR.len() {
        if DECOR[i].0 == c {
            return true;
        }
        i += 1;
    }
    false
}

/// Most markers a map may hold.
pub const MAX_MARKERS: usize = 128;

/// A map's markers in row-major order, found when the game is compiled rather than by
/// reading the map as a level starts: on the GBA the map sits in cartridge ROM, and
/// reading every cell of a tall one takes most of a frame.
#[derive(Debug)]
pub struct Markers {
    list: [(u8, u16, u16); MAX_MARKERS],
    len: usize,
}

impl Markers {
    /// Finds the markers in `rows`. Fails to compile with more than [`MAX_MARKERS`].
    pub const fn scan(rows: &[&str]) -> Markers {
        let mut list = [(0, 0, 0); MAX_MARKERS];
        let mut len = 0;
        let mut ty = 0;
        while ty < rows.len() {
            let row = rows[ty].as_bytes();
            let mut tx = 0;
            while tx < row.len() {
                if !is_scenery(row[tx]) {
                    assert!(len < MAX_MARKERS, "too many markers in the map");
                    list[len] = (row[tx], tx as u16, ty as u16);
                    len += 1;
                }
                tx += 1;
            }
            ty += 1;
        }
        Markers { list, len }
    }

    pub fn iter(&self) -> impl Iterator<Item = (u8, i32, i32)> + '_ {
        self.list[..self.len].iter().map(|&(c, x, y)| (c, x as i32, y as i32))
    }
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
    /// An explorable map, Metroidvania style: several screens tall, with routes up,
    /// down and back that aren't in left-to-right order. A resumed run restores exactly
    /// what was taken rather than clearing everything left of the beacon.
    pub explore: bool,
    /// Rows of open sky from the top. Below them is underground, where the sky and the
    /// city fade to black. 0 for a level that is all sky.
    pub sky_rows: i32,
    /// Waypoints for the route bot, in order: tile column and the row the feet stand
    /// in. Empty for a level the bot simply runs right through.
    pub route: &'static [(i32, i32)],
    pub rows: &'static [&'static str],
    /// The map's markers, listed when the game is compiled ([`Markers::scan`]); `None`
    /// to read them off the map, as test levels do.
    pub markers: Option<&'static Markers>,
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
        explore: false,
        sky_rows: 0,
        route: &[],
        rows: &[],
        markers: None,
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

    /// Whether any fence emitter stands between pixel columns `x0` and `x1`.
    pub fn fence_between(&self, x0: i32, x1: i32) -> bool {
        (x0.div_euclid(TILE)..=(x1 - 1).div_euclid(TILE))
            .any(|tx| (0..self.height()).any(|ty| self.tile(tx, ty) == Tile::Spike))
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
        let listed = self.markers.map(Markers::iter);
        let scanned = self.markers.is_none().then(|| {
            self.rows.iter().enumerate().flat_map(|(ty, row)| {
                row.bytes()
                    .enumerate()
                    .filter(|&(_, c)| !is_scenery(c))
                    .map(move |(tx, c)| (c, tx as i32, ty as i32))
            })
        });
        listed.into_iter().flatten().chain(scanned.into_iter().flatten())
    }

    /// The first `marker` in row-major order: from the listed markers, or a plain byte
    /// search of the map.
    pub fn find(&self, marker: u8) -> Option<(i32, i32)> {
        if let Some(markers) = self.markers {
            return markers.iter().find(|m| m.0 == marker).map(|(_, x, y)| (x, y));
        }
        self.rows.iter().enumerate().find_map(|(ty, row)| {
            row.bytes()
                .position(|c| c == marker)
                .map(|tx| (tx as i32, ty as i32))
        })
    }

    /// How far underground the screen with its top at pixel `camera_y` is: 0 under
    /// open sky, 16 when all of it is below the sky rows.
    pub fn underground(&self, camera_y: i32) -> u8 {
        if self.sky_rows <= 0 {
            return 0;
        }
        let below = camera_y + crate::SCREEN_H - self.sky_rows * TILE;
        (below * 16 / crate::SCREEN_H).clamp(0, 16) as u8
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
