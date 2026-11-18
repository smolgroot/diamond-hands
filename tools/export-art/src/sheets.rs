//! Collects painted sprites into sheets and writes them as PNG files.

use std::{collections::BTreeMap, fs::File, io::BufWriter, path::Path};

use agb::display::{Palette16, object::SpriteVram};

/// One sheet: frames of one size drawn in one palette, side by side.
struct Sheet {
    owner: String,
    width: usize,
    height: usize,
    palette: Palette16,
    frames: Vec<Vec<u8>>,
    /// Field name to its frames, in the order added.
    animations: Vec<(String, Vec<usize>)>,
}

#[derive(Default)]
pub struct Sheets {
    sheets: Vec<Sheet>,
    /// `(owner, field)` of drawings added one at a time rather than as an array: the art
    /// code holds them as a single sprite, and the bindings generated from the manifest
    /// hand them back that way.
    singles: Vec<(String, String)>,
}

impl Sheets {
    pub fn add(&mut self, owner: &str, field: &str, sprite: &SpriteVram) {
        self.singles.push((owner.to_owned(), field.to_owned()));
        self.add_all(owner, field, core::slice::from_ref(sprite));
    }

    pub fn add_all(&mut self, owner: &str, field: &str, sprites: &[SpriteVram]) {
        for sprite in sprites {
            let d = &sprite.0;
            let index = self
                .sheets
                .iter()
                .position(|s| s.owner == owner && s.width == d.width && s.height == d.height && s.palette == d.palette)
                .unwrap_or_else(|| {
                    self.sheets.push(Sheet {
                        owner: owner.to_owned(),
                        width: d.width,
                        height: d.height,
                        palette: d.palette.clone(),
                        frames: Vec::new(),
                        animations: Vec::new(),
                    });
                    self.sheets.len() - 1
                });
            let sheet = &mut self.sheets[index];
            // The same drawing twice (a pose reused) is one frame.
            let frame = sheet.frames.iter().position(|f| *f == d.pixels).unwrap_or_else(|| {
                sheet.frames.push(d.pixels.clone());
                sheet.frames.len() - 1
            });
            match sheet.animations.iter_mut().find(|(name, _)| name == field) {
                Some((_, frames)) => frames.push(frame),
                None => sheet.animations.push((field.to_owned(), vec![frame])),
            }
        }
    }

    /// Writes every sheet as `<name>.png` in `dir`, every distinct palette as a palette
    /// file in `dir/../palettes`, and returns the sheets' manifest as JSON.
    pub fn write(&self, dir: &Path) -> String {
        let palette_dir = dir.join("../palettes");
        std::fs::create_dir_all(&palette_dir).unwrap();
        // Sheets with the same colours in the same slots share a palette file, named for
        // the first owner that uses it ("sprite" for the common sprite palette).
        let mut palettes: Vec<(String, &Palette16)> = Vec::new();
        for sheet in &self.sheets {
            if !palettes.iter().any(|(_, p)| **p == sheet.palette) {
                let name = if sheet.palette == crate::art::SPRITE_PALETTE { "sprite".to_owned() } else { sheet.owner.clone() };
                palettes.push((name, &sheet.palette));
            }
        }
        for (name, palette) in &palettes {
            write_palette_png(&palette_dir.join(format!("{name}.png")), palette);
        }
        let mut names: BTreeMap<String, usize> = BTreeMap::new();
        let mut entries = Vec::new();
        for sheet in &self.sheets {
            // Named for its owner, with its frame size when the owner has several.
            let sizes = self.sheets.iter().filter(|s| s.owner == sheet.owner).count();
            let base = if sizes > 1 { format!("{}-{}x{}", sheet.owner, sheet.width, sheet.height) } else { sheet.owner.clone() };
            let n = names.entry(base.clone()).or_default();
            *n += 1;
            let name = if *n > 1 { format!("{base}-{n}") } else { base };
            write_png(&dir.join(format!("{name}.png")), sheet);
            let animations: Vec<String> = sheet
                .animations
                .iter()
                .map(|(field, frames)| {
                    let single = self.singles.iter().any(|(o, f)| *o == sheet.owner && f == field);
                    format!("\"{field}\": {{\"frames\": {frames:?}, \"single\": {single}}}")
                })
                .collect();
            let palette = &palettes.iter().find(|(_, p)| **p == sheet.palette).unwrap().0;
            entries.push(format!(
                "  {{\"name\": \"{name}\", \"palette\": \"{palette}\", \"width\": {}, \"height\": {}, \"frames\": {}, \"animations\": {{{}}}}}",
                sheet.width,
                sheet.height,
                sheet.frames.len(),
                animations.join(", ")
            ));
        }
        format!("[\n{}\n]", entries.join(",\n"))
    }
}

/// A sheet as an RGBA PNG: palette index 0 is transparent.
fn write_png(path: &Path, sheet: &Sheet) {
    let (w, h) = (sheet.width * sheet.frames.len(), sheet.height);
    let mut rgba = vec![0u8; w * h * 4];
    for (f, frame) in sheet.frames.iter().enumerate() {
        for (i, &index) in frame.iter().enumerate() {
            if index == 0 {
                continue;
            }
            let (x, y) = (f * sheet.width + i % sheet.width, i / sheet.width);
            let [r, g, b] = sheet.palette.colour(index as usize).to_rgb8();
            rgba[(y * w + x) * 4..][..4].copy_from_slice(&[r, g, b, 255]);
        }
    }
    save_rgba(path, w, h, &rgba);
}

/// A palette file: 16 pixels, 4 by 4, one per slot in reading order. Slot 0 is transparent.
fn write_palette_png(path: &Path, palette: &Palette16) {
    let mut rgba = vec![0u8; 16 * 4];
    for slot in 1..16 {
        let [r, g, b] = palette.colour(slot).to_rgb8();
        rgba[slot * 4..][..4].copy_from_slice(&[r, g, b, 255]);
    }
    save_rgba(path, 4, 4, &rgba);
}

pub fn save_rgba(path: &Path, w: usize, h: usize, rgba: &[u8]) {
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(path).unwrap()), w as u32, h as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(rgba).unwrap();
}
