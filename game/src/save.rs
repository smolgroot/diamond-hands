//! Best score, options and the save files in battery-backed SRAM.
//!
//! The data is postcard-encoded. Version 5 added the levels each file has cleared. It
//! starts with [`TAG`], which no version 4 save can begin with, so a cartridge from
//! before is read as version 4 and moved over rather than wiped.

use agb::save::{SaveManager, SaveSlotManager};
use diamond_hands_logic::{
    Difficulty, Game, Progress,
    game::FILES,
    levels::{self, Cleared},
};
use serde::{Deserialize, Serialize};

/// Changing this wipes existing saves, so bump the suffix only for incompatible formats.
/// Formats that can be read and moved over keep it and change [`TAG`] instead.
const MAGIC: [u8; 32] = *b"diamond-hands-save-v4___________";
/// Leads every version 5 save.
const TAG: [u8; 4] = *b"DHv5";

#[derive(Clone, Serialize, Deserialize)]
struct SaveData {
    tag: [u8; 4],
    best: u32,
    /// [`Difficulty`] as `u8`.
    difficulty: u8,
    files: [Option<FileData>; FILES],
}

/// [`Progress`], as stored.
#[derive(Clone, Copy, Serialize, Deserialize)]
struct FileData {
    level: u8,
    checkpoint: Option<(i32, i32)>,
    beacons: u8,
    banked: u32,
    diamonds: u32,
    kills: u32,
    frames: u32,
    complete: bool,
    cleared: Cleared,
}

/// The version 4 layout, from before levels unlocked one by one.
#[derive(Clone, Serialize, Deserialize)]
struct SaveDataV4 {
    best: u32,
    difficulty: u8,
    files: [Option<FileDataV4>; FILES],
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct FileDataV4 {
    level: u8,
    checkpoint: Option<(i32, i32)>,
    beacons: u8,
    banked: u32,
    diamonds: u32,
    kills: u32,
    frames: u32,
    complete: bool,
}

impl From<FileDataV4> for FileData {
    /// A version 4 run had cleared every level before the one it was on. Version 4
    /// had one level, so a complete run had cleared just that, and the levels added
    /// since are still ahead of it.
    fn from(f: FileDataV4) -> Self {
        const V4_LEVELS: u8 = 1;
        let done = if f.complete { V4_LEVELS } else { f.level.min(V4_LEVELS) };
        let cleared = ((1u16 << done) - 1) as Cleared;
        Self {
            level: if f.complete { levels::frontier(cleared) } else { f.level },
            checkpoint: f.checkpoint,
            beacons: f.beacons,
            banked: f.banked,
            diamonds: f.diamonds,
            kills: f.kills,
            frames: f.frames,
            complete: cleared == levels::ALL_CLEARED,
            cleared,
        }
    }
}

impl From<SaveDataV4> for SaveData {
    fn from(v4: SaveDataV4) -> Self {
        Self {
            tag: TAG,
            best: v4.best,
            difficulty: v4.difficulty,
            files: v4.files.map(|f| f.map(FileData::from)),
        }
    }
}

impl From<Progress> for FileData {
    fn from(p: Progress) -> Self {
        Self {
            level: p.level,
            checkpoint: p.checkpoint,
            beacons: p.beacons,
            banked: p.banked,
            diamonds: p.diamonds,
            kills: p.kills,
            frames: p.frames,
            complete: p.complete,
            cleared: p.cleared,
        }
    }
}

impl From<FileData> for Progress {
    fn from(f: FileData) -> Self {
        Self {
            level: f.level,
            checkpoint: f.checkpoint,
            beacons: f.beacons,
            banked: f.banked,
            diamonds: f.diamonds,
            kills: f.kills,
            frames: f.frames,
            complete: f.complete,
            cleared: f.cleared,
        }
    }
}

/// What a cartridge holds: best score, difficulty and the save files.
pub type Loaded = (u32, Difficulty, [Option<Progress>; FILES]);

pub struct Store {
    /// `None` when the cartridge has no working SRAM; the game still plays.
    slots: Option<SaveSlotManager>,
}

impl Store {
    pub fn new(save: &mut SaveManager) -> Self {
        Self {
            slots: save.init_sram(1, MAGIC).ok(),
        }
    }

    /// The saved state, or defaults on a fresh cartridge.
    pub fn load(&mut self) -> Loaded {
        self.slots
            .as_mut()
            .and_then(|slots| {
                slots
                    .read::<SaveData>(0)
                    .ok()
                    .filter(|data| data.tag == TAG)
                    .or_else(|| slots.read::<SaveDataV4>(0).ok().map(SaveData::from))
            })
            .map_or((0, Difficulty::default(), [None; FILES]), |data| {
                (
                    data.best,
                    Difficulty::from_u8(data.difficulty).unwrap_or_default(),
                    data.files.map(|f| f.map(Progress::from)),
                )
            })
    }

    pub fn save(&mut self, game: &Game) {
        if let Some(slots) = &mut self.slots {
            // A failed write only loses progress; never crash the game over it.
            let data = SaveData {
                tag: TAG,
                best: game.best,
                difficulty: game.difficulty as u8,
                files: game.files.map(|f| f.map(FileData::from)),
            };
            let _ = slots.write(0, &data, &());
        }
    }
}
