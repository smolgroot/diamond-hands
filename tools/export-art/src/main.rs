//! Writes the art that `game/src/art` draws in code out as PNG files for GBA-Studio.
//!
//! The old ROM paints its sprites, tiles and backgrounds at boot from ASCII drawings and
//! shape primitives. This tool runs that same code on the host, against a stand-in for
//! the few agb types it paints into (`agb-shim`), and saves what it painted: the art
//! comes out pixel for pixel as the ROM shows it, as files an artist can open and edit.
//!
//! Sprites are grouped into sheets by owner, frame size and palette: each sheet is one
//! palette on the GBA, and only 16 fit at once, so one sheet per drawing would run out.
//! `export.json` lists every drawing's frames by its field name in the art code; the
//! studio import names each sheet's animations after them.
//!
//! ```sh
//! cargo run -p export-art -- <out dir>
//! ```

extern crate alloc;

#[path = "../../../game/src/art/mod.rs"]
#[allow(dead_code, clippy::all)]
mod art;

mod font {
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/font.rs"));
}

mod backgrounds;
mod sheets;

use std::{env, fs, path::PathBuf};

use art::{arrival, badlands::Villains, casino, ui, Common};
use sheets::Sheets;

/// The font's characters, in the order of the `font` sheet's frames.
const FONT: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ!:/-+.<>,'?";

/// A glyph as `game/src/font.rs` renders it into its 8x8 sprite.
fn glyph(rows: &[u8; 7]) -> agb::display::object::SpriteVram {
    use agb::display::object::{DynamicSprite16, Size};
    let mut sprite = DynamicSprite16::new_in(Size::S8x8, agb::ExternalAllocator);
    for (d, colour) in [(1, art::ix::OUTLINE), (0, art::ix::WHITE)] {
        for (y, bits) in rows.iter().enumerate() {
            for x in 0..5 {
                if bits & (0b10000 >> x) != 0 {
                    sprite.set_pixel(x + d, y + d, colour);
                }
            }
        }
    }
    sprite.to_vram(&art::SPRITE_PALETTE)
}

fn main() {
    let out = PathBuf::from(env::args().nth(1).expect("usage: export-art <out dir>"));
    fs::create_dir_all(out.join("sprites")).unwrap();
    fs::create_dir_all(out.join("backgrounds")).unwrap();

    let mut s = Sheets::default();

    let c = Common::new();
    s.add_all("pickups", "diamond", &c.diamond);
    s.add("pickups", "heal", &c.heal);
    s.add_all("hud", "life_caps", &c.life_caps);
    for (fill, cells) in c.life.iter().enumerate() {
        s.add_all("hud", &format!("life_{fill}"), cells);
    }
    s.add_all("hud", "boss_cell", &c.boss_cell);
    s.add("laser", "bolt", &c.bolt);
    s.add("laser", "bolt_up", &c.bolt_up);
    s.add("laser", "bolt_diagonal", &c.bolt_diagonal);
    s.add("laser", "beam", &c.beam);
    s.add("laser", "beam_up", &c.beam_up);
    s.add("laser", "beam_diagonal", &c.beam_diagonal);
    s.add_all("effects", "muzzle", &c.muzzle);
    s.add_all("effects", "charge", &c.charge);
    s.add("prompts", "cursor", &c.cursor);
    s.add_all("flag", "flag", &c.flag);
    s.add("prompts", "alert", &c.alert);
    s.add_all("effects", "impact", &c.impact);
    s.add_all("effects", "explosion", &c.explosion);
    s.add_all("effects", "glint", &c.glint);
    s.add_all("effects", "sparks", &c.sparks);
    s.add_all("effects", "puff", &c.puff);
    s.add_all("effects", "muzzle_ring", &c.muzzle_ring);
    s.add_all("dust", "dust", &c.dust);
    s.add_all("dust", "kick", &c.kick);
    s.add("effects", "speed_lines", &c.speed_lines);
    s.add_all("fence", "lightning", &c.lightning);
    s.add_all("fence", "lightning_short", &c.lightning_short);
    s.add("fence", "fence_spark", &c.fence_spark);

    let v = Villains::new();
    // The crawler, drone, mine and sentry walker are already studio sheets of their own.
    s.add("villains", "plasma", &v.plasma);
    s.add("villains", "bomb", &v.bomb);
    s.add("villains", "shockwave", &v.shockwave);
    s.add("villains", "missile", &v.missile);
    s.add("villains", "star", &v.star);

    let b = art::badlands::Bera::new();
    s.add("bera", "head", &b.head);
    s.add("bera", "head_dead", &b.head_dead);
    s.add("bera", "torso", &b.torso);
    s.add("bera", "torso_aim", &b.torso_aim);
    s.add("bera", "legs", &b.legs);
    s.add_all("bera", "core", &b.core);
    s.add("bera", "smoke", &b.smoke);
    s.add("bera", "charge", &b.charge);
    s.add("bera-portrait", "portrait", &b.portrait);

    let a = arrival::ArrivalArt::new();
    s.add("cl207", "guide_top", &a.guide_top);
    s.add_all("cl207", "guide_legs", &a.guide_legs);
    s.add("ship", "ship_half", &a.ship_half);
    s.add("ship", "hatch_open", &a.hatch_open);
    s.add_all("ship", "flames", &a.flames);
    s.add_all("cl207-portrait", "portrait", &arrival::portrait());

    let w = art::wassie::Wassie::new();
    // Its poses, cannon, jetpack and flame are already studio sheets: only the talking face is new.
    s.add("wassie-talk", "portrait", &w.portrait);
    s.add_all("wassie-talk", "portrait_talk", &w.portrait_talk);

    let casino = casino::CasinoArt::new();
    s.add_all("casino", "coin", &casino.coin);
    let f = casino.floor.as_ref().unwrap();
    s.add("casino", "cell", &f.cell);
    s.add("casino", "terminal", &f.terminal);
    s.add("casino", "terminal_on", &f.terminal_on);
    s.add("casino", "lift", &f.lift);
    s.add_all("casino", "lift_door", &f.lift_door);
    s.add("casino", "up_prompt", &f.up_prompt);
    s.add("casino", "phone", &f.phone);
    s.add_all("casino", "dealer", &f.dealer);
    s.add("casino", "cards_in_hand", &f.cards_in_hand);
    s.add_all("casino", "card", &f.card);
    s.add("casino", "chip", &f.chip);
    let ninja: Vec<_> = (0..casino::NINJA_POSES).map(|i| f.ninja.sprite(i)).collect();
    s.add_all("ninja", "pose", &ninja);
    let arena = casino::ArenaArt::new();
    use casino::SamPose::*;
    let poses = [Idle0, Idle1, Walk0, Walk1, Walk2, Walk3, Hold, Raise, Throw, Charge, Punch, Ouch, Dazed, Shock, Kneel, Down];
    let sam: Vec<_> = poses.iter().map(|&p| arena.sam.sprite(p)).collect();
    s.add_all("sam", "pose", &sam);
    let faces: Vec<_> = (0..3).map(|m| arena.sam.portrait(m)).collect();
    s.add_all("sam-portrait", "portrait", &faces);
    let fx = &arena.sam_fx;
    s.add_all("sam-fx", "cucumber", &fx.cucumber);
    s.add("sam-fx", "mega", &fx.mega);
    s.add_all("sam-fx", "splat", &fx.splat);
    s.add_all("sam-fx", "aura", &fx.aura);
    s.add("sam-candle", "candle", &fx.candle);
    s.add("sam-fx", "orb", &fx.orb);
    s.add("penguin", "penguin", &arena.penguin);

    s.add("map-markers", "soon", &ui::soon_marker());
    s.add("map-markers", "locked", &ui::lock_marker());
    s.add("map-markers", "clear_badge", &ui::clear_badge());

    // The font, as the old ROM renders it: white glyphs with a dark drop shadow.
    let glyphs: Vec<_> = FONT.bytes().map(|b| glyph(font::dialog_glyph(b).unwrap())).collect();
    s.add_all("font", "glyphs", &glyphs);

    let manifest = s.write(&out.join("sprites"));
    let backgrounds = backgrounds::export(&out.join("backgrounds"));
    let json = format!("{{\n\"sprites\": {manifest},\n\"backgrounds\": {backgrounds}\n}}\n");
    fs::write(out.join("export.json"), json).unwrap();
}
