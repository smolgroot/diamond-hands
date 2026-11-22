//! Reads the levels painted in GBA-Studio (`../project/studio/`) and writes them out as
//! the map rows and dialogue `logic` plays: one character per 8x8 cell, in the legend of
//! `src/level.rs`. The studio rooms are the only place such a level is edited; this keeps
//! the rules, the host tests and the route bot on exactly the map the ROM draws.
//!
//! A level is several rooms, stitched into one map where they sit on the studio's world
//! map; cells no room covers are rock. Each room, or each run of side-by-side rooms of one
//! tileset and height, is a camera zone: the camera stays inside it, and cuts to the next
//! zone when the wassie walks through a doorway (`ZONES`, in tiles).
//!
//! A cell's character comes from its collision, resolved as the studio's build does (a
//! painted override, else the frontmost tile that has a class). An entity becomes its
//! marker on the cell its feet stand in. An entity with a `talk` property brings its
//! lines: pages split by `|`, rows by a newline, and a page starting with `>` is the
//! wassie's reply.

use std::{collections::HashMap, env, fmt::Write, fs, path::PathBuf};

use serde_json::Value;

/// Levels made in the studio: the name of the generated module, and its rooms.
const LEVELS: &[(&str, &[&str])] = &[("sol_city", &["beach", "beach-tunnels", "crab-lair", "harbor", "jurassic-zoo", "service-tunnels", "marina", "wormhole-hq"])];

/// Entity type to map marker.
const MARKERS: &[(&str, u8)] = &[
    ("player-start", b'P'),
    ("checkpoint", b'C'),
    ("diamond", b'o'),
    ("heal", b'+'),
    ("sentry", b'b'),
    ("crawler", b's'),
    ("drone", b'v'),
    ("mine", b't'),
    ("guard", b'g'),
    ("dealer", b'd'),
    ("coin", b'$'),
    ("plasma-cell", b'p'),
    ("jetpack", b'J'),
    ("breaker", b'E'),
    ("zoo-breaker", b'E'),
    ("dino", b'x'),
    ("zookeeper", b'e'),
    ("cosmo-wassie", b'c'),
    ("portal", b'w'),
    ("power-gate", b'h'),
    ("bot-blockade", b'm'),
    ("crab-bot", b'a'),
    ("arena-edge", b'X'),
    ("crab-boss", b'B'),
    ("smolsnip", b'f'),
];

/// Breakable blocks: entity type to the marker of its bottom-left tile. A block is 16x16
/// pixels with its feet at the middle of its bottom edge, on the 8 px grid, over 2x2
/// cells of air; only its marker goes in the map, since it moves (see `src/block.rs`).
const BLOCKS: &[(&str, u8)] = &[("sand-block", b'n'), ("dock-crate", b'u'), ("soil-block", b'y')];

/// The speaker of an entity type's lines.
const SPEAKERS: &[(&str, &str)] = &[("zookeeper", "Keeper"), ("cosmo-wassie", "Cosmo"), ("smolsnip", "Smolsnip")];

fn read(path: &PathBuf) -> Value {
    println!("cargo:rerun-if-changed={}", path.display());
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Run-length pairs `[count, cell, ...]` to cells.
fn decode(data: &Value, cells: usize) -> Vec<u32> {
    let pairs: Vec<u32> = data.as_array().expect("run-length data").iter().map(|v| v.as_u64().unwrap() as u32).collect();
    let mut out = Vec::with_capacity(cells);
    for pair in pairs.chunks(2) {
        out.extend(std::iter::repeat_n(pair[1], pair[0] as usize));
    }
    assert_eq!(out.len(), cells, "run-length data of the wrong size");
    out
}

fn main() {
    let studio = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../project/studio");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let project = read(&studio.join("project.json"));
    let classes: Vec<(String, String)> = project["collisionClasses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["id"].as_str().unwrap().to_owned(), c["kind"].as_str().unwrap().to_owned()))
        .collect();
    let class_char = |id: &str, kind: &str| -> u8 {
        match (kind, id) {
            ("empty", _) => b'.',
            ("solid", _) => b'#',
            ("oneway", _) => b'=',
            // In a studio level, the only hazard is water.
            ("hazard", _) => b'~',
            (_, "slope-up") => b'/',
            (_, "slope-down") => b'\\',
            (_, "slope-up-low" | "slope-up-high") => b'{',
            (_, "slope-down-high" | "slope-down-low") => b'}',
            _ => panic!("collision class {id} ({kind}) has no map character"),
        }
    };

    let world = read(&studio.join("world.json"));
    for &(module, ids) in LEVELS {
        // Each room's screen on the world map, then the box around them all.
        let placed: Vec<(&str, Value, i64, i64)> = ids
            .iter()
            .map(|&id| {
                let room = read(&studio.join(format!("rooms/{id}.json")));
                let place = world["rooms"].as_array().unwrap().iter().find(|p| p["room"] == id).unwrap_or_else(|| panic!("{id} is not on the world map"));
                (id, room, place["x"].as_i64().unwrap(), place["y"].as_i64().unwrap())
            })
            .collect();
        let sx0 = placed.iter().map(|p| p.2).min().unwrap();
        let sy0 = placed.iter().map(|p| p.3).min().unwrap();
        let sx1 = placed.iter().map(|p| p.2 + p.1["width"].as_i64().unwrap()).max().unwrap();
        let sy1 = placed.iter().map(|p| p.3 + p.1["height"].as_i64().unwrap()).max().unwrap();
        let (cols, rows) = ((sx1 - sx0) as usize * 30, (sy1 - sy0) as usize * 20);
        let mut map = vec![vec![b'#'; cols]; rows];
        let mut people: Vec<(usize, usize, Vec<String>)> = Vec::new();
        // Zones as (left, top, right, bottom) in tiles, right and bottom exclusive, with
        // the tileset they are drawn in.
        let mut zones: Vec<(usize, usize, usize, usize, String)> = Vec::new();

        for (id, room, sx, sy) in &placed {
            let (left, top) = ((*sx - sx0) as usize * 30, (*sy - sy0) as usize * 20);
            let tileset_id = room["tileset"].as_str().unwrap();
            let tileset = read(&studio.join(format!("tilesets/{tileset_id}.json")));
            let tile_class: HashMap<u32, usize> = tileset["collision"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(tile, class)| {
                    let class = classes.iter().position(|c| c.0 == class.as_str().unwrap()).expect("known class");
                    (tile.parse::<u32>().unwrap(), class)
                })
                .collect();
            let (room_cols, room_rows) = (room["width"].as_u64().unwrap() as usize * 30, room["height"].as_u64().unwrap() as usize * 20);
            let cells = room_cols * room_rows;
            let layers: Vec<Vec<u32>> = room["layers"].as_array().unwrap().iter().map(|l| decode(&l["data"], cells)).collect();
            let overrides = room.get("collision").filter(|v| !v.is_null()).map(|d| decode(d, cells));
            for row in 0..room_rows {
                for col in 0..room_cols {
                    let i = row * room_cols + col;
                    let class = match overrides.as_ref().map(|o| o[i]).filter(|&o| o > 0) {
                        Some(o) => o as usize - 1,
                        None => layers
                            .iter()
                            .rev()
                            .filter(|layer| layer[i] > 0)
                            .filter_map(|layer| tile_class.get(&(layer[i] - 1)).copied())
                            .find(|&c| classes[c].1 != "empty")
                            .unwrap_or(0),
                    };
                    map[top + row][left + col] = class_char(&classes[class].0, &classes[class].1);
                }
            }
            zones.push((left, top, left + room_cols, top + room_rows, tileset_id.to_owned()));
            // People, in map order (row by row) as `logic` numbers them: (row, col, lines).
            for entity in room["entities"].as_array().unwrap() {
            let kind = entity["type"].as_str().unwrap();
            if let Some(&(_, marker)) = BLOCKS.iter().find(|b| b.0 == kind) {
                let (x, y) = (entity["x"].as_i64().unwrap(), entity["y"].as_i64().unwrap());
                assert!(x % 8 == 0 && y % 8 == 0, "{id}: {kind} at {x},{y} is off the 8 px grid");
                let (col, row) = (left + (x as usize - 8) / 8, top + (y as usize - 16) / 8);
                for (dy, dx) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                    let cell = &mut map[row + dy][col + dx];
                    assert_eq!(*cell, b'.', "{id}: {kind} at {x},{y} overlaps a cell that isn't air ({})", *cell as char);
                    if (dy, dx) == (1, 0) {
                        *cell = marker;
                    }
                }
                continue;
            }
            let &(_, marker) = MARKERS.iter().find(|m| m.0 == kind).unwrap_or_else(|| panic!("{id}: entity type {kind} has no marker"));
            let (x, y) = (entity["x"].as_i64().unwrap(), entity["y"].as_i64().unwrap());
            let (col, row) = (left + x.div_euclid(8) as usize, top + (y - 1).div_euclid(8) as usize);
            let cell = &mut map[row][col];
            assert_eq!(*cell, b'.', "{id}: {kind} at {x},{y} stands in a cell that isn't air ({})", *cell as char);
            *cell = marker;
            if let Some(&(_, speaker)) = SPEAKERS.iter().find(|s| s.0 == kind) {
                let text = entity["props"]["talk"].as_str().unwrap_or("");
                let mut lines = Vec::new();
                for page in text.split('|').filter(|p| !p.is_empty()) {
                    let (who, page) = match page.strip_prefix('>') {
                        Some(rest) => ("Wassie", rest),
                        None => (speaker, page),
                    };
                    let rows: Vec<String> = page.split('\n').map(|r| format!("{r:?}")).collect();
                    lines.push(format!("Line {{ speaker: Speaker::{who}, rows: &[{}] }}", rows.join(", ")));
                }
                people.push((row, col, lines));
            }
        }
        }
        people.sort_by_key(|p| (p.0, p.1));
        // Side-by-side zones of one tileset and the same rows scroll as one.
        zones.sort_by_key(|z| (z.1, z.0));
        let mut merged: Vec<(usize, usize, usize, usize, String)> = Vec::new();
        for z in zones {
            match merged.iter_mut().find(|m| m.2 == z.0 && m.1 == z.1 && m.3 == z.3 && m.4 == z.4) {
                Some(m) => m.2 = z.2,
                None => merged.push(z),
            }
        }
        let mut talks = String::new();
        let mut talk_names = Vec::new();
        for (n, (_, _, lines)) in people.iter().enumerate() {
            writeln!(talks, "static TALK_{n}: [Line; {}] = [{}];", lines.len(), lines.join(", ")).unwrap();
            talk_names.push(format!("TALK_{n}"));
        }

        let mut code = String::new();
        writeln!(code, "// Generated by logic/build.rs from the studio rooms {}. Do not edit.", ids.join(", ")).unwrap();
        writeln!(code, "const ROWS: &[&str] = &[").unwrap();
        for row in &map {
            writeln!(code, "    {:?},", String::from_utf8(row.clone()).unwrap()).unwrap();
        }
        writeln!(code, "];").unwrap();
        code.push_str(&talks);
        writeln!(code, "static TALKS: [&[Line]; {}] = [{}];", talk_names.len(), talk_names.iter().map(|n| format!("&{n}")).collect::<Vec<_>>().join(", ")).unwrap();
        let zones: Vec<String> = merged.iter().map(|z| format!("({}, {}, {}, {})", z.0, z.1, z.2, z.3)).collect();
        writeln!(code, "static ZONES: [(i32, i32, i32, i32); {}] = [{}];", zones.len(), zones.join(", ")).unwrap();
        fs::write(out.join(format!("{module}.rs")), code).unwrap();
    }
}
