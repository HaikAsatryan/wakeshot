//! The screen as a person would see it, from the pseudo console's output.
//!
//! TUIs redraw only the cells that change, so plain text with the escapes cut out would split words; a terminal
//! model (`vt100`) gives the real screen. It also answers the queries a terminal answers, since some TUIs wait for
//! them: only queries that reach us, so a reply never doubles one the pseudo console gave itself.

use std::collections::BTreeSet;
use std::num::NonZeroU16;

#[derive(Default)]
struct Replies {
    out: Vec<u8>,
    seen: BTreeSet<String>,
    fresh: Vec<String>,
}

impl Replies {
    fn note(&mut self, kind: String) {
        if self.seen.insert(kind.clone()) {
            self.fresh.push(kind);
        }
    }
}

impl vt100::Callbacks for Replies {
    fn unhandled_csi(
        &mut self,
        screen: &mut vt100::Screen,
        i1: Option<u8>,
        i2: Option<u8>,
        params: &[&[u16]],
        c: char,
    ) {
        let first = params.first().and_then(|p| p.first()).copied();
        let reply = match (i1, i2, c, first) {
            (None, None, 'n', Some(6)) => {
                let (row, col) = screen.cursor_position();
                Some(format!("\x1b[{};{}R", row + 1, col + 1))
            }
            (None, None, 'n', Some(5)) => Some("\x1b[0n".to_string()),
            // A plain VT100 with advanced video: the safest answer, and what tells a kitty keyboard probe (`CSI ? u`,
            // left unanswered) that the protocol is off, so keys stay in their legacy form.
            (None, None, 'c', None | Some(0)) => Some("\x1b[?1;2c".to_string()),
            (None, None, 't', Some(18)) => {
                let (rows, cols) = screen.size();
                Some(format!("\x1b[8;{rows};{cols}t"))
            }
            _ => None,
        };
        let mut kind = String::from("CSI ");
        for b in [i1, i2].into_iter().flatten() {
            kind.push(char::from(b));
        }
        let shown: Vec<String> = params
            .iter()
            .map(|p| p.iter().map(u16::to_string).collect::<Vec<_>>().join(":"))
            .collect();
        kind.push_str(&shown.join(";"));
        kind.push(c);
        if let Some(reply) = reply {
            self.out.extend_from_slice(reply.as_bytes());
            kind.push_str(" (answered)");
        }
        self.note(kind);
    }

    fn unhandled_osc(&mut self, _: &mut vt100::Screen, params: &[&[u8]]) {
        // Windows Terminal's default dark scheme (Campbell): apps that pick colors by theme get a dark terminal.
        let reply = match params {
            [b"10", b"?"] => Some("\x1b]10;rgb:cccc/cccc/cccc\x1b\\"),
            [b"11", b"?"] => Some("\x1b]11;rgb:0c0c/0c0c/0c0c\x1b\\"),
            _ => None,
        };
        let name = params
            .first()
            .map(|p| String::from_utf8_lossy(p).into_owned())
            .unwrap_or_default();
        let mut kind = format!("OSC {name}");
        if let Some(reply) = reply {
            self.out.extend_from_slice(reply.as_bytes());
            kind.push_str(" ? (answered)");
        }
        self.note(kind);
    }
}

pub struct Term {
    parser: vt100::Parser<Replies>,
}

impl Term {
    pub fn new(rows: u16, cols: u16) -> Self {
        let rows = NonZeroU16::new(rows).unwrap_or(NonZeroU16::MIN);
        let cols = NonZeroU16::new(cols).unwrap_or(NonZeroU16::MIN);
        Self {
            parser: vt100::Parser::new_with_callbacks(rows, cols, 0, Replies::default()),
        }
    }

    /// Feeds output and returns the replies to write back to the child's input.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.parser.process(bytes);
        std::mem::take(&mut self.parser.callbacks_mut().out)
    }

    /// One line per screen row. `contents()` joins rows that a full-width write wrapped, and TUIs that paint every
    /// cell of a row (OpenCode) would come out as one line.
    pub fn text(&self) -> String {
        let screen = self.parser.screen();
        let (_, cols) = screen.size();
        let rows: Vec<String> = screen
            .rows(0, cols.get())
            .map(|r| r.trim_end().to_string())
            .collect();
        let used = rows
            .iter()
            .rposition(|r| !r.is_empty())
            .map_or(0, |i| i + 1);
        rows[..used].join("\n")
    }

    pub fn bracketed_paste(&self) -> bool {
        self.parser.screen().bracketed_paste()
    }

    /// Kinds of unhandled sequences seen for the first time since the last call, for the log.
    pub fn new_kinds(&mut self) -> Vec<String> {
        std::mem::take(&mut self.parser.callbacks_mut().fresh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_redraws_read_as_whole_words() {
        let mut t = Term::new(5, 20);
        // A diff renderer writes cells out of order with cursor moves, as ratatui does.
        t.feed(b"Allow\x1b[1;12Honce");
        t.feed(b"\x1b[1;7Hthis");
        assert!(t.text().contains("Allow this once"));
        t.feed(b"\x1b[2J\x1b[H");
        assert!(!t.text().contains("Allow"));
    }

    #[test]
    fn full_width_rows_stay_separate_lines() {
        let mut t = Term::new(4, 10);
        t.feed(b"0123456789abc");
        assert_eq!(t.text(), "0123456789\nabc");
    }

    #[test]
    fn answers_queries_that_reach_it() {
        let mut t = Term::new(10, 40);
        t.feed(b"\x1b[3;7H");
        assert_eq!(t.feed(b"\x1b[6n"), b"\x1b[3;7R");
        assert_eq!(t.feed(b"\x1b[c"), b"\x1b[?1;2c");
        assert_eq!(t.feed(b"\x1b[18t"), b"\x1b[8;10;40t");
        assert_eq!(
            t.feed(b"\x1b]11;?\x07"),
            b"\x1b]11;rgb:0c0c/0c0c/0c0c\x1b\\"
        );
        assert_eq!(t.feed(b"\x1b[?u"), b"");
        let kinds = t.new_kinds();
        assert!(
            kinds.contains(&"CSI 6n (answered)".to_string()),
            "{kinds:?}"
        );
        assert!(kinds.contains(&"CSI ?0u".to_string()), "{kinds:?}");
        assert!(
            kinds.contains(&"OSC 11 ? (answered)".to_string()),
            "{kinds:?}"
        );
        t.feed(b"\x1b[6n");
        assert!(t.new_kinds().is_empty());
    }
}
