//! Reads the levels painted in GBA-Studio (`../project/studio/`) and writes them out as
//! the map rows and dialogue `logic` plays: one character per 8x8 cell, in the legend of
//! `src/level.rs`. The studio room is the only place such a level is edited; this keeps
//! the rules, the host tests and the route bot on exactly the map the ROM draws.
//!
//! A cell's character comes from its collision, resolved as the studio's build does (a
//! painted override, else the frontmost tile that has a class). An entity becomes its
//! marker on the cell its feet stand in. An entity with a `talk` property brings its
//! lines: pages split by `|`, rows by a newline, and a page starting with `>` is the
//! wassie's reply.

use std::{collections::HashMap, env, fmt::Write, fs, path::PathBuf};

use serde_json::Value;

/// Studio rooms played as levels: room id, and the name of the generated module.
const ROOMS: &[(&str, &str)] = &[("sol-city", "sol_city")];

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
];

/// The speaker of an entity type's lines.
const SPEAKERS: &[(&str, &str)] = &[("zookeeper", "Keeper"), ("cosmo-wassie", "Cosmo")];

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

    for &(id, module) in ROOMS {
        let room = read(&studio.join(format!("rooms/{id}.json")));
        let tileset = read(&studio.join(format!("tilesets/{}.json", room["tileset"].as_str().unwrap())));
        let tile_class: HashMap<u32, usize> = tileset["collision"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(tile, class)| {
                let class = classes.iter().position(|c| c.0 == class.as_str().unwrap()).expect("known class");
                (tile.parse::<u32>().unwrap(), class)
            })
            .collect();
        let (cols, rows) = (room["width"].as_u64().unwrap() as usize * 30, room["height"].as_u64().unwrap() as usize * 20);
        let cells = cols * rows;
        let layers: Vec<Vec<u32>> = room["layers"].as_array().unwrap().iter().map(|l| decode(&l["data"], cells)).collect();
        let overrides = room.get("collision").filter(|v| !v.is_null()).map(|d| decode(d, cells));

        let mut map: Vec<Vec<u8>> = (0..rows)
            .map(|row| {
                (0..cols)
                    .map(|col| {
                        let i = row * cols + col;
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
                        class_char(&classes[class].0, &classes[class].1)
                    })
                    .collect()
            })
            .collect();

        // People, in map order (row by row) as `logic` numbers them: (row, col, lines).
        let mut people: Vec<(usize, usize, Vec<String>)> = Vec::new();
        for entity in room["entities"].as_array().unwrap() {
            let kind = entity["type"].as_str().unwrap();
            let &(_, marker) = MARKERS.iter().find(|m| m.0 == kind).unwrap_or_else(|| panic!("{id}: entity type {kind} has no marker"));
            let (x, y) = (entity["x"].as_i64().unwrap(), entity["y"].as_i64().unwrap());
            let (col, row) = (x.div_euclid(8) as usize, (y - 1).div_euclid(8) as usize);
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
        people.sort_by_key(|p| (p.0, p.1));
        let mut talks = String::new();
        let mut talk_names = Vec::new();
        for (n, (_, _, lines)) in people.iter().enumerate() {
            writeln!(talks, "static TALK_{n}: [Line; {}] = [{}];", lines.len(), lines.join(", ")).unwrap();
            talk_names.push(format!("TALK_{n}"));
        }

        let mut code = String::new();
        writeln!(code, "// Generated by logic/build.rs from project/studio/rooms/{id}.json. Do not edit.").unwrap();
        writeln!(code, "const ROWS: &[&str] = &[").unwrap();
        for row in &map {
            writeln!(code, "    {:?},", String::from_utf8(row.clone()).unwrap()).unwrap();
        }
        writeln!(code, "];").unwrap();
        code.push_str(&talks);
        writeln!(code, "static TALKS: [&[Line]; {}] = [{}];", talk_names.len(), talk_names.iter().map(|n| format!("&{n}")).collect::<Vec<_>>().join(", ")).unwrap();
        fs::write(out.join(format!("{module}.rs")), code).unwrap();
    }
}
