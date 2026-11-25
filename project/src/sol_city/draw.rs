//! Draws a frame of the game from the studio's room and sprite sheets: the room under
//! the camera, then the actors, shots, effects, the wassie, the HUD and the dialogue.
//! agb puts objects shown first on top, so the frame is shown front to back.

use agb::{
    display::{Graphics, GraphicsFrame, object::Object},
    fixnum::vec2,
};
use diamond_hands_logic::{
    Game, PlayerState, Scene, SCREEN_H, SCREEN_W,
    block::BlockLook,
    boss::{APE_DYING_FRAMES, BossState, CRAB_DYING_FRAMES, HALO_LIVE_FRAMES, SPRAY_TELL_FRAMES},
    enemy::{Actor, ActorKind, COP_TELL_FRAMES, CRAB_SNAP_FRAMES, DINO_ROAR_FRAMES, state},
    fx::FxKind,
    level::{BossKind, Theme},
    player::{Aim, HOVER_FRAMES},
    shot::ShotKind,
    talk::Speaker,
    world::{CHARGE_FRAMES, SHIELD_MAX, WARP_FRAMES},
};
use pecten_studio_rt::{
    data::World,
    debug::{self, GameState},
    gfx::{SpriteSheet, WorldView},
    physics::Body,
};

use agb::display::object::SpriteVram;

use crate::{
    art::{LIFE_CELLS, studio, ui as box_ui},
    data::room,
    font::Font,
    gfx,
    screens::{self, HudArt},
};

/// Frames of the `ui` sheet after the font.
mod ui {
    pub const UP: usize = 51;
    pub const ALERT: usize = 52;
}

/// Frames of the `fx` sheet.
mod fx {
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

/// Tells GBA-Studio that the game is somewhere it has no map of (the menus, the world
/// map, the levels drawn in code), so its overlays have nothing to draw. The block can
/// only be published, not cleared, so this parks the camera far from every room
/// instead of leaving the last place in Sol City on the screen.
// Only the full game has such screens: the Sol City ROM never calls it.
#[allow(dead_code)]
pub fn publish_away() {
    const FAR: i32 = -1_000_000;
    let mut shown = GameState::new();
    (shown.camera_x, shown.camera_y) = (FAR, FAR);
    debug::publish(&shown);
}

pub struct Screen {
    /// The level painted in the studio that this screen draws.
    theme: Theme,
    /// The studio's map, and a view of its rooms that share the tileset of the room on
    /// screen. `None` for the frame between dropping one tileset's view and making the
    /// next: their tiles don't fit in VRAM together.
    world: &'static World<'static>,
    view: Option<WorldView<'static>>,
    /// The room whose backgrounds are behind the view.
    backdrop: usize,
    /// The font, HUD, pickups and dialogue box every other level shows.
    font: Font,
    life_caps: [SpriteVram; 2],
    life: [[SpriteVram; LIFE_CELLS]; 9],
    diamond: [SpriteVram; 2],
    heal: SpriteVram,
    /// The wassie's portrait: bill shut, then half and wide open.
    wassie_face: [SpriteVram; 3],
    /// The dialogue box as sprites (the room's backgrounds use all four layers), made on
    /// the level card or else when first needed, and the page last painted into it.
    dialog: Option<box_ui::DialogText>,
    dialog_painted: Option<(u8, u16, Option<i32>)>,
    /// The room's top-left in world pixels: the level's coordinates start there.
    origin: (i32, i32),
    /// Where the view's camera was last put, in world pixels; `None` before the first frame.
    camera: Option<(i32, i32)>,
}

/// A frame of a sheet with its top-left at screen pixel (x, y), unless well off screen.
/// Frames between the sheen sweeps over a breakable block, and how long one lasts (its
/// sprite's `sheen` animation: 8 steps of 3 frames).
const SHEEN_EVERY: u32 = 120;
const SHEEN_FRAMES: u32 = 24;

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

/// The first frame of an animation of a sheet, or 0 if it has none of that name.
fn first_of(sheet: &'static SpriteSheet, name: &str) -> usize {
    anim(sheet, name, 0)
}

impl Screen {
    /// `world` is the studio's map, read once at boot (`data::world()`); `theme` is the
    /// studio level to draw.
    pub fn new(world: &'static World<'static>, theme: Theme) -> Self {
        // The level's map starts at the top-left of its top-left room: Sol City's beach,
        // Ethereum City's Wormhole office.
        let first = match theme {
            Theme::EthCity => room::ETH_OFFICE,
            _ => room::BEACH,
        };
        let origin = world.origin(first).unwrap_or((0, 0));
        use studio::{hud_8x8 as hud, pickups, wassie_talk};
        let [half, wide] = wassie_talk::portrait_talk();
        Self {
            theme,
            world,
            view: None,
            backdrop: first,
            origin,
            camera: None,
            font: Font::new(),
            life_caps: hud::life_caps(),
            life: [
                hud::life_0(),
                hud::life_1(),
                hud::life_2(),
                hud::life_3(),
                hud::life_4(),
                hud::life_5(),
                hud::life_6(),
                hud::life_7(),
                hud::life_8(),
            ],
            diamond: pickups::diamond(),
            heal: pickups::heal(),
            wassie_face: [wassie_talk::portrait(), half, wide],
            dialog: None,
            dialog_painted: None,
        }
    }

    /// The level this screen draws.
    // Only the full game swaps levels: the Sol City ROM never asks.
    #[allow(dead_code)]
    pub fn theme(&self) -> Theme {
        self.theme
    }

    /// Puts the rooms the camera is in on the backgrounds: call before taking the
    /// frame. Into a room of another tileset, the old view goes first, leaving a black
    /// frame, and the new one is made on the next call; within a tileset, only the
    /// backgrounds behind change.
    pub fn prepare(&mut self, game: &Game, graphics: &mut Graphics) {
        let world = &game.world;
        let (x, y) = (self.origin.0 + world.camera_x + SCREEN_W / 2, self.origin.1 + world.camera_y + SCREEN_H / 2);
        let Some(room) = self.world.room_at_screen(x.div_euclid(SCREEN_W), y.div_euclid(SCREEN_H)) else {
            return;
        };
        match &mut self.view {
            Some(view) if !view.shows(room) => {
                self.view = None;
                self.camera = None;
            }
            Some(view) if room != self.backdrop => {
                view.set_backgrounds(room, graphics);
                self.backdrop = room;
            }
            Some(_) => {}
            None => {
                self.view = Some(WorldView::new(self.world, gfx::ART, room, graphics));
                self.backdrop = room;
                self.camera = None;
            }
        }
    }

    /// Tells GBA-Studio where the game is, for the Play panel's overlays: the room the
    /// wassie is in, the camera and the wassie's body, in the studio's world pixels (the level's
    /// coordinates plus the room it starts in). Call it once a frame, after the game
    /// has stepped. A build without the debug map does nothing with it.
    pub fn publish(&self, game: &Game) {
        let world = &game.world;
        let (ox, oy) = self.origin;
        let player = &world.player;
        let r = player.body.rect();
        let (x, y) = (ox + r.x, oy + r.y);
        let room = self.world.room_at_screen((x + r.w / 2).div_euclid(SCREEN_W), (y + r.h / 2).div_euclid(SCREEN_H)).unwrap_or(self.backdrop);
        let mut body = Body::new(x, y, r.w, r.h);
        // The game's speeds are in 1/256 pixel a frame, as the studio's are.
        (body.vx, body.vy, body.on_ground) = (player.body.vx_fx, player.body.vy_fx, player.grounded);
        let mut shown = GameState::new();
        shown.frame = game.frame;
        shown.room = room as u32;
        (shown.camera_x, shown.camera_y) = (ox + world.camera_x, oy + world.camera_y);
        shown.player = body;
        debug::publish(&shown);
    }

    pub fn draw(&mut self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        // Between two tilesets' views, the screen goes black for a frame.
        let Some(view) = &mut self.view else {
            return;
        };
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
                    view.set_camera(camera.0, camera.1);
                }
            }
            // The first frame, or a jump (a respawn, a cut into another zone): fill the
            // whole view at once.
            _ => view.load(camera.0, camera.1),
        }
        self.camera = Some(camera);

        // Front to back.
        let font = &self.font;
        match game.scene {
            Scene::Intro => screens::level_card(font, game, frame),
            // Only the standalone Sol City ROM gets here: the full game's renderer draws
            // its pause menu and map.
            Scene::Paused => {
                font.draw_centred(frame, "PAUSED", 60, true);
                font.draw_centred(frame, "START TO PLAY ON", 80, false);
            }
            Scene::LevelClear => {
                let r = world.player.body.rect();
                screens::level_clear(font, &self.diamond, game, frame, (r.centre_x() - cx, r.centre_y() - cy));
            }
            Scene::Map => {
                font.draw_centred(frame, "NEXT STOP: ETHEREUM CITY", 56, false);
                font.draw_centred(frame, "COMING SOON", 72, false);
                font.draw_centred(frame, "START TO PLAY SOL CITY AGAIN", 96, false);
            }
            _ => {}
        }
        if game.talking() {
            self.dialogue(game, frame);
        } else {
            self.dialog_painted = None;
            // Making the box takes a good many frames' work: do it on the level card,
            // where it can't be seen, rather than when someone first speaks.
            if game.scene == Scene::Intro && game.scene_frames() > 8 && self.dialog.is_none() {
                self.dialog = Some(box_ui::DialogText::for_sprites());
            }
        }
        if matches!(game.scene, Scene::Play | Scene::Intro) {
            let art = HudArt { life_caps: &self.life_caps, life: &self.life, diamond: &self.diamond };
            screens::hud(&self.font, &art, None, game, frame);
            self.shield_hud(game, frame);
            self.boss_bar(game, frame);
        }
        // Title cards stand alone, on black.
        if !matches!(game.scene, Scene::Map | Scene::LevelClear | Scene::Intro) {
            self.prompts(game, frame, cx, cy);
            self.effects(game, frame, cx, cy);
            self.shots(game, frame, cx, cy);
            self.wassie(game, frame, cx, cy);
            self.boss(game, frame, cx, cy);
            self.actors(game, frame, cx, cy);
            if let Some(view) = &self.view {
                view.show(frame);
            }
        }
    }

    /// The speaker's portrait in the dialogue box across the top, as every level shows
    /// it: the wassie on the left, anyone else on the right.
    fn dialogue(&mut self, game: &Game, frame: &mut GraphicsFrame) {
        let Some(line) = game.talk_lines().get(game.talk_line as usize) else {
            return;
        };
        let shown = game.talk_shown as usize;
        let typing = shown < line.len();
        let right = line.speaker != Speaker::Wassie;
        // The speaker bobs, and flaps their bill or jaw, while the words come out.
        let bob = (typing && (game.frame / 4).is_multiple_of(2)) as i32;
        let mouth = match (typing, (game.frame / 5) % 4) {
            (true, 1 | 3) => 1,
            (true, 2) => 2,
            _ => 0,
        };
        let (px, py) = box_ui::portrait_pos(right);
        match line.speaker {
            Speaker::Wassie => screens::show(frame, &self.wassie_face[mouth], px, py - bob),
            // The studio's portrait sheet: the zookeeper, then the cosmonaut, each shut
            // then talking.
            Speaker::Keeper => put(frame, &gfx::SPRITE_PORTRAITS, (mouth > 0) as usize, px, py - bob, false),
            Speaker::Cosmo => put(frame, &gfx::SPRITE_PORTRAITS, 2 + (mouth > 0) as usize, px, py - bob, false),
            Speaker::Cl207 => put(frame, &gfx::SPRITE_CL207_PORTRAIT, mouth, px, py - bob, false),
            // Cut from their sprites (`tools/crab-boss-sheet.py`): one face each, so
            // they only bob while they talk.
            Speaker::CrabBot => put(frame, &gfx::SPRITE_CRAB_BOT_BOSS_PORTRAIT, 0, px, py - bob, false),
            Speaker::Smolsnip => put(frame, &gfx::SPRITE_SMOLSNIP_PORTRAIT, 0, px, py - bob, false),
            Speaker::Ape => put(frame, &gfx::SPRITE_APE_BOSS_PORTRAIT, 0, px, py - bob, false),
            _ => {}
        }

        let arrow = (shown >= line.len()).then(|| ((game.frame / 8) % 2) as i32);
        let key = (game.talk_line, game.talk_shown, arrow);
        if self.dialog_painted != Some(key) {
            self.dialog_painted = Some(key);
            let mut rows = [""; diamond_hands_logic::talk::MAX_ROWS];
            for (slot, row) in rows.iter_mut().zip(line.visible(shown)) {
                *slot = row;
            }
            self.dialog
                .get_or_insert_with(box_ui::DialogText::for_sprites)
                .paint(right, &box_ui::Page { name: line.speaker.name(), rows, arrow });
        }
        if let Some(dialog) = &mut self.dialog {
            dialog.show_sprites(frame);
        }
    }

    /// The plasma shield's charges under the life bar, once CL207 has handed it over:
    /// a lit hexagon a charge, a dark one for each spent.
    fn shield_hud(&self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        if !world.has_shield() {
            return;
        }
        for i in 0..SHIELD_MAX {
            let lit = i < world.shield;
            // The last charge left blinks, as a warning.
            let warn = world.shield == 1 && lit && (game.frame / 16).is_multiple_of(2);
            put(frame, &gfx::SPRITE_SHIELD_HUD, (!lit || warn) as usize, 8 + 9 * i as i32, 14, false);
        }
    }

    /// The boss's name and health along the bottom of the screen while it fights, as
    /// every level shows it.
    fn boss_bar(&self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        if !world.boss.is_active() || world.boss.is_beaten() {
            return;
        }
        let y = 148;
        let name = world.level.boss_name;
        self.font.draw(frame, name, 4, y, false);
        let bar_x = (8 + Font::width(name, false) + 4).max(56);
        let full = world.boss.hp * 64 / world.boss.max_hp();
        for i in 0..8 {
            let filled = (full - i * 8).clamp(0, 8);
            put(frame, &gfx::SPRITE_HUD_8X8_2, filled as usize, bar_x + i * 8, y, false);
        }
    }

    /// The Big Crab Bot, from its sheet (`studio/sprites/crab-bot-boss.json`): one row
    /// of the author's mockup per attack. It faces the wassie; the sheet rushes right.
    fn boss(&self, game: &Game, frame: &mut GraphicsFrame, cx: i32, cy: i32) {
        let boss = &game.world.boss;
        if !boss.present {
            return;
        }
        if boss.kind == BossKind::Ape {
            self.ape(game, frame, cx, cy);
            return;
        }
        let s = &gfx::SPRITE_CRAB_BOT_BOSS;
        let r = boss.rect();
        if r.right() < cx - 32 || r.x > cx + SCREEN_W + 32 || r.bottom() < cy || r.y > cy + SCREEN_H + 48 {
            return;
        }
        let tick = game.frame;
        let first = |name| s.animation(name).map_or(0, |a| a.frame_at(0));
        let index = match boss.state {
            BossState::Dying => anim(s, "death", (CRAB_DYING_FRAMES - boss.timer) as u32),
            // The wreck stays where it fell: the last frame of its death.
            BossState::Defeated => first("death") + 5,
            _ if boss.flash > 0 => first("hit") + (6 - boss.flash.min(6) as usize) / 2,
            // Dazed against the wall after a Rush: its eyes gone orange.
            BossState::Stunned => first("hit") + 2,
            // Crouched, kicking up sand, then the charge itself.
            BossState::Telegraph => first("rush") + (tick / 6 % 2) as usize,
            BossState::Charge => first("rush") + 2 + (tick / 4 % 3) as usize,
            // The cannons tilt, then the jets leave them as the volley goes.
            BossState::Spray => match boss.timer {
                0..=4 => first("water") + 4,
                5..=8 => first("water") + 3,
                t => first("water") + ((SPRAY_TELL_FRAMES.saturating_sub(t)) / 6).min(2) as usize,
            },
            // The ring gathers, crackles, then dies down.
            BossState::Halo if boss.timer > HALO_LIVE_FRAMES => first("halo") + 1,
            BossState::Halo if boss.timer > 8 => first("halo") + 2 + (tick / 4 % 3) as usize,
            BossState::Halo => first("halo") + 5,
            BossState::Walk if boss.body.vx_fx != 0 => anim(s, "move", (r.x.unsigned_abs() / 2) * 6),
            _ => anim(s, "idle", tick),
        };
        if boss.flash > 0 && boss.state != BossState::Dying && (tick / 2).is_multiple_of(2) {
            return;
        }
        at(frame, s, index, r.centre_x() - cx, r.bottom() - cy, boss.facing < 0);
        // "!" over it when an attack is coming, as over any villain.
        let tell = matches!(boss.state, BossState::Telegraph | BossState::Spray)
            || (boss.state == BossState::Halo && boss.timer > HALO_LIVE_FRAMES);
        if tell && game.scene == Scene::Play {
            put(frame, &gfx::SPRITE_UI, ui::ALERT, r.centre_x() - 4 - cx, r.y - 22 - cy, false);
        }
    }

    /// BORED KONG, from its sheet (`studio/sprites/ape-boss.json`), facing right there.
    fn ape(&self, game: &Game, frame: &mut GraphicsFrame, cx: i32, cy: i32) {
        let boss = &game.world.boss;
        let s = &gfx::SPRITE_APE_BOSS;
        let r = boss.rect();
        if r.right() < cx - 32 || r.x > cx + SCREEN_W + 32 || r.bottom() < cy || r.y > cy + SCREEN_H + 64 {
            return;
        }
        let tick = game.frame;
        let first = |name| s.animation(name).map_or(0, |a| a.frame_at(0));
        let index = match boss.state {
            BossState::Dying => anim(s, "death", (APE_DYING_FRAMES - boss.timer) as u32),
            BossState::Defeated => first("death") + 2,
            _ if boss.flash > 0 => first("hit"),
            BossState::Stunned => first("stunned"),
            BossState::Windup | BossState::Intro => anim(s, "pound", tick),
            BossState::Leap => first("leap"),
            BossState::Recover if boss.timer > 12 => first("slam"),
            BossState::Recover => first("crouch"),
            // The masonry held back, then flung.
            BossState::Throw if boss.timer > 8 => first("throw"),
            BossState::Throw => first("throw") + 1,
            BossState::Telegraph => first("crouch"),
            BossState::Charge => anim(s, "charge", tick),
            BossState::Walk if boss.body.vx_fx != 0 => anim(s, "walk", (r.x.unsigned_abs() / 2) * 4),
            _ => anim(s, "idle", tick),
        };
        if boss.flash > 0 && boss.state != BossState::Dying && (tick / 2).is_multiple_of(2) {
            return;
        }
        at(frame, s, index, r.centre_x() - cx, r.bottom() - cy, boss.facing < 0);
        let tell = matches!(boss.state, BossState::Windup | BossState::Telegraph) || (boss.state == BossState::Throw && boss.timer > 8);
        if tell && game.scene == Scene::Play {
            put(frame, &gfx::SPRITE_UI, ui::ALERT, r.centre_x() - 4 - cx, r.y - 22 - cy, false);
        }
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
                FxKind::CopWreck => {
                    // Down behind its shield, smoking, then it blinks out.
                    let s = &gfx::SPRITE_BAD_COP;
                    if e.age > 66 && e.age % 4 < 2 {
                        continue;
                    }
                    at(frame, s, anim(s, "defeat", e.age as u32), e.x - cx, e.y - cy, e.flip);
                    continue;
                }
                FxKind::CrabWreck => {
                    // Sputters, slumps flat and smokes, then blinks out.
                    let s = &gfx::SPRITE_CRAB_BOT;
                    if e.age > 66 && e.age % 4 < 2 {
                        continue;
                    }
                    at(frame, s, anim(s, "death", e.age as u32), e.x - cx, e.y - cy, false);
                    continue;
                }
                // The Big Crab Bot's water jets burst into droplets where they land.
                FxKind::Splat => (fx::GLINT, 3),
                // Casino effects: Sol City has none.
                FxKind::CoinPop | FxKind::NinjaWreck => continue,
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
                // The Big Crab Bot's Water Cannons lob jets (the code's thrown kind, as
                // SAM's cucumbers are), and its halo throws sparks along the floor.
                ShotKind::Cucumber => at(frame, sheet, fx::GLINT + 1 + (game.frame / 4 % 2) as usize, x, y, false),
                ShotKind::Shockwave => at(frame, sheet, fx::SPARKS + (game.frame / 3 % 2) as usize, x, y, false),
                // A Bad Cop's canister spins as it arcs; BORED KONG's masonry tumbles.
                ShotKind::Canister => {
                    let s = &gfx::SPRITE_BAD_COP_CANISTER;
                    at(frame, s, anim(s, "spin", game.frame), x, y, false);
                }
                ShotKind::Rubble => {
                    let s = &gfx::SPRITE_APE_RUBBLE;
                    at(frame, s, anim(s, "spin", game.frame), x, y, false);
                }
                _ => at(frame, sheet, fx::BULLET, x, y, left),
            }
        }
    }

    /// The wassie and its gear: the cannon, the jetpack and its flame, the muzzle flash.
    fn wassie(&self, game: &Game, frame: &mut GraphicsFrame, cx: i32, cy: i32) {
        let world = &game.world;
        let p = &world.player;
        // The shield's bubble, in front of the wassie: shown even while it blinks after
        // the bubble soaked up a hit.
        self.shield_bubble(game, frame, cx, cy);
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

    /// CL207's plasma shield around the wassie while it has charges: a humming ring, a
    /// bright flash when it soaks up a hit or takes an emerald, flickering on its last
    /// charge.
    fn shield_bubble(&self, game: &Game, frame: &mut GraphicsFrame, cx: i32, cy: i32) {
        let world = &game.world;
        let p = &world.player;
        if world.warp > 0 || p.is_dead() || (world.shield == 0 && world.shield_flash == 0) {
            return;
        }
        let s = &gfx::SPRITE_PLASMA_SHIELD;
        let index = if world.shield_flash > 0 {
            first_of(s, "hit") + (world.shield_flash / 3 % 2) as usize
        } else {
            if world.shield == 1 && (game.frame / 4) % 3 == 0 {
                return;
            }
            anim(s, "hum", game.frame)
        };
        let r = p.body.rect();
        at(frame, s, index, r.centre_x() - cx, r.centre_y() - 1 - cy, false);
    }

    fn actors(&self, game: &Game, frame: &mut GraphicsFrame, cx: i32, cy: i32) {
        let world = &game.world;
        let mut person = 0;
        for a in world.placed_actors() {
            // People are numbered in map order, whoever they are, as the level's lines are.
            let speaker = if a.kind.is_person() {
                person += 1;
                world.level.npcs.get(person - 1).and_then(|lines| lines.iter().map(|l| l.speaker).find(|&s| s != Speaker::Wassie))
            } else {
                None
            };
            if !a.alive || (a.flash > 0 && (game.frame / 2).is_multiple_of(2)) {
                continue;
            }
            let r = a.rect();
            // Off screen first: the console wraps sprite positions, so one far away
            // would show up somewhere on screen.
            if r.right() < cx - 32 || r.x > cx + SCREEN_W + 32 || r.bottom() < cy - 64 || r.y > cy + SCREEN_H + 64 {
                continue;
            }
            match a.kind {
                // Mostly still, with a glint now and then, as in every level.
                ActorKind::Diamond => {
                    let glint = (game.frame + r.x as u32) % 48 < 6;
                    screens::show(frame, &self.diamond[glint as usize], r.x - 4 - cx, r.y - 4 - cy);
                    continue;
                }
                ActorKind::Heal => {
                    screens::show(frame, &self.heal, r.x - 3 - cx, r.y - 3 - cy);
                    continue;
                }
                // Green, bobbing a pixel, its facets catching the light now and then.
                ActorKind::Emerald => {
                    let s = &gfx::SPRITE_EMERALD;
                    let phase = (game.frame + r.x as u32 * 3) % 72;
                    let index = if phase < 32 { anim(s, "glint", phase) } else { 0 };
                    let bob = (game.frame / 20 % 2) as i32;
                    put(frame, s, index, r.x - 3 - cx, r.y - 3 - bob - cy, false);
                    continue;
                }
                _ => {}
            }
            self.actor(game, frame, a, speaker, r.centre_x() - cx, r.bottom() - cy);
        }
    }

    /// One actor, with its feet at (x, feet) on screen.
    #[allow(clippy::too_many_arguments)]
    fn actor(&self, game: &Game, frame: &mut GraphicsFrame, a: &Actor, speaker: Option<Speaker>, x: i32, feet: i32) {
        let tick = game.frame;
        let walk = (a.body.x().unsigned_abs() / 4) as u32;
        let lit = a.state == state::LIT;
        match a.kind {
            ActorKind::Diamond | ActorKind::Heal | ActorKind::Emerald => {}
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
            ActorKind::Crab => {
                let s = &gfx::SPRITE_CRAB_BOT;
                let index = match a.state {
                    _ if a.flash > 0 => anim(s, "hit", 6u32.saturating_sub(a.flash as u32) * 2),
                    // Claws up: the tell, held until the snap.
                    state::AIM => anim(s, "snap", 4),
                    state::LUNGE => anim(s, "snap", 8 + CRAB_SNAP_FRAMES.saturating_sub(a.timer) as u32),
                    _ if a.body.vx_fx == 0 => anim(s, "idle", tick),
                    _ => anim(s, "walk", walk * 2),
                };
                // Facing the wassie in its sheet; it snaps toward its facing side.
                at(frame, s, index, x, feet, a.facing < 0);
            }
            // Smolsnip, the Big Crab Bot's prisoner, facing out of its cell.
            ActorKind::Prisoner => at(frame, &gfx::SPRITE_SMOLSNIP, 0, x, feet, false),
            ActorKind::Cop => {
                let s = &gfx::SPRITE_BAD_COP;
                let index = match a.state {
                    _ if a.flash > 0 => anim(s, "hit", 6u32.saturating_sub(a.flash as u32) * 2),
                    // The wind-up, shield coming down, then the throw held a moment.
                    state::AIM => anim(s, "attack", (COP_TELL_FRAMES.saturating_sub(a.timer) as u32) * 25 / COP_TELL_FRAMES as u32),
                    state::RECOIL => first_of(s, "attack") + 5,
                    _ if a.body.vx_fx == 0 => anim(s, "idle", tick),
                    _ => anim(s, "walk", walk * 2),
                };
                // Its sheet faces right, shield forward.
                at(frame, s, index, x, feet, a.facing < 0);
            }
            // CL207 in the hazmat suit: the top half over the legs, facing the wassie.
            ActorKind::Mentor => {
                let s = &gfx::SPRITE_CL207;
                let (w, flip) = (s.width as i32, a.facing > 0);
                put(frame, s, 1, x - w / 2, feet - 16, flip);
                put(frame, s, 0, x - w / 2, feet - 32, flip);
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
            ActorKind::Block(look) => {
                let s = match look {
                    BlockLook::Sand => &gfx::SPRITE_SAND_BLOCK_SHEEN,
                    BlockLook::Crate => &gfx::SPRITE_DOCK_CRATE_SHEEN,
                    BlockLook::Soil => &gfx::SPRITE_SOIL_BLOCK_SHEEN,
                };
                // A band of light sweeps across it now and then, as on a loading
                // skeleton, to say it can be shot apart. Neighbours sweep at different
                // times; where it stood, so a falling block keeps its rhythm.
                let (hx, hy) = a.home;
                let phase = (tick + (hx / 16 * 29 + hy / 16 * 13).unsigned_abs()) % SHEEN_EVERY;
                let index = if phase < SHEEN_FRAMES { anim(s, "sheen", phase) } else { anim(s, "block", tick) };
                at(frame, s, index, x, feet, false);
            }
            // Casino and Badlands things: not in Sol City.
            ActorKind::Coin | ActorKind::Part | ActorKind::Jetpack | ActorKind::Dealer | ActorKind::Guard => {}
        }
    }
}
