//! Checks every level's map for authoring mistakes, then plays it with the route bot.

use diamond_hands_logic::{
    Difficulty, Input, Tile, World, bot,
    boss::BossState,
    enemy::{ActorKind, MAX_ACTORS},
    level::ARENA_TILES,
    levels,
    player::{JUMP_APEX, PLAYER_H},
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
        assert!(world.sats > 0 && world.kills > 0);
        assert!(arena < 6_000, "{}: took {arena} frames to reach the arena", level.name);
    }
}
