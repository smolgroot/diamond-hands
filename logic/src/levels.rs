//! The levels in play order, and the places on the world map.

pub mod badlands;

use crate::level::Level;

pub static ALL: [&Level; 1] = [&badlands::BADLANDS];

/// Level `index`, or the last one when out of range.
pub fn get(index: u8) -> &'static Level {
    ALL[(index as usize).min(ALL.len() - 1)]
}

/// A stop on the world map. Every place with a level is open from the start.
#[derive(Debug)]
pub struct Place {
    pub name: &'static str,
    pub blurb: &'static str,
    /// Where its marker sits on the 240x160 map screen.
    pub x: i32,
    pub y: i32,
    /// Index into [`ALL`], or `None` while the level is still being built.
    pub level: Option<u8>,
}

impl Place {
    pub fn open(&self) -> bool {
        self.level.is_some()
    }
}

/// West to east, in the order the map cursor walks them.
pub static MAP: [Place; 3] = [
    Place {
        name: "THE BADLANDS",
        blurb: "MOSSY CLIFFS. URSA-9000 WAITS.",
        x: 44,
        y: 104,
        level: Some(0),
    },
    Place {
        name: "SOLANA TRENCHES",
        blurb: "A NEON CASINO STRIP. COMING SOON.",
        x: 120,
        y: 64,
        level: None,
    },
    Place {
        name: "ETHEREUM CITY",
        blurb: "GAS DRONES AND MEV BOTS. COMING SOON.",
        x: 196,
        y: 92,
        level: None,
    },
];

/// The map place for level `index`.
pub fn place_of(index: u8) -> usize {
    MAP.iter().position(|p| p.level == Some(index)).unwrap_or(0)
}
