#![no_std]
#![no_main]

// Placeholder: shows nothing yet. Port the first levels into studio/, then bring in
// the game code from ../game and ../logic. See sample/src/main.rs in gba-studio.
#[agb::entry]
fn main(_gba: agb::Gba) -> ! {
    loop {
        agb::halt();
    }
}
