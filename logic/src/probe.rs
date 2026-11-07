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
    /// Levels the active file has cleared, one bit each.
    pub cleared: u32,
    /// The pause menu: tab, item << 8, and 1 << 16 while an item is inspected.
    pub menu: u32,
    /// Gold coins picked up in this level.
    pub coins: u32,
    /// The file's banked diamonds, kept coins and shield parts.
    pub wallet: u32,
    pub purse: u32,
    pub parts: u32,
    /// The elevator's [`crate::world::LiftState`], or `u32::MAX` for none.
    pub lift: u32,
    /// The penguin's [`crate::game::Deal`] while the exchange is open, else `u32::MAX`.
    pub deal: u32,
}

impl Probe {
    pub const MAGIC: u32 = u32::from_le_bytes(*b"DHPR");
    pub const VERSION: u32 = 6;
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
        cleared: 0,
        menu: 0,
        coins: 0,
        wallet: 0,
        purse: 0,
        parts: 0,
        lift: 0,
        deal: 0,
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
            cleared: game.cleared as u32,
            menu: game.menu.tab as u32 | (game.menu.item as u32) << 8 | (game.menu.inspecting as u32) << 16,
            coins: world.coins,
            wallet: game.wallet,
            purse: game.purse,
            parts: (game.parts | world.parts) as u32,
            lift: world.lift.map_or(u32::MAX, |l| l.state as u32),
            deal: if game.scene == crate::Scene::Exchange { game.exchange.deal as u32 } else { u32::MAX },
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
    fn layout_is_twenty_eight_words() {
        assert_eq!(Probe::WORDS, 28);
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
