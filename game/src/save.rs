//! Best score, options and the save files in battery-backed SRAM.

use agb::save::{SaveManager, SaveSlotManager};
use diamond_hands_logic::{Difficulty, Game, Progress, game::FILES};
use serde::{Deserialize, Serialize};

/// Changing this wipes existing saves, so bump the suffix only for incompatible formats.
const MAGIC: [u8; 32] = *b"diamond-hands-save-v4___________";

#[derive(Clone, Serialize, Deserialize)]
struct SaveData {
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
            .and_then(|slots| slots.read::<SaveData>(0).ok())
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
                best: game.best,
                difficulty: game.difficulty as u8,
                files: game.files.map(|f| f.map(FileData::from)),
            };
            let _ = slots.write(0, &data, &());
        }
    }
}
