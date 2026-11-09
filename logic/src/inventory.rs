//! The wassie's equipment, as the pause menu's bag lists it. Add an entry to [`ITEMS`]
//! (and an icon for it in the ROM's `art::menu`) for each new piece of gear or
//! collectible.

use crate::{
    arrival::Beat,
    game::{Game, Scene},
    player::HOVER_FRAMES,
    world::cells,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Blaster,
    Jetpack,
    Scarf,
    /// Parts for CL207's plasma shield, found in the Trenches.
    Cells,
}

/// A gauge the bag can show under an item: the live value and its maximum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gauge {
    None,
    /// The jetpack's hover fuel.
    Fuel,
    /// Plasma cells found, out of those the shield needs.
    Cells,
}

/// Plasma cells CL207's shield needs.
pub const SHIELD_CELLS: u32 = 3;

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

pub static ITEMS: [ItemInfo; 4] = [
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
    ItemInfo {
        item: Item::Cells,
        name: "PLASMA CELLS",
        kind: "PARTS",
        about: ["FOR CL207'S NEW GADGET:", "A PLASMA SHIELD."],
        detail: &["THREE ARE HIDDEN", "IN THE SOLANA", "TRENCHES. CL207", "NEEDS THEM ALL."],
        gauge: Gauge::Cells,
    },
];

impl Item {
    /// Whether the wassie has it on now. CL207 hands over the blaster and the jetpack
    /// during the arrival (only the blaster where the jetpack is hidden in the level);
    /// the scarf was always there.
    pub fn equipped(self, game: &Game) -> bool {
        match self {
            Item::Scarf => true,
            Item::Cells => cells(game.parts | game.world.parts) != 0,
            Item::Blaster => game.scene != Scene::Arrival || game.beat > Beat::Gift,
            // A level that hides the jetpack holds it back until it is found.
            Item::Jetpack => game.world.player.jetpack && (game.scene != Scene::Arrival || game.beat > Beat::Gift),
        }
    }
}

impl Gauge {
    /// The value and its maximum, or `None` for no gauge.
    pub fn reading(self, game: &Game) -> Option<(u32, u32)> {
        match self {
            Gauge::None => None,
            Gauge::Fuel => Some((game.world.player.hover_fuel as u32, HOVER_FRAMES as u32)),
            Gauge::Cells => Some((cells(game.parts | game.world.parts).count_ones().min(SHIELD_CELLS), SHIELD_CELLS)),
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
        // In a level that doesn't hide the jetpack.
        game.world = alloc::boxed::Box::new(crate::World::new(1, crate::Difficulty::Normal));
        assert!(ITEMS.iter().filter(|i| i.item != Item::Cells).all(|i| i.item.equipped(&game)));
        assert!(!Item::Cells.equipped(&game), "no cells until one is found");
    }

    #[test]
    fn a_hidden_jetpack_shows_once_found_and_is_no_plasma_cell() {
        let mut game = Game::new(0);
        game.scene = Scene::Play;
        game.world = alloc::boxed::Box::new(crate::World::new(0, crate::Difficulty::Normal));
        assert!(game.world.jetpack_here, "the Badlands hide the jetpack");
        assert!(!Item::Jetpack.equipped(&game));
        game.world.own_parts(crate::world::GEAR_JETPACK);
        assert!(Item::Jetpack.equipped(&game));
        assert!(!Item::Cells.equipped(&game), "the jetpack is not a plasma cell");
    }
}
