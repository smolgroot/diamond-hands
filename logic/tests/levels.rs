//! Checks every level's map for authoring mistakes, then plays it with the route bot.

use diamond_hands_logic::{
    Difficulty, Input, Tile, World, bot,
    boss::BossState,
    enemy::{ActorKind, MAX_ACTORS},
    keys,
    level::ARENA_TILES,
    levels,
    physics::Body,
    player::{JUMP_APEX, PLAYER_H, PLAYER_W, Player},
};

#[test]
fn maps_are_well_formed() {
    for level in levels::ALL {
        let name = level.name;
        let width = level.width();
        assert!(level.rows.iter().all(|r| r.len() as i32 == width), "{name}: ragged rows");
        assert_eq!(level.height(), 20, "{name}: one screen tall");
        let count = |c| level.markers().filter(|&(m, _, _)| m == c).count();
        assert_eq!(count(b'P'), 1, "{name}: one start");
        assert_eq!(count(b'X'), 1, "{name}: one arena");
        assert_eq!(count(b'B'), 1, "{name}: one boss");
        let actors = level
            .markers()
            .filter(|&(c, _, _)| ActorKind::from_marker(c).is_some())
            .count();
        assert!(actors <= MAX_ACTORS, "{name}: {actors} actors");

        let (arena_x, _) = level.find(b'X').unwrap();
        assert_eq!(width - arena_x, ARENA_TILES, "{name}: the arena is one screen wide");
        let (boss_x, _) = level.find(b'B').unwrap();
        assert!(boss_x > arena_x, "{name}: boss inside the arena");

        for (c, tx, ty) in level.markers() {
            assert_eq!(level.tile(tx, ty), Tile::Empty, "{name}: {} at ({tx},{ty}) is in rock", c as char);
            let grounded = matches!(c, b'P' | b'C' | b'b' | b's' | b't' | b'B');
            if grounded {
                assert!(level.is_floor(tx, ty + 1), "{name}: {} at ({tx},{ty}) floats", c as char);
            }
        }
    }
}

/// Every rise along the ground must be within a full jump: compares the height of
/// neighbouring ground columns. Ground is rock connected to the bottom of the map, so
/// floating ledges and arches overhead don't count.
#[test]
fn steps_are_jumpable() {
    for level in levels::ALL {
        let floor = |tx: i32| {
            let bottom = level.height() - 1;
            if level.tile(tx, bottom) != Tile::Solid {
                return None;
            }
            let mut top = bottom;
            while top > 0 && level.tile(tx, top - 1) == Tile::Solid {
                top -= 1;
            }
            Some(top)
        };
        let mut last = floor(0);
        for tx in 1..level.width() {
            let here = floor(tx);
            if let (Some(a), Some(b)) = (last, here) {
                let rise = (a - b) * 8;
                assert!(rise < JUMP_APEX, "{}: {rise}px wall at column {tx}", level.name);
            }
            if here.is_some() {
                last = here;
            }
        }
    }
}

/// Nowhere the wassie can stand has a rock ceiling lower than it is tall: such a gap
/// would trap it.
#[test]
fn every_standing_spot_has_headroom() {
    let need = (PLAYER_H + 7) / 8;
    for level in levels::ALL {
        for tx in 0..level.width() {
            for ty in 1..level.height() {
                let standable = level.tile(tx, ty) != Tile::Solid && level.is_floor(tx, ty + 1);
                if !standable {
                    continue;
                }
                let room = (0..=ty).take_while(|&up| level.tile(tx, ty - up) != Tile::Solid).count() as i32;
                assert!(
                    room >= need || room == ty + 1,
                    "{}: only {room} tiles of headroom at ({tx},{ty})",
                    level.name
                );
            }
        }
    }
}

#[test]
fn bot_clears_every_level_in_god_mode() {
    for (index, level) in levels::ALL.iter().enumerate() {
        let mut world = World::new(index as u8, Difficulty::Normal);
        world.god = true;
        let mut previous = 0;
        let mut reached_arena = None;
        for frame in 0..30_000 {
            let keys = bot::keys_for(&world, previous);
            world.step(&Input::from_keys(keys, previous));
            previous = keys;
            if world.arena && reached_arena.is_none() {
                reached_arena = Some(frame);
            }
            if world.cleared {
                break;
            }
        }
        let arena = reached_arena.unwrap_or_else(|| {
            panic!(
                "{}: stuck at x={} y={}",
                level.name,
                world.player.body.x(),
                world.player.body.y()
            )
        });
        assert!(world.cleared, "{}: boss not beaten, state {:?} hp {}", level.name, world.boss.state, world.boss.hp);
        assert_eq!(world.boss.state, BossState::Defeated);
        assert!(world.diamonds > 0 && world.kills > 0);
        assert!(arena < 6_000, "{}: took {arena} frames to reach the arena", level.name);
    }
}

/// Diamonds on floating rock must be reachable: from the floor just left of the rock
/// face, holding jump rides the jetpack up, Right once above the top drifts over it,
/// and letting go there drops onto it.
#[test]
fn floating_diamond_ledges_are_reachable_on_the_jetpack() {
    for (index, level) in levels::ALL.iter().enumerate() {
        let mut checked = 0;
        for (c, tx, ty) in level.markers() {
            let top = ty + 1;
            let floating = level.tile(tx, top) == Tile::Solid
                && (top..level.height()).any(|y| level.tile(tx, y) == Tile::Empty);
            if c != b'o' || !floating {
                continue;
            }
            let mut left = tx;
            while level.tile(left - 1, top) == Tile::Solid && level.tile(left - 1, top - 1) != Tile::Solid {
                left -= 1;
            }
            // Hug the rock face, standing on whatever floor is below.
            let x = left * 8 - PLAYER_W - 1;
            let cols = [x.div_euclid(8), (x + PLAYER_W - 1).div_euclid(8)];
            let standable = |y: i32| {
                cols.iter().all(|&c| level.tile(c, y) != Tile::Solid)
                    && cols.iter().any(|&c| level.is_floor(c, y + 1))
            };
            let Some(floor) = (top..level.height()).find(|&y| standable(y)) else {
                continue;
            };
            let mut world = World::new(index as u8, Difficulty::Normal);
            world.god = true;
            for a in world.actors.iter_mut().filter(|a| a.kind.is_enemy()) {
                a.alive = false;
            }
            let mut body = Body::standing_on_tile(0, floor, PLAYER_W, PLAYER_H);
            body.x_fx = x * diamond_hands_logic::FX;
            world.player = Player::new(body);
            let mut previous = 0;
            let mut landed = false;
            for _ in 0..240 {
                let r = world.player.body.rect();
                let above = r.bottom() <= top * 8;
                let over = r.centre_x() >= left * 8 + 4;
                let held = if over { 0 } else { keys::A } | if above { keys::RIGHT } else { 0 };
                world.step(&Input::from_keys(held, previous));
                previous = held;
                let p = &world.player;
                if p.grounded && p.body.bottom() == top * 8 {
                    landed = true;
                    break;
                }
            }
            let rise = (floor + 1 - top) * 8;
            assert!(landed, "{}: diamond at ({tx},{ty}) on rock {rise} px up is out of reach", level.name);
            checked += 1;
        }
        assert!(checked > 0, "{}: no floating ledges checked", level.name);
    }
}

