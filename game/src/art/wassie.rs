//! The wassie, copied from the player sprite sheet
//! (`.claude/skills/diamond-hands-gba/sprite-sheet.png`) at about 3.4 sheet pixels to
//! one GBA pixel, in the sheet's three-quarter view: a tall teal bean of a head with a
//! gold rim light down the back and a pale crown, two blank, half-lidded eyes (the far
//! one peeking past the brow, no pupils: a wassie is not a clever animal), a broad
//! white bill shaded lilac underneath over a red throat; a red scarf with a gold fold;
//! a round red belly with the arm curling round it to a big green fist; two little
//! white feet and no legs to speak of; a red cape; and a lilac blaster with a teal
//! light, vents, and a separate magenta muzzle block.
//!
//! The parts are drawn in the sheet's coordinates (its idle frame is a 36x32 box, the
//! body's 32x32 sprite at its left). A body pose is layered from back to front: cape,
//! upper body (head, scarf, belly), feet. Only the cape and feet change between poses,
//! with the upper body nudged for the lean of a run, a recoil, a squash or a slump, as
//! the sheet does. The blaster and fist are a second 32x32 object, one per aim, placed
//! [`BLASTER_SHIFT`] further forward so the cannon can run past the body's frame.
//! Hovering straps on a jetpack with a flickering flame.

use agb::display::{
    Palette16, Rgb15,
    object::{Size, SpriteVram},
};
use diamond_hands_logic::{
    PlayerState,
    player::{Aim, Player},
    world::DEATH_FRAMES,
};

use super::{Canvas, rgb};

/// Sprite column of the hitbox centre when facing right; mirrored when facing left.
pub const CENTRE_COL: i32 = 14;
/// The blaster's sprite sits this far forward of the body's.
pub const BLASTER_SHIFT: i32 = 8;
/// Blasters, top-left in the blaster's sprite. Forward is where the sheet holds it; up
/// and down are the same drawing turned. The muzzles in `logic/src/player.rs` must
/// match.
const BLASTER_AT: (i32, i32) = (17 - BLASTER_SHIFT, 17);
const BLASTER_UP_AT: (i32, i32) = (24 - BLASTER_SHIFT, 0);
const BLASTER_DOWN_AT: (i32, i32) = (21 - BLASTER_SHIFT, 13);
/// Where the barrel axis crosses the fist, for the diagonal aims.
const GRIP_UP_FORWARD: (i32, i32) = (20 - BLASTER_SHIFT, 25);
const GRIP_DOWN_FORWARD: (i32, i32) = (19 - BLASTER_SHIFT, 20);
/// Jetpack and its flame, top-left in the 32x32 frame facing right.
pub const JETPACK_AT: (i32, i32) = (1, 15);
pub const FLAME_AT: (i32, i32) = (1, 29);
/// Frames of the knockout spent sagging before it keels over.
const SLUMP_FRAMES: u16 = 14;
/// Recoil: how far the cannon and the upper body are pushed back.
const RECOIL_BLASTER: i32 = 2;
const RECOIL_BODY: i32 = 1;

/// Colours picked from the sheet.
/// The dialogue portrait: the one in the top-left of the player sprite sheet, fitted
/// to 32x32 and 15 colours by `scripts/make-portrait.py`. Colour indices, row by row.
static PORTRAIT: &[u8; 1024] = include_bytes!("../../gfx/wassie-portrait.idx");

/// The portrait's own palette, read from the script's little-endian BGR555 colours.
static PORTRAIT_PALETTE: Palette16 = {
    let raw: &[u8; 32] = include_bytes!("../../gfx/wassie-portrait.pal");
    let mut colours = [Rgb15::new(0); 16];
    let mut i = 0;
    while i < 16 {
        colours[i] = Rgb15::new(u16::from_le_bytes([raw[2 * i], raw[2 * i + 1]]));
        i += 1;
    }
    Palette16::new(colours)
};

pub static PALETTE: Palette16 = Palette16::new([
    rgb(0, 0, 0),
    rgb(1, 1, 3),
    rgb(31, 31, 31),
    rgb(22, 21, 27),
    rgb(5, 24, 19),
    rgb(2, 19, 15),
    rgb(1, 12, 10),
    rgb(31, 9, 12),
    rgb(27, 4, 7),
    rgb(16, 1, 4),
    rgb(31, 27, 5),
    rgb(19, 29, 27),
    rgb(24, 22, 28),
    rgb(16, 15, 21),
    rgb(31, 19, 29),
    rgb(31, 6, 22),
]);

/// Characters in the drawings below: outline, white, bill shade, three teals, three
/// reds, gold, pale crown highlight, two lilacs, laser pink and magenta.
const KEY: &[(u8, u8)] = &[
    (b'k', 1),
    (b'w', 2),
    (b'b', 3),
    (b'T', 4),
    (b't', 5),
    (b'd', 6),
    (b'P', 7),
    (b'p', 8),
    (b'q', 9),
    (b'y', 10),
    (b'c', 11),
    (b'S', 12),
    (b's', 13),
    (b'C', 14),
    (b'm', 15),
];

/// Head, throat, scarf and belly, from the top of the sheet's frame. The near eye is a
/// white slit under a heavy black lid; the far one peeks past the brow. The arm comes
/// round the belly, green like the head, to the fist on the blaster.
const UPPER: [&str; 28] = [
    "..............kkkkk",
    "............kkcccccTk",
    "...........kyyTTtttcck",
    "..........kyyTTTttttTck",
    ".........kyydttttttttTck",
    ".........kyydttkkkkkkttk",
    "........kyydttkkkkkkkktk.kkk",
    "........kyydttkwwwwwwktkkwwk",
    "........kyydtttkkkkkkkkkkkkk",
    "........kyttttttttkkwwwwwkk",
    "........kyttttttkkkwwwwwwwwk",
    "........kyttttttkkwwwwwwwwwwwk",
    "........kyttttttkkbwwwwwwwwbbk",
    "........kdtttttpkkkkbbbbbbbkk",
    "........kddttpppppkkkkkkkk",
    "........kkkkPPpppppqk",
    ".......kkPppPyyPppqk",
    "......kPPppyyyyyppqk",
    ".....kPPpqqyyyyqqk",
    "........kPpppppkTtk",
    "........kPppppkTttk",
    "........kPpppkkTttk",
    "........kPppppkTtttk",
    "........kPpppppkTtttk",
    "........kqppppppkdttt",
    "........kqqpppppqkdtt",
    ".........kqqqqqqqkkkk",
    "..........kkkkkkk",
];
/// Both eyes squeezed shut, replacing upper rows 5..8, for hurt and knocked-out poses.
const EYES_SHUT: [&str; 3] = [
    ".........kyydttttttttttk",
    "........kyydtttttttttttk",
    "........kyydttkkkkkkkttkkkkk",
];
const EYES_ROW: usize = 5;

/// The upper body with the eyes open or shut.
fn upper(eyes_shut: bool) -> [&'static str; UPPER.len()] {
    let mut rows = UPPER;
    if eyes_shut {
        rows[EYES_ROW..EYES_ROW + EYES_SHUT.len()].copy_from_slice(&EYES_SHUT);
    }
    rows
}

/// A drawing and its top-left in the sheet's frame.
type Part = (i32, i32, &'static [&'static str]);

/// The cape hangs behind at rest, streams out flat while running (three frames),
/// trails down behind on the way up and flies up on the way down.
const CAPE_HANG: Part = (
    0,
    17,
    &[
        "......kk.",
        ".....kPPk",
        "....kPPpk",
        "...kPPppk",
        "..kPPPppk",
        ".kPPPpppk",
        "kPPPppqqk",
        "kPpPPpqk.",
        "kkkPPqk..",
        "..kPPk...",
        "..kPqk...",
        "...kk....",
    ],
);
/// Standing still, the cape keeps moving: blown out, then flared up with its tips
/// flicking, in time with the breathing.
const CAPE_SWAY: [Part; 2] = [
    (
        0,
        17,
        &[
            "......kk.",
            ".....kPPk",
            "...kkPPpk",
            "..kPPPppk",
            ".kPPPPppk",
            "kPPPPpppk",
            "kPPqPppqk",
            "kPPPqPqk.",
            ".kkPPqqk.",
            "...kkPPk.",
            ".....kPk.",
            "......k..",
        ],
    ),
    (
        0,
        17,
        &[
            "......kk.",
            "..kk.kPPk",
            ".kPPkPPpk",
            "kPPPPPppk",
            "kPPqPPppk",
            ".kPPqPppk",
            "..kPPqpqk",
            "...kPPqk.",
            "....kPqk.",
            ".....kk..",
        ],
    ),
];
const CAPE_RUN: [Part; 3] = [
    (
        -1,
        14,
        &[
            "...kkk..........",
            "..kPPPkk........",
            ".kPPPPPPkkk.....",
            "kPPPPPPPPPPkkk..",
            ".kkPPPPPPPPPPPk.",
            "..kPPpppPPPPPPk.",
            ".kPPpqqppppPPk..",
            "kPPkkkqqpppk....",
            ".kk...kkkkk.....",
        ],
    ),
    (
        -1,
        14,
        &[
            "................",
            "kkk.............",
            "kPPkkk..........",
            ".kPPPPkkkkk.....",
            "..kPPPPPPPPkkk..",
            "kkPPPPPPPPPPPPk.",
            "kPPPqqPPPPPPPPk.",
            ".kkkkkqqppPPk...",
            "......kkkkkk....",
        ],
    ),
    (
        -1,
        14,
        &[
            "................",
            ".kk.............",
            "kPPkkkkk........",
            "kPPPPPPPkkkk....",
            ".kkPPPPPPPPPkkk.",
            "..kPPPPPPPPPPPk.",
            ".kPPqqqpPPPPk...",
            "kPkkkkqqpPk.....",
            ".k....kkkk......",
        ],
    ),
];
const CAPE_JUMP: Part = (
    -1,
    16,
    &[
        "...........kPP",
        ".........kkPPk",
        "......kkkPPPk.",
        "...kkkPPPPPk..",
        ".kkPPPPPPPk...",
        "kPPPqPPPPk....",
        ".kPPkqPPk.....",
        "..kPkkPk......",
        "...k..k.......",
    ],
);
const CAPE_FALL: Part = (
    0,
    1,
    &[
        ".kk..........",
        "kPPk.........",
        "kPPPk........",
        ".kPPPk.......",
        ".kPPPPk......",
        "..kPPPPk.....",
        "..kPqPPPk....",
        "...kqPPPPk...",
        "...kqqPPPPk..",
        "....kqPPPPk..",
        ".....kqPPPk..",
        "......kPPk...",
        ".......kk....",
    ],
);

/// Feet: two white feet with a lilac top, apart at rest, striding (both ways) and
/// passing while running, tucked up on the way up, dangling on the way down, and
/// spread flat in the landing squash.
const FEET_STAND: Part = (
    10,
    28,
    &[
        ".kbbbk...kbbbk.",
        "kwwwwbk.kbwwwwk",
        "kwwwwwk.kwwwwwk",
        ".kkkkk...kkkkk.",
    ],
);
const FEET_STRIDE: Part = (
    8,
    28,
    &[
        "kk........kbbbk.",
        "kwwk.....kbwwwwk",
        ".kwwk....kwwwwwk",
        "..kk......kkkkk.",
    ],
);
const FEET_STRIDE_BACK: Part = (
    9,
    28,
    &[
        ".kbbbk.....kk..",
        "kwwwwbk...kwwk.",
        "kwwwwwk..kwwk..",
        ".kkkkk....kk...",
    ],
);
const FEET_PASS: Part = (
    12,
    28,
    &[
        ".kbbbk.",
        "kwwwwbk",
        "kwwwwwk",
        ".kkkkk.",
    ],
);
const FEET_TUCK: Part = (
    11,
    25,
    &[
        ".kbbk..kbbk.",
        "kwwwk.kwwwwk",
        ".kkk...kkkk.",
    ],
);
const FEET_DANGLE: Part = (
    11,
    28,
    &[
        "kbk....kbk",
        "kwk....kwk",
        "kwwk...kwwk",
        ".kk.....kk.",
    ],
);
const FEET_SQUAT: Part = (
    8,
    29,
    &[
        ".kbbbk.....kbbbk.",
        "kwwwwwk...kwwwwwk",
        ".kkkkkk...kkkkkk.",
    ],
);

/// The blaster facing right: a lilac body with a white top edge, a teal light and
/// three vents, then a gap and the magenta muzzle block with its white-hot core.
const BLASTER: [&str; 8] = [
    "kkkkkkkkkkkkk.kkkkk",
    "kwwwwwwwwwwwk.kmmmk",
    "kSTTSSSSSSSSk.kmwmk",
    "kTcTSSkSkSkSkkkmwmk",
    "kSTTSSSSSSSSk.kmmmk",
    "ksssssssssssk.kmmmk",
    "kkkkkkkkkkkkk.kSSSk",
    "..............kkkkk",
];
/// The big green fist under the blaster and the other hand at its side, relative to
/// the blaster.
const HANDS: [(i32, i32, &[&str]); 2] = [
    (3, 4, &[".kkkk.", "kTTTtk", "kTcttk", "kTttdk", "ktdddk", ".kkkk."]),
    (8, 6, &[".kk", "kttk", "kdk"]),
];

/// Upper-body offsets for each pose.
mod lean {
    pub const NONE: (i32, i32) = (0, 0);
    /// Breathing out: everything above the feet one pixel lower.
    pub const SETTLED: (i32, i32) = (0, 1);
    /// Running: head and belly pitched forward and down.
    pub const RUN: (i32, i32) = (1, 1);
    /// Mid-stride, bobbing back up.
    pub const PASS: (i32, i32) = (1, 0);
    /// Touching down hard: squashed into the feet.
    pub const SQUASH: (i32, i32) = (0, 2);
    /// Skidding: leaning back on the heels.
    pub const BRAKE: (i32, i32) = (-1, 1);
    /// Hit: rocked back.
    pub const HURT: (i32, i32) = (-1, 0);
    /// Knocked out: sagging before it keels over.
    pub const SLUMP: (i32, i32) = (0, 3);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cape {
    Hang,
    Sway(usize),
    Run(usize),
    Jump,
    Fall,
}

impl Cape {
    fn part(self) -> Part {
        match self {
            Cape::Hang => CAPE_HANG,
            Cape::Sway(i) => CAPE_SWAY[i],
            Cape::Run(i) => CAPE_RUN[i],
            Cape::Jump => CAPE_JUMP,
            Cape::Fall => CAPE_FALL,
        }
    }
}

fn draw(c: &mut Canvas, (x, y, rows): Part, (dx, dy): (i32, i32)) {
    c.art_with(KEY, x + dx, y + dy, rows);
}

/// One body pose: the cape and upper body moved by `lean`, the feet planted.
fn pose(feet: Part, cape: Cape, lean: (i32, i32), eyes_shut: bool) -> SpriteVram {
    let mut c = Canvas::new(Size::S32x32);
    draw(&mut c, cape.part(), lean);
    c.art_with(KEY, lean.0, lean.1, &upper(eyes_shut));
    draw(&mut c, feet, (0, 0));
    c.vram_with(&PALETTE)
}

/// Out cold, flat on its face: the standing layers turned a quarter clockwise, so the
/// head lies at the right with the bill in the dirt and the cape over the back.
fn lying() -> SpriteVram {
    let mut c = Canvas::new(Size::S32x32);
    // Upright (x, y) lands at (31 - y, x + Y0) once turned.
    const Y0: i32 = 2;
    let mut turned = |x: i32, y: i32, rows: &[&str]| {
        c.art_turned_cw(KEY, 32 - y - rows.len() as i32, x + Y0, rows);
    };
    turned(CAPE_HANG.0, CAPE_HANG.1, CAPE_HANG.2);
    turned(0, 0, &upper(true));
    turned(FEET_STAND.0, FEET_STAND.1, FEET_STAND.2);
    c.vram_with(&PALETTE)
}

/// The blaster and hands for one aim, in the blaster's 32x32 frame.
fn blaster(aim: Aim) -> SpriteVram {
    let mut c = Canvas::new(Size::S32x32);
    match aim {
        Aim::Forward => {
            let (bx, by) = BLASTER_AT;
            c.art_with(KEY, bx, by, &BLASTER);
            for (hx, hy, hand) in HANDS {
                c.art_with(KEY, bx + hx, by + hy, hand);
            }
        }
        Aim::Up => c.art_turned(KEY, BLASTER_UP_AT.0, BLASTER_UP_AT.1, &BLASTER),
        Aim::Down => c.art_turned_cw(KEY, BLASTER_DOWN_AT.0, BLASTER_DOWN_AT.1, &BLASTER),
        Aim::UpForward => blaster_diagonal(&mut c, GRIP_UP_FORWARD, -1),
        Aim::DownForward => blaster_diagonal(&mut c, GRIP_DOWN_FORWARD, 1),
    }
    c.vram_with(&PALETTE)
}

/// Palette indices by name, for the shapes drawn in code.
mod px {
    pub const OUTLINE: u8 = 1;
    pub const WHITE: u8 = 2;
    pub const TEAL_LIGHT: u8 = 4;
    pub const TEAL: u8 = 5;
    pub const TEAL_DARK: u8 = 6;
    pub const CROWN: u8 = 11;
    pub const LILAC: u8 = 12;
    pub const LILAC_DARK: u8 = 13;
    pub const MAGENTA: u8 = 15;
}

/// The blaster at 45°, pointing forward and up (`dy` = -1) or down (`dy` = 1), drawn
/// from its shape so the outline stays one clean pixel staircase. `t` runs along the
/// barrel from the grip (two per diagonal step), `s` across it, positive on the
/// underside; the two always share parity. The parts match the side-on drawing: white
/// top edge, teal light, vents, a gap, and the magenta muzzle block with a hot core.
fn blaster_diagonal(c: &mut Canvas, (gx, gy): (i32, i32), dy: i32) {
    const T_END: i32 = 22;
    const T_BACK: i32 = -4;
    const HALF: i32 = 3;
    const FIST_T: i32 = 2;
    let colour = |t: i32, s: i32| -> Option<u8> {
        if s.abs() > HALF || !(T_BACK..=T_END).contains(&t) {
            return None;
        }
        if s.abs() == HALF || t <= T_BACK + 1 || t >= T_END - 1 {
            return Some(px::OUTLINE);
        }
        // Muzzle block beyond a dark gap.
        if t >= T_END - 8 {
            return Some(match () {
                _ if t <= T_END - 7 => px::OUTLINE,
                _ if s.abs() <= 1 && (T_END - 5..=T_END - 3).contains(&t) => px::WHITE,
                _ => px::MAGENTA,
            });
        }
        Some(match s {
            -2 => px::WHITE,
            2 => px::LILAC_DARK,
            0 if t == 0 => px::CROWN,
            _ if t.abs() + s.abs() <= 2 => px::TEAL_LIGHT,
            0 if t == 6 || t == 8 || t == 10 => px::OUTLINE,
            _ => px::LILAC,
        })
    };
    // Back from (t, s) to pixels: x = gx + (t - s * dy) / 2, y = gy + (t * dy + s) / 2.
    for y in 0..32 {
        for x in 0..32 {
            let (px_, py) = (x - gx, y - gy);
            let t = px_ + py * dy;
            let s = py - px_ * dy;
            if let Some(colour) = colour(t, s) {
                c.px(x, y, colour);
            }
        }
    }
    // The fist round the grip, under the barrel.
    let (t, s) = (FIST_T, HALF + 1);
    let (hx, hy) = (gx + (t - s * dy) / 2, gy + (t * dy + s) / 2);
    c.rect(hx - 1, hy - 1, 5, 5, px::OUTLINE);
    c.rect(hx, hy, 3, 3, px::TEAL);
    c.px(hx, hy, px::TEAL_LIGHT);
    c.px(hx + 1, hy, px::TEAL_LIGHT);
    c.px(hx + 2, hy + 2, px::TEAL_DARK);
}

/// A jetpack from the future strapped over the cape: a lilac tank with magenta energy
/// bands, a teal status light, and a nozzle glowing laser pink.
const JETPACK: [&str; 15] = [
    "..kkkk..",
    ".kSSssk.",
    ".kSSssk.",
    ".kSsssk.",
    ".kmCmmk.",
    ".kSsssk.",
    ".kSsssk.",
    ".kSTssk.",
    ".kSsssk.",
    ".kSsssk.",
    ".kmCmmk.",
    "..kssk..",
    "..kssk..",
    ".kCwwCk.",
    "..kkkk..",
];
/// Exhaust under the nozzle, the same energy as the laser: a white-hot core in pink and
/// magenta. Two flickering frames and a sputter.
const FLAMES: [&[&str]; 3] = [
    &["..ww....", ".wCCw...", ".CmmC...", "..mm....", "..m.....", "........"],
    &["..ww....", ".wwCw...", ".CCmC...", ".CmmC...", "..mm....", "...m...."],
    &["..wC....", "..Cm....", "...m....", "........", "........", "........"],
];

/// Locomotion poses, in [`Wassie::poses`] order: feet, cape and lean.
const STANCES: [(Part, Cape, (i32, i32)); 6] = [
    (FEET_STAND, Cape::Hang, lean::NONE),
    (FEET_STRIDE, Cape::Run(0), lean::RUN),
    (FEET_PASS, Cape::Run(1), lean::PASS),
    (FEET_STRIDE_BACK, Cape::Run(2), lean::RUN),
    (FEET_TUCK, Cape::Jump, lean::NONE),
    (FEET_DANGLE, Cape::Fall, lean::NONE),
];
/// The run cycle as indices into [`STANCES`].
const RUN: [usize; 4] = [1, 2, 3, 2];
/// Standing still: cape and breathing for each step of [`Wassie::idle`]'s loop, after
/// the plain stance.
const IDLE: [(Cape, (i32, i32)); 3] = [
    (Cape::Sway(0), lean::NONE),
    (Cape::Sway(1), lean::SETTLED),
    (Cape::Sway(0), lean::SETTLED),
];
/// Frames per step of the idle loop.
const IDLE_STEP: u32 = 12;

/// Every pose, built once at boot.
pub struct Wassie {
    /// Each stance ready, and recoiling from a shot.
    poses: [[SpriteVram; 2]; 6],
    /// Standing still, after the plain stance: see [`IDLE`].
    idle: [SpriteVram; 3],
    hurt: SpriteVram,
    /// Touching down hard, and skidding to a stop.
    landing: SpriteVram,
    braking: SpriteVram,
    /// Sagging, then flat out.
    knocked_out: [SpriteVram; 2],
    /// Blaster and hands per [`Aim`], in [`Aim::ALL`] order.
    blasters: [SpriteVram; 5],
    pub jetpack: SpriteVram,
    pub flames: [SpriteVram; 3],
    /// The portrait from the player sprite sheet, for the dialogue box.
    pub portrait: SpriteVram,
}

impl Wassie {
    pub fn new() -> Self {
        Self {
            poses: STANCES.map(|(feet, cape, (dx, dy))| {
                let recoil = (dx - RECOIL_BODY, dy);
                [pose(feet, cape, (dx, dy), false), pose(feet, cape, recoil, false)]
            }),
            idle: IDLE.map(|(cape, lean)| pose(FEET_STAND, cape, lean, false)),
            hurt: pose(FEET_TUCK, Cape::Run(1), lean::HURT, true),
            landing: pose(FEET_SQUAT, Cape::Hang, lean::SQUASH, false),
            braking: pose(FEET_STRIDE, Cape::Run(0), lean::BRAKE, false),
            knocked_out: [pose(FEET_SQUAT, Cape::Hang, lean::SLUMP, true), lying()],
            blasters: Aim::ALL.map(blaster),
            jetpack: {
                let mut c = Canvas::new(Size::S8x16);
                c.art_with(KEY, 0, 0, &JETPACK);
                c.vram_with(&PALETTE)
            },
            flames: FLAMES.map(|rows| {
                let mut c = Canvas::new(Size::S8x8);
                c.art_with(KEY, 0, 0, rows);
                c.vram_with(&PALETTE)
            }),
            portrait: {
                let mut c = Canvas::new(Size::S32x32);
                for (i, &colour) in PORTRAIT.iter().enumerate() {
                    c.px((i % 32) as i32, (i / 32) as i32, colour);
                }
                c.vram_with(&PORTRAIT_PALETTE)
            },
        }
    }

    /// The body pose for this frame. `anim` advances the run cycle; `frame` paces the
    /// idle loop.
    pub fn sprite(&self, player: &Player, anim: u32, frame: u32) -> &SpriteVram {
        let firing = player.recoil > 2;
        let stance = match player.state {
            PlayerState::Hurt => return &self.hurt,
            PlayerState::Dead => {
                let sagging = player.dead_timer > DEATH_FRAMES - SLUMP_FRAMES;
                return &self.knocked_out[!sagging as usize];
            }
            // Firing keeps its own recoil pose; otherwise landing and skidding win.
            PlayerState::Idle | PlayerState::Run if player.land > 0 && !firing => return &self.landing,
            PlayerState::Run if player.braking && !firing => return &self.braking,
            PlayerState::Idle => 0,
            PlayerState::Run => RUN[(anim / 6 % 4) as usize],
            PlayerState::Jump | PlayerState::Hover => 4,
            PlayerState::Fall => 5,
        };
        let poses = &self.poses[stance];
        if firing {
            &poses[1]
        } else if stance == 0 {
            self.idle(frame)
        } else {
            &poses[0]
        }
    }

    /// Standing still: the plain stance, then the cape blowing out and flaring up while
    /// the wassie breathes.
    fn idle(&self, frame: u32) -> &SpriteVram {
        match (frame / IDLE_STEP % 4) as usize {
            0 => &self.poses[0][0],
            step => &self.idle[step - 1],
        }
    }

    /// Standing at ease, blaster forward: the body and the blaster.
    /// Each with its x offset from the body's sprite, facing right.
    pub fn standing(&self, frame: u32) -> [(&SpriteVram, i32); 2] {
        [(self.idle(frame), 0), (&self.blasters[Aim::Forward as usize], BLASTER_SHIFT)]
    }

    /// The blaster for this frame and how far recoil pushes it back, facing right. Its
    /// sprite sits [`BLASTER_SHIFT`] forward of the body's.
    pub fn blaster(&self, player: &Player) -> (&SpriteVram, (i32, i32)) {
        let aim = if matches!(player.state, PlayerState::Hurt | PlayerState::Dead) {
            Aim::Forward
        } else {
            player.aim
        };
        let kick = if player.recoil > 2 {
            let (x, y) = aim.dir();
            (-x * RECOIL_BLASTER, -y * RECOIL_BLASTER)
        } else {
            (0, 0)
        };
        (&self.blasters[aim as usize], kick)
    }
}
