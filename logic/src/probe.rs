//! A snapshot of game state that the ROM mirrors into RAM every frame, so emulator
//! tests can observe the game without debug hooks. The harness finds it by scanning
//! RAM for [`Probe::MAGIC`] followed by [`Probe::VERSION`].

use crate::game::Game;

/// Every field is a 32-bit word so the layout is trivial to read from emulator memory.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Probe {
    pub magic: u32,
    pub version: u32,
    pub frame: u32,
    /// [`crate::Scene`] as `u32`.
    pub scene: u32,
    pub level: u32,
    /// Player hitbox top-left, level pixels.
    pub player_x: i32,
    pub player_y: i32,
    /// [`crate::PlayerState`] as `u32`.
    pub player_state: u32,
    pub hp: i32,
    pub camera_x: i32,
    pub checkpoints: u32,
    pub boss_hp: i32,
    /// [`crate::boss::BossState`] as `u32`.
    pub boss_state: u32,
    pub score: u32,
    pub diamonds: u32,
    pub kills: u32,
    pub best: u32,
    /// [`crate::Difficulty`] as `u32`.
    pub difficulty: u32,
    pub cursor: u32,
    /// Blaster shots in flight.
    pub shots: u32,
}

impl Probe {
    pub const MAGIC: u32 = u32::from_le_bytes(*b"DHPR");
    pub const VERSION: u32 = 4;
    pub const WORDS: usize = size_of::<Probe>() / 4;

    /// The value the ROM starts with, before the first frame runs.
    pub const BOOT: Probe = Probe {
        magic: Self::MAGIC,
        version: Self::VERSION,
        frame: 0,
        scene: 0,
        level: 0,
        player_x: 0,
        player_y: 0,
        player_state: 0,
        hp: 0,
        camera_x: 0,
        checkpoints: 0,
        boss_hp: 0,
        boss_state: 0,
        score: 0,
        diamonds: 0,
        kills: 0,
        best: 0,
        difficulty: 0,
        cursor: 0,
        shots: 0,
    };

    pub fn from_game(game: &Game) -> Self {
        let world = &game.world;
        Self {
            frame: game.frame,
            scene: game.scene as u32,
            level: world.level_index as u32,
            player_x: world.player.body.x(),
            player_y: world.player.body.y(),
            player_state: world.player.state as u32,
            hp: world.player.hp,
            camera_x: world.camera_x,
            checkpoints: world.checkpoints_lit as u32,
            boss_hp: world.boss.hp,
            boss_state: world.boss.state as u32,
            score: game.score(),
            diamonds: world.diamonds,
            kills: world.kills,
            best: game.best,
            difficulty: game.difficulty as u32,
            cursor: game.cursor as u32,
            shots: world.player_shots() as u32,
            ..Self::BOOT
        }
    }

    pub fn from_words(words: [u32; Self::WORDS]) -> Option<Self> {
        if words[0] != Self::MAGIC || words[1] != Self::VERSION {
            return None;
        }
        // SAFETY: Probe is repr(C) and made only of 32-bit fields, so any bit pattern
        // is a valid value.
        Some(unsafe { core::mem::transmute::<[u32; Self::WORDS], Probe>(words) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_twenty_words() {
        assert_eq!(Probe::WORDS, 20);
    }

    #[test]
    fn round_trips_through_words() {
        let probe = Probe::from_game(&Game::new(1234));
        // SAFETY: as in `from_words`.
        let words: [u32; Probe::WORDS] = unsafe { core::mem::transmute(probe) };
        assert_eq!(Probe::from_words(words), Some(probe));
    }

    #[test]
    fn rejects_wrong_magic() {
        let mut words = [0u32; Probe::WORDS];
        words[1] = Probe::VERSION;
        assert_eq!(Probe::from_words(words), None);
    }
}
