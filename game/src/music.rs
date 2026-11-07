//! Background music: one tracker module per cue, chosen from the game state each frame.
//!
//! The songs are written by `game/music/src/soundtrack.py`; agb-tracker converts the XM
//! files at compile time and plays them through agb's mixer on DirectSound, which leaves
//! the DMG channels to the blaster effects in `sound`.

use agb::sound::mixer::Mixer;
use agb_tracker::{Track, Tracker, include_xm};
use diamond_hands_logic::{Game, Scene, game::Talk};

static TITLE: Track = include_xm!("music/title.xm");
static BADLANDS: Track = include_xm!("music/badlands.xm");
static BOSS: Track = include_xm!("music/boss.xm");
static CLEAR: Track = include_xm!("music/clear.xm");
static DOWN: Track = include_xm!("music/down.xm");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cue {
    Title,
    Level,
    Boss,
    /// From the boss going down through the level-clear screen.
    Clear,
    /// Knocked out; the level music starts over on respawn.
    Down,
}

impl Cue {
    fn of(game: &Game) -> Self {
        let world = &game.world;
        match game.scene {
            Scene::Splash
            | Scene::Title
            | Scene::Options
            | Scene::Map
            | Scene::Files
            | Scene::FileChoice => Cue::Title,
            Scene::LevelClear | Scene::Exchange => Cue::Clear,
            Scene::Intro | Scene::Arrival | Scene::Play | Scene::Paused | Scene::Dialog => {
                if world.player.is_dead() && world.player.hp <= 0 {
                    Cue::Down
                } else if world.boss.is_beaten() {
                    Cue::Clear
                } else if (game.scene == Scene::Dialog && game.talk == Talk::Boss) || world.boss.is_active() {
                    Cue::Boss
                } else {
                    Cue::Level
                }
            }
        }
    }

    fn track(self) -> &'static Track {
        match self {
            Cue::Title => &TITLE,
            // Both levels share the Badlands theme for now; a Trenches song would go by `world.level_index`.
            Cue::Level => &BADLANDS,
            Cue::Boss => &BOSS,
            Cue::Clear => &CLEAR,
            Cue::Down => &DOWN,
        }
    }
}

pub struct Music {
    cue: Cue,
    tracker: Tracker,
}

impl Music {
    pub fn new() -> Self {
        Self {
            cue: Cue::Title,
            tracker: Tracker::new(Cue::Title.track()),
        }
    }

    /// Switches cue if the game state calls for another, then advances the song one
    /// frame. Call once per frame, before `mixer.frame()`.
    pub fn play(&mut self, game: &Game, mixer: &mut Mixer) {
        let cue = Cue::of(game);
        if cue != self.cue {
            self.tracker.stop(mixer);
            self.tracker = Tracker::new(cue.track());
            self.cue = cue;
        }
        self.tracker.step(mixer);
    }
}
