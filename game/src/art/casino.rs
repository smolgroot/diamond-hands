//! The Solana Trenches' sprites: SAM, his cucumbers and his Market Crash, the penguin
//! cashier, gold coins, CL207's plasma cells, the elevator and its terminal, and the
//! casino's bots. They are loaded only while the Trenches are on screen (see
//! `render::LevelArt`): sprite VRAM can't hold them and BERA 3000 at once. Even alone
//! they don't all fit, so they come in two halves swapped as the camera goes: the
//! floors' ([`FloorArt`]) and the arena's ([`ArenaArt`]). Only the coins stay loaded.

use core::cell::RefCell;

use agb::{
    ExternalAllocator,
    display::{
        Palette16,
        object::{DynamicSprite16, Size, SpriteVram},
    },
};

use super::{Canvas, LazyPoses, POSE_BYTES, drawing, ix, rgb};

/// The colours of SAM's cucumbers and attack effects. His body has its own palette,
/// picked from the sprite sheet with his poses (see [`SamPose`]).
pub static SAM_PALETTE: Palette16 = Palette16::new([
    rgb(0, 0, 0),
    rgb(3, 1, 6),
    rgb(30, 30, 31),
    rgb(20, 19, 27),
    rgb(5, 14, 4),
    rgb(31, 26, 21),
    rgb(28, 19, 14),
    rgb(19, 11, 10),
    rgb(22, 12, 29),
    rgb(13, 6, 21),
    rgb(7, 3, 12),
    rgb(5, 5, 13),
    rgb(6, 27, 30),
    rgb(31, 7, 22),
    rgb(31, 21, 30),
    rgb(13, 26, 6),
]);

/// The [`SAM_PALETTE`] indices the effects use.
mod sx {
    pub const OUTLINE: u8 = 1;
    pub const WHITE: u8 = 2;
    pub const CUKE_DARK: u8 = 4;
    pub const SKIN_LIGHT: u8 = 5;
    pub const CYAN: u8 = 12;
    pub const CUKE: u8 = 15;
}

/// SAM's poses, cut from `.claude/skills/diamond-hands-gba/sprite-sheet-SBF.png` by
/// `scripts/make-sam.py`: 64x64 frames, facing right, feet on row 63 and his body on
/// column 32, as 4-bit tiles in this order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SamPose {
    /// Standing, breathing in and out.
    Idle0,
    Idle1,
    Walk0,
    Walk1,
    Walk2,
    Walk3,
    /// A cucumber in hand, ready.
    Hold,
    /// The cucumber raised behind his head.
    Raise,
    /// Arm flung out as it leaves his hand.
    Throw,
    /// Crouched in his crackling cyan aura: the Cucumber Hit's tell.
    Charge,
    /// The Cucumber Hit itself, fist out, aura still on.
    Punch,
    /// Hit: wincing, mouth open.
    Ouch,
    /// Winded and doubled over.
    Dazed,
    /// Beaten: shocked, then sinking to his knees, then face down with steam rising.
    Shock,
    Kneel,
    Down,
}

impl SamPose {
    pub const WALK: [SamPose; 4] = [SamPose::Walk0, SamPose::Walk1, SamPose::Walk2, SamPose::Walk3];
}

const SAM_FRAME_BYTES: usize = 64 * 64 / 2;
static SAM_TILES: &[u8; 16 * SAM_FRAME_BYTES] = include_bytes!("../../gfx/sam.tiles");

static SAM_BODY_PALETTE: Palette16 = super::gfx_palette(include_bytes!("../../gfx/sam.pal"));
static SAM_PORTRAIT_PALETTE: Palette16 = super::gfx_palette(include_bytes!("../../gfx/sam-portrait.pal"));
/// The dialogue portrait, from the card in the sheet's corner. Colour indices, row by row.
static SAM_PORTRAIT: &[u8; 1024] = include_bytes!("../../gfx/sam-portrait.idx");

/// SAM. Only the pose on screen sits in sprite VRAM: all of them would take 32 KB, more
/// than there is. Changing pose copies the new one in from ROM.
pub struct Sam {
    shown: RefCell<Option<(SamPose, SpriteVram)>>,
    pub portrait: SpriteVram,
}

impl Sam {
    pub fn new() -> Self {
        let mut c = Canvas::new(Size::S32x32);
        for (i, &colour) in SAM_PORTRAIT.iter().enumerate() {
            c.px(i as i32 % 32, i as i32 / 32, colour);
        }
        Self { shown: RefCell::new(None), portrait: c.vram_with(&SAM_PORTRAIT_PALETTE) }
    }

    /// The 64x64 sprite for `pose`, loading it if it isn't the one already loaded.
    pub fn sprite(&self, pose: SamPose) -> SpriteVram {
        let mut shown = self.shown.borrow_mut();
        if let Some((loaded, sprite)) = shown.as_ref()
            && *loaded == pose
        {
            return sprite.clone();
        }
        // Let the old pose go first, so the two never need VRAM at once.
        *shown = None;
        let at = pose as usize * SAM_FRAME_BYTES;
        let bytes = &SAM_TILES[at..at + SAM_FRAME_BYTES];
        let sprite = DynamicSprite16::from_bytes_in(Size::S64x64, bytes, ExternalAllocator).to_vram(&SAM_BODY_PALETTE);
        *shown = Some((pose, sprite.clone()));
        sprite
    }
}

/// A cucumber from (x0, y0) to (x1, y1), `r` pixels thick, with bumps and a lit side.
fn cucumber(c: &mut Canvas, (x0, y0): (i32, i32), (x1, y1): (i32, i32), r: i32) {
    let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for pass in 0..3 {
        for i in 0..=n {
            let x = x0 + (x1 - x0) * i / n;
            let y = y0 + (y1 - y0) * i / n;
            // Rounded ends.
            let taper = if i < r || i > n - r { 1 } else { 0 };
            let rr = r - taper;
            match pass {
                0 => c.ellipse(x, y, rr + 1, rr + 1, sx::OUTLINE),
                1 => c.ellipse(x, y, rr, rr, sx::CUKE_DARK),
                _ => {
                    if rr > 1 {
                        c.ellipse(x - 1, y - 1, rr - 1, rr - 1, sx::CUKE);
                    }
                }
            }
        }
    }
    // Bumps.
    for i in (2..n - 1).step_by(3) {
        let x = x0 + (x1 - x0) * i / n;
        let y = y0 + (y1 - y0) * i / n;
        c.px(x, y + r / 2, sx::CUKE_DARK);
        c.px(x + 1, y - r / 2, sx::SKIN_LIGHT);
    }
}

/// SAM's projectiles and the effects of his attacks.
pub struct SamFx {
    /// A thrown cucumber, flat then turned an eighth (flip for the rest of the spin).
    pub cucumber: [SpriteVram; 2],
    /// The Cucumber Hit's giant cucumber, 32x16, flying left with speed lines behind.
    pub mega: SpriteVram,
    /// Green burst where a cucumber lands.
    pub splat: [SpriteVram; 2],
    /// Cyan lightning crackling round him as the Cucumber Hit charges.
    pub aura: [SpriteVram; 2],
    /// A red candle falling out of the chart, and a ball of magenta energy.
    pub candle: SpriteVram,
    pub orb: SpriteVram,
}

impl SamFx {
    fn new() -> Self {
        Self {
            cucumber: [
                {
                    let mut c = Canvas::new(Size::S16x16);
                    cucumber(&mut c, (3, 8), (12, 8), 2);
                    c.vram_with(&SAM_PALETTE)
                },
                {
                    let mut c = Canvas::new(Size::S16x16);
                    cucumber(&mut c, (4, 11), (11, 4), 2);
                    c.vram_with(&SAM_PALETTE)
                },
            ],
            mega: {
                let mut c = Canvas::new(Size::S32x16);
                for (y, x0) in [(4, 22), (8, 25), (12, 23)] {
                    c.rect(x0, y, 32 - x0, 1, sx::CYAN);
                }
                cucumber(&mut c, (4, 8), (22, 8), 5);
                c.vram_with(&SAM_PALETTE)
            },
            splat: [0, 1].map(|f| {
                let mut c = Canvas::new(Size::S16x16);
                let r = 3 + f * 3;
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
                    let (x, y) = (8 + dx * r, 8 + dy * r);
                    c.ellipse(x, y, 2 - f, 2 - f, if f == 0 { sx::CUKE } else { sx::CUKE_DARK });
                }
                if f == 0 {
                    c.ellipse(8, 8, 3, 3, sx::CUKE);
                    c.ellipse(8, 8, 1, 1, sx::SKIN_LIGHT);
                }
                c.vram_with(&SAM_PALETTE)
            }),
            aura: [0, 1].map(|f| {
                let mut c = Canvas::new(Size::S8x16);
                let mut x = 3 + f;
                for y in 0..16 {
                    c.px(x, y, sx::CYAN);
                    if y % 3 == 2 {
                        x = if x > 3 { x - 2 } else { x + 2 };
                        c.px(x, y, sx::WHITE);
                    }
                }
                c.vram_with(&SAM_PALETTE)
            }),
            candle: drawing(
                Size::S8x16,
                1,
                0,
                &[
                    "..R...", "..R...", "..R...", ".kkk..", "krrrk.", "kroRk.", "kroRk.", "krrRk.", "kroRk.",
                    "krrRk.", "krrRk.", ".kkk..", "..R...", "..R...", "..R...", "......",
                ],
            ),
            orb: drawing(Size::S8x8, 1, 1, &[".ccc.", "cCwCc", "cwwwc", "cCwCc", ".ccc."]),
        }
    }
}

/// What the arena needs: SAM, his attacks, and the penguin cashier.
pub struct ArenaArt {
    pub sam: Sam,
    pub sam_fx: SamFx,
    /// The penguin cashier, also his portrait. He waddles by bobbing.
    pub penguin: SpriteVram,
}

impl ArenaArt {
    pub fn new() -> Self {
        Self { sam: Sam::new(), sam_fx: SamFx::new(), penguin: penguin() }
    }
}

/// What the street, the casino floor and the suites need.
pub struct FloorArt {
    /// A plasma cell.
    pub cell: SpriteVram,
    /// The elevator terminal, and its screen lit once it is on.
    pub terminal: SpriteVram,
    pub terminal_on: SpriteVram,
    /// The elevator car's frame, and a door's two panels (eight pixels and four), which
    /// slide away one after the other; mirrored for the right door.
    pub lift: SpriteVram,
    pub lift_door: [SpriteVram; 2],
    /// A bubble with an up arrow: press Up here.
    pub up_prompt: SpriteVram,
    /// CL207's call coming in.
    pub phone: SpriteVram,
    /// Croupier bot walking; a fan of cards in hand laid over it while dealing.
    pub dealer: [SpriteVram; 2],
    pub cards_in_hand: SpriteVram,
    /// The Guard, a ninja bot: see [`NinjaPose`].
    pub ninja: LazyPoses<NINJA_POSES>,
    /// A playing card spinning through the air.
    pub card: [SpriteVram; 2],
    /// A casino chip rolling: the Trenches' rolling mine.
    pub chip: SpriteVram,
}

/// The Trenches' sprites: the coins always, and one of the two halves.
pub struct CasinoArt {
    /// A gold coin spinning: face, turning, edge on.
    pub coin: [SpriteVram; 3],
    pub floor: Option<FloorArt>,
    pub arena: Option<ArenaArt>,
}

impl CasinoArt {
    pub fn new() -> Self {
        Self {
            coin: [&COIN0[..], &COIN1, &COIN2].map(|rows| drawing(Size::S8x8, 0, 0, rows)),
            floor: Some(FloorArt::new()),
            arena: None,
        }
    }
}

impl FloorArt {
    pub fn new() -> Self {
        Self {
            cell: plasma_cell(),
            terminal: terminal(),
            terminal_on: terminal_screen(),
            lift: lift_frame(),
            lift_door: [8, 4].map(lift_door),
            up_prompt: drawing(
                Size::S16x16,
                1,
                0,
                &[
                    ".kkkkkkkkkkk..",
                    "kwwwwwwwwwwwk.",
                    "kwwwwwkwwwwwk.",
                    "kwwwwkykwwwwk.",
                    "kwwwkyyykwwwk.",
                    "kwwkyyyyykwwk.",
                    "kwkkkyyykkkwk.",
                    "kwwwkyyykwwwk.",
                    "kwwwkyyykwwwk.",
                    "kwwwkkkkkwwwk.",
                    "kwwwwwwwwwwwk.",
                    ".kkkkkkwkkkk..",
                    "......kwk.....",
                    ".......k......",
                ],
            ),
            phone: {
                let mut c = Canvas::new(Size::S16x16);
                c.art(
                    3,
                    3,
                    &[
                        "kk......",
                        "kyk.....",
                        "kyyk....",
                        ".kyk....",
                        ".kyk....",
                        "..kyk...",
                        "..kyyk.k",
                        "...kyykyk",
                        "....kyyyk",
                        ".....kkk.",
                    ],
                );
                for (x, y) in [(11, 2), (12, 3), (13, 5), (1, 11), (2, 13)] {
                    c.px(x, y, ix::CYAN);
                }
                c.vram()
            },
            dealer: [dealer(&LEGS0), dealer(&LEGS1)],
            cards_in_hand: drawing(Size::S8x8, 0, 0, &DEALER_CARDS),
            ninja: LazyPoses::new(NINJA_TILES, &NINJA_PALETTE),
            card: [
                drawing(Size::S8x8, 1, 0, &[".kkkk.", "kwwwwk", "kwrwwk", "kwrrwk", "kwwrwk", "kwwwwk", ".kkkk."]),
                drawing(Size::S8x8, 0, 2, &[".kkkkkk.", "kwwrrwwk", "kwwwrwwk", ".kkkkkk."]),
            ],
            chip: drawing(
                Size::S16x16,
                2,
                2,
                &[
                    "...kkkkkk...",
                    "..kcwccwck..",
                    ".kwccccccwk.",
                    "kcckkkkkkcck",
                    "kwckwwwwkcwk",
                    "kcckwyywkcck",
                    "kcckwyywkcck",
                    "kwckwwwwkcwk",
                    "kcckkkkkkcck",
                    ".kwccccccwk.",
                    "..kcwccwck..",
                    "...kkkkkk...",
                ],
            ),
        }
    }
}

/// Where the dealer's cards sit on its 16x32 sprite.
pub const CARDS_AT: (i32, i32) = (0, 20);

/// The Guard's poses in `game/gfx/ninja.tiles`, cut from
/// `.claude/skills/diamond-hands-gba/lvl2-bot-ninja-sprite-sheet.png` by
/// `scripts/make-bots.py`: a hooded ninja bot with a red visor, a pink scarf and a
/// pistol. Each is 32x32 facing right, feet on the bottom row, legs centred on column
/// [`NINJA_ANCHOR`]. In the script's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NinjaPose {
    Walk0,
    Walk1,
    Walk2,
    Walk3,
    Walk4,
    Walk5,
    /// Hand up, signalling: the wind-up before a burst.
    Signal,
    /// Arm out, gun levelled.
    Point,
    /// The muzzle flame alone, at the frame's left edge: drawn [`NINJA_FLAME_X`] right
    /// of a [`NinjaPose::Point`].
    Flame,
    Hit,
    /// Beaten: rocking back, on its knees, face down.
    Stagger,
    Kneel,
    Down,
}

impl NinjaPose {
    pub const WALK: [NinjaPose; 6] =
        [NinjaPose::Walk0, NinjaPose::Walk1, NinjaPose::Walk2, NinjaPose::Walk3, NinjaPose::Walk4, NinjaPose::Walk5];
}

pub const NINJA_POSES: usize = 13;
/// The column of a pose the hitbox's centre sits on (facing right).
pub const NINJA_ANCHOR: i32 = 12;
/// How far right of a pose's left edge its muzzle flame starts (the script prints it).
pub const NINJA_FLAME_X: i32 = 30;
static NINJA_TILES: &[u8; NINJA_POSES * POSE_BYTES] = include_bytes!("../../gfx/ninja.tiles");
static NINJA_PALETTE: Palette16 = super::gfx_palette(include_bytes!("../../gfx/ninja.pal"));

const COIN0: [&str; 8] = [
    "..kkkk..",
    ".kyyyok.",
    "kyywyyok",
    "kyyoyyok",
    "kyyyoyok",
    "kyywyyok",
    ".kooook.",
    "..kkkk..",
];
const COIN1: [&str; 8] = [
    "...kk...",
    "..kyok..",
    "..kwyok.",
    "..kyyok.",
    "..kyyok.",
    "..kyyok.",
    "..kooo..",
    "...kk...",
];
const COIN2: [&str; 8] = [
    "...kk...",
    "...ko...",
    "...ky...",
    "...ky...",
    "...ky...",
    "...ky...",
    "...ko...",
    "...kk...",
];

/// The penguin cashier, 32x32, after the round, cosy look of Pudgy Penguins: a chubby
/// pale-blue body, a big white face and belly, a little orange beak and feet, rosy
/// cheeks, and a cashier's gold-trimmed visor and bow tie.
fn penguin() -> SpriteVram {
    let mut c = Canvas::new(Size::S32x32);
    let lean = 0;
    for x in [10, 18] {
        c.block(x, 29, 6, 3, ix::ORANGE, ix::OUTLINE);
    }
    // Body: one big rounded blob, darker blue down the back.
    c.ellipse(16 + lean, 18, 12, 12, ix::OUTLINE);
    c.ellipse(16 + lean, 18, 11, 11, ix::BLUE);
    c.ellipse(15 + lean, 17, 10, 10, ix::CYAN);
    // White face and belly.
    c.ellipse(15 + lean, 21, 7, 8, ix::WHITE);
    c.ellipse(15 + lean, 14, 7, 5, ix::WHITE);
    // Flippers.
    c.ellipse(4 + lean, 20, 2, 5, ix::OUTLINE);
    c.ellipse(4 + lean, 20, 1, 4, ix::BLUE);
    c.ellipse(27 + lean, 20, 2, 5, ix::OUTLINE);
    c.ellipse(27 + lean, 20, 1, 4, ix::BLUE);
    // Eyes, cheeks and beak.
    let ex = 15 + lean;
    for x in [ex - 4, ex + 3] {
        c.rect(x, 12, 2, 3, ix::OUTLINE);
        c.px(x, 12, ix::WHITE);
    }
    c.rect(ex - 7, 15, 2, 1, ix::LASER_LIGHT);
    c.rect(ex + 6, 15, 2, 1, ix::LASER_LIGHT);
    c.rect(ex - 1, 15, 3, 2, ix::ORANGE);
    c.px(ex, 17, ix::ORANGE);
    c.rect(ex - 1, 15, 3, 1, ix::GOLD);
    // The cashier's visor and bow tie.
    c.rect(ex - 8, 6, 17, 2, ix::OUTLINE);
    c.rect(ex - 7, 6, 15, 1, ix::PURPLE);
    c.rect(ex - 9, 8, 12, 1, ix::GOLD);
    c.rect(ex - 9, 9, 10, 1, ix::OUTLINE);
    for (x, d) in [(ex - 3, 0), (ex + 2, 1)] {
        c.rect(x, 20, 2, 3, ix::RED);
        c.px(x + d, 21, ix::RED_DARK);
    }
    c.rect(ex - 1, 20, 2, 3, ix::RED_DARK);
    c.vram()
}

/// A plasma cell: steel end caps on a glass tube of cyan plasma with a white-hot core.
fn plasma_cell() -> SpriteVram {
    let flare = false;
    let mut c = Canvas::new(Size::S16x16);
    c.block(4, 1, 8, 3, ix::STEEL_LIGHT, ix::OUTLINE);
    c.block(4, 12, 8, 3, ix::STEEL, ix::OUTLINE);
    c.block(5, 3, 6, 10, ix::BLUE, ix::OUTLINE);
    c.rect(6, 4, 4, 8, ix::CYAN);
    c.rect(7, 5, 2, if flare { 6 } else { 4 }, ix::WHITE);
    c.px(6, 4, ix::WHITE);
    if flare {
        for (x, y) in [(2, 5), (13, 9), (1, 10), (14, 4)] {
            c.px(x, y, ix::CYAN);
        }
    }
    c.vram()
}

/// The elevator's access terminal, 16x32 with its foot on the bottom row: a steel
/// kiosk with a screen, a status lamp and a keypad.
fn terminal() -> SpriteVram {
    let on = false;
    let mut c = Canvas::new(Size::S16x32);
    c.pod(2, 10, 12, 22, ix::STEEL_LIGHT, ix::STEEL, ix::STEEL_DARK);
    c.block(3, 12, 10, 7, if on { ix::CYAN } else { ix::PURPLE_DARK }, ix::OUTLINE);
    if on {
        // An up arrow on the screen.
        c.px(8, 13, ix::WHITE);
        c.rect(7, 14, 3, 1, ix::WHITE);
        c.rect(8, 15, 1, 3, ix::WHITE);
    } else {
        c.line(6, 14, 9, 17, ix::RED);
        c.line(9, 14, 6, 17, ix::RED);
    }
    for y in [21, 24] {
        for x in [4, 7, 10] {
            c.rect(x, y, 2, 2, ix::STEEL_DARK);
        }
    }
    // The lamp on a stalk.
    c.rect(7, 5, 2, 5, ix::OUTLINE);
    c.block(6, 2, 4, 4, if on { ix::GOLD } else { ix::RED }, ix::OUTLINE);
    c.rect(2, 31, 12, 1, ix::OUTLINE);
    c.vram()
}

/// The terminal's screen and lamp once it is on, laid over [`terminal`]: cyan with an
/// up arrow, and a gold lamp. 16x16, over the sprite's top half.
fn terminal_screen() -> SpriteVram {
    let mut c = Canvas::new(Size::S16x16);
    c.block(3, 12, 10, 4, ix::CYAN, ix::OUTLINE);
    c.px(8, 13, ix::WHITE);
    c.rect(7, 14, 3, 1, ix::WHITE);
    c.rect(7, 5, 2, 5, ix::OUTLINE);
    c.block(6, 2, 4, 4, ix::GOLD, ix::OUTLINE);
    c.px(7, 3, ix::WHITE);
    c.vram()
}

/// The elevator car, 32x32: a gold frame round a dark shaft, a floor dial over it.
fn lift_frame() -> SpriteVram {
    let mut c = Canvas::new(Size::S32x32);
    c.rect(0, 0, 32, 32, ix::OUTLINE);
    c.rect(1, 1, 30, 6, ix::GOLD);
    c.rect(1, 6, 30, 1, ix::ORANGE);
    // The dial: a little arrow pointing up.
    c.block(12, 1, 8, 5, ix::PURPLE_DARK, ix::OUTLINE);
    c.px(16, 2, ix::CYAN);
    c.rect(15, 3, 3, 1, ix::CYAN);
    c.rect(2, 7, 2, 25, ix::GOLD);
    c.rect(28, 7, 2, 25, ix::GOLD);
    c.rect(2, 7, 1, 25, ix::WHITE);
    c.rect(4, 7, 24, 25, ix::PURPLE_DARK);
    c.rect(4, 7, 24, 2, ix::OUTLINE);
    c.vram()
}

/// A door panel `w` pixels wide, its edge against the right of an 8x32 sprite.
fn lift_door(w: i32) -> SpriteVram {
    let mut c = Canvas::new(Size::S8x32);
    let x = 8 - w;
    c.rect(x, 0, w, 25, ix::STEEL);
    c.rect(x, 0, 1, 25, ix::STEEL_LIGHT);
    c.rect(7, 0, 1, 25, ix::OUTLINE);
    c.rect(x, 10, w, 2, ix::GOLD);
    c.vram()
}

/// The croupier bot, facing left: a steel head with a green-tinted dealer's visor, a
/// white shirt, a purple waistcoat and a red bow tie; a fan of cards in hand.
const DEALER_BODY: [&str; 14] = [
    "....kkkkkk......",
    "...kgggggSk.....",
    "..kkkkkkkkkk....",
    "..kiiiiiiiik....",
    "...kgcgggGk.....",
    "...kgggggGk.....",
    "....kkkkkk......",
    "...kwkrrkwk.....",
    "..kvvwkkwvvk....",
    ".kSkvvwwvvkSk...",
    ".kSkvvwwvvkSk...",
    ".kgkvvyyvvkgk...",
    "..kkVVVVVVkk....",
    "...kVVVVVVk.....",
];
const DEALER_CARDS: [&str; 4] = ["kkk.", "kwrk", "kwwk", ".kk."];
const LEGS0: [&str; 6] = [
    "....kgk.kgk.....",
    "....kGk.kGk.....",
    "...kgk...kgk....",
    "...kGk...kGk....",
    "..kkkkk.kkkkk...",
    "..kkkkk.kkkkk...",
];
const LEGS1: [&str; 6] = [
    "....kgk.kgk.....",
    "...kGk...kGk....",
    "..kgk.....kgk...",
    "..kGk.....kGk...",
    ".kkkkk...kkkkk..",
    ".kkkkk...kkkkk..",
];

fn dealer(legs: &[&str]) -> SpriteVram {
    let mut c = Canvas::new(Size::S16x32);
    let top = 32 - legs.len() as i32 - DEALER_BODY.len() as i32;
    c.art(0, top, &DEALER_BODY);
    c.art(0, 32 - legs.len() as i32, legs);
    c.vram()
}

