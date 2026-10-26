use std::collections::VecDeque;

use crate::config::{OutputFrom, OutputSlice};

/// Keeps only the lines `slice` asks for while a run's output streams in, so
/// a chatty program can't grow it without bound. Blank lines don't count.
pub(super) struct Output {
    slice: OutputSlice,
    lines: VecDeque<Line>,
    current: Line,
}

#[derive(Default)]
struct Line {
    bytes: Vec<u8>,
    /// More bytes came than were kept.
    cut: bool,
}

/// UTF-8 takes up to 4 bytes per character.
const MAX_CHAR_BYTES: usize = 4;

impl Output {
    pub fn new(slice: OutputSlice) -> Self {
        Self {
            slice,
            lines: VecDeque::new(),
            current: Line::default(),
        }
    }

    pub fn push(&mut self, mut bytes: &[u8]) {
        while !bytes.is_empty() {
            let (part, rest) = match bytes.iter().position(|&b| b == b'\n') {
                Some(at) => (&bytes[..at], Some(&bytes[at + 1..])),
                None => (bytes, None),
            };
            let room = (self.slice.max_line_length * MAX_CHAR_BYTES)
                .saturating_sub(self.current.bytes.len());
            self.current
                .bytes
                .extend_from_slice(&part[..part.len().min(room)]);
            self.current.cut |= part.len() > room;
            match rest {
                Some(rest) => {
                    self.end_line();
                    bytes = rest;
                }
                None => break,
            }
        }
    }

    /// The kept lines, each cut to `max_line_length`, one per line.
    pub fn finish(mut self) -> String {
        self.end_line();
        let max = self.slice.max_line_length;
        let lines: Vec<String> = self
            .lines
            .iter()
            .map(|line| {
                let text = String::from_utf8_lossy(&line.bytes);
                let text = text.trim_end();
                if line.cut || text.chars().count() > max {
                    let kept: String = text.chars().take(max.saturating_sub(1)).collect();
                    format!("{kept}…")
                } else {
                    text.to_owned()
                }
            })
            .collect();
        lines.join("\n")
    }

    fn end_line(&mut self) {
        let line = std::mem::take(&mut self.current);
        if line.bytes.iter().all(u8::is_ascii_whitespace) {
            return;
        }
        match self.slice.from {
            OutputFrom::Head if self.lines.len() < self.slice.lines => self.lines.push_back(line),
            OutputFrom::Head => {}
            OutputFrom::Tail => {
                self.lines.push_back(line);
                if self.lines.len() > self.slice.lines {
                    self.lines.pop_front();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(from: OutputFrom, lines: usize, max: usize, chunks: &[&str]) -> String {
        let mut output = Output::new(OutputSlice {
            from,
            lines,
            max_line_length: max,
        });
        for chunk in chunks {
            output.push(chunk.as_bytes());
        }
        output.finish()
    }

    #[test]
    fn keeps_the_head_or_the_tail() {
        let text = ["one\ntwo\n", "\n  \nthree\nfour"];
        assert_eq!(collect(OutputFrom::Head, 2, 100, &text), "one\ntwo");
        assert_eq!(collect(OutputFrom::Tail, 2, 100, &text), "three\nfour");
        assert_eq!(collect(OutputFrom::Tail, 1, 100, &["done\n\n\n"]), "done");
        assert_eq!(collect(OutputFrom::Tail, 1, 100, &[]), "");
    }

    #[test]
    fn joins_lines_split_across_chunks() {
        let chunks = ["DISK ", "OK - 4", "2% used\r\n"];
        assert_eq!(
            collect(OutputFrom::Head, 1, 100, &chunks),
            "DISK OK - 42% used"
        );
    }

    #[test]
    fn cuts_long_lines() {
        assert_eq!(collect(OutputFrom::Tail, 1, 5, &["abcdefgh\n"]), "abcd…");
        assert_eq!(collect(OutputFrom::Tail, 1, 5, &["abcde\n"]), "abcde");
        let long = "x".repeat(10_000);
        assert_eq!(collect(OutputFrom::Tail, 1, 3, &[&long, &long]), "xx…");
        assert_eq!(collect(OutputFrom::Tail, 1, 3, &["ééééé"]), "éé…");
    }
}
