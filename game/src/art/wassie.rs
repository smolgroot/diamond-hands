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
/// Cannons, top-left in the blaster's sprite. Forward is held across the belly, the
/// muzzle well ahead of the bill; up and down are the same drawing turned. The muzzles
/// in `logic/src/player.rs` must match: forward, the tip is 23 px ahead of the hitbox
/// centre, level with the bill's underside.
const BLASTER_AT: (i32, i32) = (CENTRE_COL - BLASTER_SHIFT, 15);
/// Straight up, the cannon is raised over the head on the wassie's centre line, so the
/// shot leaves right above it: its core lands on [`CENTRE_COL`].
const BLASTER_UP_AT: (i32, i32) = (CENTRE_COL - GUN_CORE_ROW - BLASTER_SHIFT, 0);
/// How far the raised cannon's sprite sits above the body's, clear of the crown.
const BLASTER_UP_RAISE: i32 = 14;
/// Straight down, the core is 12 px ahead of the centre and the tip at the sprite's
/// bottom row.
const BLASTER_DOWN_AT: (i32, i32) = (CENTRE_COL + 12 - (10 - GUN_CORE_ROW) - BLASTER_SHIFT, 32 - GUN_LEN);
/// Where the barrel axis crosses the fist, for the diagonal aims.
const GRIP_UP_FORWARD: (i32, i32) = (20 - BLASTER_SHIFT, 25);
const GRIP_DOWN_FORWARD: (i32, i32) = (19 - BLASTER_SHIFT, 20);
/// Knocked out, the cannon lies on the floor ahead of the wassie: how far its sprite
/// is moved from where it is held.
const DROPPED_AT: (i32, i32) = (9, 8);
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

/// The portrait with the lower bill dropped one pixel, then two: two 1024-index frames
/// back to back, from the same script. Shown in turn while the wassie talks.
static PORTRAIT_TALK: &[u8; 2048] = include_bytes!("../../gfx/wassie-portrait-talk.idx");

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

/// Feet: two white feet with a lilac top, apart at rest, striding into a skid,
/// tucked up on the way up, dangling on the way down, and spread flat in the landing
/// squash.
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
/// Running, each foot is its own stamp: the near foot white, the far one shaded and
/// drawn behind the belly. Reaching forward heel first, flat under the body, pushing
/// off the toe behind, kicked up behind, and swinging through.
const STEP_REACH: &[&str] = &[".....kk.", "..kkkwbk", ".kwwwwwk", "..kkkkk."];
const STEP_FLAT: &[&str] = &[".kbbbk.", "kwwwwbk", "kwwwwwk", ".kkkkk."];
const STEP_PUSH: &[&str] = &["kk....", "kwk...", "kwwk..", ".kwwk.", "..kkk."];
const STEP_KICK: &[&str] = &["..kkk.", ".kwwbk", "kwwwk.", "kwwk..", ".kk..."];
const STEP_SWING: &[&str] = &[".kkkk.", "kwwwbk", ".kkkk."];
/// The far foot in shade: white to the bill's lilac shade, lilac to the dark lilac.
const FAR_KEY: &[(u8, u8)] = &[(b'k', 1), (b'w', 3), (b'b', 13)];

/// One step of the run: upper-body lean (x, and the bob down), cape frame, then the
/// near and far feet with their top-left.
struct Stride {
    lean: (i32, i32),
    cape: usize,
    near: (i32, i32, &'static [&'static str]),
    far: (i32, i32, &'static [&'static str]),
}
/// Six steps: contact, down, and up off the ground, once per foot. The body bobs
/// lowest as the planted foot passes under it and highest in the flight between steps.
const RUN: [Stride; 6] = [
    Stride { lean: (1, 1), cape: 0, near: (20, 28, STEP_REACH), far: (3, 26, STEP_PUSH) },
    Stride { lean: (1, 2), cape: 1, near: (13, 28, STEP_FLAT), far: (3, 23, STEP_KICK) },
    Stride { lean: (1, 0), cape: 2, near: (2, 22, STEP_KICK), far: (17, 26, STEP_SWING) },
    Stride { lean: (1, 1), cape: 0, near: (3, 26, STEP_PUSH), far: (20, 28, STEP_REACH) },
    Stride { lean: (1, 2), cape: 1, near: (3, 23, STEP_KICK), far: (13, 28, STEP_FLAT) },
    Stride { lean: (1, 0), cape: 2, near: (17, 26, STEP_SWING), far: (2, 22, STEP_KICK) },
];
/// Running, the upper body pitches forward: each row moves ahead by one pixel per
/// [`PITCH_STEP`] rows above [`PITCH_PIVOT`] (the bill), and back below it.
const PITCH_PIVOT: i32 = 15;
const PITCH_STEP: i32 = 5;
/// Run steps advance with travel: one per this many `anim` units (2 px each).
const RUN_STEP: u32 = 4;

fn pitch(row: i32) -> i32 {
    (PITCH_PIVOT - row).div_euclid(PITCH_STEP)
}

/// How far the arm's end moves with the pitch, so the cannon goes with it.
const ARM_ROW: i32 = 24;

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

/// The cannon has its own palette: gunmetal steels and navy, the laser's pinks and a
/// deep magenta for its energy, cyan for the diamond insignia, and the body's teals
/// for the fists that grip it.
pub static GUN_PALETTE: Palette16 = Palette16::new([
    rgb(0, 0, 0),
    rgb(1, 1, 3),
    rgb(31, 31, 31),
    rgb(22, 24, 30),
    rgb(15, 17, 25),
    rgb(9, 10, 18),
    rgb(4, 4, 10),
    rgb(5, 24, 19),
    rgb(2, 19, 15),
    rgb(1, 12, 10),
    rgb(19, 29, 27),
    rgb(31, 19, 29),
    rgb(31, 6, 22),
    rgb(18, 2, 14),
    rgb(12, 26, 31),
    rgb(31, 27, 5),
]);

/// Characters in the cannon drawings: outline, white, three steels from light to dark,
/// navy, three teals and the pale highlight (fists), laser pink, magenta, deep magenta,
/// diamond cyan and gold.
const GUN_KEY: &[(u8, u8)] = &[
    (b'k', 1),
    (b'w', 2),
    (b'L', 3),
    (b'S', 4),
    (b's', 5),
    (b'n', 6),
    (b'T', 7),
    (b't', 8),
    (b'd', 9),
    (b'c', 10),
    (b'C', 11),
    (b'm', 12),
    (b'M', 13),
    (b'D', 14),
    (b'y', 15),
];

/// The cannon facing right, back to front: a glowing magenta power cell; the
/// gunmetal housing with a carry handle and its cyan sight on top, a bevelled white
/// top edge, the diamond insignia, two magenta vents and a grip underneath; a finned
/// barrel; and the big muzzle ring with its white-hot core. The core is rows
/// [`GUN_CORE_ROW`] ± 1 and the tip is the last column.
const BLASTER: [&str; 11] = [
    "......kkkkkkk...........",
    ".....kwLLLLLDk......kkk.",
    "..kkkkSkkkkkSkk....kmCmk",
    ".kMkwwwwwwwwwwwkkkkkCwCk",
    "kmCkLLLkkkLkLkLLkLkkCwwk",
    "kCwkSSkDwDkmSmSSkSkkCwwk",
    "kmCkSSSkDkSmSmSskskkCwwk",
    ".kMkssssksskskskkkkkCwCk",
    "..kkkkkkkkkkkkk....kmCmk",
    ".......kssk.........kkk.",
    ".......kkkk.............",
];
const GUN_LEN: i32 = 24;
const GUN_CORE_ROW: i32 = 5;
/// The big green fist round the grip and the other hand under the barrel, relative
/// to the cannon.
const HANDS: [(i32, i32, &[&str]); 2] = [
    (6, 8, &[".kkkk.", "kTTTtk", "kTcttk", "kTttdk", "ktdddk", ".kkkk."]),
    (15, 7, &[".kk", "kttk", "kdk"]),
];

/// Upper-body offsets for each pose.
mod lean {
    pub const NONE: (i32, i32) = (0, 0);
    /// Breathing out: everything above the feet one pixel lower.
    pub const SETTLED: (i32, i32) = (0, 1);
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

/// One step of the run: the cape and upper body pitched forward and bobbing, the far
/// foot behind the belly and the near one in front.
fn run_pose(stride: &Stride) -> SpriteVram {
    let mut c = Canvas::new(Size::S32x32);
    let (dx, dy) = stride.lean;
    let pitched = |c: &mut Canvas, (x, y, rows): Part| {
        for (i, row) in rows.iter().enumerate() {
            let r = y + i as i32;
            c.art_with(KEY, x + dx + pitch(r), r + dy, core::slice::from_ref(row));
        }
    };
    pitched(&mut c, CAPE_RUN[stride.cape]);
    let (fx, fy, far) = stride.far;
    c.art_with(FAR_KEY, fx, fy, far);
    pitched(&mut c, (0, 0, &UPPER));
    let (nx, ny, near) = stride.near;
    c.art_with(KEY, nx, ny, near);
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

/// The cannon and hands for one aim, in the blaster's 32x32 frame.
fn blaster(aim: Aim) -> SpriteVram {
    let mut c = Canvas::new(Size::S32x32);
    match aim {
        Aim::Forward => {
            let (bx, by) = BLASTER_AT;
            c.art_with(GUN_KEY, bx, by, &BLASTER);
            for (hx, hy, hand) in HANDS {
                c.art_with(GUN_KEY, bx + hx, by + hy, hand);
            }
        }
        Aim::Up => c.art_turned(GUN_KEY, BLASTER_UP_AT.0, BLASTER_UP_AT.1, &BLASTER),
        Aim::Down => c.art_turned_cw(GUN_KEY, BLASTER_DOWN_AT.0, BLASTER_DOWN_AT.1, &BLASTER),
        Aim::UpForward => blaster_diagonal(&mut c, GRIP_UP_FORWARD, -1),
        Aim::DownForward => blaster_diagonal(&mut c, GRIP_DOWN_FORWARD, 1),
    }
    c.vram_with(&GUN_PALETTE)
}

/// The cannon let go of, lying on the floor: the forward drawing without hands, and
/// without the grip that would sink into the ground.
fn dropped() -> SpriteVram {
    let mut c = Canvas::new(Size::S32x32);
    let (bx, by) = BLASTER_AT;
    c.art_with(GUN_KEY, bx, by, &BLASTER[..9]);
    c.vram_with(&GUN_PALETTE)
}

/// [`GUN_PALETTE`] indices by name, for the cannon drawn in code.
mod px {
    pub const OUTLINE: u8 = 1;
    pub const WHITE: u8 = 2;
    pub const STEEL_LIGHT: u8 = 3;
    pub const STEEL: u8 = 4;
    pub const STEEL_DARK: u8 = 5;
    pub const NAVY: u8 = 6;
    pub const TEAL_LIGHT: u8 = 7;
    pub const TEAL: u8 = 8;
    pub const TEAL_DARK: u8 = 9;
    pub const PINK: u8 = 11;
    pub const MAGENTA: u8 = 12;
    pub const MAGENTA_DARK: u8 = 13;
    pub const DIAMOND: u8 = 14;
}

/// The cannon at 45°, pointing forward and up (`dy` = -1) or down (`dy` = 1), drawn
/// from its shape so the outline stays one clean pixel staircase. `t` runs along the
/// barrel from the grip (two per diagonal step), `s` across it, positive on the
/// underside; the two always share parity. The parts match the side-on drawing: the
/// power cell at the back, the housing with its white top edge, diamond insignia and
/// vents, a finned barrel, and the wider muzzle ring with a hot core.
fn blaster_diagonal(c: &mut Canvas, (gx, gy): (i32, i32), dy: i32) {
    const T_END: i32 = 22;
    const T_BACK: i32 = -8;
    /// Half-width across the housing, in `s` (two per diagonal pixel step). The barrel
    /// is narrower and the muzzle ring wider, as in the side-on drawing.
    const HALF: i32 = 7;
    /// The barrel between the housing and the muzzle is narrower.
    const BARREL: (i32, i32) = (10, 15);
    const MUZZLE: i32 = 16;
    const INSIGNIA_T: i32 = 2;
    const FIST_T: i32 = 0;
    let colour = |t: i32, s: i32| -> Option<u8> {
        if !(T_BACK..=T_END).contains(&t) {
            return None;
        }
        // The muzzle ring stands a pixel proud of the housing.
        if t >= MUZZLE {
            let half = HALF + 1;
            return match s.abs() {
                a if a > half => None,
                a if a == half || t >= T_END - 1 || t == MUZZLE => Some(px::OUTLINE),
                a if a <= 1 && t >= T_END - 3 => Some(px::WHITE),
                a if a <= 4 && t >= MUZZLE + 2 => Some(px::PINK),
                _ => Some(px::MAGENTA),
            };
        }
        let half = if (BARREL.0..=BARREL.1).contains(&t) { HALF - 3 } else { HALF };
        if s.abs() > half {
            return None;
        }
        if s.abs() == half || t <= T_BACK + 1 {
            return Some(px::OUTLINE);
        }
        // The power cell, split from the housing by a seam.
        if t <= T_BACK + 4 {
            return Some(match s.abs() {
                0 | 1 => px::WHITE,
                2..=4 => px::PINK,
                _ => px::MAGENTA_DARK,
            });
        }
        if t == T_BACK + 5 {
            return Some(px::OUTLINE);
        }
        // Barrel fins.
        if t >= BARREL.0 {
            return Some(match () {
                _ if t % 4 == 0 => px::OUTLINE,
                _ if s < 0 => px::STEEL_LIGHT,
                _ => px::STEEL_DARK,
            });
        }
        // The diamond insignia, white at its heart. (`t` and `s` share parity, so the
        // distance is even.)
        match (t - INSIGNIA_T).abs() + s.abs() {
            0 => return Some(px::WHITE),
            2 => return Some(px::DIAMOND),
            _ => {}
        }
        Some(match s {
            -6 => px::WHITE,
            6 => px::NAVY,
            _ if (t == 7 || t == 9) && s.abs() <= 3 => px::MAGENTA,
            _ if t == 8 && s.abs() <= 3 => px::OUTLINE,
            3..=5 => px::STEEL_DARK,
            -5..=-3 => px::STEEL_LIGHT,
            _ => px::STEEL,
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
    // The fist round the grip, under the housing.
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
    /// Standing ready, and recoiling from a shot with the feet planted.
    stand: [SpriteVram; 2],
    /// Jumping and falling. In the air and on the run, recoil moves the whole sprite
    /// back instead of having a pose of its own: sprite memory is tight.
    jump: SpriteVram,
    fall: SpriteVram,
    /// Each step of [`RUN`].
    run: [SpriteVram; 6],
    /// Standing still, after the plain stance: see [`IDLE`].
    idle: [SpriteVram; 3],
    hurt: SpriteVram,
    /// Touching down hard, and skidding to a stop.
    landing: SpriteVram,
    braking: SpriteVram,
    /// Down: sagging on impact, then flat out.
    knocked_out: [SpriteVram; 2],
    /// Blaster and hands per [`Aim`], in [`Aim::ALL`] order.
    blasters: [SpriteVram; 5],
    /// The cannon on the floor, let go of.
    dropped: SpriteVram,
    pub jetpack: SpriteVram,
    pub flames: [SpriteVram; 3],
    /// The portrait from the player sprite sheet, for the dialogue box.
    pub portrait: SpriteVram,
    /// The portrait with the bill half open, then wide open.
    pub portrait_talk: [SpriteVram; 2],
}

impl Wassie {
    pub fn new() -> Self {
        Self {
            stand: [
                pose(FEET_STAND, Cape::Hang, lean::NONE, false),
                pose(FEET_STAND, Cape::Hang, (-RECOIL_BODY, 0), false),
            ],
            jump: pose(FEET_TUCK, Cape::Jump, lean::NONE, false),
            fall: pose(FEET_DANGLE, Cape::Fall, lean::NONE, false),
            run: RUN.each_ref().map(run_pose),
            idle: IDLE.map(|(cape, lean)| pose(FEET_STAND, cape, lean, false)),
            hurt: pose(FEET_TUCK, Cape::Run(1), lean::HURT, true),
            landing: pose(FEET_SQUAT, Cape::Hang, lean::SQUASH, false),
            braking: pose(FEET_STRIDE, Cape::Run(0), lean::BRAKE, false),
            knocked_out: [pose(FEET_SQUAT, Cape::Hang, lean::SLUMP, true), lying()],
            blasters: Aim::ALL.map(blaster),
            dropped: dropped(),
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
            portrait_talk: core::array::from_fn(|f| {
                let mut c = Canvas::new(Size::S32x32);
                for (i, &colour) in PORTRAIT_TALK[f * 1024..][..1024].iter().enumerate() {
                    c.px((i % 32) as i32, (i / 32) as i32, colour);
                }
                c.vram_with(&PORTRAIT_PALETTE)
            }),
        }
    }

    /// The step of the run cycle for this much travel.
    fn run_step(anim: u32) -> usize {
        (anim / RUN_STEP % RUN.len() as u32) as usize
    }

    /// Knocked out and down on the floor, rather than still falling.
    fn down(player: &Player) -> bool {
        player.state == PlayerState::Dead && player.grounded
    }

    /// The body pose for this frame, and how far its sprite is moved back by recoil
    /// (facing right). `anim` advances the run cycle; `frame` paces the idle loop.
    pub fn sprite(&self, player: &Player, anim: u32, frame: u32) -> (&SpriteVram, i32) {
        let firing = player.recoil > 2;
        let pushed = |sprite| (sprite, if firing { -RECOIL_BODY } else { 0 });
        match player.state {
            // Knocked out in the air, it tumbles down eyes shut; only once it hits the
            // floor does it sag and keel over.
            PlayerState::Hurt => (&self.hurt, 0),
            PlayerState::Dead if !player.grounded => (&self.hurt, 0),
            PlayerState::Dead => {
                let sagging = player.dead_timer > DEATH_FRAMES - SLUMP_FRAMES;
                (&self.knocked_out[!sagging as usize], 0)
            }
            // Firing keeps its own recoil pose; otherwise landing and skidding win.
            PlayerState::Idle | PlayerState::Run if player.land > 0 && !firing => (&self.landing, 0),
            PlayerState::Run if player.braking && !firing => (&self.braking, 0),
            PlayerState::Run => pushed(&self.run[Self::run_step(anim)]),
            PlayerState::Idle if firing => (&self.stand[1], 0),
            PlayerState::Idle => (self.idle(frame), 0),
            PlayerState::Jump | PlayerState::Hover => pushed(&self.jump),
            PlayerState::Fall => pushed(&self.fall),
        }
    }

    /// Standing still: the plain stance, then the cape blowing out and flaring up while
    /// the wassie breathes.
    fn idle(&self, frame: u32) -> &SpriteVram {
        match (frame / IDLE_STEP % 4) as usize {
            0 => &self.stand[0],
            step => &self.idle[step - 1],
        }
    }

    /// Standing at ease, blaster forward: the body and the blaster.
    /// Each with its x offset from the body's sprite, facing right.
    pub fn standing(&self, frame: u32) -> [(&SpriteVram, i32); 2] {
        [(self.idle(frame), 0), (&self.blasters[Aim::Forward as usize], BLASTER_SHIFT)]
    }

    /// The blaster for this frame, how far its sprite is moved facing right, and
    /// whether it goes behind the body. It is pushed back by recoil, carried along by
    /// the run's lean and bob, raised over the head (and behind it) when aiming up,
    /// and dropped on the floor once a knocked-out wassie is down. Its sprite sits
    /// [`BLASTER_SHIFT`] forward of the body's.
    pub fn blaster(&self, player: &Player, anim: u32) -> (&SpriteVram, (i32, i32), bool) {
        if Self::down(player) {
            return (&self.dropped, DROPPED_AT, true);
        }
        let aim = if matches!(player.state, PlayerState::Hurt | PlayerState::Dead) {
            Aim::Forward
        } else {
            player.aim
        };
        let (kx, ky) = if player.recoil > 2 {
            let (x, y) = aim.dir();
            (-x * RECOIL_BLASTER, -y * RECOIL_BLASTER)
        } else {
            (0, 0)
        };
        // Running, the arm carries the cannon with the body. Not while a shot is
        // leaving it, so the flash stays on the muzzle.
        let (bx, by) = match player.state {
            PlayerState::Run if player.recoil == 0 && player.land == 0 && !player.braking => {
                let lean = RUN[Self::run_step(anim)].lean;
                (lean.0 + pitch(ARM_ROW), lean.1)
            }
            _ => (0, 0),
        };
        let raise = if aim == Aim::Up { BLASTER_UP_RAISE } else { 0 };
        (&self.blasters[aim as usize], (kx + bx, ky + by - raise), aim == Aim::Up)
    }
}
