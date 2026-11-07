//! The Solana Trenches' sprites: SAM, his cucumbers and his Market Crash, the penguin
//! cashier, gold coins, CL207's plasma cells, the elevator and its terminal, and the
//! casino's bots. They are loaded only while the Trenches are on screen (see
//! `render::LevelArt`): sprite VRAM can't hold them and BERA 3000 at once. Even alone
//! they don't all fit, so they come in two halves swapped as the camera goes: the
//! floors' ([`FloorArt`]) and the arena's ([`ArenaArt`]). Only the coins stay loaded.

use agb::display::{
    Palette16,
    object::{Size, SpriteVram},
};

use super::{Canvas, drawing, ix, rgb};

/// SAM's colours, after `.claude/skills/diamond-hands-gba/sprite-sheet-SBF.png`: a
/// white jacket over a navy shirt and shorts, pink cyber glasses and implants, purple
/// curls, and his cucumbers' greens.
pub static SAM_PALETTE: Palette16 = Palette16::new([
    rgb(0, 0, 0),
    rgb(3, 1, 6),
    rgb(30, 30, 31),
    rgb(20, 19, 27),
    rgb(5, 14, 4),
    rgb(31, 26, 21),
    rgb(28, 19, 14),
    rgb(19, 11, 10),
    rgb(22, 12, 29),
    rgb(13, 6, 21),
    rgb(7, 3, 12),
    rgb(5, 5, 13),
    rgb(6, 27, 30),
    rgb(31, 7, 22),
    rgb(31, 21, 30),
    rgb(13, 26, 6),
]);

/// [`SAM_PALETTE`] indices.
mod sx {
    pub const OUTLINE: u8 = 1;
    pub const WHITE: u8 = 2;
    pub const JACKET_SHADE: u8 = 3;
    pub const CUKE_DARK: u8 = 4;
    pub const SKIN_LIGHT: u8 = 5;
    pub const SKIN: u8 = 6;
    pub const SKIN_SHADE: u8 = 7;
    pub const HAIR_LIGHT: u8 = 8;
    pub const HAIR: u8 = 9;
    pub const HAIR_DARK: u8 = 10;
    pub const SHIRT: u8 = 11;
    pub const CYAN: u8 = 12;
    pub const PINK: u8 = 13;
    pub const PINK_LIGHT: u8 = 14;
    pub const CUKE: u8 = 15;
}

/// Where each of SAM's parts sits in his 64x64 frame, facing left, feet on row 63.
pub const SAM_HEAD_AT: (i32, i32) = (14, 2);
pub const SAM_TORSO_AT: (i32, i32) = (16, 26);
pub const SAM_LEGS_AT: (i32, i32) = (16, 48);
/// The near arm: hanging with the cucumber, raised behind for a throw, flung out after
/// it, and levelling the giant cucumber for the Cucumber Hit.
pub const SAM_ARM_HOLD_AT: (i32, i32) = (6, 26);
pub const SAM_ARM_UP_AT: (i32, i32) = (34, 0);
pub const SAM_ARM_OUT_AT: (i32, i32) = (0, 26);
pub const SAM_ARM_SWING_AT: (i32, i32) = (0, 30);

pub struct Sam {
    pub head: SpriteVram,
    /// Overloaded: the glasses and implant glow, sparks crackle off them.
    pub head_glow: SpriteVram,
    /// Beaten: glasses knocked askew, eyes crossed out, mouth open.
    pub head_dead: SpriteVram,
    pub torso: SpriteVram,
    /// Standing, then the two strides.
    pub legs: [SpriteVram; 3],
    pub arm_hold: SpriteVram,
    pub arm_up: SpriteVram,
    pub arm_out: SpriteVram,
    pub arm_swing: SpriteVram,
    /// Face down on the floor at the end, 64 wide in two 32x16 halves, floor on row 15.
    pub down: [SpriteVram; 2],
}

impl Sam {
    pub fn new() -> Self {
        Self {
            head: sam_head(Face::Smug),
            head_glow: sam_head(Face::Glow),
            head_dead: sam_head(Face::Dead),
            torso: sam_torso(),
            legs: [0, 1, 2].map(sam_legs),
            arm_hold: sam_arm_hold(),
            arm_up: sam_arm_up(),
            arm_out: sam_arm_out(),
            arm_swing: sam_arm_swing(),
            down: [0, 32].map(sam_down),
        }
    }
}

fn sam_canvas(size: Size, (x, y): (i32, i32)) -> Canvas {
    Canvas::at(size, x, y)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Face {
    Smug,
    Glow,
    Dead,
}

/// The wild purple curls: a ring of round curls, each lit on its upper left.
fn curls(c: &mut Canvas, curls: &[(i32, i32, i32)]) {
    for &(cx, cy, r) in curls {
        c.ellipse(cx, cy, r + 1, r + 1, sx::OUTLINE);
    }
    for &(cx, cy, r) in curls {
        c.ellipse(cx, cy, r, r, sx::HAIR);
    }
    for &(cx, cy, r) in curls {
        // A dark core and a lit arc make each one read as a ringlet.
        c.px(cx + 1, cy + 1, sx::HAIR_DARK);
        c.px(cx, cy + 1, sx::HAIR_DARK);
        c.px(cx - r / 2, cy - r / 2, sx::HAIR_LIGHT);
        c.px(cx - r / 2 + 1, cy - r / 2, sx::HAIR_LIGHT);
    }
}

/// SAM's head, facing left: a big mop of curls, a round clean-shaven face with a soft
/// double chin, pink rectangular cyber glasses, a mechanical implant over the far ear
/// with a pink lamp and an antenna, and a smug toothy grin.
fn draw_head(c: &mut Canvas, face: Face) {
    // Curls behind the head, framing it.
    curls(
        c,
        &[
            (20, 9, 4),
            (25, 5, 4),
            (31, 3, 4),
            (37, 4, 4),
            (42, 8, 4),
            (44, 14, 3),
            (43, 20, 3),
            (18, 15, 3),
            (40, 25, 3),
        ],
    );
    // The face and its double chin.
    c.ellipse(28, 19, 10, 10, sx::OUTLINE);
    c.ellipse(28, 19, 9, 9, sx::SKIN);
    c.ellipse(26, 17, 7, 7, sx::SKIN_LIGHT);
    c.ellipse(29, 26, 6, 3, sx::OUTLINE);
    c.ellipse(29, 26, 5, 2, sx::SKIN);
    c.rect(25, 27, 8, 1, sx::SKIN_SHADE);
    // The far cheek in shadow, and a few freckles.
    c.rect(35, 17, 2, 6, sx::SKIN_SHADE);
    for (x, y) in [(22, 21), (24, 22), (33, 20)] {
        c.px(x, y, sx::SKIN_SHADE);
    }
    // Curls falling over the forehead.
    curls(c, &[(22, 10, 3), (27, 9, 3), (32, 9, 3), (36, 11, 3), (19, 12, 2)]);
    // Glasses: two pink frames joined at the bridge.
    let (frame, lens) = match face {
        Face::Glow => (sx::PINK_LIGHT, sx::PINK),
        _ => (sx::PINK, sx::OUTLINE),
    };
    let tilt = if face == Face::Dead { 1 } else { 0 };
    for (x0, w) in [(18, 7), (27, 7)] {
        let y0 = 14 + if x0 > 20 { tilt } else { 0 };
        c.rect(x0, y0, w, 5, frame);
        c.rect(x0 + 1, y0 + 1, w - 2, 3, sx::WHITE);
        match face {
            // Half-lidded and pleased with himself.
            Face::Smug | Face::Glow => {
                c.rect(x0 + 1, y0 + 1, w - 2, 1, lens);
                c.rect(x0 + 2, y0 + 2, 2, 2, sx::OUTLINE);
            }
            Face::Dead => {
                c.line(x0 + 1, y0 + 1, x0 + 3, y0 + 3, sx::OUTLINE);
                c.line(x0 + 3, y0 + 1, x0 + 1, y0 + 3, sx::OUTLINE);
            }
        }
    }
    c.rect(25, 15, 2, 1, frame);
    c.rect(34, 15, 3, 1, frame);
    // The implant over the far ear: a steel-dark disc with a pink lamp and antenna.
    c.ellipse(39, 18, 4, 4, sx::OUTLINE);
    c.ellipse(39, 18, 3, 3, sx::SHIRT);
    c.ellipse(39, 18, 1, 1, if face == Face::Dead { sx::SHIRT } else { sx::PINK });
    c.px(38, 17, sx::PINK_LIGHT);
    c.line(42, 15, 46, 12, sx::OUTLINE);
    c.px(46, 11, if face == Face::Glow { sx::PINK_LIGHT } else { sx::PINK });
    c.line(36, 21, 34, 24, sx::CYAN);
    // The nose, a nub pointing left.
    c.rect(18, 19, 2, 2, sx::SKIN);
    c.px(17, 20, sx::OUTLINE);
    c.px(19, 21, sx::SKIN_SHADE);
    // The mouth.
    match face {
        Face::Dead => {
            c.ellipse(24, 24, 2, 2, sx::OUTLINE);
            c.px(24, 24, sx::SHIRT);
        }
        _ => {
            // A lopsided grin up to the far cheek, teeth showing.
            c.line(20, 23, 30, 24, sx::OUTLINE);
            c.line(30, 24, 32, 22, sx::OUTLINE);
            c.rect(21, 24, 8, 1, sx::WHITE);
            c.line(21, 25, 28, 25, sx::OUTLINE);
            c.px(24, 24, sx::OUTLINE);
        }
    }
    if face == Face::Glow {
        // Sparks off the overloaded implant.
        for (x, y) in [(45, 18), (47, 16), (44, 22), (16, 13)] {
            c.px(x, y, sx::CYAN);
        }
        c.px(46, 20, sx::PINK_LIGHT);
    }
}

fn sam_head(face: Face) -> SpriteVram {
    let mut c = sam_canvas(Size::S32x32, SAM_HEAD_AT);
    draw_head(&mut c, face);
    c.vram_with(&SAM_PALETTE)
}

/// Tiny three-by-five capitals for the jacket's lettering.
fn mini_text(c: &mut Canvas, text: &[u8], x: i32, y: i32, colour: u8) {
    for (i, &ch) in text.iter().enumerate() {
        let rows: [u8; 5] = match ch {
            b'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
            b'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
            b'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
            _ => [0; 5],
        };
        for (dy, bits) in rows.iter().enumerate() {
            for dx in 0..3 {
                if bits & (0b100 >> dx) != 0 {
                    c.px(x + i as i32 * 4 + dx, y + dy as i32, colour);
                }
            }
        }
    }
}

/// The oversized white exchange-operator jacket hanging open over a navy shirt with
/// the exchange's bars on it, riding up over his belly; the far sleeve and fist.
fn sam_torso() -> SpriteVram {
    let mut c = sam_canvas(Size::S32x32, SAM_TORSO_AT);
    // The far sleeve, behind.
    c.pod(40, 30, 7, 15, sx::WHITE, sx::JACKET_SHADE, sx::JACKET_SHADE);
    c.ellipse(43, 46, 3, 2, sx::OUTLINE);
    c.ellipse(43, 46, 2, 1, sx::SKIN);
    // The shirt and the belly under it.
    c.ellipse(30, 40, 11, 8, sx::OUTLINE);
    c.ellipse(30, 40, 10, 7, sx::SHIRT);
    c.ellipse(29, 45, 8, 3, sx::OUTLINE);
    c.ellipse(29, 45, 7, 2, sx::SKIN);
    c.rect(24, 44, 10, 1, sx::SKIN_LIGHT);
    c.px(29, 46, sx::SKIN_SHADE);
    // The exchange's bars on the shirt.
    c.rect(25, 34, 6, 1, sx::CYAN);
    c.rect(26, 36, 6, 1, sx::CYAN);
    c.rect(27, 38, 4, 1, sx::CYAN);
    // Jacket panels: the near one over his front, the far one wide over the back.
    c.pod(16, 27, 9, 19, sx::WHITE, sx::WHITE, sx::JACKET_SHADE);
    c.pod(32, 27, 12, 19, sx::WHITE, sx::WHITE, sx::JACKET_SHADE);
    c.rect(33, 42, 10, 3, sx::JACKET_SHADE);
    c.rect(18, 42, 6, 3, sx::JACKET_SHADE);
    // Lapels and collar.
    c.line(24, 27, 27, 33, sx::JACKET_SHADE);
    c.line(33, 27, 31, 33, sx::JACKET_SHADE);
    c.rect(25, 26, 8, 2, sx::OUTLINE);
    c.rect(26, 26, 6, 1, sx::SKIN_SHADE);
    // A cyan stripe down the sleeve seam and the lettering on the back panel.
    c.rect(43, 30, 1, 12, sx::CYAN);
    mini_text(&mut c, b"FTX", 34, 33, sx::OUTLINE);
    c.vram_with(&SAM_PALETTE)
}

/// Navy shorts with a cyan side stripe, bare calves, chunky white sneakers with pink
/// soles. `pose` 0 stands; 1 and 2 stride.
fn sam_legs(pose: i32) -> SpriteVram {
    let mut c = sam_canvas(Size::S32x16, SAM_LEGS_AT);
    let (front, back) = match pose {
        1 => (-3, 3),
        2 => (3, -3),
        _ => (0, 0),
    };
    // Shorts.
    c.pod(19, 47, 24, 8, sx::SHIRT, sx::SHIRT, sx::OUTLINE);
    c.rect(41, 48, 1, 6, sx::CYAN);
    c.rect(30, 50, 1, 5, sx::OUTLINE);
    for (x, dx) in [(22, front), (34, back)] {
        let x = x + dx;
        // Calf.
        c.rect(x, 55, 6, 4, sx::OUTLINE);
        c.rect(x + 1, 55, 4, 4, sx::SKIN);
        c.px(x + 4, 56, sx::SKIN_SHADE);
        // Sneaker, toe to the left.
        c.pod(x - 4, 58, 12, 6, sx::WHITE, sx::WHITE, sx::JACKET_SHADE);
        c.rect(x - 3, 62, 10, 1, sx::PINK);
        c.rect(x, 59, 3, 1, sx::CYAN);
    }
    c.vram_with(&SAM_PALETTE)
}

/// A cucumber from (x0, y0) to (x1, y1), `r` pixels thick, with bumps and a lit side.
fn cucumber(c: &mut Canvas, (x0, y0): (i32, i32), (x1, y1): (i32, i32), r: i32) {
    let n = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for pass in 0..3 {
        for i in 0..=n {
            let x = x0 + (x1 - x0) * i / n;
            let y = y0 + (y1 - y0) * i / n;
            // Rounded ends.
            let taper = if i < r || i > n - r { 1 } else { 0 };
            let rr = r - taper;
            match pass {
                0 => c.ellipse(x, y, rr + 1, rr + 1, sx::OUTLINE),
                1 => c.ellipse(x, y, rr, rr, sx::CUKE_DARK),
                _ => {
                    if rr > 1 {
                        c.ellipse(x - 1, y - 1, rr - 1, rr - 1, sx::CUKE);
                    }
                }
            }
        }
    }
    // Bumps.
    for i in (2..n - 1).step_by(3) {
        let x = x0 + (x1 - x0) * i / n;
        let y = y0 + (y1 - y0) * i / n;
        c.px(x, y + r / 2, sx::CUKE_DARK);
        c.px(x + 1, y - r / 2, sx::SKIN_LIGHT);
    }
}

/// A white sleeve from the shoulder at `from` to the wrist at `to`, then a fist.
fn sleeve(c: &mut Canvas, from: (i32, i32), to: (i32, i32)) {
    for (colour, r) in [(sx::OUTLINE, 4), (sx::JACKET_SHADE, 3), (sx::WHITE, 2)] {
        let n = (to.0 - from.0).abs().max((to.1 - from.1).abs()).max(1);
        for i in 0..=n {
            let x = from.0 + (to.0 - from.0) * i / n;
            let y = from.1 + (to.1 - from.1) * i / n;
            c.ellipse(x, y, r, r, colour);
        }
    }
}

fn fist(c: &mut Canvas, x: i32, y: i32) {
    c.ellipse(x, y, 3, 3, sx::OUTLINE);
    c.ellipse(x, y, 2, 2, sx::SKIN);
    c.px(x - 1, y - 1, sx::SKIN_LIGHT);
}

/// The near arm at his side, a cucumber in his fist pointing down.
fn sam_arm_hold() -> SpriteVram {
    let mut c = sam_canvas(Size::S16x32, SAM_ARM_HOLD_AT);
    sleeve(&mut c, (17, 30), (14, 41));
    cucumber(&mut c, (11, 44), (9, 55), 2);
    fist(&mut c, 13, 44);
    c.vram_with(&SAM_PALETTE)
}

/// The arm cocked back over his shoulder, the cucumber raised high behind his head.
fn sam_arm_up() -> SpriteVram {
    let mut c = sam_canvas(Size::S32x32, SAM_ARM_UP_AT);
    sleeve(&mut c, (42, 30), (49, 17));
    cucumber(&mut c, (50, 14), (60, 4), 2);
    fist(&mut c, 50, 15);
    c.vram_with(&SAM_PALETTE)
}

/// Flung out ahead after a throw, the hand open.
fn sam_arm_out() -> SpriteVram {
    let mut c = sam_canvas(Size::S32x16, SAM_ARM_OUT_AT);
    sleeve(&mut c, (19, 30), (7, 32));
    fist(&mut c, 3, 32);
    c.px(0, 31, sx::SKIN);
    c.px(0, 33, sx::SKIN);
    c.vram_with(&SAM_PALETTE)
}

/// Both hands on the giant cucumber, levelled ahead like a bat.
fn sam_arm_swing() -> SpriteVram {
    let mut c = sam_canvas(Size::S32x16, SAM_ARM_SWING_AT);
    sleeve(&mut c, (19, 34), (12, 39));
    cucumber(&mut c, (1, 39), (13, 37), 3);
    fist(&mut c, 11, 39);
    c.vram_with(&SAM_PALETTE)
}

/// Face down on the floor, curls spread out, sneakers up.
fn sam_down(left: i32) -> SpriteVram {
    let mut c = Canvas::at(Size::S32x16, left, 16);
    // Jacket and shorts along the floor.
    c.pod(18, 18, 26, 13, sx::WHITE, sx::WHITE, sx::JACKET_SHADE);
    mini_text(&mut c, b"FTX", 28, 21, sx::OUTLINE);
    c.pod(42, 20, 12, 10, sx::SHIRT, sx::SHIRT, sx::OUTLINE);
    for y in [19, 24] {
        c.pod(52, y, 11, 6, sx::WHITE, sx::WHITE, sx::JACKET_SHADE);
        c.rect(62, y + 1, 1, 4, sx::PINK);
    }
    // The head, face down, the curls fanned on the floor.
    curls(&mut c, &[(6, 22, 4), (11, 17, 4), (17, 16, 3), (4, 28, 3), (13, 27, 4), (19, 23, 3)]);
    c.ellipse(12, 24, 6, 5, sx::OUTLINE);
    c.ellipse(12, 24, 5, 4, sx::HAIR);
    c.rect(14, 28, 6, 2, sx::PINK);
    c.rect(0, 31, 64, 1, sx::OUTLINE);
    c.vram_with(&SAM_PALETTE)
}

/// SAM's projectiles and the effects of his attacks.
pub struct SamFx {
    /// A thrown cucumber, flat then turned an eighth (flip for the rest of the spin).
    pub cucumber: [SpriteVram; 2],
    /// The Cucumber Hit's giant cucumber, 32x16, flying left with speed lines behind.
    pub mega: SpriteVram,
    /// Green burst where a cucumber lands.
    pub splat: [SpriteVram; 2],
    /// Cyan lightning crackling round him as the Cucumber Hit charges.
    pub aura: [SpriteVram; 2],
    /// A red candle falling out of the chart, and a ball of magenta energy.
    pub candle: SpriteVram,
    pub orb: SpriteVram,
}

impl SamFx {
    fn new() -> Self {
        Self {
            cucumber: [
                {
                    let mut c = Canvas::new(Size::S16x16);
                    cucumber(&mut c, (3, 8), (12, 8), 2);
                    c.vram_with(&SAM_PALETTE)
                },
                {
                    let mut c = Canvas::new(Size::S16x16);
                    cucumber(&mut c, (4, 11), (11, 4), 2);
                    c.vram_with(&SAM_PALETTE)
                },
            ],
            mega: {
                let mut c = Canvas::new(Size::S32x16);
                for (y, x0) in [(4, 22), (8, 25), (12, 23)] {
                    c.rect(x0, y, 32 - x0, 1, sx::CYAN);
                }
                cucumber(&mut c, (4, 8), (22, 8), 5);
                c.vram_with(&SAM_PALETTE)
            },
            splat: [0, 1].map(|f| {
                let mut c = Canvas::new(Size::S16x16);
                let r = 3 + f * 3;
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1)] {
                    let (x, y) = (8 + dx * r, 8 + dy * r);
                    c.ellipse(x, y, 2 - f, 2 - f, if f == 0 { sx::CUKE } else { sx::CUKE_DARK });
                }
                if f == 0 {
                    c.ellipse(8, 8, 3, 3, sx::CUKE);
                    c.ellipse(8, 8, 1, 1, sx::SKIN_LIGHT);
                }
                c.vram_with(&SAM_PALETTE)
            }),
            aura: [0, 1].map(|f| {
                let mut c = Canvas::new(Size::S8x16);
                let mut x = 3 + f;
                for y in 0..16 {
                    c.px(x, y, sx::CYAN);
                    if y % 3 == 2 {
                        x = if x > 3 { x - 2 } else { x + 2 };
                        c.px(x, y, sx::WHITE);
                    }
                }
                c.vram_with(&SAM_PALETTE)
            }),
            candle: drawing(
                Size::S8x16,
                1,
                0,
                &[
                    "..R...", "..R...", "..R...", ".kkk..", "krrrk.", "kroRk.", "kroRk.", "krrRk.", "kroRk.",
                    "krrRk.", "krrRk.", ".kkk..", "..R...", "..R...", "..R...", "......",
                ],
            ),
            orb: drawing(Size::S8x8, 1, 1, &[".ccc.", "cCwCc", "cwwwc", "cCwCc", ".ccc."]),
        }
    }
}

/// What the arena needs: SAM, his attacks, and the penguin cashier.
pub struct ArenaArt {
    pub sam: Sam,
    pub sam_fx: SamFx,
    /// The penguin cashier, also his portrait. He waddles by bobbing.
    pub penguin: SpriteVram,
}

impl ArenaArt {
    pub fn new() -> Self {
        Self { sam: Sam::new(), sam_fx: SamFx::new(), penguin: penguin() }
    }
}

/// What the street, the casino floor and the suites need.
pub struct FloorArt {
    /// A plasma cell.
    pub cell: SpriteVram,
    /// The elevator terminal, and its screen lit once it is on.
    pub terminal: SpriteVram,
    pub terminal_on: SpriteVram,
    /// The elevator car's frame, and a door's two panels (eight pixels and four), which
    /// slide away one after the other; mirrored for the right door.
    pub lift: SpriteVram,
    pub lift_door: [SpriteVram; 2],
    /// A bubble with an up arrow: press Up here.
    pub up_prompt: SpriteVram,
    /// CL207's call coming in.
    pub phone: SpriteVram,
    /// Croupier bot walking; a fan of cards in hand laid over it while dealing.
    pub dealer: [SpriteVram; 2],
    pub cards_in_hand: SpriteVram,
    /// Security bot walking; its levelled cannon laid over it while firing.
    pub guard: [SpriteVram; 2],
    pub guard_aim: SpriteVram,
    /// A playing card spinning through the air.
    pub card: [SpriteVram; 2],
    /// A casino chip rolling: the Trenches' rolling mine.
    pub chip: SpriteVram,
}

/// The Trenches' sprites: the coins always, and one of the two halves.
pub struct CasinoArt {
    /// A gold coin spinning: face, turning, edge on.
    pub coin: [SpriteVram; 3],
    pub floor: Option<FloorArt>,
    pub arena: Option<ArenaArt>,
}

impl CasinoArt {
    pub fn new() -> Self {
        Self {
            coin: [&COIN0[..], &COIN1, &COIN2].map(|rows| drawing(Size::S8x8, 0, 0, rows)),
            floor: Some(FloorArt::new()),
            arena: None,
        }
    }
}

impl FloorArt {
    pub fn new() -> Self {
        Self {
            cell: plasma_cell(),
            terminal: terminal(),
            terminal_on: terminal_screen(),
            lift: lift_frame(),
            lift_door: [8, 4].map(lift_door),
            up_prompt: drawing(
                Size::S16x16,
                1,
                0,
                &[
                    ".kkkkkkkkkkk..",
                    "kwwwwwwwwwwwk.",
                    "kwwwwwkwwwwwk.",
                    "kwwwwkykwwwwk.",
                    "kwwwkyyykwwwk.",
                    "kwwkyyyyykwwk.",
                    "kwkkkyyykkkwk.",
                    "kwwwkyyykwwwk.",
                    "kwwwkyyykwwwk.",
                    "kwwwkkkkkwwwk.",
                    "kwwwwwwwwwwwk.",
                    ".kkkkkkwkkkk..",
                    "......kwk.....",
                    ".......k......",
                ],
            ),
            phone: {
                let mut c = Canvas::new(Size::S16x16);
                c.art(
                    3,
                    3,
                    &[
                        "kk......",
                        "kyk.....",
                        "kyyk....",
                        ".kyk....",
                        ".kyk....",
                        "..kyk...",
                        "..kyyk.k",
                        "...kyykyk",
                        "....kyyyk",
                        ".....kkk.",
                    ],
                );
                for (x, y) in [(11, 2), (12, 3), (13, 5), (1, 11), (2, 13)] {
                    c.px(x, y, ix::CYAN);
                }
                c.vram()
            },
            dealer: [dealer(&LEGS0), dealer(&LEGS1)],
            cards_in_hand: drawing(Size::S8x8, 0, 0, &DEALER_CARDS),
            guard: [guard(&GUARD_LEGS0), guard(&GUARD_LEGS1)],
            guard_aim: drawing(Size::S8x8, 0, 2, &GUARD_AIM),
            card: [
                drawing(Size::S8x8, 1, 0, &[".kkkk.", "kwwwwk", "kwrwwk", "kwrrwk", "kwwrwk", "kwwwwk", ".kkkk."]),
                drawing(Size::S8x8, 0, 2, &[".kkkkkk.", "kwwrrwwk", "kwwwrwwk", ".kkkkkk."]),
            ],
            chip: drawing(
                Size::S16x16,
                2,
                2,
                &[
                    "...kkkkkk...",
                    "..kcwccwck..",
                    ".kwccccccwk.",
                    "kcckkkkkkcck",
                    "kwckwwwwkcwk",
                    "kcckwyywkcck",
                    "kcckwyywkcck",
                    "kwckwwwwkcwk",
                    "kcckkkkkkcck",
                    ".kwccccccwk.",
                    "..kcwccwck..",
                    "...kkkkkk...",
                ],
            ),
        }
    }
}

/// Where the dealer's cards and the guard's cannon sit on their 16x32 sprites.
pub const CARDS_AT: (i32, i32) = (0, 20);
pub const GUARD_AIM_AT: (i32, i32) = (0, 19);

const COIN0: [&str; 8] = [
    "..kkkk..",
    ".kyyyok.",
    "kyywyyok",
    "kyyoyyok",
    "kyyyoyok",
    "kyywyyok",
    ".kooook.",
    "..kkkk..",
];
const COIN1: [&str; 8] = [
    "...kk...",
    "..kyok..",
    "..kwyok.",
    "..kyyok.",
    "..kyyok.",
    "..kyyok.",
    "..kooo..",
    "...kk...",
];
const COIN2: [&str; 8] = [
    "...kk...",
    "...ko...",
    "...ky...",
    "...ky...",
    "...ky...",
    "...ky...",
    "...ko...",
    "...kk...",
];

/// The penguin cashier, 32x32, after the round, cosy look of Pudgy Penguins: a chubby
/// pale-blue body, a big white face and belly, a little orange beak and feet, rosy
/// cheeks, and a cashier's gold-trimmed visor and bow tie.
fn penguin() -> SpriteVram {
    let mut c = Canvas::new(Size::S32x32);
    let lean = 0;
    for x in [10, 18] {
        c.block(x, 29, 6, 3, ix::ORANGE, ix::OUTLINE);
    }
    // Body: one big rounded blob, darker blue down the back.
    c.ellipse(16 + lean, 18, 12, 12, ix::OUTLINE);
    c.ellipse(16 + lean, 18, 11, 11, ix::BLUE);
    c.ellipse(15 + lean, 17, 10, 10, ix::CYAN);
    // White face and belly.
    c.ellipse(15 + lean, 21, 7, 8, ix::WHITE);
    c.ellipse(15 + lean, 14, 7, 5, ix::WHITE);
    // Flippers.
    c.ellipse(4 + lean, 20, 2, 5, ix::OUTLINE);
    c.ellipse(4 + lean, 20, 1, 4, ix::BLUE);
    c.ellipse(27 + lean, 20, 2, 5, ix::OUTLINE);
    c.ellipse(27 + lean, 20, 1, 4, ix::BLUE);
    // Eyes, cheeks and beak.
    let ex = 15 + lean;
    for x in [ex - 4, ex + 3] {
        c.rect(x, 12, 2, 3, ix::OUTLINE);
        c.px(x, 12, ix::WHITE);
    }
    c.rect(ex - 7, 15, 2, 1, ix::LASER_LIGHT);
    c.rect(ex + 6, 15, 2, 1, ix::LASER_LIGHT);
    c.rect(ex - 1, 15, 3, 2, ix::ORANGE);
    c.px(ex, 17, ix::ORANGE);
    c.rect(ex - 1, 15, 3, 1, ix::GOLD);
    // The cashier's visor and bow tie.
    c.rect(ex - 8, 6, 17, 2, ix::OUTLINE);
    c.rect(ex - 7, 6, 15, 1, ix::PURPLE);
    c.rect(ex - 9, 8, 12, 1, ix::GOLD);
    c.rect(ex - 9, 9, 10, 1, ix::OUTLINE);
    for (x, d) in [(ex - 3, 0), (ex + 2, 1)] {
        c.rect(x, 20, 2, 3, ix::RED);
        c.px(x + d, 21, ix::RED_DARK);
    }
    c.rect(ex - 1, 20, 2, 3, ix::RED_DARK);
    c.vram()
}

/// A plasma cell: steel end caps on a glass tube of cyan plasma with a white-hot core.
fn plasma_cell() -> SpriteVram {
    let flare = false;
    let mut c = Canvas::new(Size::S16x16);
    c.block(4, 1, 8, 3, ix::STEEL_LIGHT, ix::OUTLINE);
    c.block(4, 12, 8, 3, ix::STEEL, ix::OUTLINE);
    c.block(5, 3, 6, 10, ix::BLUE, ix::OUTLINE);
    c.rect(6, 4, 4, 8, ix::CYAN);
    c.rect(7, 5, 2, if flare { 6 } else { 4 }, ix::WHITE);
    c.px(6, 4, ix::WHITE);
    if flare {
        for (x, y) in [(2, 5), (13, 9), (1, 10), (14, 4)] {
            c.px(x, y, ix::CYAN);
        }
    }
    c.vram()
}

/// The elevator's access terminal, 16x32 with its foot on the bottom row: a steel
/// kiosk with a screen, a status lamp and a keypad.
fn terminal() -> SpriteVram {
    let on = false;
    let mut c = Canvas::new(Size::S16x32);
    c.pod(2, 10, 12, 22, ix::STEEL_LIGHT, ix::STEEL, ix::STEEL_DARK);
    c.block(3, 12, 10, 7, if on { ix::CYAN } else { ix::PURPLE_DARK }, ix::OUTLINE);
    if on {
        // An up arrow on the screen.
        c.px(8, 13, ix::WHITE);
        c.rect(7, 14, 3, 1, ix::WHITE);
        c.rect(8, 15, 1, 3, ix::WHITE);
    } else {
        c.line(6, 14, 9, 17, ix::RED);
        c.line(9, 14, 6, 17, ix::RED);
    }
    for y in [21, 24] {
        for x in [4, 7, 10] {
            c.rect(x, y, 2, 2, ix::STEEL_DARK);
        }
    }
    // The lamp on a stalk.
    c.rect(7, 5, 2, 5, ix::OUTLINE);
    c.block(6, 2, 4, 4, if on { ix::GOLD } else { ix::RED }, ix::OUTLINE);
    c.rect(2, 31, 12, 1, ix::OUTLINE);
    c.vram()
}

/// The terminal's screen and lamp once it is on, laid over [`terminal`]: cyan with an
/// up arrow, and a gold lamp. 16x16, over the sprite's top half.
fn terminal_screen() -> SpriteVram {
    let mut c = Canvas::new(Size::S16x16);
    c.block(3, 12, 10, 4, ix::CYAN, ix::OUTLINE);
    c.px(8, 13, ix::WHITE);
    c.rect(7, 14, 3, 1, ix::WHITE);
    c.rect(7, 5, 2, 5, ix::OUTLINE);
    c.block(6, 2, 4, 4, ix::GOLD, ix::OUTLINE);
    c.px(7, 3, ix::WHITE);
    c.vram()
}

/// The elevator car, 32x32: a gold frame round a dark shaft, a floor dial over it.
fn lift_frame() -> SpriteVram {
    let mut c = Canvas::new(Size::S32x32);
    c.rect(0, 0, 32, 32, ix::OUTLINE);
    c.rect(1, 1, 30, 6, ix::GOLD);
    c.rect(1, 6, 30, 1, ix::ORANGE);
    // The dial: a little arrow pointing up.
    c.block(12, 1, 8, 5, ix::PURPLE_DARK, ix::OUTLINE);
    c.px(16, 2, ix::CYAN);
    c.rect(15, 3, 3, 1, ix::CYAN);
    c.rect(2, 7, 2, 25, ix::GOLD);
    c.rect(28, 7, 2, 25, ix::GOLD);
    c.rect(2, 7, 1, 25, ix::WHITE);
    c.rect(4, 7, 24, 25, ix::PURPLE_DARK);
    c.rect(4, 7, 24, 2, ix::OUTLINE);
    c.vram()
}

/// A door panel `w` pixels wide, its edge against the right of an 8x32 sprite.
fn lift_door(w: i32) -> SpriteVram {
    let mut c = Canvas::new(Size::S8x32);
    let x = 8 - w;
    c.rect(x, 0, w, 25, ix::STEEL);
    c.rect(x, 0, 1, 25, ix::STEEL_LIGHT);
    c.rect(7, 0, 1, 25, ix::OUTLINE);
    c.rect(x, 10, w, 2, ix::GOLD);
    c.vram()
}

/// The croupier bot, facing left: a steel head with a green-tinted dealer's visor, a
/// white shirt, a purple waistcoat and a red bow tie; a fan of cards in hand.
const DEALER_BODY: [&str; 14] = [
    "....kkkkkk......",
    "...kgggggSk.....",
    "..kkkkkkkkkk....",
    "..kiiiiiiiik....",
    "...kgcgggGk.....",
    "...kgggggGk.....",
    "....kkkkkk......",
    "...kwkrrkwk.....",
    "..kvvwkkwvvk....",
    ".kSkvvwwvvkSk...",
    ".kSkvvwwvvkSk...",
    ".kgkvvyyvvkgk...",
    "..kkVVVVVVkk....",
    "...kVVVVVVk.....",
];
const DEALER_CARDS: [&str; 4] = ["kkk.", "kwrk", "kwwk", ".kk."];
const LEGS0: [&str; 6] = [
    "....kgk.kgk.....",
    "....kGk.kGk.....",
    "...kgk...kgk....",
    "...kGk...kGk....",
    "..kkkkk.kkkkk...",
    "..kkkkk.kkkkk...",
];
const LEGS1: [&str; 6] = [
    "....kgk.kgk.....",
    "...kGk...kGk....",
    "..kgk.....kgk...",
    "..kGk.....kGk...",
    ".kkkkk...kkkkk..",
    ".kkkkk...kkkkk..",
];

fn dealer(legs: &[&str]) -> SpriteVram {
    let mut c = Canvas::new(Size::S16x32);
    let top = 32 - legs.len() as i32 - DEALER_BODY.len() as i32;
    c.art(0, top, &DEALER_BODY);
    c.art(0, 32 - legs.len() as i32, legs);
    c.vram()
}

/// The security bot, facing left: bulky dark armour, a red visor, a gold badge, big
/// shoulder plates and a steel arm cannon.
const GUARD_BODY: [&str; 16] = [
    "....kkkkkkk.....",
    "...kVVVVVVVk....",
    "..kVkkkkkkVVk...",
    "..kkrrrrwrkVk...",
    "..kVkkkkkkVVk...",
    "...kVVVVVVVk....",
    ".kkkkkkkkkkkkk..",
    "kVVVkVVVVVkVVVk.",
    "kVvVkVVyyVkVvVk.",
    "kkkkkVVyyVkkkkk.",
    "kSgkkVVVVVkkgSk.",
    "kSgkkVVVVVkkgSk.",
    "kggk.kVVVk..kgk.",
    ".kk..kkkkk...k..",
    ".....kVVVk......",
    ".....kkkkk......",
];
const GUARD_AIM: [&str; 3] = ["kkkkkk", "rwSggk", "kkkkkk"];
const GUARD_LEGS0: [&str; 4] = ["...kVk.kVk......", "..kVVk.kVVk.....", "..kkkkkkkkkk....", "..kkkk..kkkk...."];
const GUARD_LEGS1: [&str; 4] = ["...kVk..kVk.....", "..kVVk..kVVk....", ".kkkkk..kkkkk...", ".kkkk....kkkk..."];

fn guard(legs: &[&str]) -> SpriteVram {
    let mut c = Canvas::new(Size::S16x32);
    let top = 32 - legs.len() as i32 - GUARD_BODY.len() as i32;
    c.art(0, top, &GUARD_BODY);
    c.art(0, 32 - legs.len() as i32, legs);
    c.vram()
}
