//! The game's one typeface: 5x7 pixels with one-pixel strokes, the dialogue box's
//! font. The dialogue and pause menu paint it into background tiles; everywhere else
//! (splash, title, file select, options, map, HUD...) draws it from one 8x8 sprite per
//! glyph rendered at boot, one object per character. Big text is the same sprites
//! doubled through a shared affine matrix, so it costs no VRAM of its own.

use agb::{
    display::{
        AffineMatrix, GraphicsFrame, Priority,
        object::{
            AffineMatrixObject, AffineMode, DynamicSprite16, Object, ObjectAffine, Size,
            SpriteVram,
        },
    },
    fixnum::{num, vec2},
};

use crate::art::{SPRITE_PALETTE, ix};

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
            glyphs: core::array::from_fn(|i| render(&DIALOG_GLYPHS[i].1)),
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

/// Each glyph is seven rows of five bits, most significant bit on the left.
const DIALOG_GLYPHS: &[(u8, [u8; 7])] = &[
    (b'0', [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110]),
    (b'1', [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
    (b'2', [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111]),
    (b'3', [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110]),
    (b'4', [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010]),
    (b'5', [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110]),
    (b'6', [0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110]),
    (b'7', [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000]),
    (b'8', [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110]),
    (b'9', [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110]),
    (b'A', [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
    (b'B', [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110]),
    (b'C', [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110]),
    (b'D', [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110]),
    (b'E', [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
    (b'F', [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000]),
    (b'G', [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111]),
    (b'H', [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
    (b'I', [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110]),
    (b'J', [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100]),
    (b'K', [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001]),
    (b'L', [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111]),
    (b'M', [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001]),
    (b'N', [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001]),
    (b'O', [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
    (b'P', [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000]),
    (b'Q', [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101]),
    (b'R', [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
    (b'S', [0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110]),
    (b'T', [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100]),
    (b'U', [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
    (b'V', [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100]),
    (b'W', [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001]),
    (b'X', [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001]),
    (b'Y', [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100]),
    (b'Z', [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111]),
    (b'!', [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100]),
    (b':', [0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000]),
    (b'/', [0b00001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000]),
    (b'-', [0b00000, 0b00000, 0b00000, 0b01110, 0b00000, 0b00000, 0b00000]),
    (b'+', [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000]),
    (b'.', [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100]),
    (b'<', [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010]),
    (b'>', [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000]),
    (b',', [0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b00100, 0b01000]),
    (b'\'', [0b00100, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000]),
    (b'?', [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100]),
];

/// The dialogue font's 5x7 bitmap for `byte`, if it has one: seven rows of five bits,
/// most significant bit on the left.
pub fn dialog_glyph(byte: u8) -> Option<&'static [u8; 7]> {
    DIALOG_GLYPHS.iter().find(|(c, _)| *c == byte).map(|(_, rows)| rows)
}

/// A glyph in an 8x8 sprite: white, with a dark drop shadow a pixel right and down.
fn render(rows: &[u8; 7]) -> SpriteVram {
    // EWRAM, like `art::Canvas`: IWRAM allocations can land on the stacks.
    let mut sprite = DynamicSprite16::new_in(Size::S8x8, agb::ExternalAllocator);
    sprite.clear(ix::CLEAR as usize);
    for (d, colour) in [(1, ix::OUTLINE), (0, ix::WHITE)] {
        for (y, bits) in rows.iter().enumerate() {
            for x in 0..5 {
                if bits & (0b10000 >> x) != 0 {
                    sprite.set_pixel(x + d, y + d, colour);
                }
            }
        }
    }
    sprite.to_vram(&SPRITE_PALETTE)
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
