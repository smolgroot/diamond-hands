//! The Solana Trenches' sprites: SAM, his cucumbers and his Market Crash, the penguin
//! cashier, gold coins, CL207's plasma cells, the elevator and its terminal, and the
//! casino's bots. They are loaded only while the Trenches are on screen (see
//! `render::LevelArt`): sprite VRAM can't hold them and BERA 3000 at once. Even alone
//! they don't all fit, so they come in two halves swapped as the camera goes: the
//! floors' ([`FloorArt`]) and the arena's ([`ArenaArt`]). Only the coins stay loaded.

use core::cell::RefCell;

use agb::display::object::SpriteVram;

use super::{LazyPoses, studio};
use crate::gfx;

/// The [`SAM_PALETTE`] indices the effects use.
mod sx {
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

/// The dialogue portrait, from the card in the sheet's corner. Colour indices, row by row.

/// SAM. Only the pose on screen sits in sprite VRAM: all of them would take 32 KB, more
/// than there is. Changing pose copies the new one in from ROM.
pub struct Sam {
    shown: RefCell<Option<(SamPose, SpriteVram)>>,
    /// His face for the dialogue box, with the mouth it shows (0 shut, 1 half open,
    /// 2 wide open). Only the one on screen is kept: the arena has no sprite memory
    /// for three.
    face: RefCell<Option<(usize, SpriteVram)>>,
}

impl Sam {
    pub fn new() -> Self {
        Self { shown: RefCell::new(None), face: RefCell::new(Some((0, Self::draw_face(0)))) }
    }

    fn draw_face(mouth: usize) -> SpriteVram {
        SpriteVram::from(&gfx::SPRITE_SAM_PORTRAIT.frames[mouth])
    }

    /// His portrait with `mouth` open, swapping the loaded face when it changes. The old
    /// one is dropped first so the two never sit in sprite memory together.
    pub fn portrait(&self, mouth: usize) -> SpriteVram {
        let mut face = self.face.borrow_mut();
        if face.as_ref().is_none_or(|(loaded, _)| *loaded != mouth) {
            *face = None;
            *face = Some((mouth, Self::draw_face(mouth)));
        }
        face.as_ref().map(|(_, sprite)| sprite.clone()).unwrap()
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
        let sprite = SpriteVram::from(&gfx::SPRITE_SAM.frames[pose as usize]);
        *shown = Some((pose, sprite.clone()));
        sprite
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
            cucumber: studio::sam_fx_16x16::cucumber(),
            mega: studio::sam_fx_32x16::mega(),
            splat: studio::sam_fx_16x16::splat(),
            aura: studio::sam_fx_8x16::aura(),
            candle: studio::sam_candle::candle(),
            orb: studio::sam_fx_8x8::orb(),
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
        Self { sam: Sam::new(), sam_fx: SamFx::new(), penguin: studio::penguin::penguin() }
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
            coin: studio::casino_8x8::coin(),
            floor: Some(FloorArt::new()),
            arena: None,
        }
    }
}

impl FloorArt {
    pub fn new() -> Self {
        Self {
            cell: studio::casino_16x16::cell(),
            terminal: studio::casino_16x32::terminal(),
            terminal_on: studio::casino_16x16::terminal_on(),
            lift: studio::casino_32x32::lift(),
            lift_door: studio::casino_8x32::lift_door(),
            up_prompt: studio::casino_16x16::up_prompt(),
            phone: studio::casino_16x16::phone(),
            dealer: studio::casino_16x32::dealer(),
            cards_in_hand: studio::casino_8x8::cards_in_hand(),
            ninja: LazyPoses::new(&gfx::SPRITE_NINJA.frames),
            card: studio::casino_8x8::card(),
            chip: studio::casino_16x16::chip(),
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

