//! Find and replace in the active note: plain text, with or without matching case.

use rune::{TextEdit, Tx};

use crate::app::App;
use crate::md;

pub struct Find {
    pub q: TextEdit,
    pub rep: TextEdit,
    /// The bar shows the replace row.
    pub replace: bool,
    /// Match case.
    pub case: bool,
    /// The match the editor selects, an index into `matches`.
    pub cur: Option<usize>,
}

/// Where `q` occurs in `text`, as byte ranges that don't overlap. Without `case`, letters match
/// in either case.
pub fn matches(text: &str, q: &str, case: bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    if q.is_empty() {
        return out;
    }
    if case {
        let mut from = 0;
        while let Some(i) = text[from..].find(q) {
            out.push((from + i, from + i + q.len()));
            from += i + q.len();
        }
        return out;
    }
    let want: Vec<char> = q.chars().flat_map(char::to_lowercase).collect();
    let starts: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    let mut k = 0;
    while k < starts.len() {
        let start = starts[k];
        let mut end = start;
        let mut ok = true;
        // Walk the text one char at a time, comparing its lowercase form to the query's.
        let mut need = want.iter();
        let mut chars = text[start..].char_indices();
        'outer: loop {
            let Some((ci, c)) = chars.next() else {
                ok = need.next().is_none();
                break;
            };
            for lc in c.to_lowercase() {
                match need.next() {
                    Some(w) if *w == lc => {}
                    _ => {
                        ok = false;
                        break 'outer;
                    }
                }
            }
            end = start + ci + c.len_utf8();
            if need.as_slice().is_empty() {
                break;
            }
        }
        if ok && end > start {
            out.push((start, end));
            while k < starts.len() && starts[k] < end {
                k += 1;
            }
        } else {
            k += 1;
        }
    }
    out
}

impl Find {
    pub fn new(replace: bool) -> Find {
        Find { q: TextEdit::new(""), rep: TextEdit::new(""), replace, case: false, cur: None }
    }
}

impl App {
    /// The find bar's state, for its fields.
    pub fn find_mut(&mut self) -> &mut Find {
        self.find.get_or_insert_with(|| Find::new(false))
    }

    /// Opens the find bar over the active note, or the replace row too. The note shows its
    /// source, so the editor can select what matches. A selection on one line fills the query.
    pub fn open_find(&mut self, replace: bool, tx: &mut Tx) {
        let Some(a) = self.active.clone() else { return };
        if self.note_locked(&a) {
            return;
        }
        self.tab_menu = None;
        self.palette = None;
        self.show_source();
        let picked = self.cur_edit().selected().to_string();
        let f = self.find.get_or_insert_with(|| Find::new(replace));
        if !picked.is_empty() && !picked.contains('\n') {
            f.q = TextEdit::new(picked);
        }
        let n = f.q.text.len();
        f.q.set_selection(0, n);
        let field = if replace && !f.q.text.is_empty() { "find-rep" } else { "find-q" };
        f.replace = replace;
        tx.focus(field, false);
        self.find_from_caret(tx);
    }

    pub fn close_find(&mut self, tx: &mut Tx) {
        self.find = None;
        self.focus_editor(tx);
    }

    /// The matches in the active note.
    pub fn find_matches(&self) -> Vec<(usize, usize)> {
        match &self.find {
            Some(f) => matches(self.active_text(), &f.q.text, f.case),
            None => Vec::new(),
        }
    }

    /// The query changed: select the first match at or after the caret.
    pub fn find_from_caret(&mut self, tx: &mut Tx) {
        let hits = self.find_matches();
        let from = self.cur_edit_opt().map_or(0, |e| e.range().0);
        let i = hits.iter().position(|h| h.0 >= from).or(if hits.is_empty() { None } else { Some(0) });
        self.find_go(i, &hits, tx);
    }

    /// The next match after the selected one, or the one before it, going round at the ends.
    pub fn find_next(&mut self, back: bool, tx: &mut Tx) {
        let hits = self.find_matches();
        if hits.is_empty() {
            self.find_go(None, &hits, tx);
            return;
        }
        let (sa, sb) = self.cur_edit_opt().map_or((0, 0), |e| e.range());
        let n = hits.len();
        let i = match self.find.as_ref().and_then(|f| f.cur).filter(|&i| i < n && hits[i] == (sa, sb)) {
            Some(i) if back => (i + n - 1) % n,
            Some(i) => (i + 1) % n,
            None if back => hits.iter().rposition(|h| h.1 <= sa).unwrap_or(n - 1),
            None => hits.iter().position(|h| h.0 >= sb).unwrap_or(0),
        };
        self.find_go(Some(i), &hits, tx);
    }

    fn find_go(&mut self, i: Option<usize>, hits: &[(usize, usize)], tx: &mut Tx) {
        if let Some(f) = &mut self.find {
            f.cur = i;
        }
        let Some((a, b)) = i.map(|i| hits[i]) else { return };
        if let Some(e) = self.cur_edit_opt() {
            e.set_selection(a, b);
            e.break_history_group();
        }
        let (line, _) = md::line_col(self.active_text(), a);
        tx.scroll_key_to(format!("ln-{line}"), 120.0);
    }

    /// Replaces the selected match and selects the next one.
    pub fn replace_one(&mut self, tx: &mut Tx) {
        let hits = self.find_matches();
        let Some((rep, cur)) = self.find.as_ref().map(|f| (f.rep.text.clone(), f.cur)) else { return };
        let sel = self.cur_edit_opt().map_or((0, 0), |e| e.range());
        let Some(i) = cur.filter(|&i| i < hits.len() && hits[i] == sel) else {
            // Nothing selected yet: the first press selects a match.
            self.find_next(false, tx);
            return;
        };
        let (a, b) = hits[i];
        let e = self.cur_edit();
        e.set_selection(a, b);
        e.insert(&rep);
        e.break_history_group();
        self.after_edit();
        let hits = self.find_matches();
        let after = a + rep.len();
        let next = hits.iter().position(|h| h.0 >= after).or(if hits.is_empty() { None } else { Some(0) });
        self.find_go(next, &hits, tx);
    }

    /// Replaces every match in one step that one undo takes back. Returns how many.
    pub fn replace_all(&mut self, _tx: &mut Tx) -> usize {
        let hits = self.find_matches();
        let Some(f) = &self.find else { return 0 };
        if hits.is_empty() {
            return 0;
        }
        let rep = f.rep.text.clone();
        let text = self.active_text().to_string();
        let mut out = String::with_capacity(text.len());
        let mut last = 0;
        for (a, b) in &hits {
            out.push_str(&text[last..*a]);
            out.push_str(&rep);
            last = *b;
        }
        out.push_str(&text[last..]);
        let e = self.cur_edit();
        e.break_history_group();
        let n = e.text.len();
        e.set_selection(0, n);
        e.insert(&out);
        e.set_selection(0, 0);
        e.break_history_group();
        self.after_edit();
        if let Some(f) = &mut self.find {
            f.cur = None;
        }
        hits.len()
    }

    pub fn toggle_find_case(&mut self, tx: &mut Tx) {
        if let Some(f) = &mut self.find {
            f.case = !f.case;
        }
        self.find_from_caret(tx);
    }
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn matches_find_plain_text() {
        assert_eq!(matches("Cat cat CAT", "cat", false), vec![(0, 3), (4, 7), (8, 11)]);
        assert_eq!(matches("Cat cat CAT", "cat", true), vec![(4, 7)]);
        assert_eq!(matches("aaaa", "aa", true), vec![(0, 2), (2, 4)], "matches don't overlap");
        assert_eq!(matches("a.b*c", ".b*", false), vec![(1, 4)], "no patterns");
        assert_eq!(matches("Straße STRASSE", "straße", false), vec![(0, 7)]);
        assert_eq!(matches("Éclair éclair", "éCLAIR", false), vec![(0, 7), (8, 15)]);
        assert!(matches("anything", "", false).is_empty());
        assert!(matches("short", "longer than it", false).is_empty());
    }
}
