//! Screens drawn on their own background layer: the dialogue box and the world map.
//! Both use the sprite palette's colours (see [`super::ix`]) as background palette
//! [`UI_PALETTE`], and are painted once at boot from flat fills and repeating patterns,
//! so most of their cells share tiles.

use alloc::vec;

use agb::display::{
    WIDTH,
    object::{Size, SpriteVram},
    tiled::RegularBackground,
};
use diamond_hands_logic::levels::MAP;

use super::{Canvas, TileCache, ix};

/// Background palette slot for these layers.
pub const UI_PALETTE: u8 = 2;
/// Top of the dialogue box, in screen pixels.
pub const BOX_TOP: i32 = 112;
/// The portrait frame's left edge in the box's map. The renderer scrolls the box by
/// [`BOX_SCROLL_RIGHT`] to put the frame on the right for the boss.
pub const FRAME_X: i32 = 4;
pub const BOX_SCROLL_RIGHT: i32 = 64;
/// Where a portrait sits on screen, left or right side.
pub const PORTRAIT_LEFT: (i32, i32) = (FRAME_X + 4, BOX_TOP + 8);
pub const PORTRAIT_RIGHT: (i32, i32) = (FRAME_X + 4 + 256 - BOX_SCROLL_RIGHT, BOX_TOP + 8);

/// A dark violet box across the bottom of the screen with a magenta trim, and a steel
/// portrait frame at [`FRAME_X`]. The map is 256 pixels wide, so the box wraps
/// seamlessly when scrolled.
pub fn paint_dialog_box(bg: &mut RegularBackground) {
    let frame = (FRAME_X, BOX_TOP + 4, 40, 40);
    let pixel = |x: i32, y: i32| {
        let d = y - BOX_TOP;
        let (fx, fy, fw, fh) = frame;
        let in_frame = (fx..fx + fw).contains(&x) && (fy..fy + fh).contains(&y);
        match d {
            ..0 => ix::CLEAR,
            0 | 2 => ix::OUTLINE,
            1 => ix::LASER,
            _ if in_frame => {
                let edge = x == fx || x == fx + fw - 1 || y == fy || y == fy + fh - 1;
                let inner = x == fx + 1 || x == fx + fw - 2 || y == fy + 1 || y == fy + fh - 2;
                match () {
                    _ if edge => ix::STEEL_LIGHT,
                    _ if inner => ix::STEEL,
                    _ => ix::OUTLINE,
                }
            }
            // Faint scanlines.
            _ if d % 4 == 3 => ix::PURPLE,
            _ => ix::PURPLE_DARK,
        }
    };
    let mut cache = TileCache::new();
    for ty in BOX_TOP / 8..20 {
        for tx in 0..32 {
            cache.paint(bg, (tx, ty), UI_PALETTE, &pixel);
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
