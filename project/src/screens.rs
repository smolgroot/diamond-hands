//! Text screens and the HUD that every level shows the same way, whichever renderer
//! draws the level behind them: the code-drawn one (`render`) or the studio's
//! (`sol_city/draw.rs`). Each takes the font and the few sprites it needs.

use agb::{
    display::{GraphicsFrame, Priority, WIDTH, object::Object, object::SpriteVram},
    fixnum::vec2,
};
use diamond_hands_logic::{Game, game::CLEAR_LOCK_FRAMES, levels::MAP, player::MAX_HP};

use crate::{
    art::LIFE_CELLS,
    font::{self, Font},
};

/// Shows `sprite` with its top-left at screen (x, y), above the backgrounds.
pub fn show(frame: &mut GraphicsFrame, sprite: &SpriteVram, x: i32, y: i32) {
    Object::new(sprite.clone()).set_pos(vec2(x, y)).set_priority(Priority::P0).show(frame);
}

/// The sprites of the HUD and of the diamonds in a level.
pub struct HudArt<'a> {
    pub life_caps: &'a [SpriteVram; 2],
    pub life: &'a [[SpriteVram; LIFE_CELLS]; 9],
    /// Blue diamond and its glinting frame.
    pub diamond: &'a [SpriteVram; 2],
}

/// "LEVEL n", the level's name and its subtitle, over the dark level card.
pub fn level_card(font: &Font, game: &Game, frame: &mut GraphicsFrame) {
    let level = game.world.level;
    let mut buf = [0; 10];
    let n = font::number(game.world.level_index as u32 + 1, &mut buf);
    let x = (WIDTH - Font::width("LEVEL ", true) - Font::width(n, true)) / 2;
    font.draw(frame, "LEVEL ", x, 36, true);
    font.draw(frame, n, x + Font::width("LEVEL ", true), 36, true);
    font.draw_centred(frame, level.name, 55, true);
    font.draw_centred(frame, level.subtitle, 76, false);
}

/// The life bar, the diamonds (and the casino's coins, given its coin sprite), and the
/// score on the right.
pub fn hud(font: &Font, art: &HudArt, coin: Option<&SpriteVram>, game: &Game, frame: &mut GraphicsFrame) {
    let world = &game.world;
    let hp = world.player.hp.clamp(0, MAX_HP);
    show(frame, &art.life_caps[0], 4, 4);
    for (i, cell) in art.life[hp as usize].iter().enumerate() {
        show(frame, cell, 8 + 8 * i as i32, 4);
    }
    show(frame, &art.life_caps[1], 64, 4);

    let mut buf = [0; 10];
    show(frame, &art.diamond[0], 70, 0);
    font.draw(frame, font::number(world.diamonds, &mut buf), 86, 5, false);
    if let Some(coin) = coin {
        show(frame, coin, 114, 4);
        font.draw(frame, font::number(world.coins, &mut buf), 125, 5, false);
    }
    let score = font::number(game.score(), &mut buf);
    font.draw(frame, score, WIDTH - 4 - Font::width(score, false), 5, false);
}

/// `label` then `n`, centred on row `y`.
pub fn label_number(font: &Font, frame: &mut GraphicsFrame, label: &str, n: u32, y: i32) {
    let mut buf = [0; 10];
    let digits = font::number(n, &mut buf);
    let width = Font::width(label, false) + Font::width(digits, false);
    let x = (WIDTH - width) / 2;
    font.draw(frame, label, x, y, false);
    font.draw(frame, digits, x + Font::width(label, false), y, false);
}

fn label_time(font: &Font, frame: &mut GraphicsFrame, seconds: u32, y: i32) {
    let mut text = *b"TIME 00:00";
    let (m, s) = ((seconds / 60).min(99), seconds % 60);
    for (i, d) in [m / 10, m % 10, s / 10, s % 10].into_iter().enumerate() {
        text[5 + i + (i >= 2) as usize] = b'0' + d as u8;
    }
    let text = core::str::from_utf8(&text).unwrap_or("");
    font.draw_centred(frame, text, y, false);
}

/// Sine of a sixteenth of a turn, times 64.
const SIN16: [i32; 16] = [0, 24, 45, 59, 64, 59, 45, 24, 0, -24, -45, -59, -64, -59, -45, -24];
/// Diamonds thrown out in the level-clear burst, and those raining after it.
const BURST: i32 = 8;
const RAIN: i32 = 6;

/// "LEVEL CLEAR!" dropping in, the tally row by row, and a burst of diamonds off the
/// wassie at screen `(cx, cy)` that turns into a gentle rain.
pub fn level_clear(font: &Font, diamond: &[SpriteVram; 2], game: &Game, frame: &mut GraphicsFrame, (cx, cy): (i32, i32)) {
    let world = &game.world;
    let blink_on = (game.frame / 30).is_multiple_of(2);
    let t = game.scene_frames() as i32;
    const BOUNCE: [i32; 6] = [0, -3, -4, -3, -1, 0];
    let banner_y = match t {
        0..16 => 26 - (16 - t) * 3,
        16..22 => 26 + BOUNCE[(t - 16) as usize],
        _ => 26,
    };
    font.draw_centred(frame, "LEVEL CLEAR!", banner_y, true);
    // One row of the tally every few frames once the banner lands.
    let rows = ((t - 18) / 6).max(0);
    if world.level.coins {
        // A casino's tally: coins too, and the wallet after the exchange.
        let lines = [
            ("DIAMONDS ", world.diamonds),
            ("GOLD COINS ", world.coins),
            ("BOUNTIES ", world.kills),
            ("SCORE ", game.score()),
            ("WALLET ", game.wallet),
        ];
        for (i, (label, n)) in lines.into_iter().enumerate() {
            if rows > i as i32 {
                label_number(font, frame, label, n, 44 + 8 * i as i32);
            }
        }
        if rows > 5 {
            label_time(font, frame, world.frames / 60, 84);
        }
    } else {
        if rows > 0 {
            label_number(font, frame, "DIAMONDS ", world.diamonds, 48);
        }
        if rows > 1 {
            label_number(font, frame, "BOUNTIES ", world.kills, 58);
        }
        if rows > 2 {
            label_time(font, frame, world.frames / 60, 68);
        }
        if rows > 3 {
            label_number(font, frame, "SCORE ", game.score(), 80);
            label_number(font, frame, "BEST ", game.best, 90);
        }
    }
    if rows > 4 {
        if game.new_best && blink_on {
            font.draw_centred(frame, "NEW BEST!", 100, false);
        }
        if let Some(place) = game.fresh.map(|i| &MAP[i as usize]) {
            font.draw_centred(frame, "NEW PLACE UNLOCKED", 110, false);
            font.draw_centred(frame, place.name, 118, false);
        } else if game.replay {
            font.draw_centred(frame, "REPLAY: NO SCORE BANKED", 112, false);
        }
    }
    if t >= CLEAR_LOCK_FRAMES as i32 {
        font.draw_centred(frame, "A: WORLD MAP", 132, false);
    }

    // The burst: a spinning ring off the wassie, flying out and away.
    if t < 48 {
        let radius = t * 5 / 2;
        for i in 0..BURST {
            let a = (i * 2 + t / 4) as usize;
            let (dx, dy) = (SIN16[(a + 4) % 16], SIN16[a % 16]);
            let glint = ((t / 3 + i) % 4 == 0) as usize;
            show(frame, &diamond[glint], cx - 8 + dx * radius / 64, cy - 8 + dy * radius / 64);
        }
    } else {
        // Then a gentle rain of diamonds, each at its own pace.
        for i in 0..RAIN {
            let x = 14 + i * 38 + (i * 13) % 17;
            let y = ((t - 48) * (2 + i % 3) / 2 + i * 47) % 200 - 24;
            let glint = ((t / 4 + i) % 6 == 0) as usize;
            show(frame, &diamond[glint], x, y);
        }
    }
}
