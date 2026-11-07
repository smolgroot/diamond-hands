//! Conversations: a few lines between the wassie and a boss before the fight, or a
//! guide on arrival. The text is typed out a character per frame; A finishes the line,
//! then moves to the next.

/// Characters a dialogue row may hold, so it fits beside the portrait in the ROM's big
/// font (8 pixels a character).
pub const ROW_CHARS: usize = 22;
/// Rows per line of dialogue.
pub const MAX_ROWS: usize = 3;
/// Characters (spaces excluded) a line may show at once. The ROM draws one object per
/// character and the GBA has 128, shared with the scene behind the dialogue box.
pub const MAX_GLYPHS: usize = 72;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Speaker {
    Wassie,
    Bera,
    /// The grey cat in the yellow hazmat suit who meets the wassie on CryptoTerra.
    Cl207,
    /// The Trenches boss.
    Sam,
    /// The casino's penguin cashier.
    Penguin,
}

impl Speaker {
    pub fn name(self) -> &'static str {
        match self {
            Speaker::Wassie => "WASSIE",
            Speaker::Bera => "BERA 3000",
            Speaker::Cl207 => "CL207",
            Speaker::Sam => "SAM",
            Speaker::Penguin => "WADDLES",
        }
    }
}

#[derive(Debug)]
pub struct Line {
    pub speaker: Speaker,
    pub rows: &'static [&'static str],
}

impl Line {
    /// Characters across all rows, spaces included: the typing length.
    pub fn len(&self) -> usize {
        self.rows.iter().map(|r| r.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The first `shown` characters, split back into rows.
    pub fn visible(&self, shown: usize) -> impl Iterator<Item = &'static str> + '_ {
        let mut left = shown;
        self.rows.iter().map(move |row| {
            let n = left.min(row.len());
            left -= n;
            &row[..n]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels;

    #[test]
    fn visible_types_out_row_by_row() {
        let line = Line {
            speaker: Speaker::Wassie,
            rows: &["AB", "CDE"],
        };
        assert_eq!(line.len(), 5);
        assert!(line.visible(0).eq(["", ""]));
        assert!(line.visible(3).eq(["AB", "C"]));
        assert!(line.visible(9).eq(["AB", "CDE"]));
    }

    /// Every line fits the box and only uses characters the ROM's font has.
    #[test]
    fn every_line_fits_and_is_printable() {
        const FONT: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ!:/-+.<>,'? ";
        let arrivals = (0..levels::ALL.len() as u8).filter_map(levels::arrival);
        let all = levels::ALL
            .iter()
            .flat_map(|l| [l.boss_talk, l.call])
            .chain(arrivals.map(|a| a.lines))
            .chain(crate::game::MERCHANT_LINES);
        for lines in all {
            for line in lines {
                assert!(!line.is_empty() && line.rows.len() <= MAX_ROWS, "{line:?}");
                let glyphs = line.rows.iter().flat_map(|r| r.bytes()).filter(|&c| c != b' ').count();
                assert!(glyphs <= MAX_GLYPHS, "{glyphs} glyphs: {line:?}");
                for row in line.rows {
                    assert!(row.len() <= ROW_CHARS, "too wide: {row}");
                    assert!(row.bytes().all(|c| FONT.contains(&c)), "unprintable: {row}");
                }
            }
        }
    }
}
