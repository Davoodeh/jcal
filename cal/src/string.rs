//! An additional helper module for working with terminal strings.

#![allow(dead_code)]

use std::cmp::Ordering;

/// Highlights a value in color depending on the color configuration.
pub fn highlight(s: &str) -> String {
    use colored::Colorize;
    s.reversed().to_string()
}

/// Calculate the "width" so it corresponds to columns in terminal.
pub fn ansi_width(s: &str) -> usize {
    ansi_width::ansi_width(s)
}

/// Take characters while it fits in the maximum width.
pub fn cut_end(s: &str, maximum_width: usize) -> &str {
    // TODO replace with binary search on widths
    let mut width_this_far = 0;
    for (i, c) in s.char_indices() {
        width_this_far += ansi_width(&c.to_string());
        if width_this_far > maximum_width {
            return &s[..i];
        }
    }
    s
}

/// Repeat this to fit the requested width.
///
/// If the given string has a width of 0, will return an empty string since the repeat can never
/// reach the requested width.
///
/// As the name of the parameter suggests, this may have a lesser width than requested.
pub fn repeat_for_ansi_width(s: &str, maximum_width: usize) -> String {
    let initial_width = ansi_width(s);
    if initial_width == 0 {
        return Default::default();
    }

    let mut repeat_iter = s.chars().map(|c| (c, ansi_width(&c.to_string()))).cycle();

    let mut buf = String::new();
    let mut width = 0;
    loop {
        // since `s` has a verified positive width, this won't panic
        let (fill_char, fill_width) = repeat_iter.next().unwrap();
        width += fill_width;
        if width > maximum_width {
            return buf;
        }
        buf.push(fill_char);
    }
}

/// Shift the given string to right, left or center by repeating the given filler.
// TODO since has a strict width policy and limits cuts the output if larger than given
//      width, rename to something like Cell to reflect that.
pub struct Aligner<'a> {
    filler: &'a str,
    filler_adjust: char,
}

impl Aligner<'_> {
    /// A valid instance that aligns everything with space.
    pub const SPACE: Self = Self {
        filler: " ",
        filler_adjust: ' ',
    };

    pub const CENTER_DOT: Self = Self {
        filler: "·",
        filler_adjust: '·',
    };

    pub const ZERO: Self = Self {
        filler: "0",
        filler_adjust: '0',
    };
}

impl<'a> Aligner<'a> {
    /// Create a new valid instance.
    ///
    /// To guarantee the given ANSI width even though the `filler` may fail to produce a compatible
    /// number, the `filler_adjust` is used. To keep the code simple, we delegate this strict
    /// requirement to the `filler_adjust`. Hence the [`Option`] return.
    ///
    /// Returns [`Option`] if the `filler_adjust` does not have the strict width of 1. If unsure,
    /// just pass a simple space (` `).
    pub fn new(filler: &'a str, filler_adjust: char) -> Option<Self> {
        if ansi_width(&filler_adjust.to_string()) != 1 {
            return None;
        }
        Some(Self {
            filler,
            filler_adjust,
        })
    }

    /// Return the filler with the exact width.
    ///
    /// This has the minor difference with just calling [`repeat_for_ansi_width`] that ensure the
    /// width is valid.
    pub fn filler(&self, needed_width: usize) -> String {
        let mut filler = repeat_for_ansi_width(self.filler, needed_width);
        let diff = needed_width.saturating_sub(ansi_width(&filler));

        // this may fail in multiple ways if filler does not have a width of 1.
        // - width 0: loops forever
        // - width +1: may extend the needed_width
        //
        // without a length of one this cannot be a for loop either
        for _ in 0..diff {
            filler.push(self.filler_adjust);
        }
        filler
    }

    /// Shift the given string to right by repeating the filler.
    pub fn right(&self, s: &str, width: usize) -> String {
        let actual_width = ansi_width(s);
        match actual_width.cmp(&width) {
            Ordering::Less => self.filler(width - actual_width) + s,
            Ordering::Equal => s.to_owned(),
            Ordering::Greater => cut_end(s, width).to_string(),
        }
    }

    /// Append the repeating filler to the end to fit the exact width.
    pub fn left(&self, s: &str, width: usize) -> String {
        let actual_width = ansi_width(s);
        match actual_width.cmp(&width) {
            Ordering::Less => s.to_owned() + &self.filler(width.saturating_sub(ansi_width(s))),
            Ordering::Equal => s.to_owned(),
            Ordering::Greater => cut_end(s, width).to_string(),
        }
    }

    /// Shift the value to the center and fill the surroundings with the repeating filler.
    ///
    /// This prefers "a " rather than " a" if fillers cannot neatly fit in place.
    pub fn center(&self, s: &str, width: usize) -> String {
        let actual_width = ansi_width(s);
        match actual_width.cmp(&width) {
            Ordering::Less => {
                let padding = width.saturating_sub(actual_width);
                let left = padding / 2;
                let right = left + (padding % 2);
                self.filler(left) + s + &self.filler(right)
            }
            Ordering::Equal => s.to_owned(),
            Ordering::Greater => cut_end(s, width).to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cut_end() {
        assert_eq!("", cut_end("", 5));
        assert_eq!("", cut_end("", 0));
        assert_eq!("", cut_end("12345", 0));
        assert_eq!("12345", cut_end("1234567", 5));
        assert_eq!("x", cut_end("x\u{01F980}", 1));
        assert_eq!("x\u{01F980}", cut_end("x\u{01F980}", 3));
        assert_eq!("x", cut_end("x\u{01F980}", 2));
    }

    #[test]
    fn test_center_ascii_delim1_even() {
        assert_eq!(Aligner::CENTER_DOT.center("12345", 11), "···12345···");
    }

    #[test]
    fn test_center_ascii_delim1_odd() {
        assert_eq!(Aligner::CENTER_DOT.center("1234", 11), "···1234····");
    }

    #[test]
    fn test_center_ascii_delim2_odd() {
        assert_eq!(
            Aligner::new("·.", 'x').unwrap().center("1234", 11),
            "·.·1234·.·."
        );
    }

    #[test]
    fn test_center_ascii_delim2_even() {
        assert_eq!(
            Aligner::new("·.", 'x').unwrap().center("12345", 11),
            "·.·12345·.·"
        );
    }

    #[test]
    fn test_center_uni_adjust() {
        assert_eq!(
            Aligner::new("\u{01F980}", 'x').unwrap().center("1234", 11),
            "\u{01F980}x1234\u{01F980}\u{01F980}"
        );
    }
}
