//! Paints the code-drawn backgrounds (skies, skylines, the dialogue box, the world map)
//! on the stand-in background and writes them as PNG files.

use std::path::Path;

use agb::display::{Palette16, tiled::RegularBackground};

use crate::{
    art::{SPRITE_PALETTE, badlands, trenches, ui},
    sheets::save_rgba,
};

/// Renders the painted cells, `(0, 0)` to the furthest painted cell, with background
/// palette slot `n` taken from `palettes[n]`. Colour 0 is the backdrop when `opaque`,
/// and transparent otherwise.
fn render(bg: &RegularBackground, palettes: &[(u8, Palette16)], opaque: bool, size: Option<(usize, usize)>) -> (usize, usize, Vec<u8>) {
    let (cols, rows) = size.unwrap_or_else(|| {
        let cols = bg.cells.keys().map(|&(x, _)| x + 1).max().unwrap_or(1) as usize;
        let rows = bg.cells.keys().map(|&(_, y)| y + 1).max().unwrap_or(1) as usize;
        (cols, rows)
    });
    let (w, h) = (cols * 8, rows * 8);
    let mut rgba = vec![0u8; w * h * 4];
    let backdrop = palettes.iter().find(|p| p.0 == 0).map(|p| p.1.colour(0).to_rgb8());
    if opaque && let Some([r, g, b]) = backdrop {
        for px in rgba.chunks_exact_mut(4) {
            px.copy_from_slice(&[r, g, b, 255]);
        }
    }
    for (&(tx, ty), (tile, effect)) in &bg.cells {
        if tx < 0 || ty < 0 || tx as usize >= cols || ty as usize >= rows {
            continue;
        }
        let palette = &palettes.iter().find(|p| p.0 == effect.palette).unwrap_or_else(|| panic!("no palette for slot {}", effect.palette)).1;
        for y in 0..8 {
            for x in 0..8 {
                let sx = if effect.hflip { 7 - x } else { x };
                let sy = if effect.vflip { 7 - y } else { y };
                let index = tile.pixel(sx, sy);
                if index == 0 && !(opaque && effect.palette == 0) {
                    continue;
                }
                let [r, g, b] = palette.colour(index as usize).to_rgb8();
                let (px, py) = (tx as usize * 8 + x, ty as usize * 8 + y);
                rgba[(py * w + px) * 4..][..4].copy_from_slice(&[r, g, b, 255]);
            }
        }
    }
    (w, h, rgba)
}

fn save(dir: &Path, name: &str, (w, h, rgba): (usize, usize, Vec<u8>), list: &mut Vec<String>) {
    save_rgba(&dir.join(format!("{name}.png")), w, h, &rgba);
    list.push(format!("  {{\"name\": \"{name}\", \"width\": {w}, \"height\": {h}}}"));
}

/// Writes every background and returns their list as JSON.
pub fn export(dir: &Path) -> String {
    let mut list = Vec::new();
    // A backdrop is a screen tall and a 256-pixel map wide, and repeats across.
    let screen = Some((32, 20));
    for (theme, skies, paint_sky, paint_city) in [
        ("badlands", &badlands::SKY_PALETTES[..], badlands::paint_sky as fn(&mut RegularBackground), badlands::paint_city as fn(&mut RegularBackground)),
        ("trenches", &trenches::SKY_PALETTES[..], trenches::paint_sky, trenches::paint_city),
    ] {
        let (mut sky, mut city) = (RegularBackground::new(), RegularBackground::new());
        paint_sky(&mut sky);
        paint_city(&mut city);
        // Each stage of the level's light is a palette of its own: one picture a stage.
        for (stage, palette) in skies.iter().enumerate() {
            let slots = [(0, palette.clone())];
            save(dir, &format!("{theme}-sky-{stage}"), render(&sky, &slots, true, screen), &mut list);
            save(dir, &format!("{theme}-city-{stage}"), render(&city, &slots, false, screen), &mut list);
        }
    }
    let mut dialog = RegularBackground::new();
    ui::paint_dialog_box(&mut dialog);
    save(dir, "dialog-box", render(&dialog, &[(ui::DIALOG_SLOT, ui::DIALOG_PALETTE.clone())], false, None), &mut list);
    let mut map = RegularBackground::new();
    ui::paint_map(&mut map);
    save(dir, "world-map", render(&map, &[(ui::UI_PALETTE, SPRITE_PALETTE.clone())], false, Some((30, 20))), &mut list);
    format!("[\n{}\n]", list.join(",\n"))
}
