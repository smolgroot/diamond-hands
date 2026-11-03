//! A simple route bot for tests. It runs right, fires constantly, and jumps at walls,
//! pits, cacti and villains. In the boss arena it holds a spot near the left wall,
//! faces the sheriff and jumps her charge and low shots. Playing in god mode, reaching
//! the end of a level shows the route has no wall or gap the wassie cannot clear.

use crate::{
    TILE, World,
    boss::BossState,
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
    out |= keys::RIGHT;
    if !p.grounded {
        // Hold for full-height jumps.
        return out | (previous & keys::A);
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
    let cactus = (1..=3).any(|d| level.tile((front + d * 4).div_euclid(TILE), feet_row - 1) == Tile::Spike);
    let villain = world.actors.iter().any(|a| {
        let ar = a.rect();
        a.is_threat()
            && a.kind != ActorKind::Vulture
            && (0..24).contains(&(ar.x - front))
            && ar.y < r.bottom()
            && r.y < ar.bottom()
    });
    if (wall || gap || cactus || villain) && previous & keys::A == 0 {
        out |= keys::A;
    }
    out
}

fn arena_keys(world: &World, previous: u16) -> u16 {
    let p = &world.player;
    let r = p.body.rect();
    let boss = world.boss.rect();
    // Face the sheriff (she ends up behind after a charge), otherwise hold a spot left
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
        return out | (previous & keys::A);
    }
    let charging = matches!(world.boss.state, BossState::Charge | BossState::Leap)
        && (boss.centre_x() - r.centre_x()).abs() < 72;
    let low_shot = world.shots.iter().any(|s| {
        let sr = s.rect();
        s.active
            && !s.kind.from_player()
            && matches!(s.kind, ShotKind::Bullet | ShotKind::Shockwave)
            && sr.y > r.y + 12
            && (sr.centre_x() - r.centre_x()).abs() < 40
    });
    if (charging || low_shot) && previous & keys::A == 0 {
        out |= keys::A;
    }
    out
}
