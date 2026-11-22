//! Level 3: Sol City, the morning after the casino. A Miami of the Solana ecosystem:
//! beach, cafés and palms, then the MagicEden dino zoo, the harbor and the Wormhole
//! Bridge to Ethereum City.
//!
//! The map is painted in GBA-Studio as eight rooms (`project/studio/rooms/`: `beach`,
//! `beach-tunnels`, `crab-lair`, `harbor`, `jurassic-zoo`, `service-tunnels`, `marina` and
//! `wormhole-hq`), each in its own tileset, which `build.rs` stitches where they sit on
//! the world map and turns into rows of the `level.rs` legend, with the zookeeper's and the cosmonaut's lines from
//! their `talk` properties. Each room is a camera zone (the beach and the harbor, one
//! tileset side by side, share one): the camera stays in it and cuts across at a
//! doorway. Edit them in the studio. The zoo, the tunnels and the Wormhole building were
//! laid out with the studio's `tileset_generate` and `room_generate`, then given their
//! doorways and people by hand.
//!
//! The way through (rows 0-59 of one tall map):
//!
//! - the beach (columns 0-89, rows 0-39), where crab bots scuttle over the sand. Two
//!   shafts in it (38-40 and 66-68) are plugged by sand blocks: shot apart, they drop
//!   into the beach tunnels (rows 40-59), a gallery of diamonds under the sand that
//!   climbs back out up a stack of planks through a hole at 79-82;
//! - the crab lair (columns 30-89, rows 60-79), down a hole in the tunnels' floor
//!   (33-41) and a zigzag of planks: a beacon at the foot of the shaft, then the arena
//!   (42-71) of the Big Crab Bot, the level's only boss, which doesn't end it. Its
//!   prisoner Smolsnip waits behind sand blocks at the arena's east end (72-77): they
//!   hold while the fight is on, and once the crab is beaten Smolsnip pays 50
//!   diamonds;
//! - the café terrace (90-119), where the MagicEden zookeeper explains: a power cut,
//!   the fences dead, the clay dinos loose;
//! - the Jurassic Zoo (120-209, rows 0-39): a lower paddock of loose dinos, a jungle
//!   canopy above it up the girders at the east end, and a service shaft (152-161) down
//!   to the tunnels. The gate to the harbor (206) holds until the power is back;
//! - the service tunnels (rows 40-59): the generator's breaker at the west end (130).
//!   Thrown, the fences hold the dinos again and the zoo's gate opens. Girders beside
//!   the shaft (158-161) climb back up;
//! - the harbor (210-269): piers over the water, containers stacked up to a crane, and
//!   the Wormhole building's front door, jammed by blockade bots that can't be beaten;
//! - the Wormhole building (270-329): in off the crane's jib at its roof (row 15),
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
    (153, 45),
    (159, 55),
    (147, 51),
    (134, 53),
    (130, 55),
    (134, 53),
    (147, 51),
    (154, 49),
    (154, 45),
    (159, 41),
    (159, 37),
    (164, 35),
    (210, 35),
    (255, 29),
    (259, 26),
    (265, 22),
    (260, 18),
    (264, 15),
    (275, 15),
    (292, 21),
    (296, 29),
    (292, 33),
    (296, 41),
    (292, 45),
    (297, 57),
    (310, 57),
    (318, 57),
];

static MARKERS: Markers = Markers::scan(ROWS);

/// The Big Crab Bot, when its arena closes behind the wassie.
static BOSS_TALK: [Line; 4] = [
    Line { speaker: Speaker::CrabBot, rows: &["WHO ARE YOU? HOW DARE", "YOU COME DOWN HERE?"] },
    Line { speaker: Speaker::CrabBot, rows: &["I'M DOING EXPERIMENTS", "ON THIS WEIRD CRAB", "SPECIMEN..."] },
    Line { speaker: Speaker::CrabBot, rows: &["...TO BUILD MY CRAB", "BOT ARMY! SNIP SNIP!"] },
    Line { speaker: Speaker::Wassie, rows: &["LET THE LITTLE GUY GO."] },
];

pub static SOL_CITY: Level = Level {
    name: "SOL CITY",
    subtitle: "GM FROM THE BEACH",
    theme: Theme::SolCity,
    boss_name: "BIG CRAB BOT",
    boss_talk: &BOSS_TALK,
    boss: crate::level::BossKind::Crab,
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
