//! The levels in play order, and the places on the world map.

pub mod badlands;
pub mod trenches;

use crate::{arrival::Arrival, level::Level};

pub static ALL: [&Level; 2] = [&badlands::BADLANDS, &trenches::TRENCHES];

/// Which levels a save file has cleared: bit `i` is level `i`. A `u8` holds eight
/// levels; widen it (and the save format) before adding a ninth.
pub type Cleared = u8;

/// Every level cleared.
pub const ALL_CLEARED: Cleared = ((1u16 << ALL.len()) - 1) as Cleared;

/// Whether level `index` is cleared in `cleared`.
pub const fn is_cleared(cleared: Cleared, index: u8) -> bool {
    cleared & (1 << index) != 0
}

/// Levels unlock in order: the first is always open, each later one once the level
/// before it is cleared.
pub const fn is_unlocked(cleared: Cleared, index: u8) -> bool {
    (index as usize) < ALL.len() && (index == 0 || is_cleared(cleared, index - 1))
}

/// The first level not yet cleared, or the last level when all are.
pub fn frontier(cleared: Cleared) -> u8 {
    (0..ALL.len() as u8)
        .find(|&i| !is_cleared(cleared, i))
        .unwrap_or(ALL.len() as u8 - 1)
}

/// Level `index`'s arrival scene, played when a run starts it fresh.
pub fn arrival(index: u8) -> Option<&'static Arrival> {
    match index {
        0 => Some(&badlands::ARRIVAL),
        _ => None,
    }
}

/// Level `index`, or the last one when out of range.
pub fn get(index: u8) -> &'static Level {
    ALL[(index as usize).min(ALL.len() - 1)]
}

/// A stop on the world map.
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

/// How a place shows on the map for a save file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceStatus {
    /// Beaten; it can be replayed.
    Cleared,
    /// Unlocked and not yet beaten.
    Open,
    /// Built, but the level before it isn't cleared yet.
    Locked,
    /// No level here yet.
    Soon,
}

impl Place {
    pub fn status(&self, cleared: Cleared) -> PlaceStatus {
        match self.level {
            None => PlaceStatus::Soon,
            Some(i) if is_cleared(cleared, i) => PlaceStatus::Cleared,
            Some(i) if is_unlocked(cleared, i) => PlaceStatus::Open,
            Some(_) => PlaceStatus::Locked,
        }
    }

    /// Whether A starts its level.
    pub fn playable(&self, cleared: Cleared) -> bool {
        matches!(self.status(cleared), PlaceStatus::Cleared | PlaceStatus::Open)
    }
}

/// West to east, in the order the map cursor walks them.
pub static MAP: [Place; 3] = [
    Place {
        name: "THE BADLANDS",
        blurb: "MOSSY CLIFFS. BERA 3000 WAITS.",
        x: 44,
        y: 104,
        level: Some(0),
    },
    Place {
        name: "SOLANA TRENCHES",
        blurb: "NEON CASINO STRIP. THE HOUSE WINS.",
        x: 120,
        y: 64,
        level: Some(1),
    },
    Place {
        name: "ETHEREUM CITY",
        blurb: "GAS DRONES AND MEV BOTS. COMING SOON.",
        x: 196,
        y: 92,
        level: None,
    },
];

/// The map place for level `index`, or the first place.
pub fn place_of(index: u8) -> usize {
    MAP.iter().position(|p| p.level == Some(index)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_unlock_one_after_another() {
        assert!(is_unlocked(0, 0));
        assert!(!is_unlocked(0, 1));
        assert!(is_unlocked(0b1, 1));
        assert!(!is_unlocked(ALL_CLEARED, ALL.len() as u8), "no level past the last");
        assert_eq!(frontier(0), 0);
        assert_eq!(frontier(0b1), 1);
        assert_eq!(frontier(ALL_CLEARED), ALL.len() as u8 - 1);
    }

    #[test]
    fn places_show_cleared_open_locked_and_soon() {
        use PlaceStatus::*;
        let status = |cleared| MAP.each_ref().map(|p| p.status(cleared));
        assert_eq!(status(0), [Open, Locked, Soon]);
        assert_eq!(status(0b1), [Cleared, Open, Soon]);
        assert_eq!(status(0b11), [Cleared, Cleared, Soon]);
        assert!(!MAP[1].playable(0) && MAP[1].playable(0b1));
    }
}
