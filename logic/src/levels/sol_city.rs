//! Level 3: Sol City, the morning after the casino. A Miami of the Solana ecosystem:
//! beach, cafés and palms, then the MagicEden dino zoo, the harbor and the Wormhole
//! Bridge to Ethereum City.
//!
//! The map is painted in GBA-Studio (`project/studio/rooms/sol-city.json`) and turned
//! into rows of the `level.rs` legend by `build.rs`, which also brings the zookeeper's
//! and the cosmonaut's lines from the room's `talk` properties. Edit it in the studio.
//!
//! The way through, in five storeys of one tall map (rows 0-59):
//!
//! - the beach (columns 0-43) and the café terrace (44-59), where the MagicEden
//!   zookeeper explains: a power cut, the fences dead, the clay dinos loose;
//! - the zoo (60-135): three paddocks of loose dinos, perches up into the aviary on the
//!   jetpack, a staff hatch that only opens from below, and a service shaft (129-131)
//!   down to the tunnels;
//! - the service tunnels (rows 43-55): the generator's breaker at the west end (69).
//!   Thrown, the fences hold the dinos again and the gate under MagicEden's office
//!   (143) opens. The hatch's girders climb back up;
//! - the harbor (150-211): piers over the water, containers stacked up to a crane, and
//!   the Wormhole terminal's front door, jammed by blockade bots that can't be beaten;
//! - the terminal's roof (row 15) and its skylight (244-247), down into the portal hall,
//!   where the cosmonaut wassie waits by the portal.

use crate::{
    level::{Level, Markers, Theme},
    talk::{Line, Speaker},
};

include!(concat!(env!("OUT_DIR"), "/sol_city.rs"));

/// The route bot's way through: down the service shaft, west to the breaker, back up
/// the hatch's girders, then over the harbor's containers and crane onto the roof, and
/// down the skylight to the portal.
static ROUTE: [(i32, i32); 17] = [
    (130, 55),
    (70, 55),
    (74, 51),
    (76, 47),
    (74, 43),
    (76, 39),
    (75, 35),
    (150, 35),
    (195, 29),
    (199, 26),
    (205, 22),
    (200, 18),
    (204, 15),
    (225, 15),
    (245, 21),
    (250, 35),
    (258, 35),
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
    strata: &[],
    areas: &[],
    backdrop_y: 0,
    rows: ROWS,
    npcs: &TALKS,
    studio: true,
    markers: Some(&MARKERS),
};
