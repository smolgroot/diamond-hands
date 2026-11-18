//! The arrival on CryptoTerra: CL207, a white cat in a yellow cat-eared hazmat suit,
//! and the cargo ship that drops the wassie off.

use agb::display::object::SpriteVram;


pub struct ArrivalArt {
    /// CL207's top half, 32x16, facing right: the hood and face, the same in every
    /// pose. [`Self::guide_legs`] goes underneath; together they make a 32x32 figure
    /// with its feet on the bottom row. Split so three poses cost what two whole
    /// frames would: sprite VRAM has no room for more.
    pub guide_top: SpriteVram,
    /// The bottom half standing, then the two walking frames.
    pub guide_legs: [SpriteVram; 3],
    /// The cargo ship's left half, 32x32, hatch shut; the right half is the same
    /// sprite mirrored. Sprite VRAM is nearly full, so the open hatch and the thruster
    /// flames are small overlays rather than whole ship frames.
    pub ship_half: SpriteVram,
    /// Laid over the ship at [`HATCH_AT`].
    pub hatch_open: SpriteVram,
    /// Two flickers of thruster flame, 8x16, placed at [`FLAMES_AT`].
    pub flames: [SpriteVram; 2],
}

/// Where the open hatch sits in the ship sprite.
pub const HATCH_AT: (i32, i32) = (24, 23);
/// Where each thruster's flame hangs from the ship sprite.
pub const FLAMES_AT: [(i32, i32); 2] = [(5, 20), (51, 20)];

impl ArrivalArt {
    pub fn new() -> Self {
        Self {
            guide_top: SpriteVram::from(&crate::gfx::SPRITE_CL207.frames[0]),
            guide_legs: super::studio::cl207::guide_legs(),
            ship_half: super::studio::ship_32x32::ship_half(),
            hatch_open: super::studio::ship_16x8::hatch_open(),
            flames: super::studio::ship_8x16::flames(),
        }
    }
}

/// CL207's face for the dialogue box, facing left, and the same face with its mouth
/// half open and wide open, for while he talks. Kept loaded in every level: CL207
/// calls in the Trenches too. The rest of [`ArrivalArt`] is the Badlands' alone.
pub fn portrait() -> [SpriteVram; 3] {
    super::studio::cl207_portrait::portrait()
}
