//! Blocks the wassie can shoot apart: sand on the beach, crates on the dock, soil in
//! the zoo, adobe bricks in Satoshi Canyon's town and gold ore in its mines.
//!
//! A block is a 16x16 solid that moves: the map only holds its marker (`n`, `u`, `y`,
//! `1` or `2`, air like any marker) at its bottom-left tile, and [`Map`] lays the blocks
//! still standing over the map wherever they are now. Each one is also an actor
//! ([`crate::enemy::ActorKind::Block`]), drawn as a sprite since the background can't
//! change under it, and kept broken through a respawn or a saved game by the run's
//! taken mask.
//!
//! Blocks fall: one with nothing under it, rock or another block, drops under gravity
//! until it lands ([`Blocks::fall`]). Blocks only ever move straight down, so the
//! columns they cover are known when the level starts, and [`Map`] only looks at the
//! blocks in those columns.

use crate::{
    FX, TILE,
    level::{Level, Terrain, Tile},
    physics::Rect,
    player::{GRAVITY_FX, MAX_FALL_FX},
};

/// Most breakable blocks in one level.
pub const MAX_BLOCKS: usize = 32;
/// A block's side in pixels.
pub const BLOCK: i32 = 16;
/// Map columns [`Blocks::columns`] can mark; a block further right is checked anyway.
const COLUMN_BITS: usize = 512;

/// What a block is made of: only its look, its toughness and the dust it makes differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BlockLook {
    /// A block of packed sand on the beach.
    Sand,
    /// A wooden crate on the dock.
    Crate,
    /// A clod of soil in the zoo's paddocks.
    Soil,
    /// An adobe brick in Satoshi Canyon's town: the jail's wall.
    Brick,
    /// A block of gold ore in the Satoshi mines.
    Ore,
}

impl BlockLook {
    /// The look whose marker is `c`.
    pub fn from_marker(c: u8) -> Option<BlockLook> {
        Some(match c {
            b'n' => BlockLook::Sand,
            b'u' => BlockLook::Crate,
            b'y' => BlockLook::Soil,
            b'1' => BlockLook::Brick,
            b'2' => BlockLook::Ore,
            _ => return None,
        })
    }

    /// Shots it takes to break: one laser bolt does 1, a charged beam more.
    pub fn hp(self) -> i32 {
        match self {
            BlockLook::Sand | BlockLook::Brick => 2,
            BlockLook::Soil | BlockLook::Ore => 3,
            BlockLook::Crate => 4,
        }
    }
}

/// The level's breakable blocks: where each is, how fast it falls, which actor slot is
/// it, and which are broken.
#[derive(Clone, Copy, Debug)]
pub struct Blocks {
    /// Left tile column of each block.
    cols: [i16; MAX_BLOCKS],
    /// Top edge of each block, in 1/256 pixels.
    y_fx: [i32; MAX_BLOCKS],
    /// Falling speed, in 1/256 pixels a frame.
    vy_fx: [i32; MAX_BLOCKS],
    /// The actor slot of each block.
    slots: [u8; MAX_BLOCKS],
    len: usize,
    /// One bit per block, in order.
    broken: u32,
    /// One bit per map column some block covers.
    columns: [u32; COLUMN_BITS / 32],
}

impl Blocks {
    pub const EMPTY: Blocks = Blocks {
        cols: [0; MAX_BLOCKS],
        y_fx: [0; MAX_BLOCKS],
        vy_fx: [0; MAX_BLOCKS],
        slots: [0; MAX_BLOCKS],
        len: 0,
        broken: 0,
        columns: [0; COLUMN_BITS / 32],
    };

    /// Adds the block whose marker is at tile `(tx, ty)`, its actor in `slot`. Returns
    /// false when the list is full.
    pub fn add(&mut self, tx: i32, ty: i32, slot: usize) -> bool {
        if self.len == MAX_BLOCKS {
            return false;
        }
        self.cols[self.len] = tx as i16;
        self.y_fx[self.len] = ((ty + 1) * TILE - BLOCK) * FX;
        self.slots[self.len] = slot as u8;
        self.len += 1;
        for col in tx..tx + 2 {
            if (0..COLUMN_BITS as i32).contains(&col) {
                self.columns[col as usize / 32] |= 1 << (col % 32);
            }
        }
        true
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The actor slots of the blocks, in order.
    pub fn slots(&self) -> &[u8] {
        &self.slots[..self.len]
    }

    /// Marks block `i` broken or standing.
    pub fn set_broken(&mut self, i: usize, broken: bool) {
        if broken {
            self.broken |= 1 << i;
        } else {
            self.broken &= !(1 << i);
        }
    }

    pub fn is_broken(&self, i: usize) -> bool {
        self.broken & (1 << i) != 0
    }

    /// Block `i` where it is now, in level pixels.
    pub fn rect(&self, i: usize) -> Rect {
        Rect::new(self.cols[i] as i32 * TILE, self.y_fx[i] >> 8, BLOCK, BLOCK)
    }

    /// Whether block `i` is on its way down.
    pub fn falling(&self, i: usize) -> bool {
        self.vy_fx[i] > 0
    }

    /// The standing block covering pixel `(x, y)`, if one does.
    pub fn at_px(&self, x: i32, y: i32) -> Option<usize> {
        (0..self.len).find(|&i| {
            let r = self.rect(i);
            !self.is_broken(i) && (r.x..r.right()).contains(&x) && (r.y..r.bottom()).contains(&y)
        })
    }

    /// Whether any standing block overlaps tile `(tx, ty)`.
    fn covers(&self, tx: i32, ty: i32) -> bool {
        if (0..COLUMN_BITS as i32).contains(&tx) && self.columns[tx as usize / 32] & (1 << (tx % 32)) == 0 {
            return false;
        }
        (0..self.len).any(|i| {
            let r = self.rect(i);
            !self.is_broken(i)
                && (r.x.div_euclid(TILE)..=(r.right() - 1).div_euclid(TILE)).contains(&tx)
                && (r.y.div_euclid(TILE)..=(r.bottom() - 1).div_euclid(TILE)).contains(&ty)
        })
    }

    /// Whether block `i` rests on something: a floor of the map right under it, or the
    /// top of another standing block.
    fn supported(&self, level: &Level, i: usize) -> bool {
        let r = self.rect(i);
        let bottom = r.bottom();
        let col = self.cols[i] as i32;
        if bottom.rem_euclid(TILE) == 0 {
            let row = bottom / TILE;
            if (col..col + 2).any(|tx| level.is_floor(tx, row)) {
                return true;
            }
        }
        (0..self.len).any(|j| {
            j != i && !self.is_broken(j) && (self.cols[j] as i32 - col).abs() < 2 && self.rect(j).y == bottom
        })
    }

    /// One frame of gravity: every standing block with nothing under it falls, a pixel
    /// at a time, until it lands. Lower blocks go first, so a stack comes down
    /// together. A block that falls out of the map breaks. Calls `moved` with each
    /// block that moved this frame.
    pub fn fall(&mut self, level: &Level, mut moved: impl FnMut(usize)) {
        // Bottom first: insertion sort of the few blocks by their top, lowest first.
        let mut order = [0u8; MAX_BLOCKS];
        for i in 0..self.len {
            let mut k = i;
            while k > 0 && self.y_fx[order[k - 1] as usize] < self.y_fx[i] {
                order[k] = order[k - 1];
                k -= 1;
            }
            order[k] = i as u8;
        }
        for &i in &order[..self.len] {
            let i = i as usize;
            if self.is_broken(i) {
                continue;
            }
            if self.supported(level, i) {
                self.vy_fx[i] = 0;
                continue;
            }
            self.vy_fx[i] = (self.vy_fx[i] + GRAVITY_FX).min(MAX_FALL_FX);
            let target = self.y_fx[i] + self.vy_fx[i];
            // Whole pixels, so a block never sinks past a floor or another block's top.
            while self.y_fx[i] < target {
                let next = ((self.y_fx[i] >> 8) + 1) * FX;
                self.y_fx[i] = next.min(target);
                if self.y_fx[i] & 0xFF == 0 && self.supported(level, i) {
                    self.vy_fx[i] = 0;
                    break;
                }
            }
            if self.y_fx[i] >> 8 > level.px_height() {
                self.set_broken(i, true);
            }
            moved(i);
        }
    }

    /// Lets every block fall until all have landed, as when a saved run comes back
    /// with blocks broken under others.
    pub fn settle(&mut self, level: &Level) {
        for _ in 0..level.px_height() {
            let mut any = false;
            self.fall(level, |_| any = true);
            if !any {
                break;
            }
        }
    }
}

/// The level as the wassie has left it: the blocks still standing are rock where they
/// are now.
#[derive(Clone, Copy)]
pub struct Map<'a> {
    pub level: &'a Level,
    pub blocks: &'a Blocks,
}

impl Terrain for Map<'_> {
    fn tile(&self, tx: i32, ty: i32) -> Tile {
        let tile = self.level.tile(tx, ty);
        if tile != Tile::Solid && self.blocks.covers(tx, ty) {
            return Tile::Solid;
        }
        tile
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static ROWS: [&str; 6] = ["......", "......", "......", "......", "......", "######"];
    static LEVEL: Level = Level { rows: &ROWS, ..Level::BASE };

    /// Two blocks stacked at columns 2-3 on the floor of row 5: markers at row 4 and 2.
    fn stack() -> Blocks {
        let mut blocks = Blocks::EMPTY;
        assert!(blocks.add(2, 4, 0));
        assert!(blocks.add(2, 2, 1));
        blocks
    }

    #[test]
    fn a_block_is_rock_until_it_breaks() {
        let mut blocks = stack();
        let map = Map { level: &LEVEL, blocks: &blocks };
        assert!((1..5).all(|ty| (2..4).all(|tx| map.tile(tx, ty) == Tile::Solid)));
        assert_eq!(map.tile(1, 4), Tile::Empty);
        assert_eq!(blocks.at_px(20, 36), Some(0));
        blocks.set_broken(0, true);
        let map = Map { level: &LEVEL, blocks: &blocks };
        assert_eq!(map.tile(2, 4), Tile::Empty, "broken, it is air");
        assert_eq!(map.tile(2, 5), Tile::Solid, "the floor under it stays");
    }

    #[test]
    fn a_block_with_nothing_under_it_falls_and_lands() {
        let mut blocks = stack();
        blocks.fall(&LEVEL, |_| panic!("a stack on the floor stays put"));
        blocks.set_broken(0, true);
        let mut frames = 0;
        while frames < 60 {
            let mut moved = false;
            blocks.fall(&LEVEL, |_| moved = true);
            if !moved {
                break;
            }
            frames += 1;
        }
        assert!(frames > 1 && frames < 60, "it drops over a few frames: {frames}");
        assert_eq!(blocks.rect(1).bottom(), 5 * TILE, "on the floor");
        assert!(!blocks.falling(1));
    }

    #[test]
    fn a_block_half_on_another_stays_up() {
        let mut blocks = Blocks::EMPTY;
        blocks.add(1, 4, 0);
        blocks.add(2, 2, 1);
        blocks.fall(&LEVEL, |_| panic!("it rests on the block's right half"));
    }
}
