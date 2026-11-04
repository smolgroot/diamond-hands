//! The arrival before a level: a cargo ship drops the wassie off, a guide walks up,
//! talks, and throws it the blaster and the jetpack before the ship leaves.
//!
//! Everything here is a pure function of the beat and the frames spent in it, so the
//! ROM draws exactly what the host model plays. Positions are level pixels; the camera
//! sits at the level's start throughout.

use crate::talk::Line;

/// A level's arrival script.
#[derive(Debug)]
pub struct Arrival {
    pub lines: &'static [Line],
    /// The guide hands over the gear once this many lines have been read.
    pub gift_after: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Beat {
    /// The ship flies in and hovers over the start.
    Landing,
    /// The hatch opens, the wassie drops out, and the guide walks up.
    Drop,
    /// The lines before the gift.
    Talk,
    /// The guide throws the blaster, then the jetpack.
    Gift,
    /// The lines after the gift.
    Reply,
    /// The ship lifts off and the guide walks away.
    Liftoff,
}

pub const LANDING_FRAMES: u32 = 120;
pub const DROP_FRAMES: u32 = 90;
pub const GIFT_FRAMES: u32 = 72;
pub const LIFTOFF_FRAMES: u32 = 72;

/// The ship's sprite, and where it hovers: its hatch over the wassie's start.
pub const SHIP_W: i32 = 64;
pub const SHIP_H: i32 = 32;
const SHIP_HOVER_Y: i32 = 60;
const SHIP_OFF_Y: i32 = -SHIP_H - 8;
/// The hatch opens over this many frames at the start of the drop.
const HATCH_FRAMES: u32 = 12;
/// Frames the wassie takes to fall from the hatch to the ground.
const FALL_FRAMES: u32 = 16;
/// Frames of landing squash when it touches down.
pub const LAND_SQUASH: u8 = 8;

/// The guide is 16 wide, feet on the ground; its 32x32 sprite is drawn centred on it.
pub const GUIDE_W: i32 = 16;
/// Where the guide stands to talk, left edge.
const GUIDE_STAND_X: i32 = 92;
/// Off the right edge of the screen.
const GUIDE_OFF_X: i32 = 248;
const GUIDE_WALK_IN: (u32, u32) = (24, 84);
const GUIDE_WALK_OUT: (u32, u32) = (12, 68);

/// Frames each piece of gear spends in the air, and when the second is thrown.
const THROW_FRAMES: u32 = 28;
const JETPACK_THROWN: u32 = 36;
const THROW_HEIGHT: i32 = 28;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gear {
    Blaster,
    Jetpack,
}

/// What the screen shows at one moment of the arrival.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct View {
    /// Ship top-left, and how far its hatch is open (0 shut to 4 open).
    pub ship: (i32, i32),
    pub hatch: u8,
    /// Guide left edge and feet; whether it is walking, and which way it faces.
    pub guide: (i32, i32),
    pub guide_walking: bool,
    pub guide_facing: i32,
    /// The wassie is out of the ship.
    pub wassie_out: bool,
    /// Gear in flight, centred at (x, y).
    pub thrown: Option<(Gear, i32, i32)>,
    /// What the wassie holds so far.
    pub blaster: bool,
    pub jetpack: bool,
}

/// Where things stand at frame `t` of `beat`. `start` is the wassie's centre x and
/// feet on the ground; `ground` is the floor under the guide.
pub fn view(beat: Beat, t: u32, start: (i32, i32), ground: i32) -> View {
    let ship_x = start.0 - SHIP_W / 2;
    let ship_y = match beat {
        // Easing in from above the screen.
        Beat::Landing => {
            let left = (LANDING_FRAMES - t.min(LANDING_FRAMES)) as i32;
            let span = SHIP_HOVER_Y - SHIP_OFF_Y;
            SHIP_HOVER_Y - span * left * left / (LANDING_FRAMES * LANDING_FRAMES) as i32
        }
        // Speeding away upwards.
        Beat::Liftoff => {
            let t = t.min(LIFTOFF_FRAMES) as i32;
            let span = SHIP_HOVER_Y - SHIP_OFF_Y;
            SHIP_HOVER_Y - span * t * t / (LIFTOFF_FRAMES * LIFTOFF_FRAMES) as i32
        }
        // Hovering, bobbing a pixel.
        _ => SHIP_HOVER_Y + ((t / 16) % 2) as i32,
    };
    let hatch = match beat {
        Beat::Landing => 0,
        Beat::Drop => (t * 4 / HATCH_FRAMES).min(4) as u8,
        Beat::Liftoff => 4u8.saturating_sub((t * 4 / HATCH_FRAMES) as u8),
        _ => 4,
    };

    // From x0 to x1 between frames `from` and `to`; also whether it is under way.
    let walk = |(from, to): (u32, u32), x0: i32, x1: i32| {
        let done = (t.clamp(from, to) - from) as i32;
        (x0 + (x1 - x0) * done / (to - from) as i32, (from..to).contains(&t))
    };
    let (guide_x, guide_walking, guide_facing) = match beat {
        Beat::Landing => (GUIDE_OFF_X, false, -1),
        Beat::Drop => {
            let (x, walking) = walk(GUIDE_WALK_IN, GUIDE_OFF_X, GUIDE_STAND_X);
            (x, walking, -1)
        }
        Beat::Liftoff => {
            let (x, walking) = walk(GUIDE_WALK_OUT, GUIDE_STAND_X, GUIDE_OFF_X);
            (x, walking, if t >= GUIDE_WALK_OUT.0 { 1 } else { -1 })
        }
        _ => (GUIDE_STAND_X, false, -1),
    };

    let (thrown, blaster, jetpack) = match beat {
        Beat::Gift => {
            let from = (guide_x + 2, ground - 18);
            let to = (start.0, start.1 - 14);
            let arc = |t: u32| {
                let t = t as i32;
                let n = THROW_FRAMES as i32;
                let x = from.0 + (to.0 - from.0) * t / n;
                let y = from.1 + (to.1 - from.1) * t / n - 4 * THROW_HEIGHT * t * (n - t) / (n * n);
                (x, y)
            };
            let in_air = |thrown_at: u32| (thrown_at..thrown_at + THROW_FRAMES).contains(&t);
            let thrown = if in_air(0) {
                let (x, y) = arc(t);
                Some((Gear::Blaster, x, y))
            } else if in_air(JETPACK_THROWN) {
                let (x, y) = arc(t - JETPACK_THROWN);
                Some((Gear::Jetpack, x, y))
            } else {
                None
            };
            (thrown, t >= THROW_FRAMES, t >= JETPACK_THROWN + THROW_FRAMES)
        }
        Beat::Reply | Beat::Liftoff => (None, true, true),
        _ => (None, false, false),
    };

    View {
        ship: (ship_x, ship_y),
        hatch,
        guide: (guide_x, ground),
        guide_walking,
        guide_facing,
        wassie_out: beat > Beat::Drop || (beat == Beat::Drop && t >= HATCH_FRAMES),
        thrown,
        blaster,
        jetpack,
    }
}

/// The wassie's feet while it drops out of the hatch at frame `t` of [`Beat::Drop`],
/// and whether it has landed.
pub fn drop_feet(t: u32, start: (i32, i32)) -> (i32, bool) {
    let top = SHIP_HOVER_Y + SHIP_H;
    let fall = t.saturating_sub(HATCH_FRAMES).min(FALL_FRAMES) as i32;
    let n = FALL_FRAMES as i32;
    (top + (start.1 - top) * fall * fall / (n * n), fall == n)
}

/// The frame of [`Beat::Gift`] when each piece is caught, for sound and sparkle.
pub fn caught(t: u32) -> Option<Gear> {
    match t {
        THROW_FRAMES => Some(Gear::Blaster),
        _ if t == JETPACK_THROWN + THROW_FRAMES => Some(Gear::Jetpack),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: (i32, i32) = (28, 128);

    #[test]
    fn the_ship_comes_down_to_hover_then_leaves_off_screen() {
        assert!(view(Beat::Landing, 0, START, 128).ship.1 + SHIP_H <= 0);
        let hover = view(Beat::Landing, LANDING_FRAMES, START, 128).ship;
        assert_eq!(hover, (START.0 - SHIP_W / 2, SHIP_HOVER_Y));
        assert!(view(Beat::Liftoff, LIFTOFF_FRAMES, START, 128).ship.1 + SHIP_H <= 0);
    }

    #[test]
    fn the_wassie_drops_from_the_hatch_to_its_start() {
        assert!(!view(Beat::Drop, 0, START, 128).wassie_out);
        let (feet, landed) = drop_feet(HATCH_FRAMES, START);
        assert_eq!((feet, landed), (SHIP_HOVER_Y + SHIP_H, false));
        assert_eq!(drop_feet(DROP_FRAMES, START), (START.1, true));
    }

    #[test]
    fn the_guide_walks_up_and_away() {
        let v = view(Beat::Drop, 0, START, 128);
        assert_eq!((v.guide.0, v.guide_walking), (GUIDE_OFF_X, false));
        let v = view(Beat::Drop, 50, START, 128);
        assert!(v.guide_walking && v.guide.0 < GUIDE_OFF_X && v.guide.0 > GUIDE_STAND_X);
        assert_eq!(view(Beat::Drop, DROP_FRAMES, START, 128).guide.0, GUIDE_STAND_X);
        let v = view(Beat::Liftoff, LIFTOFF_FRAMES, START, 128);
        assert_eq!((v.guide.0, v.guide_facing), (GUIDE_OFF_X, 1));
    }

    #[test]
    fn the_gear_flies_to_the_wassie_one_piece_at_a_time() {
        let v = view(Beat::Gift, 0, START, 128);
        assert_eq!((v.blaster, v.jetpack), (false, false));
        assert!(matches!(v.thrown, Some((Gear::Blaster, x, _)) if x > START.0));
        let v = view(Beat::Gift, THROW_FRAMES, START, 128);
        assert_eq!((v.blaster, v.jetpack, v.thrown), (true, false, None));
        assert_eq!(caught(THROW_FRAMES), Some(Gear::Blaster));
        let v = view(Beat::Gift, JETPACK_THROWN + 10, START, 128);
        assert!(matches!(v.thrown, Some((Gear::Jetpack, _, _))));
        let v = view(Beat::Gift, GIFT_FRAMES, START, 128);
        assert_eq!((v.blaster, v.jetpack), (true, true));
        assert!(JETPACK_THROWN + THROW_FRAMES <= GIFT_FRAMES);
    }
}
