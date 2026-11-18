//! Lifts the glyph table and the number formatter out of `game/src/font.rs`: the art
//! reads them, and the rest of that file draws with agb objects the host doesn't have.
use std::{env, fs, path::PathBuf};

fn main() {
    let font = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../game/src/font.rs");
    println!("cargo:rerun-if-changed={}", font.display());
    let src = fs::read_to_string(&font).unwrap();
    let take = |start: &str, end: &str| {
        let from = src.find(start).unwrap_or_else(|| panic!("font.rs has no {start}"));
        let to = from + src[from..].find(end).unwrap() + end.len();
        src[from..to].to_owned()
    };
    let mut out = String::new();
    out += &take("const DIALOG_GLYPHS", "];\n");
    out += &take("pub fn dialog_glyph", "\n}\n");
    out += &take("pub fn number", "\n}\n");
    fs::write(PathBuf::from(env::var("OUT_DIR").unwrap()).join("font.rs"), out).unwrap();
}
