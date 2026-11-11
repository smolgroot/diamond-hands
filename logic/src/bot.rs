//! A simple route bot for tests. It runs right, fires constantly, and jumps at walls,
//! pits and bots, and waits at electric fences for the current to go off. At an elevator it switches the terminal on, waits for
//! the doors and rides up. In the boss arena it holds a spot near the left wall, faces
//! the boss and jumps charges, lunges and low shots. Playing in god mode, reaching
//! the end of a level shows the route has no wall or gap the wassie cannot clear.
//!
//! An explorable level lists waypoints ([`crate::level::Level::route`]): [`Bot`] walks
//! to each in turn, dropping down to those below and jumping (riding the jetpack once
//! it has it) up to those above, then runs right as usual. Wherever Up would do
//! something on the way (a dark terminal, an open elevator) it presses it.

use crate::{
    TILE, World,
    boss::BossState,
    world::{LIFT_W, LiftState},
    enemy::ActorKind,
    game::keys,
    level::Tile,
    shot::ShotKind,
};

/// How near a beacon a waypoint must be, in tiles across plus down, to pick the route
/// up from it after a knockout.
const REROUTE_TILES: i32 = 20;

/// The route bot, with its place along the level's waypoints.
#[derive(Clone, Copy, Debug, Default)]
pub struct Bot {
    level: u8,
    step: usize,
    /// Knocked out last frame: back at a beacon, it finds its place on the route again.
    down: bool,
}

impl Bot {
    /// Buttons to hold this frame, as [`keys_for`], following the level's route first.
    pub fn keys(&mut self, world: &World, previous: u16) -> u16 {
        if world.level_index != self.level {
            *self = Bot { level: world.level_index, ..Bot::default() };
        }
        let route = world.level.route;
        let dead = world.player.is_dead();
        if self.down && !dead {
            // Pick up from the waypoint nearest the beacon, if one is near: a beacon
            // past the route's end (up the elevator, say) is far from them all.
            let r = world.player.body.rect();
            let (x, y) = (r.centre_x().div_euclid(TILE), r.bottom().div_euclid(TILE) - 1);
            let distance = |w: &(i32, i32)| (w.0 - x).abs() + (w.1 - y).abs();
            if let Some((i, _)) = route.iter().enumerate().filter(|(_, w)| distance(w) <= REROUTE_TILES).min_by_key(|(_, w)| distance(w)) {
                self.step = i;
            }
        }
        self.down = dead;
        while let Some(&target) = route.get(self.step)
            && reached(world, target)
        {
            self.step += 1;
        }
        let p = &world.player;
        if !world.arena && !dead && p.grounded && p.hurt == 0 && world.up_prompt().is_some() {
            // Where Up does something on the way (a breaker), it presses it.
            return if previous & keys::UP == 0 { keys::UP } else { 0 };
        }
        match route.get(self.step) {
            Some(&target) if !world.arena && !dead => toward(world, previous, target),
            _ => keys_for(world, previous),
        }
    }

    /// How many waypoints it has reached.
    pub fn step(&self) -> usize {
        self.step
    }
}

/// Centre x and feet of waypoint `(tx, row)`, in pixels.
fn waypoint((tx, row): (i32, i32)) -> (i32, i32) {
    (tx * TILE + TILE / 2, (row + 1) * TILE)
}

fn reached(world: &World, target: (i32, i32)) -> bool {
    let p = &world.player;
    let r = p.body.rect();
    let (x, feet) = waypoint(target);
    p.grounded && (r.centre_x() - x).abs() <= 6 && (r.bottom() - feet).abs() <= 4
}

/// Heads for a waypoint: walks over to it, dropping off ledges for one below, and
/// jumps and hovers for one above.
fn toward(world: &World, previous: u16, target: (i32, i32)) -> u16 {
    let p = &world.player;
    let level = world.level;
    let r = p.body.rect();
    let (x, feet) = waypoint(target);
    let dx = x - r.centre_x();
    let dir = if dx.abs() <= 3 { 0 } else { dx.signum() };
    // Well above, and close enough to go straight up for it. Once on the way up, keep
    // climbing until the feet clear it.
    let near = dx.abs() < 64;
    let above = near && feet < r.bottom() - 12;
    // Not while settling onto it from above, so no jump is buffered for the landing.
    let climbing = near && r.bottom() >= feet - 2 && (p.body.vy_fx <= 0 || r.bottom() > feet + 8);
    let below = feet > r.bottom() + 4;
    let mut out = if previous & keys::B == 0 { keys::B } else { 0 };
    let side = |d: i32| match d {
        1 => keys::RIGHT,
        -1 => keys::LEFT,
        _ => 0,
    };
    // An electric fence ahead: stop short unless it stays dead long enough to cross.
    let probe_x = if dir < 0 { r.x - 20 } else { r.right() };
    let fence_ahead = dir != 0 && level.fence_hit(probe_x, r.y, 20, r.h).is_some();
    if fence_ahead && p.grounded && crate::world::fence_open_for(world.frames) < 40 {
        return out;
    }

    if !p.grounded {
        if climbing {
            // Keep climbing; only drift sideways where no rock overhangs the way.
            let ahead = (if dir < 0 { r.x - 3 } else { r.right() + 2 }).div_euclid(TILE);
            let rows = (feet.div_euclid(TILE) - 1)..=(r.bottom() - 1).div_euclid(TILE);
            let blocked = rows.into_iter().any(|ty| level.tile(ahead, ty) == Tile::Solid);
            return out | keys::A | if blocked { 0 } else { side(dir) };
        }
        out |= side(dir);
        // Full-height jumps on the way, but a short hop when about to drop down to
        // the waypoint.
        let dropping = below && dx.abs() < 20;
        return out | if p.body.vy_fx < 0 && !dropping && !p.hovering { previous & keys::A } else { 0 };
    }
    out |= side(dir);
    if previous & keys::A != 0 {
        return out;
    }
    if above {
        return out | keys::A;
    }
    if dir == 0 || dx.abs() <= 16 {
        return out;
    }
    let front = if dir > 0 { r.right() } else { r.x };
    let feet_row = r.bottom().div_euclid(TILE);
    let wall = (r.y.div_euclid(TILE)..=(r.bottom() - 1).div_euclid(TILE))
        .any(|ty| level.tile((front + dir * 3).div_euclid(TILE), ty) == Tile::Solid);
    let ahead = (front + dir * 4).div_euclid(TILE);
    let gap = !(below && dx.abs() < 48) && (feet_row..feet_row + 6).all(|ty| !level.is_floor(ahead, ty));
    let villain = world.actors.iter().any(|a| {
        let ar = a.rect();
        let gap = if dir > 0 { ar.x - front } else { front - ar.right() };
        a.is_threat() && a.kind != ActorKind::Drone && (0..24).contains(&gap) && ar.y < r.bottom() && r.y < ar.bottom()
    });
    if wall || gap || villain {
        out |= keys::A;
    }
    out
}

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
    // An electric fence ahead: stop short unless it stays dead long enough to cross.
    let r = p.body.rect();
    let fence_ahead = world.level.fence_hit(r.right(), r.y, 20, r.h).is_some();
    if fence_ahead && p.grounded && crate::world::fence_open_for(world.frames) < 40 {
        return out;
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
    let villain = world.actors.iter().any(|a| {
        let ar = a.rect();
        a.is_threat()
            && a.kind != ActorKind::Drone
            && (0..24).contains(&(ar.x - front))
            && ar.y < r.bottom()
            && r.y < ar.bottom()
    });
    if (wall || gap || villain) && previous & keys::A == 0 {
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
