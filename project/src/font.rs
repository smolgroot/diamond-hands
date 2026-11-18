//! The game's one typeface: 5x7 pixels with one-pixel strokes, the dialogue box's
//! font. The dialogue and pause menu paint it into background tiles; everywhere else
//! (splash, title, file select, options, map, HUD...) draws it from one 8x8 sprite per
//! glyph rendered at boot, one object per character. Big text is the same sprites
//! doubled through a shared affine matrix, so it costs no VRAM of its own.

use agb::{
    display::{
        AffineMatrix, GraphicsFrame, Priority,
        object::{
            AffineMatrixObject, AffineMode, Object, ObjectAffine,
            SpriteVram,
        },
    },
    fixnum::{num, vec2},
};


pub struct Font {
    glyphs: [SpriteVram; DIALOG_GLYPHS.len()],
    /// Twice the size: screen pixels step half a texel. With [`AffineMode::AffineDouble`]
    /// an 8x8 glyph fills a 16x16 box from the object's position, each texel 2x2.
    double: AffineMatrixObject,
}

impl Font {
    pub fn new() -> Self {
        let double: AffineMatrix = AffineMatrix::from_scale(vec2(num!(0.5), num!(0.5)));
        Self {
            // The font sheet's cells (`assets/sprites/font.png`), one per glyph.
            glyphs: core::array::from_fn(|i| SpriteVram::from(&crate::gfx::SPRITE_FONT.frames[i])),
            double: double.into(),
        }
    }

    /// Pixels from one character to the next: the glyph and a one-pixel gap (both
    /// doubled for big text). Small text is 8 pixels tall with its shadow, big 16.
    pub fn advance(big: bool) -> i32 {
        if big { 12 } else { 6 }
    }

    pub fn width(text: &str, big: bool) -> i32 {
        text.len() as i32 * Self::advance(big)
    }

    /// Draws ASCII `text` with its top-left at (x, y). Unknown characters (and spaces)
    /// leave a blank.
    pub fn draw(&self, frame: &mut GraphicsFrame, text: &str, x: i32, y: i32, big: bool) {
        for (i, byte) in text.bytes().enumerate() {
            let Some(index) = DIALOG_GLYPHS.iter().position(|(c, _)| *c == byte) else {
                continue;
            };
            let pos = vec2(x + i as i32 * Self::advance(big), y);
            let sprite = self.glyphs[index].clone();
            if big {
                ObjectAffine::new(sprite, self.double.clone(), AffineMode::AffineDouble)
                    .set_pos(pos)
                    .set_priority(Priority::P0)
                    .show(frame);
            } else {
                Object::new(sprite).set_pos(pos).set_priority(Priority::P0).show(frame);
            }
        }
    }

    pub fn draw_centred(&self, frame: &mut GraphicsFrame, text: &str, y: i32, big: bool) {
        let x = (agb::display::WIDTH - Self::width(text, big)) / 2;
        self.draw(frame, text, x, y, big);
    }

    /// Draws `text` ending (its trailing gap included) at `right`.
    pub fn draw_right(&self, frame: &mut GraphicsFrame, text: &str, right: i32, y: i32, big: bool) {
        self.draw(frame, text, right - Self::width(text, big), y, big);
    }
}

include!(concat!(env!("OUT_DIR"), "/dialog_glyphs.rs"));

/// The dialogue font's 5x7 bitmap for `byte`, if it has one: seven rows of five bits,
/// most significant bit on the left.
pub fn dialog_glyph(byte: u8) -> Option<&'static [u8; 7]> {
    DIALOG_GLYPHS.iter().find(|(c, _)| *c == byte).map(|(_, rows)| rows)
}

/// Formats `n` in decimal into `buf` without allocating.
pub fn number(n: u32, buf: &mut [u8; 10]) -> &str {
    let mut i = buf.len();
    let mut n = n;
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    // Only ASCII digits were written.
    core::str::from_utf8(&buf[i..]).unwrap_or("")
}
