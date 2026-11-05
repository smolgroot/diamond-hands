//! The wassie's equipment, as the pause menu's bag lists it. Add an entry to [`ITEMS`]
//! (and an icon for it in the ROM's `art::menu`) for each new piece of gear or
//! collectible.

use crate::{
    arrival::Beat,
    game::{Game, Scene},
    player::HOVER_FRAMES,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Blaster,
    Jetpack,
    Scarf,
}

/// A gauge the bag can show under an item: the live value and its maximum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gauge {
    None,
    /// The jetpack's hover fuel.
    Fuel,
}

#[derive(Debug)]
pub struct ItemInfo {
    pub item: Item,
    pub name: &'static str,
    /// The slot it fills: weapon, gear or outfit.
    pub kind: &'static str,
    /// Two rows for the description panel, at most [`ABOUT_CHARS`] each.
    pub about: [&'static str; 2],
    /// Up to [`DETAIL_ROWS`] more rows when the item is inspected with A, at most
    /// [`DETAIL_CHARS`] each.
    pub detail: &'static [&'static str],
    pub gauge: Gauge,
}

/// Widest description row, in characters of the menu's 8-pixel font.
pub const ABOUT_CHARS: usize = 23;
/// Widest row of an inspected item's detail card, and most rows.
pub const DETAIL_CHARS: usize = 17;
pub const DETAIL_ROWS: usize = 5;

pub static ITEMS: [ItemInfo; 3] = [
    ItemInfo {
        item: Item::Blaster,
        name: "BLASTER",
        kind: "WEAPON",
        about: ["OVERSIZED LASER CANNON.", "HOLD B: CHARGED BEAM."],
        detail: &["FIRES 8 WAYS, ALL", "AT ONE SPEED.", "A FULL CHARGE", "PIERCES THROUGH", "EVERY BOT."],
        gauge: Gauge::None,
    },
    ItemInfo {
        item: Item::Jetpack,
        name: "JETPACK",
        kind: "GEAR",
        about: ["HOLD A AFTER A JUMP TO", "HOVER UP TO HIGH LEDGES"],
        detail: &["LANDING OR A", "STOMP REFUELS IT.", "LET GO OF A TO", "SAVE THE REST."],
        gauge: Gauge::Fuel,
    },
    ItemInfo {
        item: Item::Scarf,
        name: "RED SCARF",
        kind: "OUTFIT",
        about: ["ALL THE WASSIE OWNED", "WHEN IT LANDED HERE."],
        detail: &["NEVER SOLD.", "NEVER WILL BE.", "A DIAMOND HAND'S", "ONLY BANNER."],
        gauge: Gauge::None,
    },
];

impl Item {
    /// Whether the wassie has it on now. CL207 hands over the blaster and the jetpack
    /// during the arrival; the scarf was always there.
    pub fn equipped(self, game: &Game) -> bool {
        match self {
            Item::Scarf => true,
            Item::Blaster | Item::Jetpack => game.scene != Scene::Arrival || game.beat > Beat::Gift,
        }
    }
}

impl Gauge {
    /// The value and its maximum, or `None` for no gauge.
    pub fn reading(self, game: &Game) -> Option<(u32, u32)> {
        match self {
            Gauge::None => None,
            Gauge::Fuel => Some((game.world.player.hover_fuel as u32, HOVER_FRAMES as u32)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_item_fits_the_panels() {
        for info in &ITEMS {
            for row in info.about {
                assert!(row.len() <= ABOUT_CHARS, "too wide: {row}");
            }
            assert!(info.detail.len() <= DETAIL_ROWS, "{} has too many detail rows", info.name);
            for row in info.detail {
                assert!(row.len() <= DETAIL_CHARS, "too wide: {row}");
            }
        }
    }

    #[test]
    fn the_wassie_plays_with_everything_equipped() {
        let mut game = Game::new(0);
        game.scene = Scene::Play;
        assert!(ITEMS.iter().all(|i| i.item.equipped(&game)));
    }
}
