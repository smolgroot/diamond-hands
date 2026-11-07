//! A simple route bot for tests. It runs right, fires constantly, and jumps at walls,
//! pits, energy spikes and bots. At an elevator it switches the terminal on, waits for
//! the doors and rides up. In the boss arena it holds a spot near the left wall, faces
//! the boss and jumps charges, lunges and low shots. Playing in god mode, reaching
//! the end of a level shows the route has no wall or gap the wassie cannot clear.

use crate::{
    TILE, World,
    boss::BossState,
    world::{LIFT_W, LiftState},
    enemy::ActorKind,
    game::keys,
    level::Tile,
    shot::ShotKind,
};

/// Buttons to hold this frame. `previous` is last frame's keys: jumping and firing
/// need fresh presses, so the bot releases for a frame when still holding from before.
pub fn keys_for(world: &World, previous: u16) -> u16 {
    let p = &world.player;
    if p.is_dead() {
        return 0;
    }
    let mut out = 0;
    if previous & keys::B == 0 {
        out |= keys::B;
    }
    if world.arena {
        return out | arena_keys(world, previous);
    }
    if let Some(keys) = lift_keys(world, previous) {
        return keys;
    }
    out |= keys::RIGHT;
    if !p.grounded {
        // Hold for full-height jumps, but let go at the top: holding on would hover.
        return out | if p.body.vy_fx < 0 { previous & keys::A } else { 0 };
    }

    let level = world.level;
    let r = p.body.rect();
    let front = r.right();
    let feet_row = r.bottom().div_euclid(TILE);
    let body_rows = r.y.div_euclid(TILE)..=(r.bottom() - 1).div_euclid(TILE);
    let wall = body_rows
        .into_iter()
        .any(|ty| level.tile((front + 3).div_euclid(TILE), ty) == Tile::Solid);
    // A pit, as opposed to a step down: no floor ahead within a few rows. Steps down
    // are walked off; leaping off a high ledge can overshoot into the next pit.
    let ahead = (front + 4).div_euclid(TILE);
    let gap = (feet_row..feet_row + 6).all(|ty| !level.is_floor(ahead, ty));
    let spikes = (1..=3).any(|d| level.tile((front + d * 4).div_euclid(TILE), feet_row - 1) == Tile::Spike);
    let villain = world.actors.iter().any(|a| {
        let ar = a.rect();
        a.is_threat()
            && a.kind != ActorKind::Drone
            && (0..24).contains(&(ar.x - front))
            && ar.y < r.bottom()
            && r.y < ar.bottom()
    });
    if (wall || gap || spikes || villain) && previous & keys::A == 0 {
        out |= keys::A;
    }
    out
}

/// At the elevator: Up at the dark terminal, then into the car once it opens.
fn lift_keys(world: &World, previous: u16) -> Option<u16> {
    let lift = world.lift?;
    let p = &world.player;
    let r = p.body.rect();
    let (car_x, feet) = lift.car;
    if lift.state == LiftState::Done || r.x > car_x + LIFT_W || (feet - r.bottom()).abs() > 8 || !p.grounded {
        return None;
    }
    let tap_up = if previous & keys::UP == 0 { keys::UP } else { 0 };
    let terminal = world.actors.iter().find(|a| a.kind == ActorKind::Terminal && a.alive)?;
    let t = terminal.rect();
    if terminal.state != crate::enemy::state::LIT {
        if r.right() < t.x + 2 {
            return None;
        }
        if r.overlaps(&t) {
            return Some(tap_up);
        }
        return Some(keys::LEFT);
    }
    if r.right() < car_x - 16 {
        return None;
    }
    let centre = car_x + LIFT_W / 2;
    let dx = centre - r.centre_x();
    Some(match lift.state {
        _ if dx.abs() > 3 => if dx > 0 { keys::RIGHT } else { keys::LEFT },
        LiftState::Open => tap_up,
        _ => 0,
    })
}

fn arena_keys(world: &World, previous: u16) -> u16 {
    let p = &world.player;
    let r = p.body.rect();
    let boss = world.boss.rect();
    // Face the mech (she ends up behind after a charge), otherwise hold a spot left
    // of the arena's first plank: bolts fired from up there fly over her head.
    let mut out = 0;
    let boss_left = boss.centre_x() < r.centre_x();
    let on_plank = p.grounded && r.bottom() < boss.bottom() - 4;
    if on_plank || (boss_left && p.facing > 0) {
        out |= keys::LEFT;
    } else if !boss_left && (p.facing < 0 || r.x < world.boss.arena.0 + 16) {
        out |= keys::RIGHT;
    }
    if !p.grounded {
        return out | if p.body.vy_fx < 0 { previous & keys::A } else { 0 };
    }
    let charging = matches!(world.boss.state, BossState::Charge | BossState::Leap | BossState::Swing)
        && (boss.centre_x() - r.centre_x()).abs() < 72;
    let low_shot = world.shots.iter().any(|s| {
        let sr = s.rect();
        s.active
            && !s.kind.from_player()
            && matches!(s.kind, ShotKind::Bullet | ShotKind::Shockwave | ShotKind::MegaCucumber)
            && sr.y > r.y + 12
            && (sr.centre_x() - r.centre_x()).abs() < 48
    });
    if (charging || low_shot) && previous & keys::A == 0 {
        out |= keys::A;
    }
    out
}
