//! Maps game state onto backgrounds and objects each frame.
//!
//! Background layers, front to back: the dialogue box (BG P0, only while talking),
//! terrain (BG P1, streamed column by column as the camera moves), the ruined city
//! (BG P2, ⅓ parallax), sky (BG P3, static). The world map and the pause menu are
//! layers of their own, each shown alone. Every object is P0, above all of them;
//! objects shown first are drawn on top, so text and HUD go first.

use alloc::{vec, vec::Vec};

use agb::{
    display::{
        GraphicsFrame, HEIGHT, Priority, WIDTH,
        object::{Object, SpriteVram},
        tiled::{DynamicTile16, RegularBackground, RegularBackgroundSize, TileEffect, TileFormat},
    },
    fixnum::{Num, num, vec2},
};
use diamond_hands_logic::{
    Difficulty, Game, Level, Progress, Scene, TILE,
    boss::BossState,
    controls::{self, BINDINGS},
    enemy::{ActorKind, GUARD_BURST_GAP, GUARD_RECOIL_FRAMES, SENTRY_MUZZLE, SENTRY_RECOIL_FRAMES, state},
    fx::FxKind,
    game::{CLEAR_LOCK_FRAMES, Deal, FILES, MAP_WALK_FRAMES, PAYOUT_FRAMES, PENGUIN_WALK_FRAMES, file_menu, options_menu, pause_menu, title_menu},
    inventory::SHIELD_CELLS,
    level::Theme,
    levels::{MAP, PlaceStatus},
    player::{Aim, HOVER_FRAMES, MAX_HP, PlayerState},
    shot::ShotKind,
    arrival::{GUIDE_W, Gear},
    talk::{self, Speaker},
    world::{CHARGE_FRAMES, LIFT_H, LIFT_RIDE_FRAMES, LIFT_W, LiftState},
};

use crate::{
    art::{
        Common, SPRITE_PALETTE, TileCache,
        arrival::{self, ArrivalArt},
        badlands::{self, Bera, SENTRY_ANCHOR, SKY_PALETTES, SentryPose, Villains},
        casino::{self, ArenaArt, CasinoArt, FloorArt, NINJA_ANCHOR, NINJA_FLAME_X, NinjaPose, SamPose},
        menu::{self, MenuScreen},
        trenches, ui,
        wassie::{self, Wassie},
    },
    font::{self, Font},
};

/// Art only one level uses, loaded while that level is on screen: sprite VRAM holds
/// one boss and its extras at a time.
enum LevelArt {
    Badlands(Bera, ArrivalArt),
    Trenches(CasinoArt),
}

impl LevelArt {
    fn new(theme: Theme) -> Self {
        match theme {
            Theme::Badlands => LevelArt::Badlands(Bera::new(), ArrivalArt::new()),
            Theme::Trenches => LevelArt::Trenches(CasinoArt::new()),
        }
    }

    fn theme(&self) -> Theme {
        match self {
            LevelArt::Badlands(..) => Theme::Badlands,
            LevelArt::Trenches(_) => Theme::Trenches,
        }
    }

    fn casino(&self) -> Option<&CasinoArt> {
        match self {
            LevelArt::Trenches(c) => Some(c),
            LevelArt::Badlands(..) => None,
        }
    }

    fn floor(&self) -> Option<&FloorArt> {
        self.casino()?.floor.as_ref()
    }

    fn arena(&self) -> Option<&ArenaArt> {
        self.casino()?.arena.as_ref()
    }

    fn casino_mut(&mut self) -> Option<&mut CasinoArt> {
        match self {
            LevelArt::Trenches(c) => Some(c),
            LevelArt::Badlands(..) => None,
        }
    }
}

/// Most terrain keys either theme has.
const MAX_TERRAIN_KEYS: usize = if badlands::TERRAIN_KEYS > trenches::TERRAIN_KEYS {
    badlands::TERRAIN_KEYS
} else {
    trenches::TERRAIN_KEYS
};

/// Terrain map size in tiles. Level cell `(col, row)` is written into slot
/// `(col % 64, row % 32)`, so a level wraps around the map both ways as it scrolls.
const TERRAIN_COLS: i32 = 64;
const TERRAIN_ROWS: i32 = 32;
/// Rows a screen can show at once, counting the part rows at its top and bottom.
const VIEW_ROWS: i32 = HEIGHT / TILE + 1;
/// Stage behind the menus: the Badlands at dusk.
const MENU_STAGE: u8 = 0;
/// Terrain uses background palette 1; the backdrop uses 0.
const TERRAIN_PALETTE: u8 = 1;
/// Most terrain cells written per frame. Scrolling needs at most a column and a row;
/// after the camera jumps (a respawn) the screen refills over a few frames, about three
/// columns a frame: each cell costs the VRAM manager a few thousand cycles, so a whole
/// screen at once would run a frame long.
const CELLS_PER_FRAME: i32 = 64;
/// Columns and rows loaded ahead off each edge of the screen, and how many cells a
/// frame may load there.
const LOOKAHEAD: i32 = 2;
const LOOKAHEAD_CELLS_PER_FRAME: i32 = 12;
/// How far the city and the sky drift up as the camera goes down a tall level, as a
/// fraction of its depth: they're far off, so they move less than the rock does. Both
/// layers are 256 pixels tall, so neither may scroll more than 96 without wrapping.
const CITY_DEPTH: i32 = 3;
const SKY_DEPTH: i32 = 6;
const MAX_BACKDROP_Y: i32 = 256 - HEIGHT;
/// Falling at least this fast draws wind streaks over the wassie.
const SPEED_LINES_FX: i32 = 4 * diamond_hands_logic::FX;
const KEY_UNSET: u16 = u16::MAX;
const KEY_BLANK: u16 = u16::MAX - 1;
/// Integer sine for 16 directions, scaled by 64.
const SIN16: [i32; 16] = [0, 24, 45, 59, 64, 59, 45, 24, 0, -24, -45, -59, -64, -59, -45, -24];
/// Diamonds thrown out in the level-clear burst, and those raining after it.
const BURST: i32 = 8;
const RAIN: i32 = 6;
/// How far BERA 3000's head lifts through one breath, a step every few frames.
const BERA_BREATH: [i32; 8] = [0, 1, 2, 2, 2, 1, 0, 0];

pub struct Renderer {
    common: Common,
    wassie: Wassie,
    villains: Villains,
    /// The loaded level's own art; `None` only between dropping one and loading the next.
    level_art: Option<LevelArt>,
    cl207: SpriteVram,
    font: Font,
    sky: RegularBackground,
    city: RegularBackground,
    terrain: RegularBackground,
    dialog_box: RegularBackground,
    dialog_text: ui::DialogText,
    /// The line, characters shown and prompt arrow last painted into `dialog_text`.
    dialog_painted: Option<(u8, u16, Option<i32>)>,
    /// The world map's layer, painted when the map opens and dropped when it closes:
    /// its tiles don't fit in background VRAM beside a level's.
    map: Option<RegularBackground>,
    /// Map markers for places still being built and places still locked, and the badge
    /// on cleared ones.
    soon: SpriteVram,
    locked: SpriteVram,
    clear_badge: SpriteVram,
    /// The pause menu's layer, only while the game is paused.
    menu: Option<MenuScreen>,
    terrain_tiles: TileCache,
    /// Tile for each [`badlands::terrain_key`], built on first use: [`KEY_UNSET`] until
    /// then, [`KEY_BLANK`] for transparent cells. On the heap, as it is a few KB.
    terrain_by_key: Vec<u16>,
    blank: DynamicTile16,
    /// The level whose terrain tiles are being made ahead of time, and the next cell
    /// (row-major) to look at.
    warm_level: Option<&'static Level>,
    warm_next: i32,
    /// Level column held by each terrain map slot, and the rows of it loaded there
    /// (first, and the one after the last).
    columns: [(i32, i32, i32); TERRAIN_COLS as usize],
    level: Option<&'static Level>,
    /// The theme and stage whose palettes are loaded.
    theme: Theme,
    stage: u8,
    /// What the terrain palette last showed: fence wire state (see
    /// [`diamond_hands_logic::world::fence_state`]), the marquee's chase step, and
    /// whether the market was crashing.
    fence: (u8, bool, bool),
    /// A theme switch under way, and its next step.
    switching: Option<(Theme, u8)>,
    /// The plasma cells the file owned last frame, to announce a new one, and when.
    cells_seen: u16,
    notice_until: u32,
    /// This frame's camera, including screen shake.
    view_x: i32,
    /// The top of the screen in level pixels: world objects are drawn this much higher.
    view_y: i32,
    /// Whether the file owned the jetpack last frame, to announce finding it, and until
    /// when.
    jetpack_seen: bool,
    gear_until: u32,
    /// Priority for the world's objects: below the dialogue box while it is up.
    world_layer: Priority,
}

fn layer(priority: Priority, size: RegularBackgroundSize) -> RegularBackground {
    RegularBackground::new(priority, size, TileFormat::FourBpp)
}

impl Renderer {
    pub fn new(gfx: &mut agb::display::Graphics) -> Self {
        gfx.set_background_palettes(&[
            SKY_PALETTES[0].clone(),
            badlands::ground_palette(0, 0),
            SPRITE_PALETTE.clone(),
            ui::DIALOG_PALETTE.clone(),
        ]);

        let mut sky = layer(Priority::P3, RegularBackgroundSize::Background32x32);
        badlands::paint_sky(&mut sky);
        let mut city = layer(Priority::P2, RegularBackgroundSize::Background32x32);
        badlands::paint_city(&mut city);
        let mut dialog_box = layer(Priority::P0, RegularBackgroundSize::Background32x64);
        ui::paint_dialog_box(&mut dialog_box);
        let dialog_text = ui::DialogText::new(&mut dialog_box);

        let mut renderer = Self {
            common: Common::new(),
            wassie: Wassie::new(),
            villains: Villains::new(),
            level_art: Some(LevelArt::new(Theme::Badlands)),
            cl207: arrival::portrait(),
            font: Font::new(),
            sky,
            city,
            terrain: layer(Priority::P1, RegularBackgroundSize::Background64x32),
            dialog_box,
            dialog_text,
            dialog_painted: None,
            map: None,
            soon: ui::soon_marker(),
            locked: ui::lock_marker(),
            clear_badge: ui::clear_badge(),
            menu: None,
            terrain_tiles: TileCache::new(),
            terrain_by_key: vec![KEY_UNSET; MAX_TERRAIN_KEYS],
            blank: DynamicTile16::new().fill_with(0),
            columns: [(i32::MIN, 0, 0); TERRAIN_COLS as usize],
            level: None,
            theme: Theme::Badlands,
            stage: 0,
            fence: (0, false, false),
            switching: None,
            cells_seen: 0,
            notice_until: 0,
            view_x: 0,
            view_y: 0,
            warm_level: None,
            warm_next: 0,
            jetpack_seen: true,
            gear_until: 0,
            world_layer: Priority::P0,
        };
        // Fill the first screen now, while booting, rather than in the first frame.
        let level = diamond_hands_logic::levels::get(0);
        while !renderer.stream_terrain(level, 0..=WIDTH / TILE, 0, HEIGHT / TILE, CELLS_PER_FRAME) {}
        renderer
    }

    pub fn draw(&mut self, game: &Game, frame: &mut GraphicsFrame) {
        if game.scene == Scene::Paused {
            self.draw_menu(game, frame);
            return;
        }
        // Off the menu, its tiles go back to the dialogue text.
        if let Some(mut menu) = self.menu.take() {
            menu.clear(&mut self.dialog_box, &self.blank);
            self.dialog_text.restore(&mut self.dialog_box);
        }
        if game.scene == Scene::Map {
            if self.map.is_none() {
                let mut map = layer(Priority::P0, RegularBackgroundSize::Background32x32);
                ui::paint_map(&mut map);
                self.map = Some(map);
            }
            self.draw_map(game, frame);
            return;
        }
        self.map = None;
        let world = &game.world;
        // Big hits jolt the view a pixel or two; each ordinary shot, a pixel each way.
        let shake = match () {
            _ if game.scene != Scene::Play => 0,
            _ if world.shake > 0 => [2, -2, 1, -1][(world.shake % 4) as usize],
            _ if world.kick > 0 => [1, -1][(world.kick % 2) as usize],
            _ => 0,
        };
        let cam = world.camera_x + shake;
        self.view_x = cam;
        self.world_layer = if game.talking() { Priority::P1 } else { Priority::P0 };
        let in_menu =
            matches!(game.scene, Scene::Title | Scene::Options | Scene::Files | Scene::FileChoice);
        // The menus show the top of the first level, as a backdrop.
        let cam_y = if in_menu { 0 } else { world.camera_y };
        self.view_y = cam_y;

        // Each level's stretches have their own light: the Badlands go from dusk to a
        // burning night; the Trenches from the strip to the casino and the exchange's
        // lobby, and up the tower to the night again. Below ground, the Trenches'
        // strata keep palettes of their own (see `trenches::TERRAIN_SLOTS`).
        let stage = if in_menu { MENU_STAGE } else { world.level.stage_at(cam + WIDTH / 2, cam_y + HEIGHT / 2) };
        let theme = world.level.theme;
        if theme != self.theme && !self.switch_theme(theme) {
            // Mid-switch: show nothing of either level this frame.
            return;
        }
        if self.level_art.as_ref().is_none_or(|a| a.theme() != theme) {
            // Drop the old level's sprites before making the new ones.
            self.level_art = None;
            self.level_art = Some(LevelArt::new(theme));
        }
        self.swap_casino_art(world);
        let crashing = theme == Theme::Trenches
            && matches!(world.boss.state, BossState::Crash | BossState::Barrage | BossState::Dying);
        let fence = (
            diamond_hands_logic::world::fence_state(world.frames),
            (game.frame / 10).is_multiple_of(2),
            crashing && (game.frame / 6).is_multiple_of(2),
        );
        if stage != self.stage || self.fence.0 == u8::MAX {
            let skies = badlands::sky_palettes(theme);
            frame.set_background_palette(0, skies.get(stage as usize).unwrap_or(&skies[0]));
            self.stage = stage;
            self.fence.0 = u8::MAX;
        }
        if fence != self.fence {
            match theme {
                Theme::Badlands => frame.set_background_palette(TERRAIN_PALETTE, &badlands::ground_palette(stage, fence.0)),
                Theme::Trenches => {
                    // Every stratum's light, and its scenery's muted copy.
                    for (stratum, (&slot, &scenery)) in
                        trenches::TERRAIN_SLOTS.iter().zip(trenches::SCENERY_SLOTS.iter()).enumerate()
                    {
                        let palette = trenches::stratum_palette(stratum, stage, fence.0, fence.1, fence.2);
                        frame.set_background_palette(scenery, &trenches::scenery_palette(&palette));
                        frame.set_background_palette(slot, &palette);
                    }
                }
            }
            self.fence = fence;
        }

        let level = world.level;
        let (left, top) = (cam.div_euclid(TILE), cam_y.div_euclid(TILE));
        let terrain_ready = self.stream_terrain(level, left..=left + WIDTH / TILE, top, top + VIEW_ROWS, CELLS_PER_FRAME);
        if terrain_ready && in_menu {
            // Off screen, a little at a time, load the rows the level starts on, so the
            // first frame of play has nothing new to draw.
            let start = diamond_hands_logic::World::start_camera_y(level).div_euclid(TILE);
            let rows = (start + VIEW_ROWS).min(TERRAIN_ROWS);
            self.stream_terrain(level, left..=left + WIDTH / TILE, 0, rows, LOOKAHEAD_CELLS_PER_FRAME);
        } else if terrain_ready && game.scene == Scene::Play {
            // And a margin round the screen, so a column or row scrolling in is usually
            // there already: no frame has to write a whole one.
            let (cols, rows) = (left - LOOKAHEAD..=left + WIDTH / TILE + LOOKAHEAD, top - LOOKAHEAD..top + VIEW_ROWS + LOOKAHEAD);
            self.stream_terrain(level, cols, rows.start, rows.end, LOOKAHEAD_CELLS_PER_FRAME);
        }
        // Play keeps the CPU busy and the menus and the level card don't; the frames that
        // paint dialogue are dear enough already.
        let warm = match game.scene {
            Scene::Play => Some((12, 1)),
            Scene::Intro | Scene::Title | Scene::Files | Scene::FileChoice | Scene::Options => Some((64, 4)),
            _ => None,
        };
        if let Some((cells, new)) = warm.filter(|_| terrain_ready) {
            self.warm_terrain(level, cells, new);
        }
        self.terrain
            .set_scroll_pos(vec2(cam.rem_euclid(TERRAIN_COLS * TILE), cam_y.rem_euclid(TERRAIN_ROWS * TILE)));
        // Going down a tall level, the far-off city and sky drift up a little, from the
        // height they were painted for.
        let drift = (cam_y - if in_menu { 0 } else { level.backdrop_y }).max(0);
        let city_y = (drift / CITY_DEPTH).min(MAX_BACKDROP_Y);
        let sky_y = (drift / SKY_DEPTH).min(MAX_BACKDROP_Y);
        self.city.set_scroll_pos(vec2((cam / 3).rem_euclid(256), city_y));
        self.sky.set_scroll_pos(vec2(0, sky_y));
        if game.talking() {
            // Everyone but the wassie talks from the right: scroll to the copy of the box
            // with its frame there.
            let right = self.speaker(game).is_some_and(|s| s != Speaker::Wassie);
            self.paint_dialog_text(game, right);
            self.dialog_box
                .set_scroll_pos(vec2(0, if right { ui::BOX_SCROLL_RIGHT } else { 0 }));
            self.dialog_box.show(frame);
        } else {
            self.dialog_painted = None;
        }
        let terrain_id = terrain_ready.then(|| self.terrain.show(frame));
        let city_id = self.city.show(frame);
        let sky_id = self.sky.show(frame);
        // Dim the scene behind text screens so the words read clearly. The level-clear
        // screen fades down to it.
        // The elevator ride fades the screen out and back in.
        let ride = world.lift.filter(|l| l.state == LiftState::Riding).map(|l| {
            let half = LIFT_RIDE_FRAMES as i32 / 2;
            (16 - (l.timer as i32 - half).abs() * 16 / half).clamp(0, 16) as u8
        });
        // Underground, the city and the sky fade out behind the rock.
        let underground = if in_menu { 0 } else { world.level.underground(cam_y) };
        if game.talking() || game.ringing() || ride.is_some() || matches!(game.scene, Scene::Intro | Scene::LevelClear | Scene::Exchange) {
            let amount = match game.scene {
                _ if ride.is_some() => Num::from_raw(ride.unwrap_or(0).min(15)),
                Scene::LevelClear => Num::from_raw(game.scene_frames().min(8) as u8),
                _ => num!(0.5),
            };
            let mut dim = frame.blend().darken(amount);
            dim.enable_background(city_id).enable_background(sky_id).enable_backdrop();
            if let Some(id) = terrain_id {
                dim.enable_background(id);
            }
        } else if underground > 0 || (!terrain_ready && !in_menu) {
            // A screen still filling in after the camera jumped (a respawn) goes dark
            // behind the sprites until the rock is all there, rather than flashing an
            // empty sky.
            let amount = if terrain_ready { underground } else { 16 };
            frame
                .blend()
                .darken(Num::from_raw(amount))
                .enable_background(city_id)
                .enable_background(sky_id)
                .enable_backdrop();
        }

        let cells = diamond_hands_logic::world::cells(game.parts | world.parts);
        if game.scene == Scene::Play && cells.count_ones() > self.cells_seen.count_ones() {
            self.notice_until = game.frame + 150;
        }
        if game.scene != Scene::Paused {
            self.cells_seen = cells;
        }
        let jetpack = world.player.jetpack;
        if game.scene == Scene::Play && jetpack && !self.jetpack_seen {
            self.gear_until = game.frame + 180;
        }
        self.jetpack_seen = jetpack;
        self.draw_overlay(game, frame);
        if matches!(game.scene, Scene::Play | Scene::Intro) {
            self.draw_hud(game, frame);
        }
        self.villains.sentry.sweep();
        if let Some(floor) = self.level_art.as_ref().and_then(LevelArt::floor) {
            floor.ninja.sweep();
        }
        if !in_menu {
            self.draw_fences(game, frame);
        }
        self.draw_fx(game, frame);
        self.draw_player(game, frame);
        self.draw_lift(game, frame);
        self.draw_arrival(game, frame);
        self.draw_shots(game, frame);
        self.draw_boss(game, frame);
        // The menus need every object for their text, and the bots are frozen anyway.
        if !in_menu && game.scene != Scene::Exchange {
            self.draw_actors(game, frame);
        }
    }

    /// Paints the current line into the dialogue box, if it changed since last frame:
    /// the speaker's nameplate, up to three rows of `talk::ROW_CHARS`, and a bobbing
    /// arrow once the line is typed out.
    fn paint_dialog_text(&mut self, game: &Game, right: bool) {
        let Some(line) = game.talk_lines().get(game.talk_line as usize) else {
            return;
        };
        let shown = game.talk_shown as usize;
        let arrow = (shown >= line.len()).then(|| ((game.frame / 8) % 2) as i32);
        let key = (game.talk_line, game.talk_shown, arrow);
        if self.dialog_painted == Some(key) {
            return;
        }
        self.dialog_painted = Some(key);
        let mut rows = [""; talk::MAX_ROWS];
        for (slot, row) in rows.iter_mut().zip(line.visible(shown)) {
            *slot = row;
        }
        let page = ui::Page { name: line.speaker.name(), rows, arrow };
        self.dialog_text.paint(right, &page);
    }

    fn speaker(&self, game: &Game) -> Option<Speaker> {
        let lines = game.talk_lines();
        lines.get(game.talk_line as usize).map(|line| line.speaker)
    }

    /// In the Trenches, swaps the floors' sprites for the arena's as the camera nears
    /// SAM, and back if a respawn takes it away. One half is dropped a frame before the
    /// other is made: sprites the last frame showed are only freed once it is on screen.
    fn swap_casino_art(&mut self, world: &diamond_hands_logic::World) {
        // In a tall level, other storeys may run under the arena: only SAM's own counts.
        let boss = world.boss.rect();
        let level_with = world.camera_y < boss.bottom() + 8 && world.camera_y + HEIGHT > boss.y - 8;
        let near_sam = world.boss.present && level_with && world.camera_x + WIDTH >= boss.x - 8;
        let far = !world.boss.present || !level_with || world.camera_x + WIDTH < boss.x - 40;
        let Some(casino) = self.level_art.as_mut().and_then(LevelArt::casino_mut) else {
            return;
        };
        if near_sam || world.arena {
            if casino.floor.take().is_none() && casino.arena.is_none() {
                casino.arena = Some(ArenaArt::new());
            }
        } else if far {
            if casino.arena.take().is_none() && casino.floor.is_none() {
                casino.floor = Some(FloorArt::new());
            }
        }
    }

    /// Moves to another theme over three frames: the old terrain's tiles are let go,
    /// then the sky is repainted, then the city. Background VRAM can't hold two levels'
    /// art at once, and tiles a shown frame still uses are only freed once it is on
    /// screen, so each step waits for the frame before. Returns true once done.
    fn switch_theme(&mut self, theme: Theme) -> bool {
        let step = match self.switching {
            Some((to, step)) if to == theme => step,
            _ => 0,
        };
        match step {
            0 => {
                for slot in 0..TERRAIN_COLS {
                    for ty in 0..TERRAIN_ROWS {
                        let effect = TileEffect::new(false, false, TERRAIN_PALETTE);
                        self.terrain.set_tile_dynamic16((slot, ty), &self.blank, effect);
                    }
                }
                self.terrain_tiles = TileCache::new();
                self.terrain_by_key.fill(KEY_UNSET);
                self.warm_level = None;
                self.columns = [(i32::MIN, 0, 0); TERRAIN_COLS as usize];
            }
            1 if theme == Theme::Trenches => trenches::paint_sky(&mut self.sky),
            1 => badlands::paint_sky(&mut self.sky),
            _ if theme == Theme::Trenches => trenches::paint_city(&mut self.city),
            _ => badlands::paint_city(&mut self.city),
        }
        if step < 2 {
            self.switching = Some((theme, step + 1));
            return false;
        }
        self.switching = None;
        self.theme = theme;
        self.fence.0 = u8::MAX;
        true
    }

    /// Writes up to `budget` terrain cells into the map, for rows `top..bottom` of
    /// columns `cols`: whole columns coming in from the side, and the rows coming in at
    /// the top or bottom of the columns already there. At most [`TERRAIN_COLS`] columns
    /// and [`TERRAIN_ROWS`] rows. Returns whether they are all loaded.
    fn stream_terrain(
        &mut self,
        level: &'static Level,
        cols: core::ops::RangeInclusive<i32>,
        top: i32,
        bottom: i32,
        budget: i32,
    ) -> bool {
        if !self.level.is_some_and(|l| core::ptr::eq(l, level)) {
            self.columns = [(i32::MIN, 0, 0); TERRAIN_COLS as usize];
            self.level = Some(level);
        }
        let (top, bottom) = (top.max(0), bottom.min(level.height()));
        let mut budget = budget;
        for col in cols {
            if col < 0 || col >= level.width() {
                continue;
            }
            let slot = col.rem_euclid(TERRAIN_COLS) as usize;
            let (held, lo, hi) = self.columns[slot];
            // The rows of the column already in the map, clipped to those wanted.
            let (lo, hi) = if held == col { (lo.max(top), hi.min(bottom)) } else { (top, top) };
            let (lo, hi) = if lo < hi { (lo, hi) } else { (top, top) };
            let count = (lo - top) + (bottom - hi);
            if count == 0 {
                continue;
            }
            if count > budget {
                // Grow the column's run by what fits, keeping it one run.
                if budget > 0 && hi > lo {
                    let (from, to) = if hi < bottom { (hi, (hi + budget).min(bottom)) } else { ((lo - budget).max(top), lo) };
                    for ty in from..to {
                        self.write_terrain(level, col, ty);
                    }
                    self.columns[slot] = (col, lo.min(from), hi.max(to));
                }
                return false;
            }
            budget -= count;
            for ty in (top..lo).chain(hi..bottom) {
                self.write_terrain(level, col, ty);
            }
            self.columns[slot] = (col, top, bottom);
        }
        true
    }

    fn terrain_key(level: &Level, col: i32, ty: i32) -> usize {
        match level.theme {
            Theme::Trenches => trenches::terrain_key(level, col, ty),
            Theme::Badlands => badlands::terrain_key(level, col, ty),
        }
    }

    /// Makes the tile for terrain `key` if it isn't made yet.
    fn make_terrain_tile(&mut self, theme: Theme, key: usize) {
        if self.terrain_by_key[key] == KEY_UNSET {
            let pixels = if theme == Theme::Trenches { trenches::terrain_pixels(key) } else { badlands::terrain_pixels(key) };
            let id = self.terrain_tiles.intern(pixels);
            self.terrain_by_key[key] = id.map_or(KEY_BLANK, |id| id as u16);
        }
    }

    /// Makes terrain tiles ahead of time, at most `new` of them, looking at up to
    /// `cells` cells of the level. Making a tile is the dear part of drawing a cell, so
    /// this runs while there is time to spare, and scrolling into new ground later
    /// only has to place tiles that already exist.
    fn warm_terrain(&mut self, level: &'static Level, cells: i32, new: i32) {
        if !self.warm_level.is_some_and(|l| core::ptr::eq(l, level)) {
            self.warm_level = Some(level);
            self.warm_next = 0;
        }
        let (w, total) = (level.width(), level.width() * level.height());
        let mut made = 0;
        for _ in 0..cells {
            if self.warm_next >= total {
                return;
            }
            let key = Self::terrain_key(level, self.warm_next % w, self.warm_next / w);
            if self.terrain_by_key[key] == KEY_UNSET {
                if made == new {
                    return;
                }
                made += 1;
                self.make_terrain_tile(level.theme, key);
            }
            self.warm_next += 1;
        }
    }

    /// Writes the level's cell `(col, ty)` into its terrain map slot.
    fn write_terrain(&mut self, level: &'static Level, col: i32, ty: i32) {
        let key = Self::terrain_key(level, col, ty);
        self.make_terrain_tile(level.theme, key);
        let tile = match self.terrain_by_key[key] {
            KEY_BLANK => &self.blank,
            id => &self.terrain_tiles.tiles[id as usize],
        };
        let slot = (col.rem_euclid(TERRAIN_COLS), ty.rem_euclid(TERRAIN_ROWS));
        let palette = match level.theme {
            Theme::Trenches => trenches::palette_slot(level, ty, key),
            Theme::Badlands => TERRAIN_PALETTE,
        };
        self.terrain.set_tile_dynamic16(slot, tile, TileEffect::new(false, false, palette));
    }

    fn draw_overlay(&self, game: &Game, frame: &mut GraphicsFrame) {
        let blink_on = (game.frame / 30).is_multiple_of(2);
        let level = game.world.level;
        match game.scene {
            Scene::Title => self.draw_title(game, frame),
            Scene::Options => self.draw_options(game, frame),
            Scene::Files => self.draw_files(game, frame),
            Scene::FileChoice => self.draw_file_choice(game, frame),
            Scene::Intro => {
                let mut buf = [0; 10];
                let n = font::number(game.world.level_index as u32 + 1, &mut buf);
                let x = (WIDTH - Font::width("LEVEL ", true) - Font::width(n, true)) / 2;
                self.font.draw(frame, "LEVEL ", x, 36, true);
                self.font.draw(frame, n, x + Font::width("LEVEL ", true), 36, true);
                self.font.draw_centred(frame, level.name, 55, true);
                self.font.draw_centred(frame, level.subtitle, 76, false);
            }
            Scene::Play | Scene::Map | Scene::Splash | Scene::Paused => {}
            Scene::Arrival if game.talking() => self.draw_dialog(game, frame),
            Scene::Arrival => {}
            Scene::Dialog if game.ringing() => self.draw_ringing(game, frame),
            Scene::Dialog => self.draw_dialog(game, frame),
            Scene::LevelClear => self.draw_clear(game, frame, blink_on),
            Scene::Exchange => self.draw_exchange(game, frame),
        }
        if game.scene == Scene::Play && game.frame < self.notice_until {
            let n = diamond_hands_logic::world::cells(game.parts | game.world.parts).count_ones().min(SHIELD_CELLS);
            let mut text = *b"PLASMA CELL 0/3";
            text[12] = b'0' + n as u8;
            text[14] = b'0' + SHIELD_CELLS as u8;
            self.font.draw_centred(frame, core::str::from_utf8(&text).unwrap_or(""), 26, false);
            if n == SHIELD_CELLS {
                self.font.draw_centred(frame, "CL207 CAN BUILD THE SHIELD!", 36, false);
            }
        }
        if game.scene == Scene::Play && game.frame < self.gear_until {
            self.font.draw_centred(frame, "JETPACK FOUND!", 26, false);
            self.font.draw_centred(frame, "HOLD A IN THE AIR TO HOVER", 36, false);
        }
    }

    /// CL207's call coming in: a phone buzzing and who's calling.
    fn draw_ringing(&self, game: &Game, frame: &mut GraphicsFrame) {
        let t = game.scene_frames();
        let buzz = ((t / 3) % 2) as i32 * 2 - 1;
        if let Some(floor) = self.level_art.as_ref().and_then(LevelArt::floor) {
            show(frame, &floor.phone, 112 + buzz, 30);
        }
        if (t / 20).is_multiple_of(2) || t > 40 {
            self.font.draw_centred(frame, "INCOMING CALL", 52, true);
        }
        self.font.draw_centred(frame, "CL207", 70, false);
        show(frame, &self.cl207, 104, 82);
        self.font.draw_centred(frame, "A: ANSWER", 122, false);
    }

    /// The penguin's exchange: he waddles in, makes his pitch, then the counter shows the
    /// coins, the rate and the diamonds they buy; on a deal the numbers count across.
    fn draw_exchange(&self, game: &Game, frame: &mut GraphicsFrame) {
        let (Some(casino), Some(arena)) =
            (self.level_art.as_ref().and_then(LevelArt::casino), self.level_art.as_ref().and_then(LevelArt::arena))
        else {
            return;
        };
        let ex = game.exchange;
        let t = game.scene_frames();
        // The penguin, walking in from the right.
        let stand = 172;
        let x = match ex.deal {
            Deal::Walk => WIDTH + 8 - (WIDTH + 8 - stand) * t.min(PENGUIN_WALK_FRAMES) as i32 / PENGUIN_WALK_FRAMES as i32,
            _ => stand,
        };
        let waddle = ex.deal == Deal::Walk && (t / 8).is_multiple_of(2);
        let hop = if ex.deal == Deal::Payout { ((t / 6) % 2) as i32 * 2 } else { waddle as i32 };
        Object::new(arena.penguin.clone())
            .set_pos(vec2(x, 128 - 32 - hop))
            .set_hflip(waddle)
            .set_priority(self.world_layer)
            .show(frame);
        if game.talking() {
            self.draw_dialog(game, frame);
        }
        if ex.deal == Deal::Offer {
            let row = 17 + (1 + game.cursor as i32) * 10;
            self.draw_cursor(game, frame, 24, row - 2);
        }
        if ex.deal < Deal::Offer || ex.coins == 0 {
            return;
        }
        // The counter: what's on it, the rate, and what it pays.
        let paying = ex.deal == Deal::Payout;
        let done = ex.deal == Deal::Bye || paying && t >= PAYOUT_FRAMES;
        let progress = if done { PAYOUT_FRAMES } else if paying { t } else { 0 };
        let accepted = ex.accepted == Some(true);
        let coins_shown = if accepted { ex.coins - ex.coins * progress / PAYOUT_FRAMES } else { ex.coins };
        let paid = if accepted { ex.diamonds() * progress / PAYOUT_FRAMES } else { 0 };
        let y = 64;
        show(frame, &casino.coin[0], 46, y + 1);
        self.label_number_at(frame, "GOLD COINS: ", coins_shown, 58, y);
        self.font.draw(frame, "EXCHANGE RATE: 1:8", 58, y + 11, false);
        show(frame, &self.common.diamond[((t / 6) % 8 == 0) as usize], 40, y + 18);
        let diamonds = if ex.deal == Deal::Offer { ex.diamonds() } else { paid };
        let label = if ex.deal == Deal::Offer { "DIAMONDS RECEIVED: " } else { "DIAMONDS PAID: " };
        self.label_number_at(frame, label, diamonds, 58, y + 22);
        if ex.deal == Deal::Bye || paying {
            // The wallet was paid when the deal was struck; count it up on screen.
            let before = if accepted { game.wallet - ex.diamonds().min(game.wallet) } else { game.wallet };
            self.label_number_at(frame, "WALLET: ", before + paid, 58, y + 33);
        }
        if paying && !done {
            // Coins hop off towards the penguin, diamonds back the other way.
            for i in 0..3 {
                let k = ((t as i32 + i * 10) % 30) as i32;
                let wx = 60 + k * 4;
                show(frame, &casino.coin[(k / 3 % 3) as usize], wx, 120 - (k * (30 - k)) / 8);
                show(frame, &self.common.diamond[0], 176 - k * 4, 116 - (k * (30 - k)) / 8);
            }
        }
    }

    /// The level-clear sequence: the screen fades down, a ring of diamonds bursts off
    /// the wassie, the banner drops in and the tally counts up row by row, then
    /// diamonds rain until the world map opens.
    fn draw_clear(&self, game: &Game, frame: &mut GraphicsFrame, blink_on: bool) {
        let world = &game.world;
        let t = game.scene_frames() as i32;
        const BOUNCE: [i32; 6] = [0, -3, -4, -3, -1, 0];
        let banner_y = match t {
            0..16 => 26 - (16 - t) * 3,
            16..22 => 26 + BOUNCE[(t - 16) as usize],
            _ => 26,
        };
        self.font.draw_centred(frame, "LEVEL CLEAR!", banner_y, true);
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
                    self.label_number(frame, label, n, 44 + 8 * i as i32);
                }
            }
            if rows > 5 {
                self.label_time(frame, world.frames / 60, 84);
            }
        } else {
            if rows > 0 {
                self.label_number(frame, "DIAMONDS ", world.diamonds, 48);
            }
            if rows > 1 {
                self.label_number(frame, "BOUNTIES ", world.kills, 58);
            }
            if rows > 2 {
                self.label_time(frame, world.frames / 60, 68);
            }
            if rows > 3 {
                self.label_number(frame, "SCORE ", game.score(), 80);
                self.label_number(frame, "BEST ", game.best, 90);
            }
        }
        if rows > 4 {
            if game.new_best && blink_on {
                self.font.draw_centred(frame, "NEW BEST!", 100, false);
            }
            if let Some(place) = game.fresh.map(|i| &MAP[i as usize]) {
                self.font.draw_centred(frame, "NEW PLACE UNLOCKED", 110, false);
                self.font.draw_centred(frame, place.name, 118, false);
            } else if game.replay {
                self.font.draw_centred(frame, "REPLAY: NO SCORE BANKED", 112, false);
            }
        }
        if t >= CLEAR_LOCK_FRAMES as i32 {
            self.font.draw_centred(frame, "A: WORLD MAP", 132, false);
        }

        // The burst: a spinning ring off the wassie, flying out and away.
        let r = world.player.body.rect();
        let (cx, cy) = (r.centre_x() - self.view_x, r.centre_y() - self.view_y);
        if t < 48 {
            let radius = t * 5 / 2;
            for i in 0..BURST {
                let a = (i * 2 + t / 4) as usize;
                let (dx, dy) = (SIN16[(a + 4) % 16], SIN16[a % 16]);
                let glint = ((t / 3 + i) % 4 == 0) as usize;
                show(frame, &self.common.diamond[glint], cx - 8 + dx * radius / 64, cy - 8 + dy * radius / 64);
            }
        } else {
            // Then a gentle rain of diamonds, each at its own pace.
            for i in 0..RAIN {
                let x = 14 + i * 38 + (i * 13) % 17;
                let y = ((t - 48) * (2 + i % 3) / 2 + i * 47) % 200 - 24;
                let glint = ((t / 4 + i) % 6 == 0) as usize;
                show(frame, &self.common.diamond[glint], x, y);
            }
        }
    }

    fn draw_title(&self, game: &Game, frame: &mut GraphicsFrame) {
        self.font.draw_centred(frame, "DIAMOND HANDS", 6, true);
        self.font.draw_centred(frame, "A WASSIE ADVENTURE", 23, false);
        if game.dev_cleared != 0 {
            self.font.draw(frame, "DEV", 4, 4, false);
        }

        let play = if game.can_continue() { "CONTINUE" } else { "PLAY" };
        let items = [
            (title_menu::PLAY, play),
            (title_menu::MAP, "MAP"),
            (title_menu::OPTIONS, "OPTIONS"),
        ];
        let x = (WIDTH - Font::width("CONTINUE", true)) / 2;
        for (i, (row, label)) in items.into_iter().enumerate() {
            let y = 36 + i as i32 * 17;
            self.font.draw(frame, label, x, y, true);
            if game.cursor == row {
                self.draw_cursor(game, frame, x, y);
            }
        }
        if game.best > 0 {
            // "FILE n" and "BEST n" share a row, two spaces apart.
            let mut a = [0; 10];
            let mut b = [0; 10];
            let file = font::number(game.file as u32 + 1, &mut a);
            let best = font::number(game.best, &mut b);
            let parts = ["FILE ", file, "  BEST ", best];
            let mut x = (WIDTH - parts.iter().map(|p| Font::width(p, false)).sum::<i32>()) / 2;
            for part in parts {
                self.font.draw(frame, part, x, 89, false);
                x += Font::width(part, false);
            }
        } else {
            self.label_number(frame, "FILE ", game.file as u32 + 1, 89);
        }

        // Controls, right of where the wassie stands.
        self.draw_controls(frame, 84, 101);
    }

    /// The controls list, keys at `x` and what they do in play beside them, from row
    /// `y`. The rows come from the game's own bindings.
    fn draw_controls(&self, frame: &mut GraphicsFrame, x: i32, y: i32) {
        let (keys_x, action_x) = (x, x + 42);
        for (i, binding) in BINDINGS.iter().enumerate() {
            let y = y + i as i32 * 9;
            self.font.draw(frame, controls::label(binding.keys), keys_x, y, false);
            self.font.draw(frame, binding.play, action_x, y, false);
        }
    }

    /// The three save files, right of where the wassie stands.
    fn draw_files(&self, game: &Game, frame: &mut GraphicsFrame) {
        self.font.draw_centred(frame, "SELECT FILE", 8, true);
        let x = 84;
        for i in 0..FILES {
            let y = 32 + i as i32 * 27;
            let mut buf = [0; 10];
            self.font.draw(frame, "FILE", x, y, true);
            let n = font::number(i as u32 + 1, &mut buf);
            self.font.draw(frame, n, x + Font::width("FILE ", true), y, true);
            // The score sits on the file's row, against the right edge.
            if let Some(run) = game.files[i] {
                let score = font::number(run.score(), &mut buf);
                self.font.draw_right(frame, score, WIDTH - 4, y + 4, false);
                let label_right = WIDTH - 4 - Font::width(score, false);
                self.font.draw_right(frame, "SCORE ", label_right, y + 4, false);
            }
            self.draw_run(frame, game.files[i], x, y + 17);
            if game.cursor as usize == i {
                self.draw_cursor(game, frame, x, y);
            }
        }
    }

    /// A file's run on one small line from (x, y): where it stands. The score is left
    /// to the caller.
    fn draw_run(&self, frame: &mut GraphicsFrame, run: Option<Progress>, x: i32, y: i32) {
        let Some(run) = run else {
            self.font.draw(frame, "NEW", x, y, false);
            return;
        };
        // A space within a field, two between fields.
        let mut x = x;
        let mut text = |frame: &mut GraphicsFrame, s: &str, gap: &str| {
            self.font.draw(frame, s, x, y, false);
            x += Font::width(s, false) + Font::width(gap, false);
        };
        let mut buf = [0; 10];
        if run.complete {
            text(frame, "ALL CLEAR", "");
        } else {
            text(frame, diamond_hands_logic::levels::get(run.level).name, "  ");
            if run.beacons > 0 {
                text(frame, "BEACON", " ");
                text(frame, font::number(run.beacons as u32, &mut buf), "");
            }
        }
    }

    /// Continue the chosen file's run, or start a new one in it.
    fn draw_file_choice(&self, game: &Game, frame: &mut GraphicsFrame) {
        let mut buf = [0; 10];
        let n = font::number(game.file as u32 + 1, &mut buf);
        let x = (WIDTH - Font::width("FILE ", true) - Font::width(n, true)) / 2;
        self.font.draw(frame, "FILE", x, 8, true);
        self.font.draw(frame, n, x + Font::width("FILE ", true), 8, true);
        let x = 84;
        let run = game.files[game.file as usize];
        self.draw_run(frame, run, x, 28);
        if let Some(run) = run {
            self.label_number_at(frame, "SCORE ", run.score(), x, 37);
        }
        let rows = [(file_menu::CONTINUE, "CONTINUE"), (file_menu::NEW_GAME, "NEW GAME")];
        for (i, (row, label)) in rows.into_iter().enumerate() {
            let y = 51 + i as i32 * 17;
            self.font.draw(frame, label, x, y, true);
            if game.cursor == row {
                self.draw_cursor(game, frame, x, y);
            }
        }
        self.font.draw(frame, "THE FILE KEEPS THIS RUN", x, 88, false);
        self.font.draw(frame, "UNTIL A NEW ONE SAVES.", x, 97, false);
        self.font.draw(frame, "B: BACK", x, 110, false);
    }

    fn draw_options(&self, game: &Game, frame: &mut GraphicsFrame) {
        self.font.draw_centred(frame, "OPTIONS", 10, true);

        let x = 48;
        let row_y = 36;
        self.font.draw(frame, "DIFFICULTY", x, row_y, true);
        let (name, about) = match game.difficulty {
            Difficulty::Chill => ("CHILL", "HALF DAMAGE. VILLAINS SHOOT LESS."),
            Difficulty::Normal => ("NORMAL", "THE WILD WEST AS INTENDED."),
            Difficulty::Degen => ("DEGEN", "DOUBLE DAMAGE. FULL SEND."),
        };
        // The value on its own row under the label: "< NORMAL >".
        let (value_x, value_y) = (x + 12, row_y + 18);
        self.font.draw(frame, "<", value_x, value_y, true);
        self.font.draw(frame, name, value_x + 16, value_y, true);
        self.font
            .draw(frame, ">", value_x + 18 + Font::width(name, true), value_y, true);
        self.font.draw_centred(frame, about, value_y + 21, false);

        let back_y = 92;
        self.font.draw(frame, "BACK", x, back_y, true);
        let cursor_y = if game.cursor == options_menu::DIFFICULTY { row_y } else { back_y };
        self.draw_cursor(game, frame, x, cursor_y);

        self.font
            .draw_centred(frame, "UP/DOWN: SELECT   LEFT/RIGHT: CHANGE", 120, false);
        self.font.draw_centred(frame, "B: BACK", 130, false);
    }

    /// The speaker's portrait beside the line of dialogue (painted into the box by
    /// [`Self::paint_dialog_text`]): the wassie on the left, anyone else on the right.
    fn draw_dialog(&self, game: &Game, frame: &mut GraphicsFrame) {
        let lines = game.talk_lines();
        let Some(line) = lines.get(game.talk_line as usize) else {
            return;
        };
        let typing = (game.talk_shown as usize) < line.len();
        let (px, py) = ui::portrait_pos(line.speaker != Speaker::Wassie);
        // The speaker bobs while the words come out.
        let bob = (typing && (game.frame / 4).is_multiple_of(2)) as i32;
        if let Some(portrait) = self.portrait(line.speaker) {
            show(frame, portrait, px, py - bob);
        }
    }

    /// Each speaker's face for the dialogue box. A new speaker needs a 32x32 portrait
    /// here and a name in `talk::Speaker`.
    fn portrait(&self, speaker: Speaker) -> Option<&SpriteVram> {
        Some(match (speaker, self.level_art.as_ref()?) {
            (Speaker::Wassie, _) => &self.wassie.portrait,
            (Speaker::Cl207, _) => &self.cl207,
            (Speaker::Bera, LevelArt::Badlands(bera, _)) => &bera.portrait,
            (Speaker::Sam, LevelArt::Trenches(c)) => &c.arena.as_ref()?.sam.portrait,
            (Speaker::Penguin, LevelArt::Trenches(c)) => &c.arena.as_ref()?.penguin,
            _ => return None,
        })
    }

    /// The world map: every place with a marker for how far the file has got there
    /// (cleared, open, locked or still being built), the wassie on the chosen one, and
    /// what lies there. After a clear, the wassie walks the road to the place it
    /// unlocked.
    fn draw_map(&mut self, game: &Game, frame: &mut GraphicsFrame) {
        self.font.draw_centred(frame, "WORLD MAP", 6, true);
        let mut buf = [0; 10];
        let mut total = [0; 10];
        let cleared = font::number(game.map_cleared().count_ones(), &mut buf);
        let built = font::number(diamond_hands_logic::levels::ALL.len() as u32, &mut total);
        let tally_w = Font::width("CLEARED ", false) + Font::width(cleared, false) + Font::width("/", false);
        let x = (WIDTH - tally_w - Font::width(built, false)) / 2;
        self.font.draw(frame, "CLEARED", x, 24, false);
        self.font.draw(frame, cleared, x + Font::width("CLEARED ", false), 24, false);
        self.font.draw(frame, "/", x + tally_w - Font::width("/", false), 24, false);
        self.font.draw(frame, built, x + tally_w, 24, false);

        let index = (game.cursor as usize).min(MAP.len() - 1);
        let place = &MAP[index];
        let status = place.status(game.map_cleared());
        self.font.draw_centred(frame, place.name, 113, true);
        match status {
            PlaceStatus::Cleared => self.font.draw_centred(frame, "CLEARED", 130, false),
            PlaceStatus::Open if game.fresh == Some(index as u8) => {
                self.font.draw_centred(frame, "NEW! UNLOCKED", 130, false)
            }
            PlaceStatus::Open => self.font.draw_centred(frame, "UNLOCKED", 130, false),
            PlaceStatus::Locked => {
                let before = MAP[index.saturating_sub(1)].name;
                let w = Font::width("LOCKED: CLEAR ", false) + Font::width(before, false);
                let x = (WIDTH - w) / 2;
                self.font.draw(frame, "LOCKED: CLEAR", x, 130, false);
                self.font.draw(frame, before, x + Font::width("LOCKED: CLEAR ", false), 130, false);
            }
            PlaceStatus::Soon => self.font.draw_centred(frame, "COMING SOON", 130, false),
            PlaceStatus::Upcoming if game.fresh == Some(index as u8) => {
                self.font.draw_centred(frame, "NEW! UNLOCKED: COMING SOON", 130, false)
            }
            PlaceStatus::Upcoming => self.font.draw_centred(frame, "UNLOCKED: COMING SOON", 130, false),
        }
        self.font.draw_centred(frame, place.blurb, 141, false);
        let hint = if place.playable(game.map_cleared()) { "A: PLAY   B: BACK" } else { "B: BACK" };
        self.font.draw_centred(frame, hint, 152, false);

        // The wassie walks the road to a newly unlocked place, then hops on the spot.
        let t = game.scene_frames() as i32;
        let walk = game.map_walk.filter(|_| t < MAP_WALK_FRAMES as i32);
        let ((px, py), hop) = match walk {
            Some((from, to)) => {
                let (a, b) = (&MAP[from as usize], &MAP[to as usize]);
                let n = MAP_WALK_FRAMES as i32;
                let at = (a.x + (b.x - a.x) * t / n, a.y + (b.y - a.y) * t / n);
                (at, (t / 4 % 2) * 2)
            }
            None => ((place.x, place.y), (game.frame / 12 % 2) as i32),
        };
        let (x, y) = (px - wassie::CENTRE_COL, py - 30 - hop);
        for (sprite, dx) in self.wassie.standing(game.frame) {
            show(frame, sprite, x + dx, y);
        }
        for (i, p) in MAP.iter().enumerate() {
            let (mx, my) = (p.x - 8, p.y - 8);
            match p.status(game.map_cleared()) {
                PlaceStatus::Soon => show(frame, &self.soon, mx, my),
                PlaceStatus::Upcoming => {
                    // Unlocked but not built: the beacon, glinting, flagged while new.
                    if game.fresh == Some(i as u8) && walk.is_none() {
                        let bob = (game.frame / 8 % 2) as i32;
                        show(frame, &self.common.alert, p.x + 6, my - 14 - bob);
                    }
                    if (game.frame + i as u32 * 16) % 24 < 6 {
                        show(frame, &self.common.glint[0], mx + 4, my - 4);
                    }
                    show(frame, &self.soon, mx, my);
                }
                PlaceStatus::Locked => show(frame, &self.locked, mx, my),
                status => {
                    // Open places glint often and, when new, wave a "!" overhead.
                    let open = status == PlaceStatus::Open;
                    let period = if open { 24 } else { 64 };
                    let glint = (game.frame + i as u32 * 16) % period < 6;
                    if status == PlaceStatus::Cleared {
                        show(frame, &self.clear_badge, mx + 10, my - 2);
                    } else if game.fresh == Some(i as u8) && walk.is_none() {
                        let bob = (game.frame / 8 % 2) as i32;
                        show(frame, &self.common.alert, p.x + 6, my - 14 - bob);
                    }
                    show(frame, &self.common.diamond[glint as usize], mx, my);
                }
            }
        }
        if let Some(map) = &self.map {
            map.show(frame);
        }
    }

    /// The pause menu, painted into the dialogue box's spare rows, plus the animated
    /// parts as sprites on top: the selection arrow and the wassie in the bag, glints
    /// on the wallet's diamond. The dialogue text's tiles are lent to the menu while
    /// it is open.
    fn draw_menu(&mut self, game: &Game, frame: &mut GraphicsFrame) {
        if self.menu.is_none() {
            self.dialog_text.release(&mut self.dialog_box, &self.blank);
            self.menu = Some(MenuScreen::new());
        }
        let m = game.menu;
        match m.tab {
            pause_menu::BAG if !m.inspecting => {
                let (cx, cy) = menu::item_cursor(m.item);
                self.draw_cursor(game, frame, cx + 12, cy - 1);
                let (fx, feet) = menu::WASSIE_FEET;
                let (x, y) = (fx - wassie::CENTRE_COL, feet - 31);
                let [(body, bdx), (blaster, gdx)] = self.wassie.standing(game.frame);
                show(frame, blaster, x + gdx, y);
                let (jx, jy) = wassie::JETPACK_AT;
                show(frame, &self.wassie.jetpack, x + jx, y + jy);
                show(frame, body, x + bdx, y);
            }
            pause_menu::WALLET => {
                for (i, (gx, gy)) in menu::GEM_GLINTS.into_iter().enumerate() {
                    let t = (game.frame + i as u32 * 23) % 70;
                    if t < 8 {
                        show(frame, &self.common.glint[(t / 4) as usize], gx - 4, gy - 4);
                    }
                }
            }
            _ => {}
        }
        if let Some(menu) = &mut self.menu {
            menu.paint(game, &mut self.dialog_box);
        }
        self.dialog_box.set_scroll_pos(vec2(0, menu::SCROLL_Y));
        self.dialog_box.show(frame);
    }

    /// Whether something spanning level pixel rows `top..bottom` shows on screen. Sprites
    /// far above or below must be skipped, not drawn: object positions wrap at 256.
    fn on_screen_y(&self, top: i32, bottom: i32) -> bool {
        bottom > self.view_y - 16 && top < self.view_y + HEIGHT + 16
    }

    /// Shows a sprite of the world (not text or HUD) at screen position (x, y).
    fn put(&self, frame: &mut GraphicsFrame, sprite: &SpriteVram, x: i32, y: i32) {
        Object::new(sprite.clone())
            .set_pos(vec2(x, y))
            .set_priority(self.world_layer)
            .show(frame);
    }

    /// Arrow left of a menu item at (x, y), nudging forward and back.
    fn draw_cursor(&self, game: &Game, frame: &mut GraphicsFrame, x: i32, y: i32) {
        let nudge = (game.frame / 16 % 2) as i32;
        show(frame, &self.common.cursor, x - 12 + nudge, y + 1);
    }

    fn draw_hud(&self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        let hp = world.player.hp.clamp(0, MAX_HP);
        show(frame, &self.common.life_caps[0], 4, 4);
        for (i, cell) in self.common.life[hp as usize].iter().enumerate() {
            show(frame, cell, 8 + 8 * i as i32, 4);
        }
        show(frame, &self.common.life_caps[1], 64, 4);

        let mut buf = [0; 10];
        show(frame, &self.common.diamond[0], 70, 0);
        self.font
            .draw(frame, font::number(world.diamonds, &mut buf), 86, 5, false);
        if let Some(casino) = self.level_art.as_ref().and_then(LevelArt::casino) {
            show(frame, &casino.coin[0], 114, 4);
            self.font.draw(frame, font::number(world.coins, &mut buf), 125, 5, false);
        }
        let score = font::number(game.score(), &mut buf);
        self.font
            .draw(frame, score, WIDTH - 4 - Font::width(score, false), 5, false);

        if world.boss.is_active() && !world.boss.is_beaten() {
            let y = 148;
            let name = world.level.boss_name;
            self.font.draw(frame, name, 4, y, false);
            let bar_x = (8 + Font::width(name, false) + 4).max(56);
            let full = world.boss.hp * 64 / world.boss.max_hp();
            for i in 0..8 {
                let filled = (full - i * 8).clamp(0, 8);
                show(frame, &self.common.boss_cell[filled as usize], bar_x + i * 8, y);
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

    /// `label` then `n`, from (x, y).
    fn label_number_at(&self, frame: &mut GraphicsFrame, label: &str, n: u32, x: i32, y: i32) {
        let mut buf = [0; 10];
        self.font.draw(frame, label, x, y, false);
        self.font.draw(frame, font::number(n, &mut buf), x + Font::width(label, false), y, false);
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
        let cam = self.view_x;
        let blinking = p.invuln > 0 && !p.is_dead() && (game.frame / 3).is_multiple_of(2);
        let r = p.body.rect();
        // On arrival the wassie starts inside the ship, and owns nothing but its scarf
        // until CL207 hands over the blaster and the jetpack.
        let arrival = game.arrival_view();
        let in_lift = world.lift.is_some_and(|l| l.riding());
        if blinking || in_lift || !self.on_screen_y(r.y, r.bottom()) || arrival.is_some_and(|v| !v.wassie_out) {
            return;
        }
        let (armed, strapped) = arrival.map_or((true, false), |v| (v.blaster, v.jetpack));
        let left = p.facing < 0;
        let anim = (r.x.unsigned_abs()) / 2;
        // Top-left of the wassie's 32x32 frame. Parts are placed in it facing right and
        // mirrored inside it when facing left.
        let col = if left { 31 - wassie::CENTRE_COL } else { wassie::CENTRE_COL };
        let (x, y) = (r.centre_x() - col - cam, r.bottom() - 32);
        let part = |(ox, oy): (i32, i32), w: i32| (x + if left { 32 - ox - w } else { ox }, y + oy);
        let flipped = |sprite: &SpriteVram, (px, py): (i32, i32), frame: &mut GraphicsFrame| {
            Object::new(sprite.clone())
                .set_pos(vec2(px, py - self.view_y))
                .set_hflip(left)
                .set_priority(self.world_layer)
                .show(frame);
        };

        // Muzzle effects first so they sit on top of the wassie.
        let (mx, my) = p.muzzle();
        if p.recoil > 0 {
            let late = (p.recoil < 4) as usize;
            flipped(&self.common.muzzle[late], (mx - 4 - cam, my - 4), frame);
            // A forward shot also throws a ring of light round the barrel's tip.
            if p.aim == Aim::Forward {
                self.put(frame, &self.common.muzzle_ring[late], mx - 4 + p.facing * 2 - cam, my - 8 - self.view_y);
            }
        } else if p.charge >= CHARGE_FRAMES {
            let pulse = (game.frame / 4 % 2) as usize;
            let glow = &self.common.charge[if pulse == 0 { 1 } else { 0 }];
            self.put(frame, glow, mx - 4 - cam, my - 4 - self.view_y);
        } else if p.charge > 12 && (game.frame / 2).is_multiple_of(2) {
            self.put(frame, &self.common.charge[0], mx - 4 - cam, my - 4 - self.view_y);
        }

        // Objects shown first sit on top. Pointing straight up, the cannon would hide
        // the bill, so it goes behind the body instead.
        let alive = !matches!(p.state, PlayerState::Dead | PlayerState::Hurt);
        let (blaster, (kx, ky), blaster_behind) = self.wassie.blaster(p, anim);
        let (bx, by) = part((wassie::BLASTER_SHIFT, 0), 32);
        let blaster_at = (bx + kx * p.facing, by + ky);
        if armed && !blaster_behind {
            flipped(blaster, blaster_at, frame);
        }

        // The jetpack straps on at the first hover and stays until landing.
        if alive && (strapped || p.hovering || (!p.grounded && p.hover_fuel < HOVER_FRAMES)) {
            flipped(&self.wassie.jetpack, part(wassie::JETPACK_AT, 8), frame);
            if p.hovering {
                // Flickers, then sputters as the fuel runs out.
                let flame = if p.hover_fuel > 10 {
                    Some((game.frame / 3 % 2) as usize)
                } else {
                    (game.frame / 2).is_multiple_of(2).then_some(2)
                };
                if let Some(flame) = flame {
                    flipped(&self.wassie.flames[flame], part(wassie::FLAME_AT, 8), frame);
                }
            }
        }

        // Seeing stars while knocked back, and wind streaks over a hard fall.
        if p.state == PlayerState::Hurt {
            for i in 0..3 {
                let a = (game.frame as i32 / 3 + i * 6) % 18;
                let sx = r.centre_x() - 4 - cam + (a - 9).abs() * 2 - 9;
                let sy = r.y - 12 + (a % 9) / 3;
                self.put(frame, &self.villains.star, sx, sy - self.view_y);
            }
        } else if p.state == PlayerState::Fall && p.body.vy_fx >= SPEED_LINES_FX {
            self.put(frame, &self.common.speed_lines, r.centre_x() - 4 - cam, r.y - 14 - self.view_y);
        }

        let (body, recoil) = self.wassie.sprite(p, anim, game.frame);
        flipped(body, (x + recoil * p.facing, y), frame);
        if armed && blaster_behind {
            flipped(blaster, blaster_at, frame);
        }
    }

    /// The cargo ship, CL207, and the gear in flight while the arrival plays.
    fn draw_arrival(&self, game: &Game, frame: &mut GraphicsFrame) {
        let (Some(v), Some(LevelArt::Badlands(_, ship))) = (game.arrival_view(), self.level_art.as_ref()) else {
            return;
        };
        let cam = self.view_x;
        let show_at = |frame: &mut GraphicsFrame, sprite: &SpriteVram, (x, y): (i32, i32), flip: bool| {
            Object::new(sprite.clone())
                .set_pos(vec2(x - cam, y - self.view_y))
                .set_hflip(flip)
                .set_priority(self.world_layer)
                .show(frame);
        };
        if let Some((gear, x, y)) = v.thrown {
            let sprite = match gear {
                Gear::Blaster => self.wassie.standing(game.frame)[1].0,
                Gear::Jetpack => &self.wassie.jetpack,
            };
            // Tumbling end over end.
            let spin = (game.frame / 6) % 2 == 1;
            let (w, h) = match gear {
                Gear::Blaster => (32, 32),
                Gear::Jetpack => (8, 16),
            };
            show_at(frame, sprite, (x - w / 2, y - h / 2), spin);
        }
        // Standing, or alternating the stride and the passing frame.
        let pose = if v.guide_walking { 1 + (game.frame / 8) % 2 } else { 0 };
        let (gx, feet) = v.guide;
        let left = gx + GUIDE_W / 2 - 16;
        let flip = v.guide_facing < 0;
        show_at(frame, &ship.guide_top, (left, feet - 32), flip);
        show_at(frame, &ship.guide_legs[pose as usize], (left, feet - 16), flip);
        let (sx, sy) = v.ship;
        if v.hatch >= 2 {
            let (hx, hy) = arrival::HATCH_AT;
            show_at(frame, &ship.hatch_open, (sx + hx, sy + hy), false);
        }
        show_at(frame, &ship.ship_half, (sx, sy), false);
        show_at(frame, &ship.ship_half, (sx + 32, sy), true);
        let flame = &ship.flames[(game.frame / 3 % 2) as usize];
        for (fx, fy) in arrival::FLAMES_AT {
            show_at(frame, flame, (sx + fx, sy + fy), false);
        }
    }

    fn draw_shots(&self, game: &Game, frame: &mut GraphicsFrame) {
        let cam = self.view_x;
        let (c, v) = (&self.common, &self.villains);
        for s in game.world.shots.iter().filter(|s| s.active) {
            let r = s.rect();
            if r.right() < cam - 32 || r.x > cam + WIDTH + 32 || !self.on_screen_y(r.y, r.bottom()) {
                continue;
            }
            let left = s.vx_fx < 0;
            let down = s.vy_fx > 0;
            let (sx, sy) = (s.vx_fx.signum(), s.vy_fx.signum());
            let diagonal = sx != 0 && sy != 0;
            let vertical = sx == 0 && sy != 0;
            let (cx, cy) = (r.centre_x(), r.centre_y());
            // Lasers are drawn with their trail behind the hitbox's leading edge;
            // diagonal ones sit a little back from the centre along their heading.
            let (sprite, x, y) = match s.kind {
                ShotKind::Bolt if diagonal => (&c.bolt_diagonal, cx - 8 - 3 * sx, cy - 8 - 3 * sy),
                ShotKind::Bolt if vertical => (&c.bolt_up, cx - 3, if down { r.bottom() - 16 } else { r.y }),
                ShotKind::Bolt if left => (&c.bolt, r.x, r.y - 2),
                ShotKind::Bolt => (&c.bolt, r.right() - 16, r.y - 2),
                ShotKind::Beam if diagonal => (&c.beam_diagonal, cx - 16 - 4 * sx, cy - 16 - 4 * sy),
                ShotKind::Beam if vertical => (&c.beam_up, cx - 4, cy - 16),
                ShotKind::Beam => (&c.beam, r.x, r.y),
                ShotKind::Bullet => (&v.plasma, r.x - 1, r.y - 2),
                ShotKind::Bomb => (&v.bomb, r.x, r.y),
                ShotKind::Shockwave => (&v.shockwave, r.x - 3, r.y - 4),
                ShotKind::Missile => (&v.missile, r.x - 1, r.y - 2),
                ShotKind::Card | ShotKind::Cucumber | ShotKind::MegaCucumber | ShotKind::Candle | ShotKind::Orb => {
                    self.draw_casino_shot(frame, s, game.frame);
                    continue;
                }
            };
            let flip_v = down && s.kind.from_player();
            Object::new(sprite.clone())
                .set_pos(vec2(x - cam, y - self.view_y))
                .set_hflip(left)
                .set_vflip(flip_v)
                .set_priority(self.world_layer)
                .show(frame);
        }
    }

    /// The casino's and SAM's projectiles. Cucumbers tumble end over end.
    fn draw_casino_shot(&self, frame: &mut GraphicsFrame, s: &diamond_hands_logic::shot::Shot, t: u32) {
        let art = self.level_art.as_ref();
        let r = s.rect();
        let (cx, cy) = (r.centre_x() - self.view_x, r.centre_y());
        let left = s.vx_fx < 0;
        let (sprite, x, y, hflip, vflip) = if s.kind == ShotKind::Card {
            let Some(floor) = art.and_then(LevelArt::floor) else {
                return;
            };
            (&floor.card[(t / 4 % 2) as usize], cx - 4, cy - 4, false, false)
        } else {
            let Some(fx) = art.and_then(LevelArt::arena).map(|a| &a.sam_fx) else {
                return;
            };
            match s.kind {
                ShotKind::Cucumber => {
                    // Eight steps of spin from two drawings and the flips.
                    let step = (t / 3 % 4) as usize;
                    (&fx.cucumber[step % 2], cx - 8, cy - 8, step >= 2, step >= 2)
                }
                ShotKind::MegaCucumber => (&fx.mega, cx - 16, cy - 8, !left, false),
                ShotKind::Candle => (&fx.candle, cx - 4, cy - 8, false, false),
                _ => (&fx.orb, cx - 4, cy - 4, false, false),
            }
        };
        Object::new(sprite.clone())
            .set_pos(vec2(x, y - self.view_y))
            .set_hflip(hflip)
            .set_vflip(vflip)
            .set_priority(self.world_layer)
            .show(frame);
    }

    /// Electric fences' lightning: while live, a crackling curtain up each emitter's
    /// column to the rock above, redrawn from shuffled pieces every couple of frames;
    /// while warning, sparks blinking over the emitters. Drawn over the wassie, so a
    /// shock reads as the current running through it.
    fn draw_fences(&self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        let state = diamond_hands_logic::world::fence_state(world.frames);
        if state == 0 {
            return;
        }
        let (cam, level, c) = (self.view_x, world.level, &self.common);
        let tick = game.frame / 2;
        let mix = |a: i32, b: i32| {
            let h = (a as u32).wrapping_mul(73_856_093) ^ (b as u32).wrapping_mul(19_349_663) ^ tick.wrapping_mul(83_492_791);
            h ^ (h >> 13)
        };
        // Emitters from the top of the screen to a little below it, whose lightning
        // can reach up into view.
        let rows = self.view_y.div_euclid(TILE).max(0)..level.height().min((self.view_y + HEIGHT).div_euclid(TILE) + 8);
        for tx in cam.div_euclid(TILE)..=(cam + WIDTH).div_euclid(TILE) {
            for ty in rows.clone() {
                if level.tile(tx, ty) != diamond_hands_logic::level::Tile::Spike {
                    continue;
                }
                let x = tx * TILE - cam;
                let base = ty * TILE;
                if state == 1 {
                    let h = mix(tx, ty);
                    Object::new(c.fence_spark.clone())
                        .set_pos(vec2(x, base - 7 - self.view_y))
                        .set_hflip(h & 1 != 0)
                        .set_priority(self.world_layer)
                        .show(frame);
                    continue;
                }
                // Stack pieces up from the emitter: 32px ones, then 8px to the top.
                let top = level.fence_top(tx, ty) * TILE;
                let mut y = base;
                let mut piece = 0;
                while y > top {
                    let h = mix(tx, piece);
                    let tall = y - top >= 32;
                    let (sprite, step) = if tall {
                        (&c.lightning[(h % 4) as usize], 32)
                    } else {
                        (&c.lightning_short[(h % 4) as usize], 8)
                    };
                    y -= step;
                    // Each piece's ends sit on the centre line: flipping either way
                    // still joins up, and doubles the crackles.
                    Object::new(sprite.clone())
                        .set_pos(vec2(x, y - self.view_y))
                        .set_hflip(h & 16 != 0)
                        .set_vflip(h & 32 != 0)
                        .set_priority(self.world_layer)
                        .show(frame);
                    piece += 1;
                }
            }
        }
    }

    fn draw_fx(&self, game: &Game, frame: &mut GraphicsFrame) {
        let cam = self.view_x;
        let c = &self.common;
        for f in game.world.fx.active() {
            if !self.on_screen_y(f.y - 48, f.y + 16) {
                continue;
            }
            let (sprite, half) = match f.kind {
                FxKind::Impact => (&c.impact[f.frame(3)], 4),
                FxKind::Explosion => (&c.explosion[f.frame(3)], 8),
                FxKind::Glint => (&c.glint[f.frame(2)], 4),
                FxKind::Sparks => (&c.sparks[f.frame(2)], 4),
                FxKind::Puff => (&c.puff[f.frame(2)], 4),
                FxKind::Dust => {
                    // Sits on the ground and drifts up a little as it thins.
                    let y = f.y + 3 - 8 - f.age as i32 / 5;
                    self.put(frame, &c.dust[f.frame(3)], f.x - 4 - cam, y - self.view_y);
                    continue;
                }
                FxKind::Wreck | FxKind::NinjaWreck => {
                    // Down in three moves, then lying there smoking until it blinks out.
                    let t = f.age as i32;
                    if t > 66 && t % 4 < 2 {
                        continue;
                    }
                    let step = match t {
                        0..8 => 0,
                        8..18 => 1,
                        _ => 2,
                    };
                    let (sprite, anchor) = if f.kind == FxKind::Wreck {
                        let pose = [SentryPose::Stagger, SentryPose::Kneel, SentryPose::Down][step];
                        (self.villains.sentry.sprite(pose as usize), SENTRY_ANCHOR)
                    } else {
                        let Some(floor) = self.level_art.as_ref().and_then(LevelArt::floor) else {
                            continue;
                        };
                        let pose = [NinjaPose::Stagger, NinjaPose::Kneel, NinjaPose::Down][step];
                        (floor.ninja.sprite(pose as usize), NINJA_ANCHOR)
                    };
                    if t >= 18 {
                        // A wisp of smoke curling up off the wreck.
                        let rise = (t - 18) % 24;
                        let sway = [0, 1, 1, 0, -1, -1][(rise / 4) as usize];
                        self.put(frame, &c.dust[(rise / 8) as usize], f.x - 4 + sway - cam, f.y - 14 - rise / 2 - self.view_y);
                    }
                    Object::new(sprite)
                        .set_pos(vec2(pose_left(f.x, anchor, f.flip) - cam, f.y - 32 - self.view_y))
                        .set_hflip(f.flip)
                        .set_priority(self.world_layer)
                        .show(frame);
                    continue;
                }
                FxKind::CoinPop | FxKind::Splat => {
                    let Some(casino) = self.level_art.as_ref().and_then(LevelArt::casino) else {
                        continue;
                    };
                    if f.kind == FxKind::Splat {
                        if let Some(arena) = &casino.arena {
                            self.put(frame, &arena.sam_fx.splat[f.frame(2)], f.x - 8 - cam, f.y - 8 - self.view_y);
                        }
                    } else {
                        // Flips up and falls back, spinning.
                        let t = f.age as i32;
                        let y = f.y - 4 - (t * (22 - t)) / 6;
                        let spin = &casino.coin[(t / 3 % 3) as usize];
                        self.put(frame, spin, f.x - 4 - cam, y - self.view_y);
                    }
                    continue;
                }
                FxKind::Kick => {
                    // Two mirrored halves spraying out from under the feet.
                    let sprite = &c.kick[f.frame(3)];
                    for left in [false, true] {
                        let x = if left { f.x - 16 } else { f.x };
                        Object::new(sprite.clone())
                            .set_pos(vec2(x - cam, f.y + 3 - 8 - self.view_y))
                            .set_hflip(left)
                            .set_priority(self.world_layer)
                            .show(frame);
                    }
                    continue;
                }
            };
            // Glints float up as they fade; exhaust sinks.
            let rise = match f.kind {
                FxKind::Glint => f.age as i32 / 2,
                FxKind::Puff => -(f.age as i32) / 3,
                _ => 0,
            };
            self.put(frame, sprite, f.x - half - cam, f.y - half - rise - self.view_y);
        }
    }

    fn draw_boss(&self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        let boss = &world.boss;
        if !boss.present || boss.state == BossState::Defeated {
            return;
        }
        let cam = self.view_x;
        let r = boss.rect();
        if r.right() < cam - 32 || r.x > cam + WIDTH + 32 || !self.on_screen_y(r.bottom() - 64, r.bottom()) {
            return;
        }
        let bera = match self.level_art.as_ref() {
            Some(LevelArt::Badlands(bera, _)) => bera,
            Some(LevelArt::Trenches(casino)) => {
                if let Some(arena) = &casino.arena {
                    self.draw_sam(game, frame, arena);
                }
                return;
            }
            None => return,
        };
        let blink = (boss.flash > 0 || boss.state == BossState::Dying) && (game.frame / 2).is_multiple_of(2);
        if blink {
            return;
        }
        let flip = boss.facing > 0;
        let stalled = matches!(boss.state, BossState::Stunned | BossState::Dying);
        // Telegraphing the charge: a crouch-and-shake.
        let shake = if boss.state == BossState::Telegraph { (game.frame % 2) as i32 * 2 - 1 } else { 0 };
        let x = r.centre_x() - 32 - cam + shake;
        let part = |sprite: &SpriteVram, (px, py): (i32, i32), w: i32, top: i32, frame: &mut GraphicsFrame| {
            Object::new(sprite.clone())
                .set_pos(vec2(x + badlands::flip_x(px, w, flip), top + py - self.view_y))
                .set_hflip(flip)
                .set_priority(self.world_layer)
                .show(frame);
        };

        if boss.state == BossState::Charge {
            // A galloping bounce, the head bobbing against the body.
            let top = r.bottom() - 32 - (game.frame / 3 % 2) as i32;
            let (hx, hy) = badlands::BERA_CHARGE_HEAD_AT;
            part(&bera.head, (hx, hy + (game.frame / 3 % 2) as i32), 32, top, frame);
            part(&bera.charge, (0, 0), 64, top, frame);
            return;
        }

        // Breathing: the chest swells and the head lifts, then both settle. Stalled, she
        // pants fast; walking, each heavy step jolts the upper body down.
        let pace = if stalled { 3 } else { 8 };
        let breath = BERA_BREATH[(game.frame / pace) as usize % BERA_BREATH.len()];
        let sink = match boss.state {
            BossState::Telegraph => 2,
            BossState::Walk => (r.x.unsigned_abs() / 8 % 2) as i32,
            _ => 0,
        };
        let top = r.bottom() - 64;
        let (hx, hy) = badlands::BERA_HEAD_AT;
        let head = (hx, hy - breath + sink * 3 / 2);
        let (tx, ty) = badlands::BERA_TORSO_AT;
        let torso = (tx, ty - (breath + 1) / 2 + sink);

        if boss.state == BossState::Stunned {
            for i in 0..3 {
                let a = (game.frame as i32 / 4 + i * 8) % 24;
                let sx = r.centre_x() - 4 - cam + (a - 12).abs() * 2 - 12;
                let sy = top + head.1 - 6 + (a % 12) / 3;
                self.put(frame, &self.villains.star, sx, sy - self.view_y);
            }
        }
        if stalled {
            // Smoke curling up off the back of her head.
            for i in 0..2 {
                let t = (game.frame as i32 + i * 20) % 40;
                part(&bera.smoke, (head.0 + 18 + t / 8, head.1 - t / 3), 8, top, frame);
            }
        } else {
            // The core rides with the torso.
            let (cx, cy) = badlands::BERA_CORE_AT;
            let core = (cx, cy + torso.1 - ty);
            part(&bera.core[(breath == 2) as usize], core, 8, top, frame);
        }
        part(if stalled { &bera.head_dead } else { &bera.head }, head, 32, top, frame);
        let body = if boss.state == BossState::Aim { &bera.torso_aim } else { &bera.torso };
        part(body, torso, 64, top, frame);
        part(&bera.legs, badlands::BERA_LEGS_AT, 64, top, frame);
    }

    /// SAM, one 64x64 pose from his sprite sheet: he breathes standing, strides as he
    /// walks, raises and throws his cucumbers, crouches in his aura for the Cucumber
    /// Hit, doubles over winded, winces and blinks when hit, and in the end sinks to his
    /// knees and falls flat on his face.
    fn draw_sam(&self, game: &Game, frame: &mut GraphicsFrame, arena: &ArenaArt) {
        let boss = &game.world.boss;
        let t = game.frame;
        let r = boss.rect();
        let cam = self.view_x;
        let state = boss.state;
        let pose = match state {
            BossState::Dying if boss.timer > 100 => SamPose::Shock,
            BossState::Dying if boss.timer > 40 => SamPose::Kneel,
            BossState::Dying => SamPose::Down,
            _ if boss.flash > 0 && (t / 2).is_multiple_of(2) => return,
            BossState::Walk | BossState::Intro if boss.flash > 0 => SamPose::Ouch,
            BossState::Walk if boss.body.vx_fx != 0 => SamPose::WALK[(r.x.unsigned_abs() / 6 % 4) as usize],
            // The cucumber comes out, goes up behind his head and leaves at 0.
            BossState::Throw if boss.timer > 14 => SamPose::Hold,
            BossState::Throw if boss.timer > 4 => SamPose::Raise,
            BossState::Throw => SamPose::Throw,
            BossState::Windup | BossState::Overload | BossState::Crash => SamPose::Charge,
            BossState::Swing => SamPose::Punch,
            // Market Crash: a punch for each spray of energy, fired at 16.
            BossState::Barrage if (8..=16).contains(&(boss.timer % 32)) => SamPose::Punch,
            BossState::Barrage => SamPose::Charge,
            BossState::Winded => SamPose::Dazed,
            _ if (t / 24).is_multiple_of(2) => SamPose::Idle0,
            _ => SamPose::Idle1,
        };
        let shaking = matches!(state, BossState::Windup | BossState::Overload)
            || (state == BossState::Dying && boss.timer > 100);
        let shake = if shaking { (t % 2) as i32 * 2 - 1 } else { 0 };
        // The sheet draws him facing right.
        Object::new(arena.sam.sprite(pose))
            .set_pos(vec2(r.centre_x() - 32 - cam + shake, r.bottom() - 64 - self.view_y))
            .set_hflip(boss.facing < 0)
            .set_priority(self.world_layer)
            .show(frame);

        // The Cucumber Hit's tell: lightning crackling round him.
        if matches!(state, BossState::Windup | BossState::Overload) {
            for i in 0..4 {
                let a = (t as i32 / 2 + i * 4) % 16;
                let (ax, ay) = (SIN16[(a as usize + 4) % 16] * 22 / 64, SIN16[a as usize] * 22 / 64);
                let spark = &arena.sam_fx.aura[((t / 2 + i as u32) % 2) as usize];
                self.put(frame, spark, r.centre_x() - 4 - cam + ax, r.centre_y() - 8 + ay - self.view_y);
            }
        }
        if boss.dazed() {
            for i in 0..3 {
                let a = (t as i32 / 4 + i * 8) % 24;
                let sx = r.centre_x() - 4 - cam + (a - 12).abs() * 2 - 12;
                self.put(frame, &self.villains.star, sx, r.y - 4 + (a % 12) / 3 - self.view_y);
            }
        }
    }

    /// The elevator cars, their doors sliding as the lift runs, and a light running up
    /// the dial while it powers up.
    fn draw_lift(&self, game: &Game, frame: &mut GraphicsFrame) {
        let (Some(lift), Some(casino)) = (game.world.lift, self.level_art.as_ref().and_then(LevelArt::floor)) else {
            return;
        };
        let cam = self.view_x;
        for (upstairs, (x, feet)) in [(false, lift.car), (true, lift.dest)] {
            if x + 32 < cam || x > cam + WIDTH || !self.on_screen_y(feet - LIFT_H, feet) {
                continue;
            }
            let (sx, sy) = (x - cam - 4, feet - 32);
            // Each door's outer panel slides away first, then the inner one.
            let open = lift.doors(upstairs) as usize;
            let panels: &[(usize, i32)] = match open * 3 / 16 {
                0 => &[(0, 8), (1, 4)],
                1 => &[(0, 8)],
                2 => &[(1, 12)],
                _ => &[],
            };
            for &(panel, right_edge) in panels {
                let door = &casino.lift_door[panel];
                self.put(frame, door, sx + 4 + right_edge - 8, sy + 7 - self.view_y);
                Object::new(door.clone())
                    .set_pos(vec2(sx + 28 - right_edge, sy + 7 - self.view_y))
                    .set_hflip(true)
                    .set_priority(self.world_layer)
                    .show(frame);
            }
            let powering = lift.state == LiftState::Powering && !upstairs;
            if powering && (game.frame / 4).is_multiple_of(2) {
                self.put(frame, &self.common.glint[0], sx + 12, sy - 2 - self.view_y);
            }
            self.put(frame, &casino.lift, sx, sy - self.view_y);
        }
        if let Some((px, py)) = game.world.up_prompt() {
            let bob = (game.frame / 10 % 2) as i32;
            self.put(frame, &casino.up_prompt, px - 8 - cam, py - 16 - bob - self.view_y);
        }
        // The dead car, its power cut somewhere else: a sign blinks over it.
        if game.world.lift_unpowered() && !(game.frame / 20).is_multiple_of(3) {
            let (x, feet) = lift.car;
            let text = "NO POWER";
            let left = x + LIFT_W / 2 - Font::width(text, false) / 2 - cam;
            self.font.draw(frame, text, left, feet - LIFT_H - 12 - self.view_y, false);
        }
    }

    fn draw_actors(&self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        let cam = self.view_x;
        let (c, v) = (&self.common, &self.villains);
        let casino = self.level_art.as_ref().and_then(LevelArt::casino);
        let floor = self.level_art.as_ref().and_then(LevelArt::floor);
        for a in world.actors.iter().filter(|a| a.alive) {
            let r = a.rect();
            if r.right() < cam - 16 || r.x > cam + WIDTH + 16 || !self.on_screen_y(r.bottom() - 32, r.bottom() + 8) {
                continue;
            }
            if a.flash > 0 && (game.frame / 2).is_multiple_of(2) {
                continue;
            }
            let x = r.x - cam;
            let step = ((r.x.unsigned_abs() / 6) % 2) as usize;
            let (sprite, sx, sy, flip) = match a.kind {
                ActorKind::Diamond => {
                    // Mostly still, with a glint now and then.
                    let glint = (game.frame + r.x as u32) % 48 < 6;
                    (&c.diamond[glint as usize], x - 4, r.y - 4, false)
                }
                ActorKind::Heal => (&c.heal, x - 3, r.y - 3, false),
                ActorKind::Jetpack => {
                    // Bobbing where it fell, glinting now and then.
                    let bob = (game.frame / 16 % 2) as i32;
                    if (game.frame + r.x as u32) % 40 < 6 {
                        self.put(frame, &c.glint[0], x - 2, r.y - 6 - self.view_y);
                    }
                    (&self.wassie.jetpack, x + 2, r.y - bob, false)
                }
                ActorKind::Checkpoint => {
                    let lit = (a.state == state::LIT) as usize;
                    (&c.flag[lit], x - 2, r.bottom() - 32, false)
                }
                ActorKind::Sentry => {
                    // Walks a step every 4px, raises the cannon and charges it, then
                    // rocks back from the shot with a flash at the muzzle.
                    let recoil = a.state == state::RECOIL && a.timer > SENTRY_RECOIL_FRAMES - 5;
                    let pose = match a.state {
                        _ if a.flash > 0 => SentryPose::Hit,
                        state::AIM if a.timer > 8 => SentryPose::Aim,
                        state::AIM => SentryPose::Charge,
                        state::RECOIL => SentryPose::Aim,
                        _ => SentryPose::WALK[(r.x.unsigned_abs() / 4 % 6) as usize],
                    };
                    let flip = a.facing < 0;
                    let kick = if recoil { -a.facing } else { 0 };
                    if recoil {
                        let (mx, my) = (r.centre_x() + a.facing * (SENTRY_MUZZLE.0 + 2), r.bottom() - SENTRY_MUZZLE.1);
                        Object::new(c.muzzle[(a.timer % 2) as usize].clone())
                            .set_pos(vec2(mx - 4 - cam, my - 4 - self.view_y))
                            .set_hflip(flip)
                            .set_priority(self.world_layer)
                            .show(frame);
                    }
                    (&v.sentry.sprite(pose as usize), pose_left(r.centre_x(), SENTRY_ANCHOR, flip) + kick - cam, r.bottom() - 32, flip)
                }
                ActorKind::Crawler => {
                    let lunge = (a.state == state::LUNGE) as usize;
                    (&v.crawler[lunge], x, r.bottom() - 16, a.facing > 0)
                }
                ActorKind::Drone => {
                    let spin = (game.frame / 3 % 2) as usize;
                    (&v.drone[spin], x, r.y - 3, a.facing > 0)
                }
                ActorKind::Mine => match (casino, floor) {
                    (None, _) => (&v.mine, x - 2, r.y - 2, step == 1),
                    (Some(_), Some(floor)) => (&floor.chip, x - 2, r.y - 2, step == 1),
                    (Some(_), None) => continue,
                },
                ActorKind::Coin => {
                    let Some(casino) = casino else {
                        continue;
                    };
                    let spin = [0, 1, 2, 1][((game.frame + r.x as u32) / 6 % 4) as usize];
                    (&casino.coin[spin], x, r.y, spin == 1 && (game.frame / 24).is_multiple_of(2))
                }
                ActorKind::Guard => {
                    let Some(floor) = floor else {
                        continue;
                    };
                    // Signals before a burst, points the gun through it with a flame
                    // at each shot, and holds the aim a moment after.
                    let flip = a.facing < 0;
                    let fired = match a.state {
                        state::LUNGE => a.timer > GUARD_BURST_GAP - 3 && a.count < 3,
                        state::RECOIL => a.timer > GUARD_RECOIL_FRAMES - 3,
                        _ => false,
                    };
                    let pose = match a.state {
                        _ if a.flash > 0 => NinjaPose::Hit,
                        state::AIM => NinjaPose::Signal,
                        state::LUNGE | state::RECOIL => NinjaPose::Point,
                        _ if a.body.vx_fx == 0 => NinjaPose::Walk0,
                        _ => NinjaPose::WALK[(r.x.unsigned_abs() / 4 % 6) as usize],
                    };
                    let left = pose_left(r.centre_x(), NINJA_ANCHOR, flip) - cam;
                    if fired {
                        let x = if flip { left - NINJA_FLAME_X } else { left + NINJA_FLAME_X };
                        Object::new(floor.ninja.sprite(NinjaPose::Flame as usize))
                            .set_pos(vec2(x, r.bottom() - 32 - self.view_y))
                            .set_hflip(flip)
                            .set_priority(self.world_layer)
                            .show(frame);
                    }
                    (&floor.ninja.sprite(pose as usize), left, r.bottom() - 32, flip)
                }
                ActorKind::Part | ActorKind::Terminal | ActorKind::Dealer => {
                    let Some(floor) = floor else {
                        continue;
                    };
                    let (left, top) = (r.centre_x() - 8 - cam, r.bottom() - 32);
                    let flip = a.facing > 0;
                    // Overlays go on top of the body: the lit screen, the cards, the cannon.
                    let overlay = match a.kind {
                        ActorKind::Terminal if a.state == state::LIT => Some((&floor.terminal_on, 0, 0)),
                        ActorKind::Dealer if a.state == state::AIM => Some((&floor.cards_in_hand, casino::CARDS_AT.0, casino::CARDS_AT.1)),
                        _ => None,
                    };
                    if let Some((sprite, ox, oy)) = overlay {
                        let w = if a.kind == ActorKind::Terminal { 16 } else { 8 };
                        let ox = if flip { 16 - ox - w } else { ox };
                        Object::new(sprite.clone())
                            .set_pos(vec2(left + ox, top + oy - self.view_y))
                            .set_hflip(flip && a.kind != ActorKind::Terminal)
                            .set_priority(self.world_layer)
                            .show(frame);
                    }
                    match a.kind {
                        ActorKind::Part => {
                            let bob = (game.frame / 16 % 2) as i32;
                            if (game.frame + r.x as u32) % 40 < 6 {
                                self.put(frame, &c.glint[0], x - 1, r.y - 4 - self.view_y);
                            }
                            (&floor.cell, x - 3, r.y - 2 - bob, false)
                        }
                        ActorKind::Terminal => (&floor.terminal, left, top, false),
                        _ => (&floor.dealer[step], left, top, flip),
                    }
                }
            };
            // The attack tell: a "!" over a bot winding up.
            let winding_up =
                matches!(a.kind, ActorKind::Sentry | ActorKind::Crawler | ActorKind::Dealer | ActorKind::Guard)
                    && a.state == state::AIM;
            if winding_up {
                self.put(frame, &c.alert, r.centre_x() - 4 - cam, r.y - 14 - self.view_y);
            }
            // Pickups in a secret passage sit behind its false wall until walked into.
            let hidden = trenches::behind_false_wall(world.level, r.centre_x().div_euclid(TILE), (r.bottom() - 1).div_euclid(TILE));
            let mut obj = Object::new(sprite.clone());
            obj.set_pos(vec2(sx, sy - self.view_y))
                .set_hflip(flip)
                .set_priority(if hidden { Priority::P2 } else { self.world_layer });
            if a.kind == ActorKind::Mine {
                obj.set_vflip(step == 1);
            }
            obj.show(frame);
        }
    }
}

/// Where a 32x32 sheet-drawn pose's left edge goes for a hitbox centred on
/// `centre_x`, the pose's legs being centred on column `anchor` when facing right.
fn pose_left(centre_x: i32, anchor: i32, flip: bool) -> i32 {
    centre_x - if flip { 31 - anchor } else { anchor }
}

fn show(frame: &mut GraphicsFrame, sprite: &SpriteVram, x: i32, y: i32) {
    Object::new(sprite.clone())
        .set_pos(vec2(x, y))
        .set_priority(Priority::P0)
        .show(frame);
}
