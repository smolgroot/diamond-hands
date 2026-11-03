//! The wassie: running, jumping, getting hurt. Firing the blaster lives in `world`,
//! which owns the shot pool.

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
/// Frames after leaving a ledge during which a jump still works.
pub const COYOTE_FRAMES: u8 = 6;
/// Frames a jump press is remembered before landing.
pub const BUFFER_FRAMES: u8 = 6;
pub const INVULN_FRAMES: u16 = 60;
/// Frames without control after a hit.
pub const HURT_FRAMES: u8 = 14;
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
    pub fire_cooldown: u8,
    /// Frames the fire button has been held, for the charged beam.
    pub charge: u16,
    /// Frames left showing the firing pose.
    pub recoil: u8,
    pub aim_up: bool,
    /// Frames left in the knocked-out pose before respawning.
    pub dead_timer: u16,
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
            fire_cooldown: 0,
            charge: 0,
            recoil: 0,
            aim_up: false,
            dead_timer: 0,
        }
    }

    pub fn is_dead(&self) -> bool {
        self.state == PlayerState::Dead
    }

    /// One frame of movement. Ignores input while hurt.
    pub fn update(&mut self, input: &Input, level: &Level, events: &mut Events) -> Contacts {
        let control = self.hurt == 0;
        let body = &mut self.body;

        let dir = if control { input.dir_x() } else { 0 };
        if dir != 0 {
            self.facing = dir;
            let accel = if self.grounded { RUN_ACCEL_FX } else { AIR_ACCEL_FX };
            // Turning around on the ground is snappy: drop the opposing speed first.
            if self.grounded && body.vx_fx * dir < 0 {
                body.vx_fx = 0;
            }
            body.vx_fx = (body.vx_fx + dir * accel).clamp(-RUN_MAX_FX, RUN_MAX_FX);
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
        body.vy_fx = (body.vy_fx + GRAVITY_FX).min(MAX_FALL_FX);

        let contacts = body.step(level);
        self.grounded = contacts.ground;
        self.aim_up = control && input.up();

        self.hurt = self.hurt.saturating_sub(1);
        self.invuln = self.invuln.saturating_sub(1);
        self.fire_cooldown = self.fire_cooldown.saturating_sub(1);
        self.recoil = self.recoil.saturating_sub(1);
        self.state = if self.hurt > 0 {
            PlayerState::Hurt
        } else if !self.grounded {
            if body.vy_fx < 0 {
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
        if self.invuln > 0 || self.is_dead() {
            return false;
        }
        self.hp -= damage;
        self.invuln = INVULN_FRAMES;
        self.hurt = HURT_FRAMES;
        self.charge = 0;
        let away = if self.body.rect().centre_x() < from_x { -1 } else { 1 };
        self.body.vx_fx = away * 3 * FX / 2;
        self.body.vy_fx = -2 * FX;
        self.grounded = false;
        true
    }
}

fn approach(value: i32, target: i32, step: i32) -> i32 {
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
        level::Theme,
        physics::Body,
    };

    const FLAT: Level = Level {
        name: "TEST",
        subtitle: "",
        theme: Theme::Canyon,
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
            step(&mut p, keys::A, keys::A);
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
}
