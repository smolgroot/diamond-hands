//! The wassie, after the cover art and the player sprite sheet
//! (`.claude/skills/diamond-hands-gba/sprite-sheet.png`): a teal bean of a head with a
//! gold rim light down the back, sleepy half-lidded eyes and a broad white bill, a red
//! scarf wrapped at the neck over a red poncho with a gold ammo bandolier, a tattered
//! cape that streams behind it, white webbed feet, and an oversized blaster with a
//! magenta muzzle and a little wassie-teal diamond insignia.
//!
//! Every body pose is a 32x32 sprite facing right (the renderer flips it), layered from
//! back to front: cape, head, poncho, legs. The blaster and hands are a second 32x32
//! object in the same frame, one per aim, laid on top, so five aims cost five sprites
//! rather than five sets of poses. The wassie has its own palette, so it can be shaded
//! more finely than the bots. At rest it breathes (the upper body settles a pixel);
//! firing kicks the cannon back two pixels along the aim and the upper body one, while
//! the legs stay planted. Hovering straps on a jetpack with a flickering flame. Hard
//! landings squash it for a few frames, skids lean it back on its heels, and a knockout
//! slumps it and then lays it flat on its face.

use agb::display::{
    Palette16,
    object::{Size, SpriteVram},
};
use diamond_hands_logic::{
    PlayerState,
    world::DEATH_FRAMES,
    player::{Aim, Player},
};

use super::{Canvas, rgb};

/// Sprite column of the hitbox centre when facing right; mirrored when facing left.
pub const CENTRE_COL: i32 = 14;
/// Where the body drawings sit in the sprite.
const BODY_X: i32 = 6;
const BLASTER_AT: (i32, i32) = (14, 14);
const BLASTER_UP_AT: (i32, i32) = (19, 0);
const BLASTER_DOWN_AT: (i32, i32) = (19, 14);
/// Where the barrel axis crosses the hands, for the diagonal aims.
const GRIP_UP_FORWARD: (i32, i32) = (16, 22);
const GRIP_DOWN_FORWARD: (i32, i32) = (16, 17);
/// Jetpack and its flame, top-left in the 32x32 frame facing right.
pub const JETPACK_AT: (i32, i32) = (0, 12);
pub const FLAME_AT: (i32, i32) = (0, 26);
/// Frames of the knockout spent sagging before it keels over.
const SLUMP_FRAMES: u16 = 14;
/// Recoil: how far the cannon and the upper body are pushed back.
const RECOIL_BLASTER: i32 = 2;
const RECOIL_BODY: i32 = 1;

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

/// Rows 0..15: the head, after the sprite sheet. A tall dome tipped back, with the
/// gold rim light down the back, one sleepy half-lidded eye and a broad white bill
/// that juts well past the face, shaded underneath.
const HEAD: [&str; 15] = [
    "....kkkkkk..........",
    "...kyTTTTTkk........",
    "..kyTTtttttTk.......",
    ".kyTttttttttdk......",
    ".kyTtttttttttdk.....",
    "kyTttttttttttdk.....",
    "kyTtttkkkkkkkdk.....",
    "kytttkwwwwwkkdk.....",
    "kytttttkkkkkkkkkkk..",
    "kYttttttkwwwwwwwwwk.",
    "kYdtttttkwwwwwwwwwwk",
    "kYdttttttkwwwwwwwbbk",
    ".kddttttttkbbbbbbbk.",
    ".kdddtttttdkkkkkkk..",
    "..kdddddddk.........",
];
/// Where the head's rows sit in the sprite.
const HEAD_Y: i32 = 0;

/// The eye squeezed shut, over head rows 6..8, for hurt and knocked-out poses.
const EYES_SHUT: [&str; 2] = [
    "kyTttttttttttdk.....",
    "kytttkkkkkkkkdk.....",
];
const EYES_Y: i32 = HEAD_Y + 6;

/// The body: the scarf wrapped under the bill with a gold fold at the shoulder, over a
/// round red belly. No neck and no legs to speak of.
const BODY: [&str; 16] = [
    "...kqPPPPPPk....",
    "..kqPPyyPPPPk...",
    ".kqpPyYPPppPk...",
    ".kqppPPppppppk..",
    "kqppPPpppppppk..",
    "kqpPPppppppppk..",
    "kqpPpppppppppk..",
    "kqpPppppppppqk..",
    "kqqpppppppppqk..",
    "kqqpppppppppqk..",
    "kqqppppppppqqk..",
    ".kqqpppppppqk...",
    ".kqqqppppqqqk...",
    "..kqqqqqqqqk....",
    "...kkkkkkkk.....",
    "................",
];
const BODY_Y: i32 = 13;

/// Little white feet tucked right under the belly, rows 29..32.
const FEET_Y: i32 = 29;
const FEET_STAND: [&str; 3] = [
    "...kwwk.kwwwk...",
    "..kwwwbkkwwwbk..",
    "..kkkkkk.kkkkkk.",
];
const FEET_STRIDE: [&str; 3] = [
    ".kwwk......kwwk.",
    "kwwbk.....kwwwbk",
    "kkkk......kkkkkk",
];
const FEET_PASS: [&str; 3] = [
    "....kwwwk.......",
    "...kwwwwbk......",
    "...kkkkkkk......",
];
/// Drawn two rows higher, pulled up against the belly.
const FEET_TUCK: [&str; 3] = [
    "...kwwk..kwwk...",
    "..kwwbk.kwwbk...",
    "..kkkk..kkkk....",
];
/// Spread flat, for the landing squash.
const FEET_SQUAT: [&str; 3] = [
    "................",
    ".kwwwk....kwwwk.",
    ".kkkkkk..kkkkkk.",
];
const FEET_DANGLE: [&str; 3] = [
    "....kwk...kwk...",
    "...kwwbk.kwwbk..",
    "...kkkk..kkkk...",
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
/// shroud with the insignia, pink cells underneath, and a muzzle block with a glowing
/// ring.
const BLASTER: [&str; 11] = [
    "....kkkkkkk.......",
    "...kSSmmmSSk......",
    ".kkkkkkkkkkkkkkkkk",
    "kSSSSSSSSSSSSkSSSk",
    "kssTssssssssskmmmk",
    "ksTwTzsszsssskmwmk",
    "kssTssssssssskmmmk",
    "kzmmzzmmzzzzzkSSSk",
    ".kkkkzzkkkkkkkkkkk",
    "....kzzk..kzzk....",
    "....kkkk..kkkk....",
];

/// Hands on the grip and foregrip, relative to the blaster.
const HANDS: [(i32, i32, &[&str]); 2] = [(3, 7, &[".kk.", "kTtk", "ktdk", ".kk."]), (10, 8, &["kttk"])];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Legs {
    Stand,
    Stride,
    Pass,
    Tuck,
    Squat,
    Dangle,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cape {
    Hang,
    Run(u8),
    Jump,
    Fall,
}

/// How the upper body is posed on top of the legs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Upper {
    Ready,
    /// Breathing out: everything above the legs one pixel lower.
    Settled,
    /// Just fired: the body pushed back.
    Recoil,
    /// Touching down hard: squashed two pixels into the legs.
    Squash,
    /// Skidding: leaning back on the heels.
    Lean,
    /// Knocked out: sagging before it keels over.
    Slump,
}

impl Upper {
    fn offset(self) -> (i32, i32) {
        match self {
            Upper::Ready => (0, 0),
            Upper::Settled => (0, 1),
            Upper::Recoil => (-RECOIL_BODY, 0),
            Upper::Squash => (0, 2),
            Upper::Lean => (-1, 1),
            Upper::Slump => (0, 3),
        }
    }
}

fn feet(legs: Legs) -> (i32, &'static [&'static str]) {
    match legs {
        Legs::Stand => (FEET_Y, &FEET_STAND),
        Legs::Stride => (FEET_Y, &FEET_STRIDE),
        Legs::Pass => (FEET_Y, &FEET_PASS),
        Legs::Tuck => (FEET_Y - 2, &FEET_TUCK),
        Legs::Squat => (FEET_Y, &FEET_SQUAT),
        Legs::Dangle => (FEET_Y, &FEET_DANGLE),
    }
}

fn pose(legs: Legs, cape: Cape, upper: Upper, eyes_shut: bool) -> SpriteVram {
    let mut c = Canvas::new(Size::S32x32);
    let (dx, dy) = upper.offset();
    let (cape_y, cape_rows): (i32, &[&str]) = match cape {
        Cape::Hang => (15, &CAPE_HANG),
        Cape::Run(0) => (15, &CAPE_RUN0),
        Cape::Run(1) => (15, &CAPE_RUN1),
        Cape::Run(_) => (16, &CAPE_RUN2),
        Cape::Jump => (15, &CAPE_JUMP),
        Cape::Fall => (8, &CAPE_FALL),
    };
    c.art_with(KEY, dx, cape_y + dy, cape_rows);
    c.art_with(KEY, BODY_X + dx, BODY_Y + dy, &BODY);
    c.art_with(KEY, BODY_X + dx, HEAD_Y + dy, &HEAD);
    if eyes_shut {
        c.art_with(KEY, BODY_X + dx, EYES_Y + dy, &EYES_SHUT);
    }
    // The feet stay planted and sit over the belly's hem.
    let (feet_y, feet_rows) = feet(legs);
    c.art_with(KEY, BODY_X, feet_y, feet_rows);
    c.vram_with(&PALETTE)
}

/// Out cold, flat on its face: the standing layers turned a quarter clockwise, so the
/// head lies at the right with the bill in the dirt and the cape draped over the back.
fn lying() -> SpriteVram {
    let mut c = Canvas::new(Size::S32x32);
    // Upright (x, y) lands at (31 - y, x + Y0) once turned; each layer is placed by
    // where its top-left corner would be upright.
    const Y0: i32 = 6;
    let turned = |c: &mut Canvas, x: i32, y: i32, rows: &[&str]| {
        c.art_turned_cw(KEY, 32 - y - rows.len() as i32, x + Y0, rows);
    };
    turned(&mut c, 0, 15, &CAPE_HANG);
    turned(&mut c, BODY_X, BODY_Y, &BODY);
    turned(&mut c, BODY_X, FEET_Y, &FEET_STAND);
    turned(&mut c, BODY_X, HEAD_Y, &HEAD);
    turned(&mut c, BODY_X, EYES_Y, &EYES_SHUT);
    c.vram_with(&PALETTE)
}

/// The blaster and hands for one aim, in the body's 32x32 frame.
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
    pub const STEEL_LIGHT: u8 = 12;
    pub const STEEL: u8 = 13;
    pub const STEEL_DARK: u8 = 14;
    pub const MAGENTA: u8 = 15;
}

/// The blaster at 45°, pointing forward and up (`dy` = -1) or down (`dy` = 1), drawn
/// from its shape so the outline stays one clean pixel staircase. `t` runs along the
/// barrel from the grip (two per diagonal step), `s` across it, positive on the
/// underside; the two always share parity. The parts match the side-on drawing:
/// scope on top, insignia, pink cells underneath, a magenta muzzle with a hot core.
fn blaster_diagonal(c: &mut Canvas, (gx, gy): (i32, i32), dy: i32) {
    const T_END: i32 = 24;
    const T_BACK: i32 = -4;
    const HALF: i32 = 5;
    const SCOPE: (i32, i32) = (5, 15);
    const GRIPS: [i32; 2] = [0, 10];
    let colour = |t: i32, s: i32| -> Option<u8> {
        // Scope rail on top, with its own outline.
        if (-HALF - 3..-HALF).contains(&s) && (SCOPE.0..=SCOPE.1).contains(&t) {
            let edge = s == -HALF - 3 || t <= SCOPE.0 + 1 || t >= SCOPE.1 - 1;
            return Some(match () {
                _ if edge => px::OUTLINE,
                _ if t >= SCOPE.1 - 4 => px::MAGENTA,
                _ => px::STEEL_LIGHT,
            });
        }
        // Grips hanging underneath.
        if (HALF..HALF + 4).contains(&s) && GRIPS.iter().any(|&g| (g..=g + 4).contains(&t)) {
            let g = if t < GRIPS[1] { GRIPS[0] } else { GRIPS[1] };
            let edge = s == HALF + 3 || t <= g + 1 || t >= g + 3;
            return Some(if edge { px::OUTLINE } else { px::STEEL_DARK });
        }
        if s.abs() > HALF || !(T_BACK..=T_END).contains(&t) {
            return None;
        }
        if s.abs() == HALF || t <= T_BACK + 1 || t >= T_END - 1 {
            return Some(px::OUTLINE);
        }
        // Muzzle block behind a collar.
        if t >= T_END - 9 {
            return Some(match () {
                _ if t <= T_END - 8 => px::OUTLINE,
                _ if s.abs() >= 3 => px::STEEL_LIGHT,
                _ if s.abs() <= 1 && (T_END - 6..=T_END - 3).contains(&t) => px::WHITE,
                _ => px::MAGENTA,
            });
        }
        // Teal diamond insignia with a white glint.
        if (t - 3).abs() + (s + 1).abs() <= 2 {
            return Some(if t == 3 && s == -1 { px::WHITE } else { px::TEAL_LIGHT });
        }
        Some(match s {
            -4 => px::STEEL_LIGHT,
            -3..=1 => px::STEEL,
            4 if t.rem_euclid(6) < 2 => px::MAGENTA,
            _ => px::STEEL_DARK,
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
    // Hands wrapped round the grips.
    for g in GRIPS {
        let (t, s) = (g + 2, HALF + 1);
        let (hx, hy) = (gx + (t - s * dy) / 2, gy + (t * dy + s) / 2);
        c.rect(hx - 1, hy - 1, 3, 3, px::OUTLINE);
        c.rect(hx, hy - 1, 2, 2, px::TEAL);
        c.px(hx, hy - 1, px::TEAL_LIGHT);
        c.px(hx + 1, hy, px::TEAL_DARK);
    }
}

/// A steel jetpack strapped over the cape: tank, gold band, a magenta status light.
const JETPACK: [&str; 15] = [
    "..kkkk..",
    ".kSSssk.",
    ".kSsssk.",
    ".kSsszkk",
    ".kyyyykk",
    ".kSsssk.",
    ".kSmssk.",
    ".kSsssk.",
    ".kSsszk.",
    ".kSsszkk",
    ".kyYYYkk",
    "..kzzk..",
    "..kzzk..",
    ".kzzzzk.",
    "..kkkk..",
];
/// Exhaust flames under the nozzle: two flickering frames and a sputter.
const FLAMES: [&[&str]; 3] = [
    &["..ww....", ".wyyw...", ".yPPy...", "..Pp....", "..p.....", "........"],
    &["..ww....", ".wwyw...", ".yyPy...", ".yPPp...", "..Pp....", "...p...."],
    &["..wy....", "..yP....", "...p....", "........", "........", "........"],
];

/// Locomotion poses, in [`Wassie::poses`] order.
const STANCES: [(Legs, Cape); 6] = [
    (Legs::Stand, Cape::Hang),
    (Legs::Stride, Cape::Run(0)),
    (Legs::Stand, Cape::Run(1)),
    (Legs::Pass, Cape::Run(2)),
    (Legs::Tuck, Cape::Jump),
    (Legs::Dangle, Cape::Fall),
];
/// The run cycle as indices into [`STANCES`].
const RUN: [usize; 4] = [1, 2, 3, 2];

/// Every pose, built once at boot.
pub struct Wassie {
    /// Each stance ready, and recoiling from a shot.
    poses: [[SpriteVram; 2]; 6],
    breathing: SpriteVram,
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
    /// The head at double size, for the dialogue box.
    pub portrait: SpriteVram,
}

impl Wassie {
    pub fn new() -> Self {
        Self {
            poses: STANCES.map(|(l, c)| [pose(l, c, Upper::Ready, false), pose(l, c, Upper::Recoil, false)]),
            breathing: pose(Legs::Stand, Cape::Hang, Upper::Settled, false),
            hurt: pose(Legs::Tuck, Cape::Jump, Upper::Ready, true),
            landing: pose(Legs::Squat, Cape::Hang, Upper::Squash, false),
            braking: pose(Legs::Stride, Cape::Run(0), Upper::Lean, false),
            knocked_out: [pose(Legs::Squat, Cape::Hang, Upper::Slump, true), lying()],
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
                // Too wide to fit at double size: the bill runs off the frame's edge.
                c.art_scaled(KEY, -2, 1, &HEAD, 2);
                c.vram_with(&PALETTE)
            },
        }
    }

    /// The body pose for this frame. `anim` advances the run cycle; `frame` paces
    /// breathing.
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
        } else if stance == 0 && frame % 64 >= 32 {
            &self.breathing
        } else {
            &poses[0]
        }
    }

    /// Standing at ease, blaster forward: the body and the blaster.
    pub fn standing(&self, frame: u32) -> [&SpriteVram; 2] {
        let body = if frame % 64 >= 32 { &self.breathing } else { &self.poses[0][0] };
        [body, &self.blasters[Aim::Forward as usize]]
    }

    /// The blaster for this frame and how far recoil pushes it back, facing right.
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
