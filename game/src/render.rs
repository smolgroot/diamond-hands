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
        GraphicsFrame, Priority, WIDTH,
        object::{Object, SpriteVram},
        tiled::{DynamicTile16, RegularBackground, RegularBackgroundSize, TileEffect, TileFormat},
    },
    fixnum::{Num, num, vec2},
};
use diamond_hands_logic::{
    Difficulty, Game, Level, Progress, Scene, TILE,
    boss::BossState,
    controls::{self, BINDINGS},
    enemy::{ActorKind, state},
    fx::FxKind,
    game::{CLEAR_LOCK_FRAMES, FILES, MAP_WALK_FRAMES, file_menu, options_menu, pause_menu, title_menu},
    level::Theme,
    levels::{MAP, PlaceStatus},
    player::{Aim, HOVER_FRAMES, MAX_HP, PlayerState},
    shot::ShotKind,
    arrival::{GUIDE_W, Gear},
    talk::{self, Speaker},
    world::CHARGE_FRAMES,
};

use crate::{
    art::{
        Common, SPRITE_PALETTE, TileCache,
        arrival::{self, ArrivalArt},
        badlands::{self, SKY_PALETTES, Villains},
        menu::{self, MenuScreen},
        ui,
        wassie::{self, Wassie},
    },
    font::{self, Font},
};

/// Terrain map width in tiles. Columns are written into slot `column % 64`.
const TERRAIN_COLS: i32 = 64;
/// Stage behind the menus: the Badlands at dusk.
const MENU_STAGE: u8 = 0;
/// Terrain uses background palette 1; the backdrop uses 0.
const TERRAIN_PALETTE: u8 = 1;
/// Most terrain columns written per frame. Scrolling needs at most one; after the
/// camera jumps (a respawn) the screen refills over a few frames, so no frame runs long.
const COLUMNS_PER_FRAME: usize = 8;
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
    arrival: ArrivalArt,
    font: Font,
    sky: RegularBackground,
    city: RegularBackground,
    terrain: RegularBackground,
    dialog_box: RegularBackground,
    dialog_text: ui::DialogText,
    /// The line, characters shown and prompt arrow last painted into `dialog_text`.
    dialog_painted: Option<(u8, u16, Option<i32>)>,
    map: RegularBackground,
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
    /// Level column held by each terrain map slot.
    columns: [i32; TERRAIN_COLS as usize],
    level: Option<&'static Level>,
    /// The theme and stage whose palettes are loaded.
    theme: Theme,
    stage: u8,
    /// Fence wire state last shown, see [`diamond_hands_logic::world::fence_state`].
    fence: u8,
    /// This frame's camera, including screen shake.
    view_x: i32,
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
            badlands::ground_palette(Theme::Badlands, 0, 0),
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
        let mut map = layer(Priority::P0, RegularBackgroundSize::Background32x32);
        ui::paint_map(&mut map);

        let mut renderer = Self {
            common: Common::new(),
            wassie: Wassie::new(),
            villains: Villains::new(),
            arrival: ArrivalArt::new(),
            font: Font::new(),
            sky,
            city,
            terrain: layer(Priority::P1, RegularBackgroundSize::Background64x32),
            dialog_box,
            dialog_text,
            dialog_painted: None,
            map,
            soon: ui::soon_marker(),
            locked: ui::lock_marker(),
            clear_badge: ui::clear_badge(),
            menu: None,
            terrain_tiles: TileCache::new(),
            terrain_by_key: vec![KEY_UNSET; badlands::TERRAIN_KEYS],
            blank: DynamicTile16::new().fill_with(0),
            columns: [i32::MIN; TERRAIN_COLS as usize],
            level: None,
            theme: Theme::Badlands,
            stage: 0,
            fence: 0,
            view_x: 0,
            world_layer: Priority::P0,
        };
        // Fill the first screen now, while booting, rather than in the first frame.
        let level = diamond_hands_logic::levels::get(0);
        while !renderer.stream_terrain(level, 0) {}
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
            self.draw_map(game, frame);
            return;
        }
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

        // Dusk, then night, then the city burning as the wassie crosses the level.
        let stage = if in_menu {
            MENU_STAGE
        } else {
            ((cam + WIDTH / 2) * 3 / world.level.px_width()).clamp(0, 2) as u8
        };
        let fence = diamond_hands_logic::world::fence_state(world.frames);
        let theme = world.level.theme;
        if stage != self.stage || theme != self.theme {
            frame.set_background_palette(0, &badlands::sky_palettes(theme)[stage as usize]);
            self.stage = stage;
            self.theme = theme;
            self.fence = u8::MAX;
        }
        if fence != self.fence {
            frame.set_background_palette(TERRAIN_PALETTE, &badlands::ground_palette(theme, stage, fence));
            self.fence = fence;
        }

        let terrain_ready = self.stream_terrain(world.level, cam);
        self.terrain
            .set_scroll_pos(vec2(cam.rem_euclid(TERRAIN_COLS * TILE), 0));
        self.city.set_scroll_pos(vec2((cam / 3).rem_euclid(256), 0));
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
        if game.talking() || matches!(game.scene, Scene::Intro | Scene::LevelClear) {
            let amount = match game.scene {
                Scene::LevelClear => Num::from_raw(game.scene_frames().min(8) as u8),
                _ => num!(0.5),
            };
            let mut dim = frame.blend().darken(amount);
            dim.enable_background(city_id).enable_background(sky_id).enable_backdrop();
            if let Some(id) = terrain_id {
                dim.enable_background(id);
            }
        }

        self.draw_overlay(game, frame);
        if matches!(game.scene, Scene::Play | Scene::Intro) {
            self.draw_hud(game, frame);
        }
        self.draw_fx(game, frame);
        self.draw_player(game, frame);
        self.draw_arrival(game, frame);
        self.draw_shots(game, frame);
        self.draw_boss(game, frame);
        // The menus need every object for their text, and the bots are frozen anyway.
        if !in_menu {
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
                let key = badlands::terrain_key(level, col, ty);
                if self.terrain_by_key[key] == KEY_UNSET {
                    let id = self.terrain_tiles.intern(badlands::terrain_pixels(key));
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
            Scene::Dialog => self.draw_dialog(game, frame),
            Scene::LevelClear => self.draw_clear(game, frame, blink_on),
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
        let (cx, cy) = (r.centre_x() - self.view_x, r.centre_y());
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
        show(frame, self.portrait(line.speaker), px, py - bob);
    }

    /// Each speaker's face for the dialogue box. A new speaker needs a 32x32 portrait
    /// here and a name in `talk::Speaker`.
    fn portrait(&self, speaker: Speaker) -> &SpriteVram {
        match speaker {
            Speaker::Wassie => &self.wassie.portrait,
            Speaker::Bera => &self.villains.bera.portrait,
            Speaker::Cl207 => &self.arrival.portrait,
        }
    }

    /// The world map: every place with a marker for how far the file has got there
    /// (cleared, open, locked or still being built), the wassie on the chosen one, and
    /// what lies there. After a clear, the wassie walks the road to the place it
    /// unlocked.
    fn draw_map(&mut self, game: &Game, frame: &mut GraphicsFrame) {
        self.font.draw_centred(frame, "WORLD MAP", 6, true);
        let mut buf = [0; 10];
        let mut total = [0; 10];
        let cleared = font::number(game.cleared.count_ones(), &mut buf);
        let built = font::number(diamond_hands_logic::levels::ALL.len() as u32, &mut total);
        let tally_w = Font::width("CLEARED ", false) + Font::width(cleared, false) + Font::width("/", false);
        let x = (WIDTH - tally_w - Font::width(built, false)) / 2;
        self.font.draw(frame, "CLEARED", x, 24, false);
        self.font.draw(frame, cleared, x + Font::width("CLEARED ", false), 24, false);
        self.font.draw(frame, "/", x + tally_w - Font::width("/", false), 24, false);
        self.font.draw(frame, built, x + tally_w, 24, false);

        let index = (game.cursor as usize).min(MAP.len() - 1);
        let place = &MAP[index];
        let status = place.status(game.cleared);
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
        }
        self.font.draw_centred(frame, place.blurb, 141, false);
        let hint = if place.playable(game.cleared) { "A: PLAY   B: BACK" } else { "B: BACK" };
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
            match p.status(game.cleared) {
                PlaceStatus::Soon => show(frame, &self.soon, mx, my),
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
        self.map.show(frame);
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
        let score = font::number(game.score(), &mut buf);
        self.font
            .draw(frame, score, WIDTH - 4 - Font::width(score, false), 5, false);

        if world.boss.is_active() && !world.boss.is_beaten() {
            let y = 148;
            let name = world.level.boss_name;
            self.font.draw(frame, name, 4, y, false);
            let bar_x = (8 + Font::width(name, false) + 4).max(56);
            let full = world.boss.hp * 64 / diamond_hands_logic::boss::BOSS_HP;
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
        if blinking || r.y > 160 || arrival.is_some_and(|v| !v.wassie_out) {
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
                .set_pos(vec2(px, py))
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
                self.put(frame, &self.common.muzzle_ring[late], mx - 4 + p.facing * 2 - cam, my - 8);
            }
        } else if p.charge >= CHARGE_FRAMES {
            let pulse = (game.frame / 4 % 2) as usize;
            let glow = &self.common.charge[if pulse == 0 { 1 } else { 0 }];
            self.put(frame, glow, mx - 4 - cam, my - 4);
        } else if p.charge > 12 && (game.frame / 2).is_multiple_of(2) {
            self.put(frame, &self.common.charge[0], mx - 4 - cam, my - 4);
        }

        // Objects shown first sit on top. Pointing straight up, the cannon would hide
        // the bill, so it goes behind the body instead.
        let alive = !matches!(p.state, PlayerState::Dead | PlayerState::Hurt);
        let (blaster, (kx, ky)) = self.wassie.blaster(p);
        let blaster_behind = alive && p.aim == Aim::Up;
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
                self.put(frame, &self.villains.bera.star, sx, sy);
            }
        } else if p.state == PlayerState::Fall && p.body.vy_fx >= SPEED_LINES_FX {
            self.put(frame, &self.common.speed_lines, r.centre_x() - 4 - cam, r.y - 14);
        }

        flipped(self.wassie.sprite(p, anim, game.frame), (x, y), frame);
        if armed && blaster_behind {
            flipped(blaster, blaster_at, frame);
        }
    }

    /// The cargo ship, CL207, and the gear in flight while the arrival plays.
    fn draw_arrival(&self, game: &Game, frame: &mut GraphicsFrame) {
        let Some(v) = game.arrival_view() else {
            return;
        };
        let cam = self.view_x;
        let show_at = |frame: &mut GraphicsFrame, sprite: &SpriteVram, (x, y): (i32, i32), flip: bool| {
            Object::new(sprite.clone())
                .set_pos(vec2(x - cam, y))
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
        show_at(frame, &self.arrival.guide_top, (left, feet - 32), flip);
        show_at(frame, &self.arrival.guide_legs[pose as usize], (left, feet - 16), flip);
        let (sx, sy) = v.ship;
        if v.hatch >= 2 {
            let (hx, hy) = arrival::HATCH_AT;
            show_at(frame, &self.arrival.hatch_open, (sx + hx, sy + hy), false);
        }
        show_at(frame, &self.arrival.ship_half, (sx, sy), false);
        show_at(frame, &self.arrival.ship_half, (sx + 32, sy), true);
        let flame = &self.arrival.flames[(game.frame / 3 % 2) as usize];
        for (fx, fy) in arrival::FLAMES_AT {
            show_at(frame, flame, (sx + fx, sy + fy), false);
        }
    }

    fn draw_shots(&self, game: &Game, frame: &mut GraphicsFrame) {
        let cam = self.view_x;
        let (c, v) = (&self.common, &self.villains);
        for s in game.world.shots.iter().filter(|s| s.active) {
            let r = s.rect();
            if r.right() < cam - 32 || r.x > cam + WIDTH + 32 {
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
            };
            let flip_v = down && s.kind.from_player();
            Object::new(sprite.clone())
                .set_pos(vec2(x - cam, y))
                .set_hflip(left)
                .set_vflip(flip_v)
                .set_priority(self.world_layer)
                .show(frame);
        }
    }

    fn draw_fx(&self, game: &Game, frame: &mut GraphicsFrame) {
        let cam = self.view_x;
        let c = &self.common;
        for f in game.world.fx.active() {
            let (sprite, half) = match f.kind {
                FxKind::Impact => (&c.impact[f.frame(3)], 4),
                FxKind::Explosion => (&c.explosion[f.frame(3)], 8),
                FxKind::Glint => (&c.glint[f.frame(2)], 4),
                FxKind::Sparks => (&c.sparks[f.frame(2)], 4),
                FxKind::Puff => (&c.puff[f.frame(2)], 4),
                FxKind::Dust => {
                    // Sits on the ground and drifts up a little as it thins.
                    let y = f.y + 3 - 8 - f.age as i32 / 5;
                    self.put(frame, &c.dust[f.frame(3)], f.x - 4 - cam, y);
                    continue;
                }
                FxKind::Kick => {
                    // Two mirrored halves spraying out from under the feet.
                    let sprite = &c.kick[f.frame(3)];
                    for left in [false, true] {
                        let x = if left { f.x - 16 } else { f.x };
                        Object::new(sprite.clone())
                            .set_pos(vec2(x - cam, f.y + 3 - 8))
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
            self.put(frame, sprite, f.x - half - cam, f.y - half - rise);
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
        if r.right() < cam - 32 || r.x > cam + WIDTH + 32 {
            return;
        }
        let blink = (boss.flash > 0 || boss.state == BossState::Dying) && (game.frame / 2).is_multiple_of(2);
        if blink {
            return;
        }
        let bera = &self.villains.bera;
        let flip = boss.facing > 0;
        let stalled = matches!(boss.state, BossState::Stunned | BossState::Dying);
        // Telegraphing the charge: a crouch-and-shake.
        let shake = if boss.state == BossState::Telegraph { (game.frame % 2) as i32 * 2 - 1 } else { 0 };
        let x = r.centre_x() - 32 - cam + shake;
        let part = |sprite: &SpriteVram, (px, py): (i32, i32), w: i32, top: i32, frame: &mut GraphicsFrame| {
            Object::new(sprite.clone())
                .set_pos(vec2(x + badlands::flip_x(px, w, flip), top + py))
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
                self.put(frame, &bera.star, sx, sy);
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

    fn draw_actors(&self, game: &Game, frame: &mut GraphicsFrame) {
        let world = &game.world;
        let cam = self.view_x;
        let (c, v) = (&self.common, &self.villains);
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
                ActorKind::Diamond => {
                    // Mostly still, with a glint now and then.
                    let glint = (game.frame + r.x as u32) % 48 < 6;
                    (&c.diamond[glint as usize], x - 4, r.y - 4, false)
                }
                ActorKind::Heal => (&c.heal, x - 3, r.y - 3, false),
                ActorKind::Checkpoint => {
                    let lit = (a.state == state::LIT) as usize;
                    (&c.flag[lit], x - 2, r.bottom() - 32, false)
                }
                ActorKind::Sentry => {
                    let pose = if a.state == state::AIM { 2 } else { step };
                    (&v.sentry[pose], r.centre_x() - 8 - cam, r.bottom() - 32, a.facing > 0)
                }
                ActorKind::Crawler => {
                    let lunge = (a.state == state::LUNGE) as usize;
                    (&v.crawler[lunge], x, r.bottom() - 16, a.facing > 0)
                }
                ActorKind::Drone => {
                    let spin = (game.frame / 3 % 2) as usize;
                    (&v.drone[spin], x, r.y - 3, a.facing > 0)
                }
                ActorKind::Mine => (&v.mine, x - 2, r.y - 2, step == 1),
            };
            // The attack tell: a "!" over a bot winding up.
            let winding_up = matches!(a.kind, ActorKind::Sentry | ActorKind::Crawler) && a.state == state::AIM;
            if winding_up {
                self.put(frame, &c.alert, r.centre_x() - 4 - cam, r.y - 14);
            }
            let mut obj = Object::new(sprite.clone());
            obj.set_pos(vec2(sx, sy))
                .set_hflip(flip)
                .set_priority(self.world_layer);
            if a.kind == ActorKind::Mine {
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
