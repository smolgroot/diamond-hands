//! The wassie: running, jumping, hovering on its jetpack, aiming, getting hurt. Firing
//! the blaster lives in `world`, which owns the shot pool.

use crate::{
    FX,
    level::Level,
    physics::{Body, Contacts},
    world::{Events, Input},
};

pub const PLAYER_W: i32 = 10;
pub const PLAYER_H: i32 = 26;
pub const MAX_HP: i32 = 8;

pub const GRAVITY_FX: i32 = 64;
pub const MAX_FALL_FX: i32 = 5 * FX;
pub const JUMP_VEL_FX: i32 = -9 * FX / 2;
/// Releasing jump while rising faster than this clamps the velocity: short hops.
pub const JUMP_CUT_FX: i32 = -3 * FX / 2;
pub const RUN_MAX_FX: i32 = 3 * FX / 2;
pub const RUN_ACCEL_FX: i32 = 40;
pub const AIR_ACCEL_FX: i32 = 28;
pub const FRICTION_FX: i32 = 48;
pub const STOMP_BOUNCE_FX: i32 = -7 * FX / 2;
/// Frames jump must be held before the jetpack lights. Matches the jump buffer, so a
/// press just before landing still jumps instead of hovering.
pub const HOVER_DELAY: u8 = BUFFER_FRAMES;
/// Frames of jetpack fuel per airborne stretch, refilled on landing or a stomp.
pub const HOVER_FRAMES: u8 = 48;
/// Vertical speed the jetpack pushes towards: a slow climb.
pub const HOVER_LIFT_FX: i32 = -FX / 2;
/// How fast the jetpack turns a fall into that climb, per frame.
pub const HOVER_ACCEL_FX: i32 = 40;
/// 1/sqrt(2) in 8-bit fixed point, for diagonal speeds.
pub const DIAGONAL: i32 = 181;
/// Frames after leaving a ledge during which a jump still works.
pub const COYOTE_FRAMES: u8 = 6;
/// Frames a jump press is remembered before landing.
pub const BUFFER_FRAMES: u8 = 6;
pub const INVULN_FRAMES: u16 = 60;
/// Frames without control after a hit.
pub const HURT_FRAMES: u8 = 14;
/// Frames of the landing squash after a real fall.
pub const LAND_FRAMES: u8 = 8;
/// Falling faster than this when touching down squashes the wassie and kicks up dust.
pub const HARD_LANDING_FX: i32 = 2 * FX;
/// Sliding faster than this with no push (or against it) is a brake: heels dug in.
pub const BRAKE_FX: i32 = FX / 2;
/// Height of a full jump in pixels; velocity is integrated before position, so this is
/// the discrete sum over the rising frames.
pub const JUMP_APEX: i32 = {
    let rising = -JUMP_VEL_FX / GRAVITY_FX - 1;
    (rising * -JUMP_VEL_FX - GRAVITY_FX * rising * (rising + 1) / 2) / FX
};
/// Frames from take-off to landing on the same height.
pub const AIRTIME_FRAMES: i32 = 2 * -JUMP_VEL_FX / GRAVITY_FX;
/// Horizontal distance a full-speed full jump covers.
pub const JUMP_REACH: i32 = AIRTIME_FRAMES * RUN_MAX_FX / FX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PlayerState {
    Idle,
    Run,
    Jump,
    Fall,
    Hurt,
    Dead,
    /// Airborne on the jetpack, jump held.
    Hover,
}

/// Where the blaster points, for a wassie facing right (mirrored when facing left).
/// The down aims only work in the air.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Aim {
    Forward,
    UpForward,
    Up,
    DownForward,
    Down,
}

impl Aim {
    pub const ALL: [Aim; 5] = [Aim::Forward, Aim::UpForward, Aim::Up, Aim::DownForward, Aim::Down];

    /// Unit steps along x and y, facing right.
    pub fn dir(self) -> (i32, i32) {
        match self {
            Aim::Forward => (1, 0),
            Aim::UpForward => (1, -1),
            Aim::Up => (0, -1),
            Aim::DownForward => (1, 1),
            Aim::Down => (0, 1),
        }
    }

    pub fn diagonal(self) -> bool {
        matches!(self, Aim::UpForward | Aim::DownForward)
    }

    /// The barrel's tip relative to the hitbox's centre x and top y, facing right. Must
    /// match the blaster drawings in game/src/art/wassie.rs.
    fn muzzle(self) -> (i32, i32) {
        match self {
            Aim::Forward => (23, 14),
            Aim::UpForward => (17, 8),
            Aim::Up => (0, -20),
            Aim::DownForward => (16, 25),
            Aim::Down => (12, 25),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub body: Body,
    pub state: PlayerState,
    /// 1 = right, -1 = left.
    pub facing: i32,
    pub hp: i32,
    pub grounded: bool,
    pub invuln: u16,
    pub hurt: u8,
    coyote: u8,
    buffer: u8,
    /// Frames the jump button has been held without a break.
    jump_held_for: u8,
    pub fire_cooldown: u8,
    /// Frames the fire button has been held, for the charged beam.
    pub charge: u16,
    /// Frames left showing the firing pose.
    pub recoil: u8,
    pub aim: Aim,
    /// Jetpack fuel left, in frames.
    pub hover_fuel: u8,
    pub hovering: bool,
    /// Frames left in the knocked-out pose before respawning.
    pub dead_timer: u16,
    /// Frames left in the landing squash.
    pub land: u8,
    /// Skidding to a stop on the ground this frame.
    pub braking: bool,
}

impl Player {
    pub fn new(body: Body) -> Self {
        Self {
            body,
            state: PlayerState::Idle,
            facing: 1,
            hp: MAX_HP,
            grounded: true,
            invuln: 0,
            hurt: 0,
            coyote: 0,
            buffer: 0,
            jump_held_for: 0,
            fire_cooldown: 0,
            charge: 0,
            recoil: 0,
            aim: Aim::Forward,
            hover_fuel: HOVER_FRAMES,
            hovering: false,
            dead_timer: 0,
            land: 0,
            braking: false,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.state == PlayerState::Dead
    }

    /// Where a shot leaves the barrel, in level pixels.
    pub fn muzzle(&self) -> (i32, i32) {
        let r = self.body.rect();
        let (x, y) = self.aim.muzzle();
        (r.centre_x() + self.facing * x, r.y + y)
    }

    /// One frame of movement. Ignores input while hurt.
    pub fn update(&mut self, input: &Input, level: &Level, events: &mut Events) -> Contacts {
        let control = self.hurt == 0;
        let body = &mut self.body;

        let dir = if control { input.dir_x() } else { 0 };
        if dir != 0 {
            self.facing = dir;
        }
        // Holding L or R on the ground plants the feet, so the d-pad only aims.
        let push = if self.grounded && input.plant() { 0 } else { dir };
        self.braking = self.grounded && body.vx_fx.abs() > BRAKE_FX && body.vx_fx * push <= 0;
        if push != 0 {
            let accel = if self.grounded { RUN_ACCEL_FX } else { AIR_ACCEL_FX };
            // Turning around on the ground is snappy: drop the opposing speed first.
            if self.grounded && body.vx_fx * push < 0 {
                body.vx_fx = 0;
            }
            body.vx_fx = (body.vx_fx + push * accel).clamp(-RUN_MAX_FX, RUN_MAX_FX);
        } else if self.grounded {
            body.vx_fx = approach(body.vx_fx, 0, FRICTION_FX);
        }

        self.coyote = if self.grounded {
            COYOTE_FRAMES
        } else {
            self.coyote.saturating_sub(1)
        };
        self.buffer = if input.jump_pressed() {
            BUFFER_FRAMES
        } else {
            self.buffer.saturating_sub(1)
        };
        if control && self.buffer > 0 && self.coyote > 0 {
            body.vy_fx = JUMP_VEL_FX;
            self.buffer = 0;
            self.coyote = 0;
            events.insert(Events::JUMP);
        }
        if control && !input.jump_held() && body.vy_fx < JUMP_CUT_FX {
            body.vy_fx = JUMP_CUT_FX;
        }

        // Jetpack: holding jump in the air, once the rise is spent, turns the fall into
        // a slow climb until the fuel runs out. Letting go keeps the rest for later.
        self.jump_held_for = if input.jump_held() {
            self.jump_held_for.saturating_add(1)
        } else {
            0
        };
        let can_hover =
            control && !self.grounded && self.jump_held_for > HOVER_DELAY && self.hover_fuel > 0;
        self.hovering = can_hover && (self.hovering || body.vy_fx >= 0);
        if self.hovering {
            self.hover_fuel -= 1;
            body.vy_fx = approach(body.vy_fx, HOVER_LIFT_FX, HOVER_ACCEL_FX);
        } else {
            body.vy_fx = (body.vy_fx + GRAVITY_FX).min(MAX_FALL_FX);
        }

        let falling = body.vy_fx;
        let was_grounded = self.grounded;
        let contacts = body.step(level);
        self.grounded = contacts.ground;
        self.land = self.land.saturating_sub(1);
        if self.grounded && !was_grounded && falling >= HARD_LANDING_FX {
            self.land = LAND_FRAMES;
            events.insert(Events::LAND);
        }
        if self.grounded {
            self.hovering = false;
            self.hover_fuel = HOVER_FRAMES;
        }
        self.aim = if !control {
            Aim::Forward
        } else if input.up() {
            if dir != 0 { Aim::UpForward } else { Aim::Up }
        } else if input.down() && !self.grounded {
            if dir != 0 { Aim::DownForward } else { Aim::Down }
        } else {
            Aim::Forward
        };

        self.hurt = self.hurt.saturating_sub(1);
        self.invuln = self.invuln.saturating_sub(1);
        self.fire_cooldown = self.fire_cooldown.saturating_sub(1);
        self.recoil = self.recoil.saturating_sub(1);
        self.state = if self.hurt > 0 {
            PlayerState::Hurt
        } else if !self.grounded {
            if self.hovering {
                PlayerState::Hover
            } else if body.vy_fx < 0 {
                PlayerState::Jump
            } else {
                PlayerState::Fall
            }
        } else if body.vx_fx != 0 {
            PlayerState::Run
        } else {
            PlayerState::Idle
        };
        contacts
    }

    /// Takes `damage` from something at `from_x`, knocking the player away from it.
    /// Returns false when invulnerable.
    pub fn hurt_by(&mut self, damage: i32, from_x: i32) -> bool {
        self.knocked(damage, from_x, false)
    }

    /// Like [`Player::hurt_by`]; a `heavy` hit throws the wassie much further and keeps
    /// it reeling a little longer.
    pub fn knocked(&mut self, damage: i32, from_x: i32, heavy: bool) -> bool {
        if self.invuln > 0 || self.is_dead() {
            return false;
        }
        self.hp -= damage;
        self.invuln = INVULN_FRAMES;
        self.hurt = HURT_FRAMES;
        self.charge = 0;
        self.land = 0;
        self.hovering = false;
        let away = if self.body.rect().centre_x() < from_x { -1 } else { 1 };
        if heavy {
            self.hurt = HURT_FRAMES + 10;
            self.body.vx_fx = away * 4 * FX;
            self.body.vy_fx = -3 * FX;
        } else {
            self.body.vx_fx = away * 3 * FX / 2;
            self.body.vy_fx = -2 * FX;
        }
        self.grounded = false;
        true
    }
}

pub(crate) fn approach(value: i32, target: i32, step: i32) -> i32 {
    if value > target {
        (value - step).max(target)
    } else {
        (value + step).min(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game::keys,
        physics::Body,
    };

    const FLAT: Level = Level {
        rows: &[
            "........................................",
            "........................................",
            "........................................",
            "........................................",
            "........................................",
            "........................................",
            "........................................",
            "........................................",
            "........................................",
            "........................................",
            "########################################",
        ],
        ..Level::BASE
    };

    fn player() -> Player {
        Player::new(Body::standing_on_tile(4, 9, PLAYER_W, PLAYER_H))
    }

    fn step(p: &mut Player, keys: u16, previous: u16) -> Events {
        let mut events = Events::default();
        p.update(&Input::from_keys(keys, previous), &FLAT, &mut events);
        events
    }

    #[test]
    fn jump_constants() {
        assert_eq!(JUMP_APEX, 38);
        assert_eq!(AIRTIME_FRAMES, 36);
        assert_eq!(JUMP_REACH, 54);
    }

    #[test]
    fn runs_up_to_max_speed_and_stops() {
        let mut p = player();
        for _ in 0..30 {
            step(&mut p, keys::RIGHT, keys::RIGHT);
        }
        assert_eq!(p.body.vx_fx, RUN_MAX_FX);
        assert_eq!(p.state, PlayerState::Run);
        for _ in 0..30 {
            step(&mut p, 0, 0);
        }
        assert_eq!(p.body.vx_fx, 0);
        assert_eq!(p.state, PlayerState::Idle);
        step(&mut p, keys::LEFT, 0);
        assert_eq!(p.facing, -1);
    }

    #[test]
    fn full_jump_reaches_apex_and_lands() {
        let mut p = player();
        step(&mut p, 0, 0);
        let ground = p.body.bottom();
        assert!(step(&mut p, keys::A, 0).contains(Events::JUMP));
        let mut highest = ground;
        let mut frames = 1;
        while !p.grounded {
            highest = highest.min(p.body.bottom());
            // Held through the rise, let go at the top so the jetpack stays off.
            let keys = if p.body.vy_fx < 0 { keys::A } else { 0 };
            step(&mut p, keys, keys::A);
            frames += 1;
            assert!(frames < 100);
        }
        assert!((ground - highest - JUMP_APEX).abs() <= 1, "apex {}", ground - highest);
        assert!((frames - AIRTIME_FRAMES).abs() <= 2, "airtime {frames}");
        assert_eq!(p.body.bottom(), ground);
    }

    #[test]
    fn short_hop_when_released() {
        let mut p = player();
        step(&mut p, 0, 0);
        let ground = p.body.bottom();
        step(&mut p, keys::A, 0);
        let mut highest = ground;
        for _ in 0..40 {
            step(&mut p, 0, 0);
            highest = highest.min(p.body.bottom());
        }
        assert!(ground - highest < JUMP_APEX / 2);
    }

    #[test]
    fn buffered_jump_fires_on_landing() {
        let mut p = player();
        p.body.y_fx -= 4 * FX;
        p.grounded = false;
        step(&mut p, 0, 0);
        step(&mut p, keys::A, 0);
        assert!(!p.grounded, "still falling when A is pressed");
        let mut jumped = false;
        for _ in 0..BUFFER_FRAMES {
            jumped |= step(&mut p, keys::A, keys::A).contains(Events::JUMP);
        }
        assert!(jumped, "a press just before landing should still jump");
    }

    /// Holds A from a standing jump to landing; returns the highest rise in pixels.
    fn held_jump_apex(p: &mut Player) -> i32 {
        step(p, 0, 0);
        let ground = p.body.bottom();
        step(p, keys::A, 0);
        let mut highest = ground;
        for _ in 0..200 {
            step(p, keys::A, keys::A);
            highest = highest.min(p.body.bottom());
            if p.grounded {
                break;
            }
        }
        ground - highest
    }

    #[test]
    fn holding_jump_hovers_higher_on_the_jetpack() {
        let mut p = player();
        let apex = held_jump_apex(&mut p);
        // The Badlands' floating ledges sit 40 and 48 pixels above the nearest floor.
        assert!(apex > JUMP_APEX + 16, "hover apex {apex}");
        assert!(apex < 2 * JUMP_APEX, "hover apex {apex}");
        assert!(p.grounded);
        assert_eq!(p.hover_fuel, HOVER_FRAMES, "refuelled on landing");
    }

    #[test]
    fn hover_starts_after_the_rise_and_runs_out() {
        let mut p = player();
        step(&mut p, 0, 0);
        step(&mut p, keys::A, 0);
        let mut seen = [false; 3];
        for _ in 0..200 {
            step(&mut p, keys::A, keys::A);
            match p.state {
                PlayerState::Jump if !seen[1] => seen[0] = true,
                PlayerState::Hover => {
                    assert!(seen[0], "the jump's own rise comes first");
                    seen[1] = true;
                }
                PlayerState::Fall if seen[1] => {
                    assert_eq!(p.hover_fuel, 0);
                    seen[2] = true;
                }
                _ => {}
            }
            if p.grounded {
                break;
            }
        }
        assert_eq!(seen, [true; 3]);
    }

    #[test]
    fn letting_go_keeps_the_rest_of_the_fuel() {
        let mut p = player();
        step(&mut p, 0, 0);
        step(&mut p, keys::A, 0);
        while p.state != PlayerState::Hover {
            step(&mut p, keys::A, keys::A);
        }
        for _ in 0..10 {
            step(&mut p, keys::A, keys::A);
        }
        let fuel = p.hover_fuel;
        step(&mut p, 0, keys::A);
        assert!(!p.hovering);
        assert_eq!(p.hover_fuel, fuel);
        step(&mut p, keys::A, 0);
        assert!(!p.hovering, "a fresh press waits out the jump buffer");
        for _ in 0..HOVER_DELAY {
            step(&mut p, keys::A, keys::A);
        }
        assert_eq!(p.state, PlayerState::Hover, "holding again relights it");
    }

    #[test]
    fn a_quick_tap_in_the_air_does_not_hover() {
        let mut p = player();
        step(&mut p, 0, 0);
        step(&mut p, keys::A, 0);
        for _ in 0..40 {
            step(&mut p, 0, 0);
        }
        step(&mut p, keys::A, 0);
        step(&mut p, 0, keys::A);
        assert!(!p.hovering);
        assert_eq!(p.hover_fuel, HOVER_FRAMES);
    }

    #[test]
    fn aims_eight_ways_but_down_only_in_the_air() {
        let mut p = player();
        step(&mut p, keys::UP | keys::RIGHT, 0);
        assert_eq!(p.aim, Aim::UpForward);
        step(&mut p, keys::UP, keys::UP);
        assert_eq!(p.aim, Aim::Up);
        step(&mut p, keys::DOWN | keys::LEFT, 0);
        assert_eq!(p.aim, Aim::Forward, "no down aim on the ground");
        assert_eq!(p.facing, -1);
        step(&mut p, keys::A, 0);
        step(&mut p, keys::DOWN | keys::RIGHT, 0);
        assert_eq!(p.aim, Aim::DownForward);
        step(&mut p, keys::DOWN, 0);
        assert_eq!(p.aim, Aim::Down);
        let (mx, my) = p.muzzle();
        assert!(my > p.body.rect().bottom() - 4 && mx > p.body.rect().centre_x());
    }

    #[test]
    fn holding_r_plants_the_feet_to_aim() {
        let mut p = player();
        for _ in 0..20 {
            step(&mut p, keys::R | keys::UP | keys::LEFT, keys::R | keys::UP | keys::LEFT);
        }
        assert_eq!(p.body.vx_fx, 0);
        assert_eq!(p.facing, -1);
        assert_eq!(p.aim, Aim::UpForward);
        assert_eq!(p.state, PlayerState::Idle);
    }

    #[test]
    fn hurt_knocks_back_and_grants_invulnerability() {
        let mut p = player();
        assert!(p.hurt_by(2, 1000));
        assert_eq!(p.hp, MAX_HP - 2);
        assert!(p.body.vx_fx < 0, "knocked away from the source");
        assert!(!p.hurt_by(2, 1000), "invulnerable right after");
        step(&mut p, keys::RIGHT, 0);
        assert_eq!(p.state, PlayerState::Hurt);
        assert!(p.body.vx_fx < 0, "no control while hurt");
    }

    #[test]
    fn a_full_jump_lands_hard_and_squashes() {
        let mut p = player();
        step(&mut p, keys::A, 0);
        let mut landed = Events::default();
        for _ in 0..60 {
            let keys = if p.body.vy_fx < 0 { keys::A } else { 0 };
            landed = step(&mut p, keys, keys::A);
            if p.grounded {
                break;
            }
        }
        assert!(landed.contains(Events::LAND));
        assert_eq!(p.land, LAND_FRAMES);
        for _ in 0..LAND_FRAMES {
            step(&mut p, 0, 0);
        }
        assert_eq!(p.land, 0, "the squash wears off");
    }

    #[test]
    fn letting_go_at_full_speed_skids() {
        let mut p = player();
        for _ in 0..30 {
            step(&mut p, keys::RIGHT, keys::RIGHT);
        }
        assert!(!p.braking, "running is not braking");
        step(&mut p, 0, keys::RIGHT);
        assert!(p.braking);
        for _ in 0..30 {
            step(&mut p, 0, 0);
        }
        assert!(!p.braking, "stopped");
    }
}
