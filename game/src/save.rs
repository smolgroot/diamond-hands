//! Best score and options in battery-backed SRAM.

use agb::save::{SaveManager, SaveSlotManager};
use diamond_hands_logic::Difficulty;
use serde::{Deserialize, Serialize};

/// Changing this wipes existing saves, so bump the suffix only for incompatible formats.
const MAGIC: [u8; 32] = *b"diamond-hands-save-v3___________";

#[derive(Clone, Serialize, Deserialize)]
struct SaveData {
    best: u32,
    /// [`Difficulty`] as `u8`.
    difficulty: u8,
}

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

    /// The saved best score and difficulty, or defaults on a fresh cartridge.
    pub fn load(&mut self) -> (u32, Difficulty) {
        self.slots
            .as_mut()
            .and_then(|slots| slots.read::<SaveData>(0).ok())
            .map_or((0, Difficulty::default()), |data| {
                (data.best, Difficulty::from_u8(data.difficulty).unwrap_or_default())
            })
    }

    pub fn save(&mut self, best: u32, difficulty: Difficulty) {
        if let Some(slots) = &mut self.slots {
            // A failed write only loses progress; never crash the game over it.
            let data = SaveData {
                best,
                difficulty: difficulty as u8,
            };
            let _ = slots.write(0, &data, &());
        }
    }
}
