//! The splash at power-on: the cover art, then a blinking "PRESS START TO PLAY" once
//! the game will take it.
//!
//! The cover fills the screen with 600 distinct tiles drawn from all 16 background
//! palettes, which `scripts/make-splash.py` fits it to and writes out in the hardware
//! formats (agb's importer can't pack palettes that full). That is more tiles than fit
//! next to the game's own layers, so it is shown before the
//! [`crate::render::Renderer`] is built and dropped first; the renderer sets its own
//! palettes again when it is built.

use agb::{
    display::{
        GraphicsFrame, Palette16, Priority, Rgb15, WIDTH, WinIn,
        tiled::{
            RegularBackground, RegularBackgroundSize, TileEffect, TileFormat, TileSet, TileSetting,
        },
    },
    fixnum::{Rect, num, vec2},
};
use diamond_hands_logic::Game;

use crate::font::Font;

#[repr(C, align(4))]
struct Aligned<T: ?Sized>(T);

static TILES: &Aligned<[u8]> = &Aligned(*include_bytes!("../gfx/splash.tiles"));
/// Palette of each tile, left to right then top to bottom.
static TILE_PALETTES: &[u8; 600] = include_bytes!("../gfx/splash.map");
/// 16 palettes of 16 BGR555 colours, little-endian.
static PALETTES: &[u8; 512] = include_bytes!("../gfx/splash.pal");

/// The prompt blinks on and off every this many frames.
const PROMPT_BLINK: u32 = 32;
/// Prompt row: over the blaster, clear of the title and the wassie's face.
const PROMPT_Y: i32 = 136;
/// The band darkened behind the prompt, so it reads over the busy art.
const BAND_TOP: i32 = PROMPT_Y - 3;
const BAND_HEIGHT: i32 = 21;

pub struct Splash {
    cover: RegularBackground,
    font: Font,
}

impl Splash {
    pub fn new(gfx: &mut agb::display::Graphics) -> Self {
        let palettes: [Palette16; 16] = core::array::from_fn(|p| {
            Palette16::new(core::array::from_fn(|c| {
                let at = (p * 16 + c) * 2;
                Rgb15::new(u16::from_le_bytes([PALETTES[at], PALETTES[at + 1]]))
            }))
        });
        gfx.set_background_palettes(&palettes);

        // SAFETY: `Aligned` keeps the data 4-byte aligned, and it is whole 32-byte tiles.
        let tiles = unsafe { TileSet::new(&TILES.0, TileFormat::FourBpp) };
        let mut cover = RegularBackground::new(
            Priority::P3,
            RegularBackgroundSize::Background32x32,
            TileFormat::FourBpp,
        );
        for (i, &palette) in TILE_PALETTES.iter().enumerate() {
            let effect = TileEffect::new(false, false, palette);
            let pos = ((i % 30) as i32, (i / 30) as i32);
            cover.set_tile(pos, &tiles, TileSetting::new(i as u16, effect));
        }
        Self { cover, font: Font::new() }
    }

    pub fn draw(&self, game: &Game, frame: &mut GraphicsFrame) {
        let cover = self.cover.show(frame);
        let Some(shown) = game.splash_prompt() else {
            return;
        };
        if (shown / PROMPT_BLINK).is_multiple_of(2) {
            self.font.draw_centred(frame, "PRESS START TO PLAY", PROMPT_Y, true);
        }
        frame.blend().darken(num!(0.5)).enable_background(cover);
        let windows = frame.windows();
        windows
            .win_in(WinIn::Win0)
            .set_pos(Rect::new(vec2(0, BAND_TOP), vec2(WIDTH, BAND_HEIGHT)))
            .enable_background(cover)
            .enable_objects()
            .enable_blending();
        windows.win_out().enable_background(cover).enable_objects();
    }
}
