//! The soundtrack: the studio's tracks, picked from the game's scene.

use agb::sound::mixer::Mixer;
use agb_tracker::Tracker;
use diamond_hands_logic::{Game, Scene};

use crate::audio::{TRACKS, track};

/// The track the game calls for now.
fn cue(game: &Game) -> usize {
    match game.scene {
        Scene::LevelClear | Scene::Map => track::CLEAR,
        _ if game.world.player.is_dead() && game.world.player.hp <= 0 => track::DOWN,
        // Sol City has no song of its own yet: it borrows the Badlands' theme, as the
        // Trenches do.
        _ => track::BADLANDS,
    }
}

pub struct Music {
    tracker: Tracker,
    cue: usize,
}

impl Music {
    pub fn new() -> Self {
        Self { tracker: Tracker::new(&TRACKS[track::BADLANDS]), cue: track::BADLANDS }
    }

    /// Switches track if the game calls for another, then advances the song one frame.
    /// Call once per frame, before `mixer.frame()`.
    pub fn play(&mut self, game: &Game, mixer: &mut Mixer) {
        let cue = cue(game);
        if cue != self.cue {
            self.tracker.stop(mixer);
            self.tracker = Tracker::new(&TRACKS[cue]);
            self.cue = cue;
        }
        self.tracker.step(mixer);
    }
}
