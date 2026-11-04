//! The arrival on CryptoTerra: CL207, a grey cat in a yellow hazmat suit, and the
//! cargo ship that drops the wassie off.

use agb::display::object::{Size, SpriteVram};

use super::{Canvas, SPRITE_KEY, ix};

/// CL207's hooded head, facing left: hood ears, and a visor showing a grey tabby face.
/// 16x16, so the portrait is the same drawing at double size.
const HEAD: [&str; 16] = [
    "................",
    "...kk.....kk....",
    "..kyyk...kyyk...",
    "..kyoyk.kyoyk...",
    ".kyyyyykyyyyyk..",
    ".kyyyyyyyyyyyyk.",
    "kyykkkkkkkkyyyok",
    "kykSgSSSgSwkyyok",
    "kykSgSSSgSSkyyok",
    "kykkiSSkiSSkyyok",
    "kykiiSgiiSSkyyok",
    "kykSScSSSgSkyyok",
    "kykwSkSSSwSkyyok",
    "kykgSSSSSgSkyook",
    ".kykkkkkkkkyyok.",
    "..kyyyyyyyyyok..",
];

/// The suit below the head, standing: an air tank on the back and a blue badge.
const BODY_STAND: [&str; 16] = [
    "..kkggkkkkkkkSk.",
    ".kyyyyyyyyyykgSk",
    ".kyyyyyyyyyyykgk",
    "kGkyyybbyyyyykgk",
    "kGkyyybiyyyyokgk",
    "kGkyyyyyyyyyokgk",
    "kGkyyyyyyyyyokSk",
    ".kkkkkkkkkkkkkk.",
    "..kyyyyyyyyyyk..",
    "..kyyyoykyyyok..",
    "..kyyyokkyyyok..",
    "..kyyyok.kyyok..",
    "..kGGGGk.kGGGk..",
    ".kGGGGGk.kGGGGk.",
    ".kkkkkkk.kkkkkk.",
    "................",
];

/// Mid-stride, front glove swinging forward.
const BODY_WALK: [&str; 16] = [
    "..kkggkkkkkkkSk.",
    ".kyyyyyyyyyykgSk",
    "kGkyyyyyyyyyykgk",
    "kGkyyybbyyyyykgk",
    "kGkyyybiyyyyokgk",
    ".kkyyyyyyyyyokgk",
    "..kyyyyyyyyyokSk",
    ".kkkkkkkkkkkkkk.",
    ".kyyyyyyyyyyyk..",
    "kyyyok..kyyyyk..",
    "kyyok....kyyok..",
    "kGGGk....kyyok..",
    "kGGGGk...kGGGk..",
    "kkkkkk..kGGGGk..",
    "........kkkkkk..",
    "................",
];

pub struct ArrivalArt {
    /// CL207 standing and mid-stride, 16x32, facing left.
    pub guide: [SpriteVram; 2],
    /// CL207's head for the dialogue box, facing left.
    pub portrait: SpriteVram,
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
        let guide = |body: &[&str; 16]| {
            let mut c = Canvas::new(Size::S16x32);
            c.art(0, 0, &HEAD);
            c.art(0, 16, body);
            c.vram()
        };
        Self {
            guide: [guide(&BODY_STAND), guide(&BODY_WALK)],
            portrait: {
                let mut c = Canvas::new(Size::S32x32);
                c.art_scaled(SPRITE_KEY, 0, 0, &HEAD, 2);
                c.vram()
            },
            ship_half: ship_half(),
            hatch_open: {
                let mut c = Canvas::new(Size::S16x8);
                c.rect(0, 0, 16, 5, ix::OUTLINE);
                c.rect(1, 1, 14, 2, ix::GOLD);
                c.rect(2, 3, 12, 1, ix::ORANGE);
                c.vram()
            },
            flames: [0, 1].map(|flicker| {
                let mut c = Canvas::new(Size::S8x16);
                let n = 5 + flicker * 2;
                c.rect(1, 0, 6, n, ix::LASER);
                c.rect(2, 0, 4, n - 2, ix::LASER_LIGHT);
                c.rect(3, 0, 2, n - 4, ix::WHITE);
                c.vram()
            }),
        }
    }
}

/// The left half of a 64-pixel cargo ship: an armoured pod with a lit window at each
/// end, a container slung underneath with its hatch in the floor, and a thruster pod at
/// each end. Drawn at full width; the canvas keeps the left 32 columns.
fn ship_half() -> SpriteVram {
    let mut c = Canvas::new(Size::S32x32);
    for x in [3, 49] {
        c.block(x, 14, 12, 8, ix::STEEL_DARK, ix::OUTLINE);
        c.rect(x + 1, 15, 10, 1, ix::STEEL);
        c.rect(x + 2, 20, 8, 1, ix::ORANGE);
    }
    // The container, with warning lights.
    c.block(18, 15, 28, 12, ix::STEEL_DARK, ix::OUTLINE);
    for x in (19..45).step_by(4) {
        c.rect(x, 17, 2, 2, ix::GOLD);
    }
    c.rect(19, 20, 26, 1, ix::STEEL);
    c.block(24, 23, 16, 4, ix::STEEL, ix::OUTLINE);
    c.rect(31, 24, 2, 2, ix::STEEL_DARK);
    // Hull, violet trim and cockpit.
    c.blob(32, 9, 29, 7, ix::STEEL);
    c.ellipse(32, 6, 24, 3, ix::STEEL_LIGHT);
    c.rect(5, 11, 54, 2, ix::PURPLE);
    c.rect(5, 13, 54, 1, ix::PURPLE_DARK);
    c.block(6, 5, 12, 5, ix::CYAN, ix::OUTLINE);
    c.rect(7, 6, 4, 1, ix::WHITE);
    // Half of a diamond insignia on the centre line, and a nav light.
    for (dy, (x0, w)) in [(30, 2), (29, 3), (30, 2), (31, 1)].into_iter().enumerate() {
        c.rect(x0, 6 + dy as i32, w, 1, if dy == 0 { ix::WHITE } else { ix::CYAN });
    }
    c.px(3, 9, ix::GOLD);
    c.vram()
}
