//! The arrival on CryptoTerra: CL207, a white cat in a yellow cat-eared hazmat suit,
//! and the cargo ship that drops the wassie off.

use agb::display::{
    Palette16,
    object::{Size, SpriteVram},
};

use super::{Canvas, ix, rgb};

/// CL207's palette, picked from `.claude/skills/diamond-hands-gba/sprite-sheet-CL207.png`
/// and the avatar beside it.
static PALETTE: Palette16 = Palette16::new([
    rgb(0, 0, 0),
    rgb(2, 1, 4),
    rgb(31, 31, 31),
    rgb(31, 26, 5),
    rgb(31, 30, 20),
    rgb(30, 17, 4),
    rgb(30, 10, 11),
    rgb(19, 7, 21),
    rgb(25, 16, 31),
    rgb(12, 5, 22),
    rgb(10, 7, 15),
    rgb(31, 13, 21),
    rgb(30, 5, 20),
    rgb(25, 25, 27),
    rgb(18, 17, 21),
    rgb(10, 9, 13),
]);

/// Characters in CL207's drawings, as [`PALETTE`] indices.
const KEY: &[(u8, u8)] = &[
    (b'k', 1),  // outline
    (b'w', 2),  // white fur, boots
    (b'y', 3),  // hazmat yellow
    (b'Y', 4),  // hood highlight
    (b'o', 5),  // yellow in shadow
    (b'r', 6),  // inner ears
    (b'v', 7),  // fringe
    (b'l', 8),  // lower iris
    (b'e', 9),  // iris
    (b'V', 10), // collar
    (b'p', 11), // cheeks, tank light
    (b'c', 12), // tank stripe
    (b'S', 13), // steel light
    (b'g', 14), // steel
    (b'G', 15), // steel dark
];

/// CL207's head, facing right: a yellow cat-eared hood with pink inner ears, a purple
/// fringe over a white face, violet eyes and pink cheeks.
const HEAD: [&str; 17] = [
    "...........kk.........kk...",
    "..........kyyk.......kyyk..",
    "..........kyrrk.....kyryk..",
    ".........kyrrryk...kyrryk..",
    ".........kyrryyykkkyyyryk..",
    "........kyyyyyyyYYYyyyyyk..",
    "........kyyyyyyYYyyyyyyyyk.",
    "........kyyyykkkkkkkkkkkyk.",
    ".......kyyyykvvvvvwvvvvvkyk",
    ".......kyyykvvvkvwwkvvvvkyk",
    ".......kyyykvkwwwwwwwkkvkyk",
    ".......koyykwkkkwwwwkkkkkyk",
    ".......koyykwkelkwwwkelkkyk",
    ".......koyykwklkwwwwklkwkyk",
    ".......koyykpwwwwwkwwwwpkk.",
    "........koyykwwwwkwkwwwkk..",
    "........kooyykkwwwwwwkkyk..",
];

/// The suit below the head, standing: white tail, air tank on the back, a violet
/// collar with the round respirator, yellow legs and white boots.
const BODY_STAND: [&str; 12] = [
    "....kk...kkkVVVVkkSSSkVVk..",
    "...kwwk.kSgkyyyykSGGSkyk...",
    "...kwwk.kgckyyyyykSSkyyyk..",
    "....kwwkkgckyyyyyykkyyyyk..",
    "....kwwkkgckoyyyyyyyyykyk..",
    ".....kwwkSSkoyyyyyyyyykwk..",
    "......kwkkkkooyyyyyyyykk...",
    ".......kk..kkkkkkkkkkkk....",
    "............koyyk.koyyk....",
    "............koyyk.koyyk....",
    "............kwwSk.kwwSk....",
    "............kkkkk.kkkkk....",
];

/// Mid-stride: legs apart, tail swung back.
const BODY_STRIDE: [&str; 12] = [
    ".........kkkVVVVkkSSSkVVk..",
    "..kk....kSgkyyyykSGGSkyk...",
    ".kwwk...kgckyyyyykSSkyyyk..",
    ".kwwkk..kgckyyyyyykkyyyyk..",
    "..kwwwk.kgckoyyyyyyyyykyk..",
    "...kwwwkkSSkoyyyyyyyyykwk..",
    "....kkwkkkkkooyyyyyyyykk...",
    "......kk...kkkkkkkkkkkk....",
    "..........koyyk....koyyk...",
    ".........koyyk......kyyk...",
    "........kwwSk......kwwSk...",
    "........kkkk.......kkkkk...",
];

/// Legs passing under the body.
const BODY_PASS: [&str; 12] = [
    "....kk...kkkVVVVkkSSSkVVk..",
    "...kwwk.kSgkyyyykSGGSkyk...",
    "...kwwk.kgckyyyyykSSkyyyk..",
    "...kwwk.kgckyyyyyykkyyyyk..",
    "....kwwkkgckoyyyyyyyyykyk..",
    ".....kwwkSSkoyyyyyyyyykwk..",
    "......kwkkkkooyyyyyyyykk...",
    ".......kk..kkkkkkkkkkkk....",
    "..............koyyyk.......",
    "..............kyykyk.......",
    ".............kwwSkwSk......",
    ".............kkkkkkkk......",
];

/// The dialogue portrait, traced from `.claude/skills/diamond-hands-gba/avatar-CL207.png`
/// at 32x32 and mirrored to face left, towards the text.
const PORTRAIT: [&str; 32] = [
    "......kk..................kk....",
    ".....kyykk..............kkyyk...",
    ".....kyyyykk..........kkyyyyk...",
    "....kyrryyyykk......kkyYYyyryk..",
    "....kyrrryyyyykkkkkkyYYyyyrryk..",
    "...kyrrrryyyYYyyyyyyyyyyyrrryk..",
    "...kyrrryyyYYyyyyyyyyyyyyrrryk..",
    "...kyrryyyyyyyyyyyyyyyyyykrryk..",
    "..kyyykkkkkkkkkkkyyyyyyyyyykyk..",
    "..kykvvvvvvvwvvvvkkyyyyyyyyyyk..",
    ".kykvvvvvvvkwkvvvvvkkyyyyyyyyk..",
    ".kkvvvvvvvkwwwkvvvvvvkoyyyyyyyk.",
    ".kvvvvkkvkwwwwwkvvvvvkooyyyyyyk.",
    ".kvvkkwwkwwwwwwwkkkvvkooyyyyyyk.",
    ".kkkwwwwwwwwwwwwwwwkvkkooyyyyyk.",
    ".kwkkkkwwwwwwwwkkkkkwkkooyyyyyk.",
    ".kwkeeYkwwwwwwkeeeYkwwkyooyyyyk.",
    ".kwkeelkwwwwwwkeeellkwkyooyyyyk.",
    ".kwwkllkwwwwwwkklllkwwkyyooyyyk.",
    ".kwwwkkwwwwwwwwwkkkwwwkkyyyyyyk.",
    ".kwppwwwwkkkwwwwwwppppwkVVVVVVk.",
    ".kwppwwwwwkwwwwwwwwppwwkVVVVVVk.",
    "..kwwwwwkwkwkwwwwwwwwwkyykkkkkk.",
    "..kkwwwwwkwkwwwwwwwwwkyyyyyoook.",
    "....kkwwwwwwwwwwwwwkkyyyyooook..",
    ".....kkkkkSSSSkkkkkyyyyoooookk..",
    "......kkSSgggSSkkyyyooookkk.....",
    "......kSggGGGggSkyooookkkkkkkk..",
    "......kSgGGGGGgSkkkkkkkyykSSSk..",
    "......kSggGGGggSkVVVVkyyyyykpgSk",
    ".....kkkSSgggSSkVVVVVkyyyyykpgSk",
    "....kyyykkSSSkkkkkkkkyyyyyykpgSk",
];

pub struct ArrivalArt {
    /// CL207's top half, 32x16, facing right: the hood and face, the same in every
    /// pose. [`Self::guide_legs`] goes underneath; together they make a 32x32 figure
    /// with its feet on the bottom row. Split so three poses cost what two whole
    /// frames would: sprite VRAM has no room for more.
    pub guide_top: SpriteVram,
    /// The bottom half standing, then the two walking frames.
    pub guide_legs: [SpriteVram; 3],
    /// CL207's face for the dialogue box, facing left.
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
        // The figure is 27 wide and 29 tall: centred in 32x32 with its feet at the
        // bottom, and cut into halves that the canvas clips.
        let (x, y) = (2, 3);
        let half = |top: i32, body: Option<&[&str; 12]>| {
            let mut c = Canvas::new(Size::S32x16);
            c.art_with(KEY, x, y - top, &HEAD);
            if let Some(body) = body {
                c.art_with(KEY, x, y - top + HEAD.len() as i32, body);
            }
            c.vram_with(&PALETTE)
        };
        Self {
            guide_top: half(0, None),
            guide_legs: [&BODY_STAND, &BODY_STRIDE, &BODY_PASS].map(|body| half(16, Some(body))),
            portrait: {
                let mut c = Canvas::new(Size::S32x32);
                c.art_with(KEY, 0, 0, &PORTRAIT);
                c.vram_with(&PALETTE)
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
