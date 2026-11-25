//! Level 4: Ethereum City, through the Wormhole from Sol City. A New York at dusk:
//! brownstones and fire escapes, the subway, and midtown's towers, with a giant ape on
//! the tallest of them. Sol City was Miami; this is Manhattan.
//!
//! The map is painted in GBA-Studio as five rooms (`project/studio/rooms/`: `eth-office`,
//! `eth-avenue`, `eth-subway`, `eth-tower` and `ape-roof`), which `build.rs` stitches where
//! they sit on the world map, as for Sol City. Each room is a camera zone. CL207's lines
//! come from the `talk` property of the `cl207` entity. The avenue, the subway and the
//! tower are drawn in tilesets made with the studio's `tileset_generate`; the office
//! uses the Wormhole building's.
//!
//! The way through (rows 0-59 of one map, columns 0-329):
//!
//! - the Wormhole building's office (columns 0-59, rows 0-39), where the wassie comes
//!   out of the portal on the top floor. CL207 waits there and hands over the plasma
//!   shield: it soaks up hits, a charge each, and green emeralds charge it back up. A
//!   hole in the floor (34-43) drops to the lobby, whose door opens onto the avenue;
//! - the avenue (60-179, rows 0-39): Bad Cop bots walk the beat behind riot shields
//!   that stop bolts from the front, and lob canisters. Fire escapes zigzag up the
//!   brownstones to the roofs, with emeralds and diamonds up there. At the east end, a
//!   building walls the avenue off: the subway's stairs (162-167) go down in front of it;
//! - the subway (150-269, rows 40-59): east along the platform, jumping the track pits
//!   (the third rail is live), to the stairs up into the tower's basement;
//! - the skyscraper (270-299, rows 0-59): up on steel girders, floor after floor, to the
//!   top floor and out onto the roof;
//! - the roof (300-329, rows 0-19): BORED KONG's arena, one screen wide. Beating it
//!   clears the level.

use crate::{
    level::{BossKind, Level, Markers, Theme, Worth},
    talk::{Line, Speaker},
};

include!(concat!(env!("OUT_DIR"), "/eth_city.rs"));

/// The route bot's way through: to CL207, down the hole to the lobby, out along the
/// avenue to the subway's stairs, east through the station over the track pits, up the
/// tower's girders to the top floor, and out onto the roof.
static ROUTE: [(i32, i32); 17] = [
    (18, 15),
    (40, 35),
    (62, 35),
    (160, 35),
    (166, 53),
    (264, 53),
    (275, 53),
    (289, 49),
    (281, 45),
    (284, 41),
    (290, 37),
    (281, 33),
    (284, 29),
    (290, 25),
    (283, 21),
    (274, 19),
    (274, 15),
];

static MARKERS: Markers = Markers::scan(ROWS);

/// BORED KONG, when the roof's arena closes behind the wassie.
static BOSS_TALK: [Line; 4] = [
    Line { speaker: Speaker::Ape, rows: &["...YAWN. ANOTHER ONE", "CLIMBING MY TOWER."] },
    Line { speaker: Speaker::Ape, rows: &["I'M SO BORED. FLOOR", "PRICE, ROOF PRICE..."] },
    Line { speaker: Speaker::Ape, rows: &["SAME THING. LET'S SEE", "IF YOU'RE ANY FUN."] },
    Line { speaker: Speaker::Wassie, rows: &["NICE JPEG. I'M HERE", "TO RIGHT-CLICK IT."] },
];

pub static ETH_CITY: Level = Level {
    name: "ETHEREUM CITY",
    subtitle: "THE CITY THAT NEVER SLEEPS",
    theme: Theme::EthCity,
    boss_name: "BORED KONG",
    boss_talk: &BOSS_TALK,
    boss: BossKind::Ape,
    call: &[],
    coins: false,
    stages: &[],
    parts_base: 3,
    explore: true,
    sky_rows: 0,
    route: &ROUTE,
    bands: &[],
    zones: &ZONES,
    strata: &[],
    areas: &[],
    backdrop_y: 0,
    rows: ROWS,
    npcs: &TALKS,
    studio: true,
    markers: Some(&MARKERS),
    worth: &WORTH,
};
