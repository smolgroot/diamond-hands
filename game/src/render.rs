//! Maps game state onto backgrounds and objects each frame.
//!
//! Background layers, front to back: terrain (BG P1, streamed column by column as the
//! camera moves), mesas (BG P2, ⅓ parallax), sky (BG P3, static). Every object is P0,
//! above all of them; objects shown first are drawn on top, so text and HUD go first.

use alloc::{vec, vec::Vec};

use agb::{
    display::{
        GraphicsFrame, Priority, WIDTH,
        object::{Object, SpriteVram},
        tiled::{DynamicTile16, RegularBackground, RegularBackgroundSize, TileEffect, TileFormat},
    },
    fixnum::vec2,
};
use diamond_hands_logic::{
    Difficulty, Game, Level, Scene, TILE,
    boss::BossState,
    enemy::{ActorKind, state},
    game::{options_menu, title_menu},
    player::MAX_HP,
    shot::ShotKind,
    world::CHARGE_FRAMES,
};

use crate::{
    art::{
        Common, TileCache,
        canyon::{self, SKY_PALETTES, Villains},
        wassie::{self, Wassie},
    },
    font::{self, Font},
};

/// Terrain map width in tiles. Columns are written into slot `column % 64`.
const TERRAIN_COLS: i32 = 64;
/// Time of day behind the menus: the canyon at sunset.
const MENU_STAGE: u8 = 1;
/// Terrain uses background palette 1; the backdrop uses 0.
const TERRAIN_PALETTE: u8 = 1;
/// Most terrain columns written per frame. Scrolling needs at most one; after the
/// camera jumps (a respawn) the screen refills over a few frames, so no frame runs long.
const COLUMNS_PER_FRAME: usize = 8;
const KEY_UNSET: u16 = u16::MAX;
const KEY_BLANK: u16 = u16::MAX - 1;

pub struct Renderer {
    common: Common,
    wassie: Wassie,
    villains: Villains,
    font: Font,
    sky: RegularBackground,
    mesas: RegularBackground,
    terrain: RegularBackground,
    terrain_tiles: TileCache,
    /// Tile for each [`canyon::terrain_key`], built on first use: [`KEY_UNSET`] until
    /// then, [`KEY_BLANK`] for transparent cells. On the heap, as it is a few KB.
    terrain_by_key: Vec<u16>,
    blank: DynamicTile16,
    /// Level column held by each terrain map slot.
    columns: [i32; TERRAIN_COLS as usize],
    level: Option<&'static Level>,
    ground_palettes: [agb::display::Palette16; 3],
    stage: u8,
}

fn layer(priority: Priority, size: RegularBackgroundSize) -> RegularBackground {
    RegularBackground::new(priority, size, TileFormat::FourBpp)
}

impl Renderer {
    pub fn new(gfx: &mut agb::display::Graphics) -> Self {
        let ground_palettes = [0, 1, 2].map(canyon::ground_palette);
        gfx.set_background_palettes(&[SKY_PALETTES[0].clone(), ground_palettes[0].clone()]);

        let mut sky = layer(Priority::P3, RegularBackgroundSize::Background32x32);
        canyon::paint_sky(&mut sky);
        let mut mesas = layer(Priority::P2, RegularBackgroundSize::Background32x32);
        canyon::paint_mesas(&mut mesas);

        let mut renderer = Self {
            common: Common::new(),
            wassie: Wassie::new(),
            villains: Villains::new(),
            font: Font::new(),
            sky,
            mesas,
            terrain: layer(Priority::P1, RegularBackgroundSize::Background64x32),
            terrain_tiles: TileCache::new(),
            terrain_by_key: vec![KEY_UNSET; canyon::TERRAIN_KEYS],
            blank: DynamicTile16::new().fill_with(0),
            columns: [i32::MIN; TERRAIN_COLS as usize],
            level: None,
            ground_palettes,
            stage: 0,
        };
        // Fill the first screen now, while booting, rather than in the first frame.
        let level = diamond_hands_logic::levels::get(0);
        while !renderer.stream_terrain(level, 0) {}
        renderer
    }

    pub fn draw(&mut self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        let cam = world.camera_x;
        let in_menu = matches!(game.scene, Scene::Title | Scene::Options);

        // Noon, then sunset, then night as the wassie crosses the level.
        let stage = if in_menu {
            MENU_STAGE
        } else {
            ((cam + WIDTH / 2) * 3 / world.level.px_width()).clamp(0, 2) as u8
        };
        if stage != self.stage {
            frame.set_background_palette(0, &SKY_PALETTES[stage as usize]);
            frame.set_background_palette(TERRAIN_PALETTE, &self.ground_palettes[stage as usize]);
            self.stage = stage;
        }

        let terrain_ready = self.stream_terrain(world.level, cam);
        self.terrain
            .set_scroll_pos(vec2(cam.rem_euclid(TERRAIN_COLS * TILE), 0));
        self.mesas.set_scroll_pos(vec2((cam / 3).rem_euclid(256), 0));
        if terrain_ready {
            self.terrain.show(frame);
        }
        self.mesas.show(frame);
        self.sky.show(frame);

        self.draw_overlay(game, frame);
        if matches!(game.scene, Scene::Play | Scene::Paused | Scene::Intro) {
            self.draw_hud(game, frame);
        }
        self.draw_player(game, frame);
        self.draw_shots(game, frame);
        self.draw_boss(game, frame);
        self.draw_actors(game, frame);
    }

    /// Writes up to [`COLUMNS_PER_FRAME`] newly visible terrain columns into the map.
    /// Returns whether the whole screen is now loaded.
    fn stream_terrain(&mut self, level: &'static Level, cam: i32) -> bool {
        if !self.level.is_some_and(|l| core::ptr::eq(l, level)) {
            self.columns = [i32::MIN; TERRAIN_COLS as usize];
            self.level = Some(level);
        }
        let first = cam.div_euclid(TILE);
        let mut written = 0;
        for col in first..=first + WIDTH / TILE {
            let slot = col.rem_euclid(TERRAIN_COLS);
            if self.columns[slot as usize] == col {
                continue;
            }
            if written == COLUMNS_PER_FRAME {
                return false;
            }
            written += 1;
            self.columns[slot as usize] = col;
            for ty in 0..level.height() {
                let key = canyon::terrain_key(level, col, ty);
                if self.terrain_by_key[key] == KEY_UNSET {
                    let id = self.terrain_tiles.intern(canyon::terrain_pixels(key));
                    self.terrain_by_key[key] = id.map_or(KEY_BLANK, |id| id as u16);
                }
                let tile = match self.terrain_by_key[key] {
                    KEY_BLANK => &self.blank,
                    id => &self.terrain_tiles.tiles[id as usize],
                };
                self.terrain.set_tile_dynamic16(
                    (slot, ty),
                    tile,
                    TileEffect::new(false, false, TERRAIN_PALETTE),
                );
            }
        }
        true
    }

    fn draw_overlay(&self, game: &Game, frame: &mut GraphicsFrame) {
        let blink_on = (game.frame / 30).is_multiple_of(2);
        let level = game.world.level;
        match game.scene {
            Scene::Title => self.draw_title(game, frame),
            Scene::Options => self.draw_options(game, frame),
            Scene::Intro => {
                let mut buf = [0; 10];
                let n = font::number(game.world.level_index as u32 + 1, &mut buf);
                let x = (WIDTH - Font::width("LEVEL ", true) - Font::width(n, true)) / 2;
                self.font.draw(frame, "LEVEL ", x, 40, true);
                self.font.draw(frame, n, x + Font::width("LEVEL ", true), 40, true);
                self.font.draw_centred(frame, level.name, 56, true);
                self.font.draw_centred(frame, level.subtitle, 72, false);
            }
            Scene::Play => {}
            Scene::Paused => self.font.draw_centred(frame, "PAUSED", 60, true),
            Scene::LevelClear => {
                let world = &game.world;
                self.font.draw_centred(frame, "LEVEL CLEAR!", 26, true);
                self.label_number(frame, "SATS ", world.sats, 50);
                self.label_number(frame, "BOUNTIES ", world.kills, 60);
                self.label_time(frame, world.frames / 60, 70);
                self.label_number(frame, "SCORE ", game.score(), 84);
                self.label_number(frame, "BEST ", game.best, 94);
                if game.new_best && blink_on {
                    self.font.draw_centred(frame, "NEW BEST!", 106, false);
                }
                self.font.draw_centred(frame, "PRESS A", 120, false);
            }
        }
    }

    fn draw_title(&self, game: &Game, frame: &mut GraphicsFrame) {
        self.font.draw_centred(frame, "DIAMOND HANDS", 12, true);
        self.font.draw_centred(frame, "A WASSIE ADVENTURE", 26, false);

        let items = [(title_menu::PLAY, "PLAY"), (title_menu::OPTIONS, "OPTIONS")];
        let x = (WIDTH - Font::width("OPTIONS", true)) / 2;
        for (i, (row, label)) in items.into_iter().enumerate() {
            let y = 40 + i as i32 * 14;
            self.font.draw(frame, label, x, y, true);
            if game.cursor == row {
                self.draw_cursor(game, frame, x, y);
            }
        }
        if game.best > 0 {
            self.label_number(frame, "BEST ", game.best, 70);
        }

        // Controls, right of where the wassie stands.
        let (keys_x, action_x) = (84, 84 + 36);
        let controls = [
            ("D-PAD", "MOVE"),
            ("A", "JUMP"),
            ("B", "FIRE"),
            ("HOLD B", "CHARGED BEAM"),
            ("UP+B", "SHOOT UP"),
            ("START", "PAUSE"),
        ];
        for (i, (keys, action)) in controls.into_iter().enumerate() {
            let y = 82 + i as i32 * 8;
            self.font.draw(frame, keys, keys_x, y, false);
            self.font.draw(frame, action, action_x, y, false);
        }
    }

    fn draw_options(&self, game: &Game, frame: &mut GraphicsFrame) {
        self.font.draw_centred(frame, "OPTIONS", 14, true);

        let x = 40;
        let row_y = 42;
        self.font.draw(frame, "DIFFICULTY", x, row_y, true);
        let (name, about) = match game.difficulty {
            Difficulty::Chill => ("CHILL", "HALF DAMAGE. VILLAINS SHOOT LESS."),
            Difficulty::Normal => ("NORMAL", "THE WILD WEST AS INTENDED."),
            Difficulty::Degen => ("DEGEN", "DOUBLE DAMAGE. FULL SEND."),
        };
        let value_x = x + Font::width("DIFFICULTY ", true);
        self.font.draw(frame, "<", value_x, row_y, true);
        self.font.draw(frame, name, value_x + 10, row_y, true);
        self.font
            .draw(frame, ">", value_x + 12 + Font::width(name, true), row_y, true);
        self.font.draw_centred(frame, about, row_y + 14, false);

        let back_y = 72;
        self.font.draw(frame, "BACK", x, back_y, true);
        let cursor_y = if game.cursor == options_menu::DIFFICULTY { row_y } else { back_y };
        self.draw_cursor(game, frame, x, cursor_y);

        self.font
            .draw_centred(frame, "UP/DOWN: SELECT   LEFT/RIGHT: CHANGE", 94, false);
        self.font.draw_centred(frame, "B: BACK", 104, false);
    }

    /// Arrow left of a menu item at (x, y), nudging forward and back.
    fn draw_cursor(&self, game: &Game, frame: &mut GraphicsFrame, x: i32, y: i32) {
        let nudge = (game.frame / 16 % 2) as i32;
        show(frame, &self.common.cursor, x - 12 + nudge, y - 1);
    }

    fn draw_hud(&self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        let hp = world.player.hp.clamp(0, MAX_HP);
        show(frame, &self.common.life[hp.min(4) as usize], 4, 4);
        show(frame, &self.common.life[(hp - 4).clamp(0, 4) as usize], 36, 4);

        let mut buf = [0; 10];
        show(frame, &self.common.sat[0], 70, 0);
        self.font
            .draw(frame, font::number(world.sats, &mut buf), 86, 6, false);
        let score = font::number(game.score(), &mut buf);
        self.font
            .draw(frame, score, WIDTH - 4 - Font::width(score, false), 6, false);

        if world.boss.is_active() && !world.boss.is_beaten() {
            let y = 148;
            self.font.draw(frame, "SHERIFF URSA", 4, y + 1, false);
            let full = world.boss.hp * 64 / diamond_hands_logic::boss::BOSS_HP;
            for i in 0..8 {
                let filled = (full - i * 8).clamp(0, 8);
                show(frame, &self.common.boss_cell[filled as usize], 56 + i * 8, y);
            }
        }
    }

    fn label_number(&self, frame: &mut GraphicsFrame, label: &str, n: u32, y: i32) {
        let mut buf = [0; 10];
        let digits = font::number(n, &mut buf);
        let width = Font::width(label, false) + Font::width(digits, false);
        let x = (WIDTH - width) / 2;
        self.font.draw(frame, label, x, y, false);
        self.font
            .draw(frame, digits, x + Font::width(label, false), y, false);
    }

    fn label_time(&self, frame: &mut GraphicsFrame, seconds: u32, y: i32) {
        let mut text = *b"TIME 00:00";
        let (m, s) = ((seconds / 60).min(99), seconds % 60);
        for (i, d) in [m / 10, m % 10, s / 10, s % 10].into_iter().enumerate() {
            text[5 + i + (i >= 2) as usize] = b'0' + d as u8;
        }
        let text = core::str::from_utf8(&text).unwrap_or("");
        self.font.draw_centred(frame, text, y, false);
    }

    fn draw_player(&self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        let p = &world.player;
        let cam = world.camera_x;
        let blinking = p.invuln > 0 && !p.is_dead() && (game.frame / 3).is_multiple_of(2);
        let r = p.body.rect();
        if blinking || r.y > 160 {
            return;
        }
        let left = p.facing < 0;
        let anim = (r.x.unsigned_abs()) / 2;
        let col = if left { 31 - wassie::CENTRE_COL } else { wassie::CENTRE_COL };
        let x = r.centre_x() - col - cam;
        let y = r.bottom() - 32;

        // Muzzle effects first so they sit on top of the wassie.
        let (mx, my) = if p.aim_up {
            (r.centre_x() + p.facing * 10, r.y - 6)
        } else {
            (r.centre_x() + p.facing * 17, r.y + 13)
        };
        if p.recoil > 0 {
            show(frame, &self.common.muzzle, mx - 4 - cam, my - 4);
        } else if p.charge >= CHARGE_FRAMES {
            let pulse = (game.frame / 4 % 2) as usize;
            let glow = &self.common.charge[if pulse == 0 { 1 } else { 0 }];
            show(frame, glow, mx - 4 - cam, my - 4);
        } else if p.charge > 12 && (game.frame / 2).is_multiple_of(2) {
            show(frame, &self.common.charge[0], mx - 4 - cam, my - 4);
        }

        Object::new(self.wassie.sprite(p, anim).clone())
            .set_pos(vec2(x, y))
            .set_hflip(left)
            .set_priority(Priority::P0)
            .show(frame);
    }

    fn draw_shots(&self, game: &Game, frame: &mut GraphicsFrame) {
        let cam = game.world.camera_x;
        for s in game.world.shots.iter().filter(|s| s.active) {
            let r = s.rect();
            let x = r.x - cam;
            if !(-32..=WIDTH).contains(&x) {
                continue;
            }
            let left = s.vx_fx < 0;
            let (sprite, dx, dy, vflip_up) = match s.kind {
                ShotKind::Bolt => (&self.common.bolt, 0, -2, true),
                ShotKind::Beam => (&self.common.beam, 0, 0, true),
                ShotKind::Bullet => (&self.villains.bullet, -1, -2, false),
                ShotKind::Dust => (&self.villains.dust, 0, 0, false),
                ShotKind::Shockwave => (&self.villains.shockwave, -3, -4, false),
                ShotKind::Candle => (&self.villains.candle, -1, 0, false),
            };
            let mut obj = Object::new(sprite.clone());
            obj.set_hflip(left).set_priority(Priority::P0);
            if vflip_up && s.vx_fx == 0 {
                // Shooting up: a vertical bolt is drawn as a short stack of dots.
                for i in 0..r.h.max(8) / 4 {
                    show(frame, &self.common.charge[0], r.x - cam - 2, r.y + i * 4 - 2);
                }
                continue;
            }
            obj.set_pos(vec2(x + dx, r.y + dy)).show(frame);
        }
    }

    fn draw_boss(&self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        let boss = &world.boss;
        if !boss.present || boss.state == BossState::Defeated {
            return;
        }
        let cam = world.camera_x;
        let r = boss.rect();
        if r.right() < cam - 32 || r.x > cam + WIDTH + 32 {
            return;
        }
        let blink = (boss.flash > 0 || boss.state == BossState::Dying) && (game.frame / 2).is_multiple_of(2);
        if blink {
            return;
        }
        let ursa = &self.villains.ursa;
        let flip = boss.facing > 0;
        let (sprite, h) = match boss.state {
            BossState::Charge => (&ursa.charge, 32),
            BossState::Aim => (&ursa.aim, 64),
            BossState::Stunned | BossState::Dying => (&ursa.dizzy, 64),
            _ => (&ursa.stand, 64),
        };
        let x = r.centre_x() - 32 - cam;
        // Telegraphing the charge: a crouch-and-shake.
        let shake = if boss.state == BossState::Telegraph { (game.frame % 2) as i32 * 2 - 1 } else { 0 };
        if boss.state == BossState::Stunned {
            for i in 0..3 {
                let a = (game.frame as i32 / 4 + i * 8) % 24;
                let sx = r.centre_x() - 4 - cam + (a - 12).abs() * 2 - 12;
                let sy = r.y - 14 + (a % 12) / 3;
                show(frame, &ursa.star, sx, sy);
            }
        }
        Object::new(sprite.clone())
            .set_pos(vec2(x + shake, r.bottom() - h))
            .set_hflip(flip)
            .set_priority(Priority::P0)
            .show(frame);
    }

    fn draw_actors(&self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        let cam = world.camera_x;
        let v = &self.villains;
        for a in world.actors.iter().filter(|a| a.alive) {
            let r = a.rect();
            if r.right() < cam - 16 || r.x > cam + WIDTH + 16 {
                continue;
            }
            if a.flash > 0 && (game.frame / 2).is_multiple_of(2) {
                continue;
            }
            let x = r.x - cam;
            let step = ((r.x.unsigned_abs() / 6) % 2) as usize;
            let (sprite, sx, sy, flip) = match a.kind {
                ActorKind::Sat => (&self.common.sat[(game.frame / 8 % 2) as usize], x - 4, r.y - 4, false),
                ActorKind::Heal => (&self.common.heal, x - 3, r.y - 3, false),
                ActorKind::Checkpoint => {
                    let lit = (a.state == state::LIT) as usize;
                    (&self.common.flag[lit], x - 2, r.bottom() - 32, false)
                }
                ActorKind::Bandit => {
                    let frame_ix = if a.state == state::AIM { 2 } else { step };
                    (&v.bandit[frame_ix], r.centre_x() - 8 - cam, r.bottom() - 32, a.facing > 0)
                }
                ActorKind::Snake => {
                    let lunge = (a.state == state::LUNGE) as usize;
                    (&v.snake[lunge], x, r.bottom() - 16, a.facing > 0)
                }
                ActorKind::Vulture => {
                    let flap = (game.frame / 10 % 2) as usize;
                    (&v.vulture[flap], x, r.y - 3, a.facing > 0)
                }
                ActorKind::Tumbleweed => (&v.tumbleweed, x - 2, r.y - 2, step == 1),
            };
            let mut obj = Object::new(sprite.clone());
            obj.set_pos(vec2(sx, sy))
                .set_hflip(flip)
                .set_priority(Priority::P0);
            if a.kind == ActorKind::Tumbleweed {
                obj.set_vflip(step == 1);
            }
            obj.show(frame);
        }
    }
}

fn show(frame: &mut GraphicsFrame, sprite: &SpriteVram, x: i32, y: i32) {
    Object::new(sprite.clone())
        .set_pos(vec2(x, y))
        .set_priority(Priority::P0)
        .show(frame);
}
