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

use agb::display::object::SpriteVram;
use diamond_hands_logic::{
    PlayerState,
    player::{Aim, Player},
    world::DEATH_FRAMES,
};

use crate::gfx;

/// Sprite column of the hitbox centre when facing right; mirrored when facing left.
pub const CENTRE_COL: i32 = 14;
/// The blaster's sprite sits this far forward of the body's.
pub const BLASTER_SHIFT: i32 = 8;
/// How far the raised cannon's sprite sits above the body's, clear of the crown.
const BLASTER_UP_RAISE: i32 = 14;
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

/// Running, each foot is its own stamp: the near foot white, the far one shaded and
/// drawn behind the belly. Reaching forward heel first, flat under the body, pushing
/// off the toe behind, kicked up behind, and swinging through.
const STEP_REACH: &[&str] = &[".....kk.", "..kkkwbk", ".kwwwwwk", "..kkkkk."];
const STEP_FLAT: &[&str] = &[".kbbbk.", "kwwwwbk", "kwwwwwk", ".kkkkk."];
const STEP_PUSH: &[&str] = &["kk....", "kwk...", "kwwk..", ".kwwk.", "..kkk."];
const STEP_KICK: &[&str] = &["..kkk.", ".kwwbk", "kwwwk.", "kwwk..", ".kk..."];
const STEP_SWING: &[&str] = &[".kkkk.", "kwwwbk", ".kkkk."];

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

/// Frame `i` of the studio's `wassie` sheet.
fn frame_body(i: usize) -> SpriteVram {
    SpriteVram::from(&gfx::SPRITE_WASSIE.frames[i])
}

impl Wassie {
    pub fn new() -> Self {
        Self {
            // The body poses, in the order of the studio's `wassie` sheet.
            stand: [0, 1].map(frame_body),
            jump: frame_body(11),
            fall: frame_body(12),
            run: [5, 6, 7, 8, 9, 10].map(frame_body),
            idle: [2, 3, 4].map(frame_body),
            hurt: frame_body(13),
            landing: frame_body(14),
            braking: frame_body(15),
            knocked_out: [16, 17].map(frame_body),
            // The cannon, in `Aim` order, and lying on the floor.
            blasters: [0, 1, 2, 3, 4].map(|i| SpriteVram::from(&gfx::SPRITE_BLASTER.frames[i])),
            dropped: SpriteVram::from(&gfx::SPRITE_BLASTER.frames[5]),
            jetpack: SpriteVram::from(&gfx::SPRITE_JETPACK.frames[0]),
            flames: [0, 1, 2].map(|i| SpriteVram::from(&gfx::SPRITE_FLAME.frames[i])),
            // The talking face sheet's first frame: the portrait with its black background, as the
            // code-drawn ROM showed it (the studio's `wassie-portrait` has transparent corners).
            portrait: super::studio::wassie_talk::portrait(),
            portrait_talk: super::studio::wassie_talk::portrait_talk(),
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
