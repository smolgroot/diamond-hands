//! Level 5: Satoshi Canyon, the far west of the Bitcoin lands. Red-rock mesas under a
//! sunset, a western town on three levels, and the Satoshi mines under it, where THE
//! INSCRIBER, a laser-eyed cyborg miner, inscribes every sat it digs up.
//!
//! The map is painted in GBA-Studio as four rooms (`project/studio/rooms/`: `btc-canyon`,
//! `btc-town`, `satoshi-mine` and `inscriber-vault`), which `build.rs` stitches where they
//! sit on the world map, as for Sol City. Each room is a camera zone. The Bitcoin Puppet's
//! lines come from the `talk` property of the `btc-puppet` entity. The rooms were laid out
//! by `tools/satoshi-canyon/build-rooms.py` on tilesets made with `tileset_generate`.
//!
//! The way through (rows 0-79 of one map, columns 0-389):
//!
//! - the canyon (columns 0-149, rows 0-39): mesas and a butte climbed in steps, with
//!   drones, sentry bots and Bad Cop bots; off the butte a rope bridge runs high over
//!   the canyon floor to a rock arch;
//! - the town (150-269, rows 0-39): the street, the bank's hill (up its slopes), and the
//!   jail on a rock shelf over the lane to the mine. Two Bad Cops guard the Bitcoin
//!   Puppet there, behind adobe bricks that hold while they stand. Once they're beaten
//!   and the bricks shot apart, the Puppet speaks up, tips the wassie and opens the
//!   mine's gate at the end of the lane (column 266);
//! - the mines (270-359, rows 20-79): the adit, through a wall of ore, under stalactites
//!   and past a runaway cart to the shaft; down it to the middle gallery, west to a hole
//!   at its end, and down to the deep gallery, which runs back east to the vault;
//! - the vault (360-389, rows 60-79): THE INSCRIBER's arena, one screen wide, its hoard
//!   on two shelves. Beating it clears the level.

use crate::{
    level::{BossKind, Level, Markers, Theme, Worth},
    talk::{Line, Speaker},
};

include!(concat!(env!("OUT_DIR"), "/satoshi_canyon.rs"));

/// The route bot's way through: across the canyon and up onto the bank's hill, a plank
/// and the jail's shelf (where it fights the guards and shoots the bricks), to the
/// Puppet; back off the shelf onto the hill and down into the lane, through the mine's
/// gate and the adit to the foot of the shaft; west along the middle gallery to its hole,
/// and down to the deep gallery, which it runs along to the vault.
static ROUTE: [(i32, i32); 9] = [
    (223, 25),
    (233, 21),
    (241, 17),
    (259, 17),
    (237, 25),
    (251, 33),
    (329, 53),
    (277, 75),
    (300, 75),
];

static MARKERS: Markers = Markers::scan(ROWS);

/// THE INSCRIBER, when the vault's arena closes behind the wassie.
static BOSS_TALK: [Line; 5] = [
    Line { speaker: Speaker::Inscriber, rows: &["HALT. THIS VAULT HOLDS", "EVERY SAT I EVER DUG."] },
    Line { speaker: Speaker::Inscriber, rows: &["EACH ONE INSCRIBED,", "NUMBERED, RARE. MINE."] },
    Line { speaker: Speaker::Wassie, rows: &["YOU LOCKED UP A PUPPET", "FOR HODLING."] },
    Line { speaker: Speaker::Inscriber, rows: &["AND I WILL INSCRIBE", "YOU NEXT. HOLD STILL."] },
    Line { speaker: Speaker::Wassie, rows: &["NOT YOUR KEYS,", "NOT YOUR VAULT."] },
];

pub static SATOSHI_CANYON: Level = Level {
    name: "SATOSHI CANYON",
    subtitle: "GM FROM THE FAR WEST",
    theme: Theme::SatoshiCanyon,
    boss_name: "THE INSCRIBER",
    boss_talk: &BOSS_TALK,
    boss: BossKind::Inscriber,
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
