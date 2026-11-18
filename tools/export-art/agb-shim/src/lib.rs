//! Host stand-ins for the slice of agb that `game/src/art` draws with: palettes, sprites
//! painted pixel by pixel, 16-colour tiles and a background to place them on. They keep
//! the pixels in memory, so the exporter can read back what the art painted.

/// Unused on the host: sprites live in ordinary memory.
pub struct ExternalAllocator;

pub mod display {
    pub const WIDTH: i32 = 240;
    pub const HEIGHT: i32 = 160;

    /// A colour as the GBA holds it: 5 bits of red, green and blue.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct Rgb15(pub u16);

    impl Rgb15 {
        pub const fn new(value: u16) -> Self {
            Rgb15(value)
        }

        /// 8-bit red, green and blue, as a PNG holds them.
        pub fn to_rgb8(self) -> [u8; 3] {
            let c = |shift: u16| (((self.0 >> shift) & 31) << 3) as u8;
            [c(0), c(5), c(10)]
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub struct Palette16 {
        colours: [Rgb15; 16],
    }

    impl Palette16 {
        pub const fn new(colours: [Rgb15; 16]) -> Self {
            Self { colours }
        }

        pub const fn colour(&self, index: usize) -> Rgb15 {
            self.colours[index]
        }

        pub const fn update_colour(&mut self, index: usize, colour: Rgb15) {
            self.colours[index] = colour;
        }

        pub fn colours(&self) -> &[Rgb15; 16] {
            &self.colours
        }
    }

    pub mod object {
        use std::rc::Rc;

        use super::Palette16;

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Size {
            S8x8,
            S16x16,
            S32x32,
            S64x64,
            S16x8,
            S32x8,
            S32x16,
            S64x32,
            S8x16,
            S8x32,
            S16x32,
            S32x64,
        }

        impl Size {
            pub const fn to_width_height(self) -> (usize, usize) {
                match self {
                    Size::S8x8 => (8, 8),
                    Size::S16x16 => (16, 16),
                    Size::S32x32 => (32, 32),
                    Size::S64x64 => (64, 64),
                    Size::S16x8 => (16, 8),
                    Size::S32x8 => (32, 8),
                    Size::S32x16 => (32, 16),
                    Size::S64x32 => (64, 32),
                    Size::S8x16 => (8, 16),
                    Size::S8x32 => (8, 32),
                    Size::S16x32 => (16, 32),
                    Size::S32x64 => (32, 64),
                }
            }
        }

        /// A sprite being painted: one palette index a pixel, row by row.
        pub struct DynamicSprite16<A = crate::ExternalAllocator> {
            pub width: usize,
            pub height: usize,
            pub pixels: Vec<u8>,
            _allocator: core::marker::PhantomData<A>,
        }

        impl<A> DynamicSprite16<A> {
            pub fn new_in(size: Size, _allocator: A) -> Self {
                let (width, height) = size.to_width_height();
                Self { width, height, pixels: vec![0; width * height], _allocator: core::marker::PhantomData }
            }

            /// From 4-bit tiles laid out as the GBA's 1D sprite mapping: 8x8 tiles left to
            /// right, then down, two pixels a byte with the left one in the low nibble.
            pub fn from_bytes_in(size: Size, bytes: &[u8], allocator: A) -> Self {
                let mut sprite = Self::new_in(size, allocator);
                let tiles_across = sprite.width / 8;
                for (i, byte) in bytes.iter().enumerate() {
                    let (tile, within) = (i / 32, i % 32);
                    let (tx, ty) = (tile % tiles_across, tile / tiles_across);
                    let (x, y) = (tx * 8 + within % 4 * 2, ty * 8 + within / 4);
                    if y < sprite.height {
                        sprite.pixels[y * sprite.width + x] = byte & 15;
                        sprite.pixels[y * sprite.width + x + 1] = byte >> 4;
                    }
                }
                sprite
            }

            pub fn clear(&mut self, paletted_pixel: usize) {
                self.pixels.fill(paletted_pixel as u8);
            }

            pub fn set_pixel(&mut self, x: usize, y: usize, paletted_pixel: u8) {
                self.pixels[y * self.width + x] = paletted_pixel;
            }

            pub fn to_vram(&self, palette: &Palette16) -> SpriteVram {
                SpriteVram(Rc::new(SpriteData {
                    width: self.width,
                    height: self.height,
                    pixels: self.pixels.clone(),
                    palette: palette.clone(),
                }))
            }
        }

        pub struct SpriteData {
            pub width: usize,
            pub height: usize,
            pub pixels: Vec<u8>,
            pub palette: Palette16,
        }

        /// A finished sprite: its pixels and the palette they are drawn in.
        #[derive(Clone)]
        pub struct SpriteVram(pub Rc<SpriteData>);
    }

    pub mod tiled {
        use std::collections::BTreeMap;

        /// An 8x8 tile of 4-bit pixels: a row a word, leftmost pixel in the low nibble.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct DynamicTile16 {
            data: [u32; 8],
        }

        impl DynamicTile16 {
            #[allow(clippy::new_without_default)]
            pub fn new() -> Self {
                Self { data: [0; 8] }
            }

            pub fn fill_with(mut self, colour_index: u8) -> Self {
                let c = u32::from(colour_index & 15);
                self.data = [(0..8).fold(0, |acc, _| acc << 4 | c); 8];
                self
            }

            pub fn data(&self) -> &[u32] {
                &self.data
            }

            pub fn data_mut(&mut self) -> &mut [u32] {
                &mut self.data
            }

            pub fn set_pixel(&mut self, x: usize, y: usize, palette_index: u8) {
                let shift = x * 4;
                self.data[y] = self.data[y] & !(15 << shift) | u32::from(palette_index & 15) << shift;
            }

            /// The palette index at (x, y).
            pub fn pixel(&self, x: usize, y: usize) -> u8 {
                (self.data[y] >> (x * 4) & 15) as u8
            }
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct TileEffect {
            pub hflip: bool,
            pub vflip: bool,
            pub palette: u8,
        }

        impl TileEffect {
            pub const fn new(hflip: bool, vflip: bool, palette: u8) -> Self {
                Self { hflip, vflip, palette }
            }
        }

        /// A background as a sparse map of the tiles placed on it.
        #[derive(Default)]
        pub struct RegularBackground {
            pub cells: BTreeMap<(i32, i32), (DynamicTile16, TileEffect)>,
        }

        impl RegularBackground {
            pub fn new() -> Self {
                Self::default()
            }

            pub fn set_tile_dynamic16(&mut self, pos: impl Into<(i32, i32)>, tile: &DynamicTile16, effect: TileEffect) -> &mut Self {
                self.cells.insert(pos.into(), (tile.clone(), effect));
                self
            }
        }
    }
}
