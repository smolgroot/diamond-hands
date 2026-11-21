//! Level 3: Sol City, the morning after the casino. A Miami of the Solana ecosystem:
//! beach, cafés and palms, then the MagicEden dino zoo, the harbor and the Wormhole
//! Bridge to Ethereum City.
//!
//! The map is painted in GBA-Studio as six rooms (`project/studio/rooms/`: `beach`,
//! `harbor`, `jurassic-zoo`, `service-tunnels`, `marina` and `wormhole-hq`), each in its
//! own tileset, which `build.rs` stitches where they sit on the world map and turns into
//! rows of the `level.rs` legend, with the zookeeper's and the cosmonaut's lines from
//! their `talk` properties. Each room is a camera zone (the beach and the harbor, one
//! tileset side by side, share one): the camera stays in it and cuts across at a
//! doorway. Edit them in the studio. The zoo, the tunnels and the Wormhole building were
//! laid out with the studio's `tileset_generate` and `room_generate`, then given their
//! doorways and people by hand.
//!
//! The way through (rows 0-59 of one tall map):
//!
//! - the beach (columns 0-29) and the café terrace (30-59), where the MagicEden
//!   zookeeper explains: a power cut, the fences dead, the clay dinos loose;
//! - the Jurassic Zoo (60-149, rows 0-39): a lower paddock of loose dinos, a jungle
//!   canopy above it up the girders at the east end, and a service shaft (92-101) down
//!   to the tunnels. The gate to the harbor (146) holds until the power is back;
//! - the service tunnels (rows 40-59): the generator's breaker at the west end (70).
//!   Thrown, the fences hold the dinos again and the zoo's gate opens. Girders beside
//!   the shaft (98-101) climb back up;
//! - the harbor (150-209): piers over the water, containers stacked up to a crane, and
//!   the Wormhole building's front door, jammed by blockade bots that can't be beaten;
//! - the Wormhole building (210-269): in off the crane's jib at its roof (row 15),
//!   through a skylight and down a zigzag of floors to the portal hall, where the
//!   cosmonaut wassie waits by the portal.

use crate::{
    level::{Level, Markers, Theme},
    talk::{Line, Speaker},
};

include!(concat!(env!("OUT_DIR"), "/sol_city.rs"));

/// The route bot's way through: down the service shaft, off the landing girder to the
/// tunnel floor, west to the breaker, back up the girders, through the zoo's gate, over
/// the harbor's containers and crane onto the roof, and down the building's floors to
/// the portal.
static ROUTE: [(i32, i32); 27] = [
    (93, 45),
    (99, 55),
    (87, 51),
    (74, 53),
    (70, 55),
    (74, 53),
    (87, 51),
    (94, 49),
    (94, 45),
    (99, 41),
    (99, 37),
    (104, 35),
    (150, 35),
    (195, 29),
    (199, 26),
    (205, 22),
    (200, 18),
    (204, 15),
    (215, 15),
    (232, 21),
    (236, 29),
    (232, 33),
    (236, 41),
    (232, 45),
    (237, 57),
    (250, 57),
    (258, 57),
];

static MARKERS: Markers = Markers::scan(ROWS);

pub static SOL_CITY: Level = Level {
    name: "SOL CITY",
    subtitle: "GM FROM THE BEACH",
    theme: Theme::SolCity,
    boss_name: "",
    boss_talk: &[],
    boss: crate::level::BossKind::Bera,
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
};
