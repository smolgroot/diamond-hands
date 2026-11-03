//! The wassie, after the cover art: a teal bean of a head with sleepy half-lidded eyes
//! and a big white bill, a red poncho with a gold ammo bandolier, a tattered cape that
//! streams behind it, white webbed feet, and a chunky blaster with a magenta muzzle.
//!
//! Every pose is a 32x32 sprite facing right (the renderer flips it), layered from
//! back to front: cape, head, poncho, legs, blaster and hands. The wassie has its own
//! palette, so it can be shaded more finely than the villains.

use agb::display::{
    Palette16,
    object::{Size, SpriteVram},
};
use diamond_hands_logic::{PlayerState, player::Player};

use super::{Canvas, rgb};

/// Sprite column of the hitbox centre when facing right; mirrored when facing left.
pub const CENTRE_COL: i32 = 14;
/// Where the body drawings sit in the sprite.
const BODY_X: i32 = 6;
const BLASTER_AT: (i32, i32) = (14, 14);
const BLASTER_UP_AT: (i32, i32) = (19, 0);

pub static PALETTE: Palette16 = Palette16::new([
    rgb(0, 0, 0),
    rgb(2, 2, 4),
    rgb(31, 31, 31),
    rgb(22, 22, 25),
    rgb(12, 29, 23),
    rgb(5, 22, 17),
    rgb(2, 13, 10),
    rgb(31, 11, 13),
    rgb(27, 4, 8),
    rgb(15, 1, 4),
    rgb(31, 25, 6),
    rgb(22, 13, 2),
    rgb(24, 25, 28),
    rgb(14, 15, 20),
    rgb(6, 6, 10),
    rgb(31, 6, 22),
]);

/// Characters in the drawings below: outline, white, bill shade, three teals, three
/// poncho reds, two golds, three steels, magenta.
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
    (b'Y', 11),
    (b'S', 12),
    (b's', 13),
    (b'z', 14),
    (b'm', 15),
];

/// Rows 0..15: head and bill.
const HEAD: [&str; 15] = [
    "................",
    "......kkkk......",
    ".....kTTttk.....",
    "....kTTttttk....",
    "...kTTtttttdk...",
    "...kTtttttttdk..",
    "..kTttttttttdk..",
    "..kTkkkttkkkkdk.",
    "..ktwwktkwwwkdk.",
    "..kttttttttttdk.",
    "..kttttttkkkkkkk",
    "..ktttttkwwwwwwk",
    "..kdttttkwwwwwbk",
    "..kdtttttkbbbbk.",
    "..kddttttdkkkk..",
];

/// Squeezed-shut eyes over rows 7..9, for hurt and knocked-out poses.
const EYES_SHUT: [&str; 2] = [
    "..kTttttttttdk..",
    "..ktkkkttkkkkdk.",
];

/// Rows 15..27: poncho with the bandolier, tattered at the hem.
const PONCHO: [&str; 12] = [
    "..kqpppppppqk...",
    ".kqpPPpppppppk..",
    ".kpPPyypppppppk.",
    "kqpPpYyyppppppk.",
    "kqpppppYyypppppk",
    "kqppppppppYyyppk",
    "kqqpppppppppYypk",
    "kqqppppPpppppppk",
    "kqqqpppPppppppqk",
    ".kqqqpppPpppqqk.",
    ".kqkqqpkqppqkqk.",
    "..k.kk.k.kk.k...",
];

/// Legs and webbed feet, rows 27..32.
const LEGS_STAND: [&str; 5] = [
    ".....dd..dd.....",
    ".....dd..dd.....",
    "....kwwk.kwwk...",
    "...kwwbwkkwwbwk.",
    "...kkkkkkkkkkkk.",
];
const LEGS_STRIDE: [&str; 5] = [
    ".....dd..dd.....",
    "....dd....dd....",
    "..kwwk.....kwwk.",
    ".kwwbwk...kwwbwk",
    ".kkkkkk...kkkkkk",
];
const LEGS_PASS: [&str; 5] = [
    "......ddd.......",
    "......dd........",
    ".....kwwk.......",
    "....kwwbwk......",
    "....kkkkkk......",
];
const LEGS_TUCK: [&str; 5] = [
    "....dd....dd....",
    "...kwwk..kwwk...",
    "..kwwbwk.kwwbwk.",
    "..kkkkkk.kkkkkk.",
    "................",
];
const LEGS_DANGLE: [&str; 5] = [
    ".....dd..dd.....",
    ".....dd...dd....",
    ".....kwk..kwwk..",
    "....kwbwk.kwwbwk",
    "....kkkkk.kkkkkk",
];

/// The cape, 12 wide at the left edge, behind the body. It hangs at rest, ripples
/// through three frames while running, trails below on the way up and blows up on the
/// way down.
/// Drawn from row 15.
const CAPE_HANG: [&str; 13] = [
    "........kqq.",
    ".......kqpq.",
    "......kqppq.",
    "......kqppq.",
    ".....kqpppq.",
    ".....kqppq..",
    ".....kqpppq.",
    "....kqqppq..",
    "....kqpppq..",
    "....kqqppq..",
    "....kqkqpq..",
    "....k.kqkq..",
    "......k.k...",
];
/// Drawn from row 15.
const CAPE_RUN0: [&str; 9] = [
    "........kkq.",
    "....kkkkqpq.",
    ".kkkqqpppq..",
    "kqqqpppPpq..",
    ".kqpppPppq..",
    "kqppPpppq...",
    ".kkqqppq....",
    "kq.kkqq.....",
    ".k...k......",
];
/// Drawn from row 15.
const CAPE_RUN1: [&str; 10] = [
    "........kkq.",
    "......kkqpq.",
    "...kkkqppq..",
    ".kkqqpPppq..",
    "kqqpppPpq...",
    ".kqppPppq...",
    "kqqppppq....",
    ".kkqqkq.....",
    "kq..k.k.....",
    ".k..........",
];
/// Drawn from row 16.
const CAPE_RUN2: [&str; 10] = [
    "........kkq.",
    ".......kqpq.",
    ".....kkqppq.",
    "...kkqppPpq.",
    ".kkqpppPpq..",
    "kqqppPppq...",
    ".kqpppppq...",
    "kqqkqqpq....",
    ".k..kqk.....",
    "....k.......",
];
/// Drawn from row 15.
const CAPE_JUMP: [&str; 13] = [
    "........kkq.",
    ".......kqpq.",
    "......kqppq.",
    ".....kqpppq.",
    "....kqppPq..",
    "....kqpPpq..",
    "...kqppPq...",
    "...kqpppq...",
    "..kqppPq....",
    "..kqpppq....",
    ".kqqkpq.....",
    ".kk.kqk.....",
    "k....k......",
];
/// Drawn from row 8.
const CAPE_FALL: [&str; 10] = [
    ".k..........",
    "kqk..k......",
    ".kqkkqk.....",
    "..kqppqk....",
    "..kqpPpqk...",
    "...kqpPpqk..",
    "....kqppPqk.",
    ".....kqpppq.",
    "......kqppq.",
    ".......kqpq.",
];

/// The blaster facing right, at [`BLASTER_AT`]: scope with a magenta lens, vented
/// shroud, pink cells underneath, and a muzzle block with a glowing ring.
const BLASTER: [&str; 11] = [
    "....kkkkkkk.......",
    "...kSSmmmSSk......",
    ".kkkkkkkkkkkkkkkkk",
    "kSSSSSSSSSSSSkSSSk",
    "ksssssssssssskmmmk",
    "kszsszsszsssskmwmk",
    "ksssssssssssskmmmk",
    "kzmmzzmmzzzzzkSSSk",
    ".kkkkzzkkkkkkkkkkk",
    "....kzzk..kzzk....",
    "....kkkk..kkkk....",
];

/// Hands on the grip and foregrip, relative to the blaster.
const HANDS: [(i32, i32, &[&str]); 2] = [(4, 8, &["kTtk", "ktdk"]), (10, 8, &["kttk"])];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Legs {
    Stand,
    Stride,
    Pass,
    Tuck,
    Dangle,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cape {
    Hang,
    Run(u8),
    Jump,
    Fall,
}

fn pose(legs: Legs, cape: Cape, aim_up: bool, eyes_shut: bool) -> SpriteVram {
    let mut c = Canvas::new(Size::S32x32);
    let (cape_y, cape_rows): (i32, &[&str]) = match cape {
        Cape::Hang => (15, &CAPE_HANG),
        Cape::Run(0) => (15, &CAPE_RUN0),
        Cape::Run(1) => (15, &CAPE_RUN1),
        Cape::Run(_) => (16, &CAPE_RUN2),
        Cape::Jump => (15, &CAPE_JUMP),
        Cape::Fall => (8, &CAPE_FALL),
    };
    c.art_with(KEY, 0, cape_y, cape_rows);
    c.art_with(KEY, BODY_X, 0, &HEAD);
    if eyes_shut {
        c.art_with(KEY, BODY_X, 7, &EYES_SHUT);
    }
    c.art_with(KEY, BODY_X, 15, &PONCHO);
    let legs: &[&str] = match legs {
        Legs::Stand => &LEGS_STAND,
        Legs::Stride => &LEGS_STRIDE,
        Legs::Pass => &LEGS_PASS,
        Legs::Tuck => &LEGS_TUCK,
        Legs::Dangle => &LEGS_DANGLE,
    };
    c.art_with(KEY, BODY_X, 27, legs);
    if aim_up {
        c.art_turned(KEY, BLASTER_UP_AT.0, BLASTER_UP_AT.1, &BLASTER);
    } else {
        let (bx, by) = BLASTER_AT;
        c.art_with(KEY, bx, by, &BLASTER);
        for (hx, hy, hand) in HANDS {
            c.art_with(KEY, bx + hx, by + hy, hand);
        }
    }
    c.vram_with(&PALETTE)
}

/// The run cycle: legs and cape frames that play together.
const RUN: [(Legs, Cape); 4] = [
    (Legs::Stride, Cape::Run(0)),
    (Legs::Stand, Cape::Run(1)),
    (Legs::Pass, Cape::Run(2)),
    (Legs::Stand, Cape::Run(1)),
];

/// Every pose, built once at boot.
pub struct Wassie {
    /// [idle, run 0..4, jump, fall] × [forward, up].
    poses: [[SpriteVram; 2]; 7],
    hurt: SpriteVram,
    knocked_out: SpriteVram,
}

impl Wassie {
    pub fn new() -> Self {
        let all = [
            (Legs::Stand, Cape::Hang),
            RUN[0],
            RUN[1],
            RUN[2],
            RUN[3],
            (Legs::Tuck, Cape::Jump),
            (Legs::Dangle, Cape::Fall),
        ];
        Self {
            poses: all.map(|(l, c)| [pose(l, c, false, false), pose(l, c, true, false)]),
            hurt: pose(Legs::Tuck, Cape::Jump, false, true),
            knocked_out: pose(Legs::Stand, Cape::Hang, false, true),
        }
    }

    /// The pose for this frame; `anim` advances the run cycle.
    pub fn sprite(&self, player: &Player, anim: u32) -> &SpriteVram {
        let pose = match player.state {
            PlayerState::Hurt => return &self.hurt,
            PlayerState::Dead => return &self.knocked_out,
            PlayerState::Idle => 0,
            PlayerState::Run => 1 + (anim / 6 % 4) as usize,
            PlayerState::Jump => 5,
            PlayerState::Fall => 6,
        };
        &self.poses[pose][player.aim_up as usize]
    }
}
