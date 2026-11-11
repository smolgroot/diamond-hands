//! Levels are hand-drawn ASCII maps, one character per 8x8 tile, read in place.
//!
//! Terrain: `#` solid rock, `r` crystal boulder (solid), `=` one-way steel girder
//! (solid from above only), `T` one-way casino table top, `^` an electric fence's
//! emitter: its lightning curtain runs from there up to the rock above (see
//! [`Level::fence_hit`]). Slopes are floors that rise or fall across their tile, with
//! rock under them: `/` rises a whole tile to the right and `\` falls one; `{` rises
//! and `}` falls a tile over two, so they come in pairs (see [`Slope`]). Scenery,
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
    /// A floor across the tile at the height its shape gives.
    Slope(Slope),
}

impl Tile {
    /// The tile a map character stands for. A gentle slope's character is its lower
    /// half (rising) or upper half (falling): [`Level::tile`] works out which half of
    /// its pair a cell is.
    pub fn from_char(c: u8) -> Tile {
        match c {
            b'#' | b'r' => Tile::Solid,
            b'=' | b'T' => Tile::OneWay,
            b'^' => Tile::Spike,
            b'/' => Tile::Slope(Slope::Up),
            b'\\' => Tile::Slope(Slope::Down),
            b'{' => Tile::Slope(Slope::UpLow),
            b'}' => Tile::Slope(Slope::DownHigh),
            _ => Tile::Empty,
        }
    }
}

/// A slope's shape. Bodies stand on it where its floor crosses the column under their
/// centre; it is not a wall or a ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Slope {
    /// `/`: rises a whole tile to the right, 45 degrees.
    Up,
    /// `\`: falls a whole tile to the right.
    Down,
    /// `{` twice: rises a tile over two cells, its lower and upper half.
    UpLow,
    UpHigh,
    /// `}` twice: falls a tile over two cells, its upper and lower half.
    DownHigh,
    DownLow,
}

impl Slope {
    pub const ALL: [Slope; 6] = [Slope::Up, Slope::Down, Slope::UpLow, Slope::UpHigh, Slope::DownHigh, Slope::DownLow];

    /// How high the floor is over the tile's bottom edge, 0 to 8 pixels, at pixel
    /// column `x` (0 to 7) of the tile. Neighbouring cells of a ramp meet without a
    /// step.
    pub const fn height(self, x: i32) -> i32 {
        match self {
            Slope::Up => x + 1,
            Slope::Down => TILE - x,
            Slope::UpLow => (x + 1) / 2,
            Slope::UpHigh => TILE / 2 + (x + 1) / 2,
            Slope::DownHigh => TILE / 2 + (TILE - x) / 2,
            Slope::DownLow => (TILE - x) / 2,
        }
    }

    /// Whether it climbs going right.
    pub const fn rises(self) -> bool {
        matches!(self, Slope::Up | Slope::UpLow | Slope::UpHigh)
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
pub const DECOR: [(u8, i32, i32); 16] = [
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
    (b'Y', 2, 3),  // crypto ATM, the back room's cash machine
    (b'H', 3, 3),  // sewer outflow pipe
    (b'G', 4, 2),  // graffiti
];

/// The size of the picture `c` draws, if it is a picture.
pub fn decor_size(c: u8) -> Option<(i32, i32)> {
    DECOR.iter().find(|d| d.0 == c).map(|&(_, w, h)| (w, h))
}

/// Characters that are terrain or scenery, never markers.
const fn is_scenery(c: u8) -> bool {
    if matches!(
        c,
        b'.' | b'#' | b'r' | b'=' | b'T' | b'^' | b',' | b'*' | b'&' | b'|' | b'%' | b'/' | b'\\' | b'{' | b'}'
    ) {
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

/// A block of tiles, `x0..x1` by `y0..y1`, lit as backdrop stage `stage`.
#[derive(Clone, Copy, Debug)]
pub struct Area {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub stage: u8,
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
    /// Rows where the camera's floors split, for a map stacked in storeys: the camera
    /// stays inside the storey the wassie is in, and pans when it moves to another.
    /// Empty to follow it anywhere.
    pub bands: &'static [i32],
    /// Rows where each deeper stratum of terrain starts, drawn in its own tileset and
    /// palette: the renderer's business. Empty for one stratum.
    pub strata: &'static [i32],
    /// Places with their own light, overriding [`Level::stages`]: the first that holds
    /// a tile wins.
    pub areas: &'static [Area],
    /// The camera top at which the sky and the city sit where they were painted. They
    /// drift up only as the camera goes below it.
    pub backdrop_y: i32,
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
        bands: &[],
        strata: &[],
        areas: &[],
        backdrop_y: 0,
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
        let c = self.char_at(tx, ty);
        match c {
            b'{' | b'}' => {
                // Gentle slopes pair up along their run: every other cell is the
                // second half.
                let mut run = 0;
                while self.char_at(tx - run - 1, ty) == c {
                    run += 1;
                }
                let shape = match (c, run % 2) {
                    (b'{', 0) => Slope::UpLow,
                    (b'{', _) => Slope::UpHigh,
                    (_, 0) => Slope::DownHigh,
                    _ => Slope::DownLow,
                };
                Tile::Slope(shape)
            }
            _ => Tile::from_char(c),
        }
    }

    /// Tile at a pixel position.
    pub fn tile_at_px(&self, x: i32, y: i32) -> Tile {
        self.tile(x.div_euclid(TILE), y.div_euclid(TILE))
    }

    /// Whether something can stand on tile `(tx, ty)`: rock, a plank or a slope.
    pub fn is_floor(&self, tx: i32, ty: i32) -> bool {
        matches!(self.tile(tx, ty), Tile::Solid | Tile::OneWay | Tile::Slope(_))
    }

    /// The floor of the slope under pixel column `x` near feet at `bottom`, as a pixel
    /// row: looks down from the tile above the feet to the one below them, stopping at
    /// flat floor. `None` where there is no slope there.
    pub fn slope_floor(&self, x: i32, bottom: i32) -> Option<i32> {
        let (tx, lx) = (x.div_euclid(TILE), x.rem_euclid(TILE));
        let first = (bottom - TILE).div_euclid(TILE);
        for ty in first..=first + 2 {
            match self.tile(tx, ty) {
                Tile::Slope(s) => return Some((ty + 1) * TILE - s.height(lx)),
                Tile::Solid | Tile::OneWay => return None,
                _ => {}
            }
        }
        None
    }

    /// Whether pixel `(x, y)` is inside rock: a solid tile, or under a slope's floor.
    pub fn solid_px(&self, x: i32, y: i32) -> bool {
        match self.tile_at_px(x, y) {
            Tile::Solid => true,
            Tile::Slope(s) => y.rem_euclid(TILE) >= TILE - s.height(x.rem_euclid(TILE)),
            _ => false,
        }
    }

    /// Whether a walker with its leading foot at pixel column `x` and its feet at
    /// `bottom` has ground ahead: a floor under the foot, or a slope going on down.
    pub fn ground_ahead(&self, x: i32, bottom: i32) -> bool {
        let (tx, ty) = (x.div_euclid(TILE), bottom.div_euclid(TILE));
        self.is_floor(tx, ty) || matches!(self.tile(tx, ty + 1), Tile::Slope(_))
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

    /// The backdrop stage at level pixel `(x, y)`: an [`Area`]'s, or else 0, 1 or 2 by
    /// column.
    pub fn stage_at(&self, x: i32, y: i32) -> u8 {
        let (tx, ty) = (x.div_euclid(TILE), y.div_euclid(TILE));
        if let Some(area) = self.areas.iter().find(|a| (a.x0..a.x1).contains(&tx) && (a.y0..a.y1).contains(&ty)) {
            return area.stage;
        }
        if self.stages.is_empty() {
            return (x * 3 / self.px_width().max(1)).clamp(0, 2) as u8;
        }
        self.stages.iter().filter(|&&col| x >= col * TILE).count().min(2) as u8
    }

    /// Which stratum row `ty` is in: 0 for the top one, counting down
    /// [`Level::strata`].
    pub fn stratum(&self, ty: i32) -> usize {
        self.strata.iter().filter(|&&row| ty >= row).count()
    }

    /// The storey of [`Level::bands`] that pixel row `y` is in, as its first row and
    /// the row after its last.
    pub fn band(&self, y: i32) -> (i32, i32) {
        let ty = y.div_euclid(TILE);
        let top = self.bands.iter().rev().find(|&&row| row <= ty).copied().unwrap_or(0);
        let bottom = self.bands.iter().find(|&&row| row > ty).copied().unwrap_or(self.height());
        (top, bottom)
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
