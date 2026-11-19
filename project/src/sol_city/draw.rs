//! Draws a frame of the game from the studio's room and sprite sheets: the room under
//! the camera, then the actors, shots, effects, the wassie, the HUD and the dialogue.
//! agb puts objects shown first on top, so the frame is shown front to back.

use agb::{
    display::{Graphics, GraphicsFrame, object::Object},
    fixnum::vec2,
};
use diamond_hands_logic::{
    Game, PlayerState, Scene, SCREEN_H, SCREEN_W,
    enemy::{Actor, ActorKind, DINO_ROAR_FRAMES, state},
    fx::FxKind,
    player::{Aim, HOVER_FRAMES},
    shot::ShotKind,
    talk::Speaker,
    world::{CHARGE_FRAMES, WARP_FRAMES},
};
use pecten_studio_rt::gfx::{SpriteSheet, WorldView};

use crate::{
    data::{WORLD, room},
    gfx,
};

/// The font's characters, in the order of the `ui` sheet's first frames.
const FONT: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ!:/-+.<>,'?";
/// Pixels from one character to the next.
const ADVANCE: i32 = 6;

/// Frames of the `ui` sheet after the font.
mod ui {
    pub const HEART_FULL: usize = 47;
    pub const HEART_HALF: usize = 48;
    pub const HEART_EMPTY: usize = 49;
    pub const DIAMOND: usize = 50;
    pub const UP: usize = 51;
    pub const ALERT: usize = 52;
}

/// Frames of the `fx` sheet.
mod fx {
    pub const HEAL: usize = 4;
    pub const BOLT_H: usize = 6;
    pub const BOLT_DU: usize = 7;
    pub const BOLT_V: usize = 8;
    pub const BOLT_DD: usize = 9;
    pub const BEAM_H: usize = 10;
    pub const BEAM_V: usize = 12;
    pub const BEAM_DU: usize = 14;
    pub const BEAM_DD: usize = 16;
    pub const BULLET: usize = 18;
    pub const BOMB: usize = 19;
    pub const IMPACT: usize = 20;
    pub const EXPLOSION: usize = 23;
    pub const GLINT: usize = 27;
    pub const SPARKS: usize = 30;
    pub const PUFF: usize = 32;
    pub const DUST: usize = 34;
    pub const KICK: usize = 37;
    pub const MUZZLE: usize = 40;
}

/// Frames of the wassie's sheet (`studio/sprites/wassie.json`).
mod pose {
    pub const STAND: usize = 0;
    pub const RECOIL: usize = 1;
    pub const IDLE: [usize; 3] = [2, 3, 4];
    pub const RUN: [usize; 6] = [5, 6, 7, 8, 9, 10];
    pub const JUMP: usize = 11;
    pub const FALL: usize = 12;
    pub const HURT: usize = 13;
    pub const LANDING: usize = 14;
    pub const BRAKING: usize = 15;
    pub const SAG: usize = 16;
    pub const LYING: usize = 17;
}

/// Where the gear sits on the wassie, from the top-left of its 32x32 frame, facing
/// right: as the code-drawn ROM places it (`game/src/art/wassie.rs`).
const CENTRE_COL: i32 = 14;
const BLASTER_SHIFT: i32 = 8;
const BLASTER_UP_RAISE: i32 = 14;
const RECOIL_BLASTER: i32 = 2;
const DROPPED_AT: (i32, i32) = (9, 8);
const JETPACK_AT: (i32, i32) = (1, 15);
const FLAME_AT: (i32, i32) = (1, 29);

pub struct Screen {
    view: WorldView,
    /// The room's top-left in world pixels: the level's coordinates start there.
    origin: (i32, i32),
    /// Where the view's camera was last put, in world pixels; `None` before the first frame.
    camera: Option<(i32, i32)>,
}

/// A frame of a sheet with its top-left at screen pixel (x, y), unless well off screen.
fn put(frame: &mut GraphicsFrame, sheet: &'static SpriteSheet, index: usize, x: i32, y: i32, flip: bool) {
    let (w, h) = (sheet.width as i32, sheet.height as i32);
    if x + w < 0 || y + h < 0 || x >= SCREEN_W || y >= SCREEN_H {
        return;
    }
    let mut object = Object::new(&sheet.frames[index.min(sheet.frames.len() - 1)]);
    object.set_pos(vec2(x, y)).set_hflip(flip);
    object.show(frame);
}

/// A frame of a sheet with its anchor at screen pixel (x, y): mirrored around it when
/// flipped.
fn at(frame: &mut GraphicsFrame, sheet: &'static SpriteSheet, index: usize, x: i32, y: i32, flip: bool) {
    let (w, ax, ay) = (sheet.width as i32, sheet.anchor.0 as i32, sheet.anchor.1 as i32);
    let left = if flip { x - (w - ax) } else { x - ax };
    put(frame, sheet, index, left, y - ay, flip);
}

/// The frame an animation of a sheet shows at `tick`, or 0 if it has none of that name.
fn anim(sheet: &'static SpriteSheet, name: &str, tick: u32) -> usize {
    sheet.animation(name).map_or(0, |a| a.frame_at(tick))
}

/// ASCII `text` with its top-left at (x, y); unknown characters and spaces are gaps.
fn text(frame: &mut GraphicsFrame, s: &str, x: i32, y: i32) {
    for (i, c) in s.bytes().enumerate() {
        if let Some(glyph) = FONT.iter().position(|&f| f == c) {
            put(frame, &gfx::SPRITE_UI, glyph, x + i as i32 * ADVANCE, y, false);
        }
    }
}

fn centred(frame: &mut GraphicsFrame, s: &str, y: i32) {
    text(frame, s, (SCREEN_W - s.len() as i32 * ADVANCE) / 2, y);
}

/// A number in decimal, right to left from `right`.
fn number(frame: &mut GraphicsFrame, mut n: u32, right: i32, y: i32) {
    let mut x = right - ADVANCE;
    loop {
        put(frame, &gfx::SPRITE_UI, (n % 10) as usize, x, y, false);
        n /= 10;
        x -= ADVANCE;
        if n == 0 {
            break;
        }
    }
}

impl Screen {
    pub fn new(graphics: &mut Graphics) -> Self {
        let view = WorldView::new(&gfx::ROOMS, room::SOL_CITY, graphics);
        let origin = WORLD.origin(room::SOL_CITY).unwrap_or((0, 0));
        Self { view, origin, camera: None }
    }

    pub fn draw(&mut self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        // A shake jolts the whole picture a pixel or two.
        let jolt = if world.shake > 0 {
            ((game.frame % 2) as i32 * 2 - 1) * 2
        } else if world.kick > 0 {
            (game.frame % 2) as i32 * 2 - 1
        } else {
            0
        };
        let (cx, cy) = (world.camera_x + jolt, world.camera_y);
        let camera = (self.origin.0 + cx, self.origin.1 + cy);
        match self.camera {
            Some(last) if (camera.0 - last.0).abs() < SCREEN_W && (camera.1 - last.1).abs() < SCREEN_H => {
                if camera != last {
                    self.view.set_camera(camera.0, camera.1);
                }
            }
            // The first frame, or a jump (a respawn): fill the whole view at once.
            _ => self.view.load(camera.0, camera.1),
        }
        self.camera = Some(camera);

        // Front to back.
        match game.scene {
            Scene::Intro => self.level_card(game, frame),
            Scene::Paused => {
                centred(frame, "PAUSED", 60);
                centred(frame, "START TO PLAY ON", 76);
            }
            Scene::LevelClear => self.clear_screen(game, frame),
            Scene::Map => {
                centred(frame, "NEXT STOP: ETHEREUM CITY", 56);
                centred(frame, "COMING SOON", 72);
                centred(frame, "START TO PLAY SOL CITY AGAIN", 96);
            }
            _ => {}
        }
        if game.talking() {
            self.dialogue(game, frame);
        }
        if matches!(game.scene, Scene::Play | Scene::Dialog | Scene::Paused) {
            self.hud(game, frame);
        }
        // Title cards stand alone, on black.
        if !matches!(game.scene, Scene::Map | Scene::LevelClear | Scene::Intro) {
            self.prompts(game, frame, cx, cy);
            self.effects(game, frame, cx, cy);
            self.shots(game, frame, cx, cy);
            self.wassie(game, frame, cx, cy);
            self.actors(game, frame, cx, cy);
            self.view.show(frame);
        }
    }

    fn level_card(&self, game: &Game, frame: &mut GraphicsFrame) {
        let level = game.world.level;
        centred(frame, "LEVEL 3", 52);
        centred(frame, level.name, 68);
        centred(frame, level.subtitle, 84);
    }

    fn clear_screen(&self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        centred(frame, "SOL CITY CLEARED", 40);
        text(frame, "DIAMONDS", 64, 64);
        number(frame, world.diamonds, 176, 64);
        text(frame, "BOTS", 64, 76);
        number(frame, world.kills, 176, 76);
        text(frame, "SCORE", 64, 88);
        number(frame, game.score(), 176, 88);
        centred(frame, "INTO THE WORMHOLE...", 116);
    }

    /// Health as hearts of two points each, and the level's diamonds.
    fn hud(&self, game: &Game, frame: &mut GraphicsFrame) {
        let hp = game.world.player.hp.max(0);
        for i in 0..diamond_hands_logic::player::MAX_HP / 2 {
            let left = hp - i * 2;
            let heart = match left {
                2.. => ui::HEART_FULL,
                1 => ui::HEART_HALF,
                _ => ui::HEART_EMPTY,
            };
            put(frame, &gfx::SPRITE_UI, heart, 4 + i * 9, 4, false);
        }
        put(frame, &gfx::SPRITE_UI, ui::DIAMOND, SCREEN_W - 34, 4, false);
        number(frame, game.world.diamonds, SCREEN_W - 4, 4);
    }

    /// The box at the bottom: the speaker's portrait and name, and the line typed out.
    fn dialogue(&self, game: &Game, frame: &mut GraphicsFrame) {
        let Some(line) = game.talk_lines().get(game.talk_line as usize) else {
            return;
        };
        let top = SCREEN_H - 48;
        let typing = (game.talk_shown as usize) < line.len();
        let mouth = typing && (game.frame / 6).is_multiple_of(2);
        text(frame, line.speaker.name(), 50, top + 6);
        for (row, s) in line.visible(game.talk_shown as usize).enumerate() {
            text(frame, s, 50, top + 18 + row as i32 * 9);
        }
        let (sheet, index): (&'static SpriteSheet, usize) = match line.speaker {
            Speaker::Keeper => (&gfx::SPRITE_PORTRAITS, mouth as usize),
            Speaker::Cosmo => (&gfx::SPRITE_PORTRAITS, 2 + mouth as usize),
            _ => (&gfx::SPRITE_WASSIE_PORTRAIT, 0),
        };
        put(frame, sheet, index, 8, top + 8, false);
        let box_sheet = &gfx::SPRITE_DIALOG_BOX;
        put(frame, box_sheet, 0, 0, top - 16, false);
        put(frame, box_sheet, 1, 64, top - 16, false);
        put(frame, box_sheet, 1, 112, top - 16, false);
        put(frame, box_sheet, 2, SCREEN_W - 64, top - 16, false);
    }

    /// "^" where Up does something, and "!" over a villain about to attack.
    fn prompts(&self, game: &Game, frame: &mut GraphicsFrame, cx: i32, cy: i32) {
        let world = &game.world;
        if game.scene == Scene::Play
            && let Some((x, y)) = world.up_prompt().or_else(|| world.talk_prompt())
        {
            let bob = (game.frame / 12 % 2) as i32;
            put(frame, &gfx::SPRITE_UI, ui::UP, x - 4 - cx, y - 8 - bob - cy, false);
        }
        for a in world.placed_actors().iter().filter(|a| a.is_threat() && a.state == state::AIM) {
            let r = a.rect();
            put(frame, &gfx::SPRITE_UI, ui::ALERT, r.centre_x() - 4 - cx, r.y - 12 - cy, false);
        }
    }

    fn effects(&self, game: &Game, frame: &mut GraphicsFrame, cx: i32, cy: i32) {
        let sheet = &gfx::SPRITE_FX;
        for e in game.world.fx.active() {
            let (first, count) = match e.kind {
                FxKind::Impact => (fx::IMPACT, 3),
                FxKind::Explosion => (fx::EXPLOSION, 4),
                FxKind::Glint => (fx::GLINT, 3),
                FxKind::Sparks => (fx::SPARKS, 2),
                FxKind::Puff => (fx::PUFF, 2),
                FxKind::Dust => (fx::DUST, 3),
                FxKind::Kick => (fx::KICK, 3),
                FxKind::Wreck => {
                    // A beaten sentry kneels, then lies there until it fades.
                    let s = &gfx::SPRITE_SENTRY;
                    let name = if e.age < 20 { "kneel" } else { "down" };
                    at(frame, s, anim(s, name, 0), e.x - cx, e.y - cy, e.flip);
                    continue;
                }
                // Casino effects: Sol City has none.
                FxKind::CoinPop | FxKind::Splat | FxKind::NinjaWreck => continue,
            };
            at(frame, sheet, first + e.frame(count), e.x - cx, e.y - cy, false);
        }
    }

    fn shots(&self, game: &Game, frame: &mut GraphicsFrame, cx: i32, cy: i32) {
        let sheet = &gfx::SPRITE_FX;
        for s in game.world.shots.iter().filter(|s| s.active) {
            let r = s.rect();
            let (x, y) = (r.centre_x() - cx, r.centre_y() - cy);
            let (left, up, down) = (s.vx_fx < 0, s.vy_fx < 0, s.vy_fx > 0);
            match s.kind {
                ShotKind::Bolt => {
                    let index = match (s.vx_fx != 0, up, down) {
                        (true, true, _) => fx::BOLT_DU,
                        (true, _, true) => fx::BOLT_DD,
                        (true, ..) => fx::BOLT_H,
                        _ => fx::BOLT_V,
                    };
                    at(frame, sheet, index, x, y, left);
                }
                ShotKind::Beam => {
                    // Two halves along its line: the tail, then the ringed head.
                    let (first, dx, dy) = match (s.vx_fx != 0, up, down) {
                        (true, true, _) => (fx::BEAM_DU, 6, -6),
                        (true, _, true) => (fx::BEAM_DD, 6, 6),
                        (true, ..) => (fx::BEAM_H, 8, 0),
                        _ => (fx::BEAM_V, 0, if up { -8 } else { 8 }),
                    };
                    let sx = if left { -dx } else { dx };
                    // The vertical pair is drawn heading down: going up, swap them.
                    let (tail, head) = if s.vx_fx == 0 && up { (first + 1, first) } else { (first, first + 1) };
                    at(frame, sheet, tail, x - sx, y - dy, left);
                    at(frame, sheet, head, x + sx, y + dy, left);
                }
                ShotKind::Bomb => at(frame, sheet, fx::BOMB, x, y, false),
                _ => at(frame, sheet, fx::BULLET, x, y, left),
            }
        }
    }

    /// The wassie and its gear: the cannon, the jetpack and its flame, the muzzle flash.
    fn wassie(&self, game: &Game, frame: &mut GraphicsFrame, cx: i32, cy: i32) {
        let world = &game.world;
        let p = &world.player;
        let blinking = p.invuln > 0 && !p.is_dead() && (game.frame / 3).is_multiple_of(2);
        if blinking {
            return;
        }
        let r = p.body.rect();
        if world.warp > 0 {
            // Drawn in by the portal: it spins away, then is gone.
            let t = world.warp as u32;
            if t < WARP_FRAMES as u32 / 2 && (t / 2) % 2 == 0 {
                at(frame, &gfx::SPRITE_WASSIE, pose::JUMP, r.centre_x() - cx, r.bottom() - cy - (t as i32 / 4), (t / 4) % 2 == 1);
            }
            return;
        }
        let left = p.facing < 0;
        let x = r.centre_x() - if left { 31 - CENTRE_COL } else { CENTRE_COL } - cx;
        let y = r.bottom() - 32 - cy;
        let part = |(ox, oy): (i32, i32), w: i32| (x + if left { 32 - ox - w } else { ox }, y + oy);

        let (mx, my) = p.muzzle();
        if p.recoil > 0 {
            let late = (p.recoil < 4) as usize;
            at(frame, &gfx::SPRITE_FX, fx::MUZZLE + late, mx - cx, my - cy, left);
        } else if p.charge >= CHARGE_FRAMES || (p.charge > 12 && (game.frame / 2).is_multiple_of(2)) {
            at(frame, &gfx::SPRITE_FX, fx::IMPACT + (game.frame / 4 % 2) as usize, mx - cx, my - cy, false);
        }

        let down = p.state == PlayerState::Dead && p.grounded;
        let aim = if matches!(p.state, PlayerState::Hurt | PlayerState::Dead) { Aim::Forward } else { p.aim };
        let (kx, ky) = if p.recoil > 2 {
            let (dx, dy) = aim.dir();
            (-dx * RECOIL_BLASTER, -dy * RECOIL_BLASTER)
        } else {
            (0, 0)
        };
        let (bframe, (ox, oy)) = if down {
            (5, DROPPED_AT)
        } else {
            (aim as usize, (kx, ky - if aim == Aim::Up { BLASTER_UP_RAISE } else { 0 }))
        };
        let (bx, by) = part((BLASTER_SHIFT, 0), 32);
        let blaster = (bx + ox * p.facing, by + oy);
        let behind = down || aim == Aim::Up;
        if !behind {
            put(frame, &gfx::SPRITE_BLASTER, bframe, blaster.0, blaster.1, left);
        }

        let alive = !matches!(p.state, PlayerState::Dead | PlayerState::Hurt);
        if alive && (p.hovering || (!p.grounded && p.hover_fuel < HOVER_FRAMES)) {
            let (jx, jy) = part(JETPACK_AT, 8);
            put(frame, &gfx::SPRITE_JETPACK, 0, jx, jy, left);
            if p.hovering {
                let name = if p.hover_fuel > 10 { "burn" } else { "sputter" };
                let (fx_, fy) = part(FLAME_AT, 8);
                put(frame, &gfx::SPRITE_FLAME, anim(&gfx::SPRITE_FLAME, name, game.frame), fx_, fy, left);
            }
        }

        let firing = p.recoil > 2;
        let step = (r.x.unsigned_abs() / 2 / 6 % 6) as usize;
        let body = match p.state {
            PlayerState::Hurt => pose::HURT,
            PlayerState::Dead if !p.grounded => pose::HURT,
            PlayerState::Dead if p.dead_timer > diamond_hands_logic::world::DEATH_FRAMES - 14 => pose::SAG,
            PlayerState::Dead => pose::LYING,
            PlayerState::Idle | PlayerState::Run if p.land > 0 && !firing => pose::LANDING,
            PlayerState::Run if p.braking && !firing => pose::BRAKING,
            PlayerState::Run => pose::RUN[step],
            PlayerState::Idle if firing => pose::RECOIL,
            PlayerState::Idle => match game.frame / 12 % 4 {
                0 => pose::STAND,
                n => pose::IDLE[n as usize - 1],
            },
            PlayerState::Jump | PlayerState::Hover => pose::JUMP,
            PlayerState::Fall => pose::FALL,
        };
        let pushed = if firing && !matches!(p.state, PlayerState::Idle) { -2 * p.facing } else { 0 };
        put(frame, &gfx::SPRITE_WASSIE, body, x + pushed, y, left);
        if behind {
            put(frame, &gfx::SPRITE_BLASTER, bframe, blaster.0, blaster.1, left);
        }
    }

    fn actors(&self, game: &Game, frame: &mut GraphicsFrame, cx: i32, cy: i32) {
        let world = &game.world;
        let mut person = 0;
        for a in world.placed_actors() {
            let speaker = if a.kind == ActorKind::Npc {
                person += 1;
                world.level.npcs.get(person - 1).and_then(|lines| lines.iter().map(|l| l.speaker).find(|&s| s != Speaker::Wassie))
            } else {
                None
            };
            if !a.alive || (a.flash > 0 && (game.frame / 2).is_multiple_of(2)) {
                continue;
            }
            let r = a.rect();
            if r.right() < cx - 32 || r.x > cx + SCREEN_W + 32 || r.bottom() < cy - 64 || r.y > cy + SCREEN_H + 64 {
                continue;
            }
            self.actor(game, frame, a, speaker, r.centre_x() - cx, r.bottom() - cy, r.centre_y() - cy);
        }
    }

    /// One actor, with its feet at (x, feet) on screen and its middle at row `mid`.
    #[allow(clippy::too_many_arguments)]
    fn actor(&self, game: &Game, frame: &mut GraphicsFrame, a: &Actor, speaker: Option<Speaker>, x: i32, feet: i32, mid: i32) {
        let tick = game.frame;
        let walk = (a.body.x().unsigned_abs() / 4) as u32;
        let lit = a.state == state::LIT;
        match a.kind {
            ActorKind::Diamond => at(frame, &gfx::SPRITE_FX, anim(&gfx::SPRITE_FX, "diamond", tick), x, mid, false),
            ActorKind::Heal => at(frame, &gfx::SPRITE_FX, fx::HEAL + (tick / 20 % 2) as usize, x, mid, false),
            ActorKind::Checkpoint => {
                let s = &gfx::SPRITE_BEACON;
                at(frame, s, anim(s, if lit { "lit" } else { "off" }, tick), x, feet, false);
            }
            ActorKind::Sentry | ActorKind::Blockade => {
                let s = &gfx::SPRITE_SENTRY;
                let name = match a.state {
                    _ if a.flash > 0 => "hit",
                    state::AIM if a.timer > 8 => "aim",
                    state::AIM => "charge",
                    state::RECOIL => "aim",
                    _ if a.kind == ActorKind::Blockade => "aim",
                    _ => "walk",
                };
                at(frame, s, anim(s, name, walk * 2), x, feet, a.facing < 0);
            }
            ActorKind::Crawler => at(frame, &gfx::SPRITE_CRAWLER, anim(&gfx::SPRITE_CRAWLER, "crawl", walk * 2), x, feet, a.facing > 0),
            ActorKind::Drone => at(frame, &gfx::SPRITE_DRONE, anim(&gfx::SPRITE_DRONE, "hover", tick), x, feet, a.facing > 0),
            ActorKind::Mine => at(frame, &gfx::SPRITE_MINE, anim(&gfx::SPRITE_MINE, "idle", tick), x, feet, false),
            ActorKind::Dino => {
                let s = &gfx::SPRITE_CLAY_DINO;
                let index = match a.state {
                    _ if a.flash > 0 => anim(s, "hit", 6u32.saturating_sub(a.flash as u32)),
                    _ if a.calm() => anim(s, "idle", tick),
                    state::AIM => anim(s, "roar", DINO_ROAR_FRAMES.saturating_sub(a.timer) as u32),
                    state::LUNGE => anim(s, "run", tick * 2),
                    _ => anim(s, "walk", walk * 2),
                };
                // The clay dino faces right in its sheet.
                at(frame, s, index, x, feet, a.facing < 0);
            }
            ActorKind::Npc => {
                let (s, name): (&'static SpriteSheet, &str) = match speaker {
                    Some(Speaker::Cosmo) => (&gfx::SPRITE_COSMONAUT_WASSIE, "idle"),
                    _ => (&gfx::SPRITE_ZOOKEEPER, if game.scene == Scene::Dialog { "talk" } else { "idle" }),
                };
                at(frame, s, anim(s, name, tick), x, feet, a.facing < 0);
            }
            ActorKind::Portal => {
                let s = &gfx::SPRITE_WORMHOLE_PORTAL;
                // Spinning faster once it lets the wassie in.
                let speed = if game.world.portal_open() { 2 } else { 1 };
                at(frame, s, anim(s, "spin", tick * speed), x, feet, false);
            }
            ActorKind::Terminal => {
                let s = &gfx::SPRITE_ZOO_BREAKER;
                at(frame, s, anim(s, if lit { "on" } else { "off" }, tick), x, feet, false);
            }
            ActorKind::Gate => {
                let s = &gfx::SPRITE_POWER_GATE;
                at(frame, s, if lit { 2 } else { 0 }, x, feet, false);
            }
            // Casino and Badlands things: not in Sol City.
            ActorKind::Coin | ActorKind::Part | ActorKind::Jetpack | ActorKind::Dealer | ActorKind::Guard => {}
        }
    }
}
