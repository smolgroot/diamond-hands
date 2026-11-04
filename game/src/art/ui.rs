//! Screens drawn on their own background layer: the dialogue box and the world map.
//! The map uses the sprite palette's colours (see [`super::ix`]) as background palette
//! [`UI_PALETTE`]; the dialogue box has its own greys in [`DIALOG_PALETTE`]. Both are
//! painted once at boot from flat fills and repeating patterns, so most of their cells
//! share tiles.

use alloc::vec;

use agb::display::{
    Palette16, WIDTH,
    object::{Size, SpriteVram},
    tiled::RegularBackground,
};
use diamond_hands_logic::levels::MAP;

use super::{Canvas, TileCache, ix, rgb};

/// Background palette slot for these layers.
pub const UI_PALETTE: u8 = 2;
/// Background palette slot for the dialogue box's greys.
pub const DIALOG_SLOT: u8 = 3;

/// The dialogue box's colours: neutral greys for a raised panel and a sunken frame.
pub static DIALOG_PALETTE: Palette16 = Palette16::new([
    rgb(0, 0, 0),
    rgb(2, 2, 3),
    rgb(31, 31, 31),
    rgb(23, 23, 24),
    rgb(16, 16, 17),
    rgb(10, 10, 11),
    rgb(5, 5, 6),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
    rgb(0, 0, 0),
]);

/// [`DIALOG_PALETTE`] indices.
mod grey {
    pub const CLEAR: u8 = 0;
    pub const BLACK: u8 = 1;
    pub const WHITE: u8 = 2;
    pub const LIGHT: u8 = 3;
    pub const MID: u8 = 4;
    pub const DARK: u8 = 5;
    pub const DARKER: u8 = 6;
}

/// The dialogue box spans the top of the screen, clear of everyone standing below.
pub const BOX_TOP: i32 = 0;
pub const BOX_HEIGHT: i32 = 64;
/// The box's map holds two copies, 256 rows apart: the portrait frame on the left
/// (the wassie talking) and on the right (anyone else). The renderer scrolls to one.
pub const BOX_SCROLL_RIGHT: i32 = 256;
/// The sunken portrait frame: 40x40, its left edge on each side.
const FRAME_SIZE: i32 = 40;
const FRAME_LEFT: i32 = 6;
const FRAME_RIGHT: i32 = WIDTH - 6 - FRAME_SIZE;
const FRAME_TOP: i32 = BOX_TOP + (BOX_HEIGHT - FRAME_SIZE) / 2;
/// Where a 32x32 portrait sits on screen, left or right side.
pub const PORTRAIT_LEFT: (i32, i32) = (FRAME_LEFT + 4, FRAME_TOP + 4);
pub const PORTRAIT_RIGHT: (i32, i32) = (FRAME_RIGHT + 4, FRAME_TOP + 4);
/// Where the text starts beside the frame on each side.
pub const TEXT_LEFT: i32 = FRAME_LEFT + FRAME_SIZE + 6;
pub const TEXT_RIGHT: i32 = 8;

/// A grey panel across the top of the screen with a raised 3D border (lit from the top
/// left), and a sunken frame for the portrait.
pub fn paint_dialog_box(bg: &mut RegularBackground) {
    // Two-step bevel: the outer step strongest. `lit` picks the highlight side.
    let bevel = |step: i32, lit: bool| match (step, lit) {
        (0, true) => grey::WHITE,
        (_, true) => grey::LIGHT,
        (0, false) => grey::DARKER,
        (_, false) => grey::DARK,
    };
    let pixel = |x: i32, y: i32| {
        let frame_x = if y >= BOX_SCROLL_RIGHT { FRAME_RIGHT } else { FRAME_LEFT };
        let y = y % BOX_SCROLL_RIGHT - BOX_TOP;
        if !(0..BOX_HEIGHT).contains(&y) || x >= WIDTH {
            return grey::CLEAR;
        }
        let (right, bottom) = (WIDTH - 1, BOX_HEIGHT - 1);
        if x == 0 || y == 0 || x == right || y == bottom {
            return grey::BLACK;
        }
        // The panel's edge, raised: lit along the top and left.
        let (top, left, low, end) = (y - 1, x - 1, bottom - 1 - y, right - 1 - x);
        if top.min(left).min(low).min(end) < 2 {
            let lit = (top < 2 && top <= end) || (left < 2 && left <= low);
            return bevel(if lit { top.min(left) } else { low.min(end) }, lit);
        }
        // The portrait frame, sunken: shadowed along the top and left.
        let (fx, fy) = (x - frame_x, y + BOX_TOP - FRAME_TOP);
        if (0..FRAME_SIZE).contains(&fx) && (0..FRAME_SIZE).contains(&fy) {
            let (fl, ft, fr, fb) = (fx, fy, FRAME_SIZE - 1 - fx, FRAME_SIZE - 1 - fy);
            if ft.min(fl).min(fr).min(fb) >= 2 {
                return grey::BLACK;
            }
            let shadowed = (ft < 2 && ft <= fr) || (fl < 2 && fl <= fb);
            return bevel(if shadowed { ft.min(fl) } else { fb.min(fr) }, !shadowed);
        }
        grey::MID
    };
    let mut cache = TileCache::new();
    for base in [0, BOX_SCROLL_RIGHT / 8] {
        for ty in base + BOX_TOP / 8..base + (BOX_TOP + BOX_HEIGHT) / 8 {
            for tx in 0..32 {
                cache.paint(bg, (tx, ty), DIALOG_SLOT, &pixel);
            }
        }
    }
}

/// One island on the world map: a union of ellipses `(cx, cy, rx, ry)` with a top
/// colour, a pattern colour, a lit rim and a cliff face below. Textures use bit masks:
/// the GBA has no divide instruction, and this runs for every pixel at boot.
struct Island {
    blobs: &'static [(i32, i32, i32, i32)],
    top: u8,
    pattern: u8,
    rim: u8,
    cliff: u8,
    /// Whether `pattern` shows at a pixel of the top.
    texture: fn(i32, i32) -> bool,
}

/// Height of the cliff face under each island, in pixels.
const CLIFF: i32 = 4;

static ISLANDS: [Island; 3] = [
    // The Badlands: orange mesas speckled with red rock.
    Island {
        blobs: &[(42, 106, 34, 20), (62, 94, 20, 14), (22, 98, 16, 12)],
        top: ix::ORANGE,
        pattern: ix::RED,
        rim: ix::GOLD,
        cliff: ix::RED_DARK,
        texture: |x, y| (x + 3 * y) & 15 == 0 && y & 1 == 0,
    },
    // Solana Trenches: a violet strip lined with neon.
    Island {
        blobs: &[(120, 62, 34, 16), (100, 70, 16, 10), (142, 70, 16, 10)],
        top: ix::PURPLE,
        pattern: ix::LASER,
        rim: ix::LASER_LIGHT,
        cliff: ix::STEEL_DARK,
        texture: |x, y| y & 7 == 0 && x & 3 != 0,
    },
    // Ethereum City: blue blocks with lit windows.
    Island {
        blobs: &[(196, 94, 30, 22), (212, 76, 16, 12), (178, 112, 14, 8)],
        top: ix::BLUE,
        pattern: ix::CYAN,
        rim: ix::CYAN,
        cliff: ix::STEEL,
        texture: |x, y| x & 7 < 2 && y & 7 < 2,
    },
];

impl Island {
    fn bounds(&self) -> (i32, i32, i32, i32) {
        self.blobs.iter().fold((i32::MAX, i32::MAX, i32::MIN, i32::MIN), |(x0, y0, x1, y1), &(cx, cy, rx, ry)| {
            (x0.min(cx - rx), y0.min(cy - ry), x1.max(cx + rx), y1.max(cy + ry))
        })
    }

    fn inside(&self, x: i32, y: i32) -> bool {
        self.blobs.iter().any(|&(cx, cy, rx, ry)| {
            let (dx, dy) = (x - cx, y - cy);
            dx * dx * ry * ry + dy * dy * rx * rx <= rx * rx * ry * ry
        })
    }
}

/// Map height in pixels.
const MAP_H: i32 = 160;

/// Which island covers each map pixel (index + 1, or 0 for sea), worked out once so
/// the outline and cliff tests are lookups rather than more ellipse maths.
struct Land(alloc::vec::Vec<u8>);

impl Land {
    fn new() -> Self {
        let mut land = vec![0; (WIDTH * MAP_H) as usize];
        for (i, island) in ISLANDS.iter().enumerate() {
            let (x0, y0, x1, y1) = island.bounds();
            for y in y0.max(0)..=y1.min(MAP_H - 1) {
                for x in x0.max(0)..=x1.min(WIDTH - 1) {
                    if island.inside(x, y) {
                        land[(y * WIDTH + x) as usize] = i as u8 + 1;
                    }
                }
            }
        }
        Self(land)
    }

    fn at(&self, x: i32, y: i32) -> u8 {
        if (0..WIDTH).contains(&x) && (0..MAP_H).contains(&y) {
            self.0[(y * WIDTH + x) as usize]
        } else {
            0
        }
    }

    /// The land colour at a pixel, or `None` for sea. Each top is outlined, lit along
    /// its upper rim, and stands on a cliff face.
    fn pixel(&self, x: i32, y: i32) -> Option<u8> {
        let here = self.at(x, y);
        if here != 0 {
            let island = &ISLANDS[here as usize - 1];
            let edge = [(-1, 0), (1, 0), (0, -1), (0, 1)]
                .iter()
                .any(|&(dx, dy)| self.at(x + dx, y + dy) != here);
            return Some(match () {
                _ if edge => ix::OUTLINE,
                _ if self.at(x, y - 2) != here => island.rim,
                _ if (island.texture)(x, y) => island.pattern,
                _ => island.top,
            });
        }
        // The cliff hangs below the top's lower edge.
        let (above, owner) = (1..=CLIFF).find_map(|d| {
            let owner = self.at(x, y - d);
            (owner != 0).then_some((d, owner))
        })?;
        let side = self.at(x - 1, y - above) != owner || self.at(x + 1, y - above) != owner;
        let cliff = ISLANDS[owner as usize - 1].cliff;
        Some(if above == CLIFF || side { ix::OUTLINE } else { cliff })
    }
}

/// Gold dashes along the road between neighbouring places.
fn road(x: i32, y: i32) -> bool {
    MAP.windows(2).any(|pair| {
        let (ax, ay, bx, by) = (pair[0].x, pair[0].y, pair[1].x, pair[1].y);
        // Cheap box test first; it also keeps the products below in range.
        if x < ax.min(bx) - 2 || x > ax.max(bx) + 2 || y < ay.min(by) - 2 || y > ay.max(by) + 2 {
            return false;
        }
        let (dx, dy) = (bx - ax, by - ay);
        let (px, py) = (x - ax, y - ay);
        let along = px * dx + py * dy;
        let len_sq = dx * dx + dy * dy;
        if along < 0 || along > len_sq {
            return false;
        }
        // Within about a pixel of the line: |cross| / len <= 1, squared to stay integer.
        let cross = px * dy - py * dx;
        cross * cross * 4 <= len_sq * 3 && (along * 12 / len_sq) % 2 == 0
    })
}

/// The world map: islands for each place on a violet sea under a faint cyber grid,
/// joined by a dashed gold road.
pub fn paint_map(bg: &mut RegularBackground) {
    let land = Land::new();
    let sea = |x: i32, y: i32| if x & 15 == 8 || y & 15 == 8 { ix::PURPLE } else { ix::PURPLE_DARK };
    let pixel = |x: i32, y: i32| {
        if road(x, y) {
            return ix::GOLD;
        }
        land.pixel(x, y).unwrap_or_else(|| sea(x, y))
    };
    // Boxes around everything that isn't open sea: each island with its cliff, and
    // each stretch of road. Cells clear of all of them are painted as plain sea.
    let mut boxes = [(0, 0, 0, 0); ISLANDS.len() + MAP.len() - 1];
    for (b, island) in boxes.iter_mut().zip(&ISLANDS) {
        let (x0, y0, x1, y1) = island.bounds();
        *b = (x0 - 1, y0 - 1, x1 + 1, y1 + CLIFF + 1);
    }
    for (b, pair) in boxes[ISLANDS.len()..].iter_mut().zip(MAP.windows(2)) {
        let (a, z) = (&pair[0], &pair[1]);
        *b = (a.x.min(z.x) - 2, a.y.min(z.y) - 2, a.x.max(z.x) + 2, a.y.max(z.y) + 2);
    }
    let mut cache = TileCache::new();
    for ty in 0..20 {
        for tx in 0..30 {
            let (x, y) = (tx * 8, ty * 8);
            let busy = boxes
                .iter()
                .any(|&(x0, y0, x1, y1)| x + 7 >= x0 && x <= x1 && y + 7 >= y0 && y <= y1);
            if busy {
                cache.paint(bg, (tx, ty), UI_PALETTE, &pixel);
            } else {
                cache.paint(bg, (tx, ty), UI_PALETTE, &sea);
            }
        }
    }
}

/// Marker for a place whose level is still being built: a steel beacon with a "?".
pub fn soon_marker() -> SpriteVram {
    let mut c = Canvas::new(Size::S16x16);
    c.art(
        3,
        2,
        &[
            "...kkkk...",
            "..kSSSgk..",
            ".kSkkkkgk.",
            "kSgkSSkggk",
            "kggggkkggk",
            "kgggkkgggk",
            ".kggggggk.",
            "..kgkkgk..",
            "...kggk...",
            "....kk....",
        ],
    );
    c.vram()
}
