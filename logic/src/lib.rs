//! Game rules for Diamond Hands, independent of the GBA hardware.
//!
//! Everything here is deterministic integer math so the same inputs produce the same
//! game on the host (unit tests), on hardware, and in emulators. Positions are in screen
//! pixels unless a name ends in `_fx`, which means 24.8 fixed point.
#![no_std]

pub mod arrival;
pub mod boss;
pub mod bot;
pub mod enemy;
pub mod fx;
pub mod game;
pub mod level;
pub mod levels;
pub mod physics;
pub mod player;
pub mod probe;
pub mod rng;
pub mod shot;
pub mod talk;
pub mod world;

pub use game::{Game, Progress, Scene, keys};
pub use level::{Level, Tile};
pub use player::PlayerState;
pub use probe::Probe;
pub use world::{Difficulty, Events, Input, World};

/// One pixel in 24.8 fixed point.
pub const FX: i32 = 256;
/// Tile edge in pixels.
pub const TILE: i32 = 8;

pub const SCREEN_W: i32 = 240;
pub const SCREEN_H: i32 = 160;
