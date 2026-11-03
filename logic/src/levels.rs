//! The levels in play order.

pub mod canyon;

use crate::level::Level;

pub static ALL: [&Level; 1] = [&canyon::CANYON];

/// Level `index`, or the last one when out of range.
pub fn get(index: u8) -> &'static Level {
    ALL[(index as usize).min(ALL.len() - 1)]
}
