//! What each button does, built from the key constants in [`crate::keys`] that the
//! game actually reads. The pause menu and the title list these, so the controls they
//! show always match the input mapping. Buttons the game doesn't use (Select) are left
//! out.

use crate::game::keys;

/// One row of the controls list: the buttons, what they do in play, and in menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub keys: u16,
    pub play: &'static str,
    /// Empty when the buttons do nothing in menus.
    pub menu: &'static str,
}

/// Every direction on the pad.
pub const DPAD: u16 = keys::UP | keys::DOWN | keys::LEFT | keys::RIGHT;

/// The controls, in the order the menus list them.
pub static BINDINGS: [Binding; 5] = [
    Binding { keys: DPAD, play: "MOVE AND AIM", menu: "NAVIGATE" },
    Binding { keys: keys::JUMP, play: "JUMP. HOLD: HOVER", menu: "CONFIRM" },
    Binding { keys: keys::FIRE, play: "FIRE. HOLD: CHARGE", menu: "BACK" },
    Binding { keys: keys::PLANT, play: "PLANT FEET AND AIM", menu: "SWITCH TAB" },
    Binding { keys: keys::START, play: "PAUSE MENU", menu: "RESUME" },
];

/// How a set of buttons is written on screen.
pub fn label(keys: u16) -> &'static str {
    match keys {
        DPAD => "D-PAD",
        keys::A => "A",
        keys::B => "B",
        keys::PLANT => "L/R",
        keys::L => "L",
        keys::R => "R",
        keys::START => "START",
        keys::SELECT => "SELECT",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Input;

    /// The list describes what `Input` really does with each button.
    #[test]
    fn bindings_match_what_the_game_reads() {
        let held = |keys| Input::from_keys(keys, 0);
        let find = |play: &str| BINDINGS.iter().find(|b| b.play == play).unwrap().keys;
        assert!(held(find("JUMP. HOLD: HOVER")).jump_pressed());
        assert!(held(find("FIRE. HOLD: CHARGE")).fire_pressed());
        assert!(held(find("PAUSE MENU")).start_pressed());
        let plant = find("PLANT FEET AND AIM");
        assert!(held(keys::L).plant() && held(keys::R).plant() && plant == keys::L | keys::R);
        assert_eq!(held(find("MOVE AND AIM") & keys::RIGHT).dir_x(), 1);
        for b in &BINDINGS {
            assert_ne!(label(b.keys), "?", "{b:?} has no label");
        }
        // No button does two things in play.
        let all = BINDINGS.iter().fold(0, |seen, b| {
            assert_eq!(seen & b.keys, 0, "{b:?} overlaps");
            seen | b.keys
        });
        assert_eq!(all & keys::SELECT, 0, "Select is unused, so it isn't listed");
    }
}
