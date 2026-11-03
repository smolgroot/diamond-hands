//! A 3x5 pixel font rendered into sprites at boot, in a small (8x8) and a double-size
//! (8x16: 7x11 with the shadow) variant. Text is drawn as one object per character.

use agb::{
    display::{
        GraphicsFrame, Priority,
        object::{DynamicSprite16, Object, Size, SpriteVram},
    },
    fixnum::vec2,
};

use crate::art::{SPRITE_PALETTE, ix};

/// Each glyph is five rows of three bits, most significant bit on the left.
const GLYPHS: &[(u8, [u8; 5])] = &[
    (b'0', [0b111, 0b101, 0b101, 0b101, 0b111]),
    (b'1', [0b010, 0b110, 0b010, 0b010, 0b111]),
    (b'2', [0b111, 0b001, 0b111, 0b100, 0b111]),
    (b'3', [0b111, 0b001, 0b111, 0b001, 0b111]),
    (b'4', [0b101, 0b101, 0b111, 0b001, 0b001]),
    (b'5', [0b111, 0b100, 0b111, 0b001, 0b111]),
    (b'6', [0b111, 0b100, 0b111, 0b101, 0b111]),
    (b'7', [0b111, 0b001, 0b010, 0b010, 0b010]),
    (b'8', [0b111, 0b101, 0b111, 0b101, 0b111]),
    (b'9', [0b111, 0b101, 0b111, 0b001, 0b111]),
    (b'A', [0b010, 0b101, 0b111, 0b101, 0b101]),
    (b'B', [0b110, 0b101, 0b110, 0b101, 0b110]),
    (b'C', [0b011, 0b100, 0b100, 0b100, 0b011]),
    (b'D', [0b110, 0b101, 0b101, 0b101, 0b110]),
    (b'E', [0b111, 0b100, 0b110, 0b100, 0b111]),
    (b'F', [0b111, 0b100, 0b110, 0b100, 0b100]),
    (b'G', [0b011, 0b100, 0b101, 0b101, 0b011]),
    (b'H', [0b101, 0b101, 0b111, 0b101, 0b101]),
    (b'I', [0b111, 0b010, 0b010, 0b010, 0b111]),
    (b'J', [0b001, 0b001, 0b001, 0b101, 0b010]),
    (b'K', [0b101, 0b101, 0b110, 0b101, 0b101]),
    (b'L', [0b100, 0b100, 0b100, 0b100, 0b111]),
    (b'M', [0b101, 0b111, 0b111, 0b101, 0b101]),
    (b'N', [0b110, 0b101, 0b101, 0b101, 0b101]),
    (b'O', [0b010, 0b101, 0b101, 0b101, 0b010]),
    (b'P', [0b110, 0b101, 0b110, 0b100, 0b100]),
    (b'Q', [0b010, 0b101, 0b101, 0b110, 0b011]),
    (b'R', [0b110, 0b101, 0b110, 0b101, 0b101]),
    (b'S', [0b011, 0b100, 0b010, 0b001, 0b110]),
    (b'T', [0b111, 0b010, 0b010, 0b010, 0b010]),
    (b'U', [0b101, 0b101, 0b101, 0b101, 0b111]),
    (b'V', [0b101, 0b101, 0b101, 0b101, 0b010]),
    (b'W', [0b101, 0b101, 0b111, 0b111, 0b101]),
    (b'X', [0b101, 0b101, 0b010, 0b101, 0b101]),
    (b'Y', [0b101, 0b101, 0b010, 0b010, 0b010]),
    (b'Z', [0b111, 0b001, 0b010, 0b100, 0b111]),
    (b'!', [0b010, 0b010, 0b010, 0b000, 0b010]),
    (b':', [0b000, 0b010, 0b000, 0b010, 0b000]),
    (b'/', [0b001, 0b001, 0b010, 0b100, 0b100]),
    (b'-', [0b000, 0b000, 0b111, 0b000, 0b000]),
    (b'+', [0b000, 0b010, 0b111, 0b010, 0b000]),
    (b'.', [0b000, 0b000, 0b000, 0b000, 0b010]),
    (b'<', [0b001, 0b010, 0b100, 0b010, 0b001]),
    (b'>', [0b100, 0b010, 0b001, 0b010, 0b100]),
    (b',', [0b000, 0b000, 0b000, 0b010, 0b100]),
    (b'\'', [0b010, 0b010, 0b000, 0b000, 0b000]),
    (b'?', [0b111, 0b001, 0b010, 0b000, 0b010]),
];

pub struct Font {
    small: [SpriteVram; GLYPHS.len()],
    big: [SpriteVram; GLYPHS.len()],
}

impl Font {
    pub fn new() -> Self {
        Self {
            small: core::array::from_fn(|i| render(&GLYPHS[i].1, 1)),
            big: core::array::from_fn(|i| render(&GLYPHS[i].1, 2)),
        }
    }

    pub fn advance(big: bool) -> i32 {
        if big { 8 } else { 4 }
    }

    pub fn width(text: &str, big: bool) -> i32 {
        text.len() as i32 * Self::advance(big)
    }

    /// Draws ASCII `text` with its top-left at (x, y). Unknown characters (and spaces)
    /// leave a blank.
    pub fn draw(&self, frame: &mut GraphicsFrame, text: &str, x: i32, y: i32, big: bool) {
        let sprites = if big { &self.big } else { &self.small };
        for (i, byte) in text.bytes().enumerate() {
            let Some(index) = GLYPHS.iter().position(|(c, _)| *c == byte) else {
                continue;
            };
            Object::new(sprites[index].clone())
                .set_pos(vec2(x + i as i32 * Self::advance(big), y))
                .set_priority(Priority::P0)
                .show(frame);
        }
    }

    pub fn draw_centred(&self, frame: &mut GraphicsFrame, text: &str, y: i32, big: bool) {
        let x = (agb::display::WIDTH - Self::width(text, big)) / 2;
        self.draw(frame, text, x, y, big);
    }
}

fn render(rows: &[u8; 5], scale: usize) -> SpriteVram {
    let size = if scale == 1 { Size::S8x8 } else { Size::S8x16 };
    // EWRAM, like `art::Canvas`: IWRAM allocations can land on the stacks.
    let mut sprite = DynamicSprite16::new_in(size, agb::ExternalAllocator);
    sprite.clear(ix::CLEAR as usize);
    let lit = |x: usize, y: usize| y < 5 && x < 3 && rows[y] & (0b100 >> x) != 0;
    // Drop shadow first, then the glyph on top.
    for (dx, colour) in [(1, ix::OUTLINE), (0, ix::WHITE)] {
        for y in 0..5 {
            for x in 0..3 {
                if !lit(x, y) {
                    continue;
                }
                for sy in 0..scale {
                    for sx in 0..scale {
                        sprite.set_pixel(x * scale + sx + dx, y * scale + sy + dx, colour);
                    }
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
