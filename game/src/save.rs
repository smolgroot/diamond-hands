//! Best score, options and the save files in battery-backed SRAM.
//!
//! The data is postcard-encoded. Version 5 added the levels each file has cleared;
//! version 6 the wallet of banked diamonds, the purse of kept gold coins, the coins
//! picked up in the level so far, and CL207's shield parts; version 7 what a run had
//! taken at its beacon, for the explorable Badlands. Version 8 is laid out as version
//! 7: it marks saves made since the Trenches were rebuilt as an explorable map too.
//! Each starts with a tag, and an older cartridge is read in its own layout and moved
//! over rather than wiped.
//!
//! Version 5 kept no diamond balance (diamonds only ever went into the score), so a
//! moved-over file starts with an empty wallet.

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
/// Leads every version 8 save.
const TAG: [u8; 4] = *b"DHv8";
const TAG_V7: [u8; 4] = *b"DHv7";
const TAG_V6: [u8; 4] = *b"DHv6";
const TAG_V5: [u8; 4] = *b"DHv5";

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
    coins: u32,
    wallet: u32,
    purse: u32,
    parts: u16,
    taken: u128,
}

impl FileData {
    /// A run saved part-way through level `level` before it was rebuilt as an
    /// explorable map starts the level again: its beacon may now be inside rock.
    fn restart_rebuilt(self, level: u8) -> Self {
        if self.level != level || self.checkpoint.is_none() {
            return self;
        }
        Self { checkpoint: None, beacons: 0, diamonds: 0, kills: 0, frames: 0, coins: 0, taken: 0, ..self }
    }

    /// From before version 7, both the Badlands and the Trenches were rebuilt since.
    fn restart_rebuilt_levels(self) -> Self {
        self.restart_rebuilt(0).restart_rebuilt(1)
    }
}

impl SaveData {
    /// A version 7 save: only the Trenches were rebuilt since.
    fn from_v7(v7: SaveData) -> Self {
        Self { tag: TAG, files: v7.files.map(|f| f.map(|f| f.restart_rebuilt(1))), ..v7 }
    }
}

/// The version 6 layout, from before the explorable Badlands.
#[derive(Clone, Serialize, Deserialize)]
struct SaveDataV6 {
    tag: [u8; 4],
    best: u32,
    difficulty: u8,
    files: [Option<FileDataV6>; FILES],
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct FileDataV6 {
    level: u8,
    checkpoint: Option<(i32, i32)>,
    beacons: u8,
    banked: u32,
    diamonds: u32,
    kills: u32,
    frames: u32,
    complete: bool,
    cleared: Cleared,
    coins: u32,
    wallet: u32,
    purse: u32,
    parts: u16,
}

impl From<FileDataV6> for FileData {
    /// A version 6 run keeps where it was, except part-way through the Badlands or the
    /// Trenches: they were rebuilt as explorable maps since, so such a run starts the
    /// level again rather than at a beacon that may now be inside rock.
    fn from(f: FileDataV6) -> Self {
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
            coins: f.coins,
            wallet: f.wallet,
            purse: f.purse,
            parts: f.parts,
            taken: 0,
        }
        .restart_rebuilt_levels()
    }
}

impl From<SaveDataV6> for SaveData {
    fn from(v6: SaveDataV6) -> Self {
        Self {
            tag: TAG,
            best: v6.best,
            difficulty: v6.difficulty,
            files: v6.files.map(|f| f.map(FileData::from)),
        }
    }
}

/// The version 5 layout, from before the casino's coins.
#[derive(Clone, Serialize, Deserialize)]
struct SaveDataV5 {
    tag: [u8; 4],
    best: u32,
    difficulty: u8,
    files: [Option<FileDataV5>; FILES],
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct FileDataV5 {
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

impl From<FileDataV5> for FileData {
    /// A version 5 run keeps where it was. The Trenches were rebuilt since, so a run
    /// part-way through them starts the level again rather than at a beacon that may
    /// now be somewhere else.
    fn from(f: FileDataV5) -> Self {
        let rebuilt = f.level == 1;
        Self {
            level: f.level,
            checkpoint: if rebuilt { None } else { f.checkpoint },
            beacons: if rebuilt { 0 } else { f.beacons },
            banked: f.banked,
            diamonds: if rebuilt { 0 } else { f.diamonds },
            kills: if rebuilt { 0 } else { f.kills },
            frames: if rebuilt { 0 } else { f.frames },
            complete: f.complete,
            cleared: f.cleared,
            coins: 0,
            wallet: 0,
            purse: 0,
            parts: 0,
            taken: 0,
        }
        .restart_rebuilt_levels()
    }
}

impl From<SaveDataV5> for SaveData {
    fn from(v5: SaveDataV5) -> Self {
        Self {
            tag: TAG,
            best: v5.best,
            difficulty: v5.difficulty,
            files: v5.files.map(|f| f.map(FileData::from)),
        }
    }
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
            coins: 0,
            wallet: 0,
            purse: 0,
            parts: 0,
            taken: 0,
        }
        .restart_rebuilt_levels()
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
            coins: p.coins,
            wallet: p.wallet,
            purse: p.purse,
            parts: p.parts,
            taken: p.taken,
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
            coins: f.coins,
            wallet: f.wallet,
            purse: f.purse,
            parts: f.parts,
            taken: f.taken,
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
                    .or_else(|| {
                        slots.read::<SaveData>(0).ok().filter(|data| data.tag == TAG_V7).map(SaveData::from_v7)
                    })
                    .or_else(|| {
                        slots
                            .read::<SaveDataV6>(0)
                            .ok()
                            .filter(|data| data.tag == TAG_V6)
                            .map(SaveData::from)
                    })
                    .or_else(|| {
                        slots
                            .read::<SaveDataV5>(0)
                            .ok()
                            .filter(|data| data.tag == TAG_V5)
                            .map(SaveData::from)
                    })
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
