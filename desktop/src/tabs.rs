//! The tab menu and what it does to tabs: close them, split the note area into two panes,
//! copy a note's path and delete a note. Note windows are in `notewin.rs`, find in `find.rs`.

use rune::Tx;

use crate::app::{App, Mode};
use crate::md;

/// The tab menu, open on the tab for `path` at a window point.
#[derive(Clone, Debug, PartialEq)]
pub struct TabMenu {
    pub path: String,
    pub x: f32,
    pub y: f32,
    /// The Copy path submenu is open.
    pub copy_open: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Right,
    Down,
}

/// Two panes under the one tab bar. The focused pane shows the active note, and this pane
/// shows `other`. A press in this pane swaps them.
#[derive(Clone, Debug, PartialEq)]
pub struct Split {
    pub dir: Dir,
    pub other: String,
    /// The other pane is the left one (Right) or the top one (Down).
    pub other_first: bool,
}

impl App {
    pub fn open_tab_menu(&mut self, path: &str, x: f32, y: f32) {
        self.tree_menu = None;
        self.tab_menu = Some(TabMenu { path: path.to_string(), x, y, copy_open: false });
    }

    /// Closes every tab but `keep`.
    pub fn close_others(&mut self, keep: &str) {
        let others: Vec<String> = self.tabs.iter().filter(|t| *t != keep).cloned().collect();
        for t in others {
            self.close_tab(&t);
        }
    }

    /// Closes the tabs to the right of `p`.
    pub fn close_tabs_after(&mut self, p: &str) {
        let Some(i) = self.tabs.iter().position(|t| t == p) else { return };
        let after: Vec<String> = self.tabs.iter().skip(i + 1).cloned().collect();
        for t in after {
            self.close_tab(&t);
        }
    }

    pub fn close_all(&mut self) {
        for t in self.tabs.clone() {
            self.close_tab(&t);
        }
    }

    /// The note's full path on disk.
    pub fn abs_path(&self, p: &str) -> Option<String> {
        self.vault.as_ref().map(|v| v.path.join(p).to_string_lossy().into_owned())
    }

    /// What Copy path copies: the full path, or a Markdown link.
    pub fn path_text(&self, p: &str, markdown: bool) -> String {
        if markdown { md_link(p) } else { self.abs_path(p).unwrap_or_default() }
    }

    pub fn copy_path(&mut self, p: &str, markdown: bool, tx: &mut Tx) {
        let text = self.path_text(p, markdown);
        if !text.is_empty() {
            tx.copy(text);
        }
    }

    /// Opens `p` in a pane of its own, focused, beside (Right) or under (Down) the note open now,
    /// which stays in the first pane. With no other note open both panes show `p`.
    pub fn split_tab(&mut self, p: &str, dir: Dir, tx: &mut Tx) {
        if !self.files().contains_key(p) {
            return;
        }
        let keep = self.active.clone().filter(|a| self.files().contains_key(a)).unwrap_or_else(|| p.to_string());
        self.split = None;
        self.focus = false;
        self.open_file(p, None, tx);
        self.split = Some(Split { dir, other: keep, other_first: true });
    }

    /// The split, while both its notes are open in tabs.
    pub fn split_view(&self) -> Option<&Split> {
        let sp = self.split.as_ref()?;
        let a = self.active.as_ref()?;
        let open = |p: &String| self.tabs.contains(p) && self.files().contains_key(p);
        (open(a) && open(&sp.other)).then_some(sp)
    }

    /// Focus moves to the other pane.
    pub fn focus_other_pane(&mut self, tx: &mut Tx) {
        let Some(sp) = self.split.clone() else { return };
        let Some(a) = self.active.clone() else { return };
        self.split = Some(Split { other: a, other_first: !sp.other_first, ..sp.clone() });
        self.active = Some(sp.other);
        self.ensure_edit();
        self.renaming = None;
        self.find = None;
        self.focus_editor(tx);
        self.persist_later();
    }

    /// Closes a pane. Its note stays open in its tab.
    pub fn close_pane(&mut self, focused: bool, tx: &mut Tx) {
        let Some(sp) = self.split.take() else { return };
        if focused {
            self.active = Some(sp.other);
            self.ensure_edit();
            self.find = None;
        }
        self.focus_editor(tx);
    }

    /// The tab menu's Delete file: the note goes to the trash, as the tree's Delete note does.
    pub fn delete_tab_note(&mut self, p: &str) {
        self.delete_path(p);
    }

    /// The tab menu's Find and Replace: the note comes to the front first.
    pub fn find_in(&mut self, p: &str, replace: bool, tx: &mut Tx) {
        if self.active.as_deref() != Some(p) {
            self.open_file(p, None, tx);
        }
        self.open_find(replace, tx);
    }

    /// Notes moved: the other pane and note windows follow them. `moved` gives a note's new
    /// path, or `None` when it did not move.
    pub(crate) fn retarget(&mut self, moved: &dyn Fn(&str) -> Option<String>) {
        if let Some(sp) = &mut self.split
            && let Some(n) = moved(&sp.other)
        {
            sp.other = n;
        }
        self.retarget_windows(moved);
    }

    /// Source mode, for find.
    pub(crate) fn show_source(&mut self) {
        if self.mode != Mode::Source {
            self.mode = Mode::Source;
            self.persist_later();
        }
    }
}

/// A Markdown link to the note, relative to the folder: `[Name](Folder/Name.md)`.
pub fn md_link(p: &str) -> String {
    let mut url = String::new();
    for c in p.chars() {
        match c {
            ' ' => url.push_str("%20"),
            '(' => url.push_str("%28"),
            ')' => url.push_str("%29"),
            '<' => url.push_str("%3C"),
            '>' => url.push_str("%3E"),
            _ => url.push(c),
        }
    }
    let name = md::base(p).replace('[', "\\[").replace(']', "\\]");
    format!("[{name}]({url})")
}

#[cfg(test)]
mod tests {
    use rune::{Env, PointerButton, Runtime};

    use super::{Dir, md_link};
    use crate::app::{App, Mode};

    type Rt = Runtime<App>;

    /// A fresh sample folder with three notes open in tabs, the first one active.
    fn start(name: &str) -> (Rt, std::path::PathBuf, Vec<String>) {
        let dir = std::env::temp_dir().join(format!("typo-tabs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // SAFETY: the ENV lock, held by each test, keeps other tests that read these out.
        unsafe {
            std::env::set_var("HOME", &dir);
            std::env::set_var("XDG_CONFIG_HOME", dir.join("cfg"));
            std::env::set_var("XDG_DATA_HOME", dir.join("data"));
        }
        let mut rt = Runtime::new(crate::app::init(), crate::ui::view, (1280.0, 800.0), Env { system_dark: false, scale: 1.0 });
        rt.frame(1.0 / 60.0);
        let mut tabs = Vec::new();
        rt.update(|s, tx| {
            let mut names: Vec<String> = s.files().keys().cloned().collect();
            names.sort();
            s.tabs.clear();
            s.active = None;
            for n in names.into_iter().take(3) {
                s.open_file(&n, None, tx);
            }
            s.open_file(&s.tabs[0].clone(), None, tx);
            s.mode = Mode::Source;
            tabs = s.tabs.clone();
        });
        frames(&mut rt);
        (rt, dir, tabs)
    }

    /// Draws until what moved has landed.
    fn frames(rt: &mut Rt) {
        rt.frame(1.0 / 60.0);
        rt.settle_all();
        for _ in 0..2 {
            rt.frame(1.0 / 60.0);
        }
    }

    fn click(rt: &mut Rt, key: &str) {
        let (x, y) = rt.find(key).unwrap_or_else(|| panic!("{key} is on screen"));
        rt.pointer_move(x, y);
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        frames(rt);
    }

    /// A right-click on the tab for `tab` opens its menu.
    fn open_menu(rt: &mut Rt, tab: &str) {
        let (x, y) = rt.find(&format!("tab:{}", tab.replace('/', "|"))).unwrap_or_else(|| panic!("the tab for {tab} shows"));
        rt.pointer_move(x, y);
        rt.button_down(x, y, PointerButton::Secondary);
        rt.button_up(x, y, PointerButton::Secondary);
        frames(rt);
        assert!(rt.find("tab-menu").is_some(), "the tab menu opens");
    }

    /// Opens the tab's menu and presses an entry in it.
    fn menu(rt: &mut Rt, tab: &str, key: &str) {
        open_menu(rt, tab);
        click(rt, key);
    }

    fn state<T>(rt: &mut Rt, f: impl FnOnce(&mut App) -> T) -> T {
        let mut out = None;
        rt.update(|s, _| out = Some(f(s)));
        out.unwrap()
    }

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        crate::scroll_tests::ENV.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn the_menu_opens_on_a_right_click_and_closes_on_escape() {
        let _env = lock();
        let (mut rt, dir, tabs) = start("open");
        open_menu(&mut rt, &tabs[1]);
        assert_eq!(state(&mut rt, |s| s.tab_menu.as_ref().map(|m| m.path.clone())), Some(tabs[1].clone()));
        for key in ["tmn-close", "tmn-others", "tmn-after", "tmn-all", "tmn-move", "tmn-split-right", "tmn-split-down", "tmn-window", "tmn-find", "tmn-replace", "tmn-copy", "tmn-delete"] {
            assert!(rt.find(key).is_some(), "{key} is in the menu");
        }
        rt.update(|s, tx| {
            s.escape(tx);
        });
        frames(&mut rt);
        assert!(rt.find("tab-menu").is_none(), "Escape closes the menu");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn close_entries_close_the_right_tabs() {
        let _env = lock();
        let (mut rt, dir, tabs) = start("close");
        menu(&mut rt, &tabs[1], "tmn-close");
        assert_eq!(state(&mut rt, |s| s.tabs.clone()), vec![tabs[0].clone(), tabs[2].clone()]);
        assert!(rt.find("tab-menu").is_none(), "the menu closes after an entry");

        rt.update(|s, tx| s.open_file(&tabs[1], None, tx));
        frames(&mut rt);
        // The tabs are now 0, 2, 1.
        menu(&mut rt, &tabs[2], "tmn-after");
        assert_eq!(state(&mut rt, |s| s.tabs.clone()), vec![tabs[0].clone(), tabs[2].clone()]);

        menu(&mut rt, &tabs[2], "tmn-others");
        assert_eq!(state(&mut rt, |s| (s.tabs.clone(), s.active.clone())), (vec![tabs[2].clone()], Some(tabs[2].clone())));

        // With one tab, Close others and Close tabs after do nothing.
        menu(&mut rt, &tabs[2], "tmn-others");
        menu(&mut rt, &tabs[2], "tmn-after");
        assert_eq!(state(&mut rt, |s| s.tabs.len()), 1);
        rt.update(|s, _| s.tab_menu = None);
        frames(&mut rt);

        rt.update(|s, tx| s.open_file(&tabs[0], None, tx));
        frames(&mut rt);
        menu(&mut rt, &tabs[0], "tmn-all");
        assert_eq!(state(&mut rt, |s| (s.tabs.len(), s.active.clone())), (0, None));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn windows_open_once_and_move_takes_the_tab() {
        let _env = lock();
        let (mut rt, dir, tabs) = start("window");
        menu(&mut rt, &tabs[1], "tmn-window");
        menu(&mut rt, &tabs[1], "tmn-window");
        let open = state(&mut rt, |s| (s.note_windows.iter().map(|w| w.path.clone()).collect::<Vec<_>>(), s.tabs.contains(&tabs[1])));
        assert_eq!(open, (vec![tabs[1].clone()], true), "one window, and the tab stays");

        menu(&mut rt, &tabs[2], "tmn-move");
        let moved = state(&mut rt, |s| (s.note_windows.len(), s.tabs.contains(&tabs[2])));
        assert_eq!(moved, (2, false), "Move to new window closes the tab");

        // An edit in a window reaches the folder; a change here goes back to the window.
        let edited = state(&mut rt, |s| {
            let key = s.note_windows[0].key.clone();
            s.window_edit(&key, &tabs[1], "# From the window".into());
            s.files()[&tabs[1]].clone()
        });
        assert_eq!(edited, "# From the window");
        let sent = state(&mut rt, |s| {
            s.set_text(&tabs[1], "# From the main window".into());
            s.poll_windows();
            s.note_windows[0].sent.clone()
        });
        assert_eq!(sent, "# From the main window");

        // A window whose note goes away closes.
        let left = state(&mut rt, |s| {
            s.delete_path(&tabs[2]);
            s.poll_windows();
            s.note_windows.iter().map(|w| w.path.clone()).collect::<Vec<_>>()
        });
        assert_eq!(left, vec![tabs[1].clone()]);

        // A note window's state shows only its note.
        let lean = state(&mut rt, |s| {
            let sat = s.satellite(&tabs[1], "note-x");
            (sat.tabs.clone(), sat.active.clone(), sat.lean.is_some())
        });
        assert_eq!(lean, (vec![tabs[1].clone()], Some(tabs[1].clone()), true));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn splits_show_two_panes_and_swap_focus() {
        let _env = lock();
        let (mut rt, dir, tabs) = start("split");
        menu(&mut rt, &tabs[1], "tmn-split-right");
        let sp = state(&mut rt, |s| (s.active.clone(), s.split.clone()));
        assert_eq!(sp.0, Some(tabs[1].clone()), "the split note has focus");
        assert_eq!(sp.1, Some(super::Split { dir: Dir::Right, other: tabs[0].clone(), other_first: true }));
        let (a, b) = (rt.find("doc-scroll").unwrap(), rt.find("doc-scroll-2").unwrap());
        assert!(b.0 < a.0 && (a.1 - b.1).abs() < 1.0, "the other pane is on the left: {a:?} {b:?}");

        // A press in the other pane focuses it.
        click(&mut rt, "doc-scroll-2");
        assert_eq!(state(&mut rt, |s| (s.active.clone(), s.split.as_ref().map(|p| p.other.clone()))), (Some(tabs[0].clone()), Some(tabs[1].clone())));

        // Opening a tab shows it in the focused pane.
        rt.update(|s, tx| s.open_file(&tabs[2], None, tx));
        frames(&mut rt);
        assert_eq!(state(&mut rt, |s| (s.active.clone(), s.split.as_ref().map(|p| p.other.clone()))), (Some(tabs[2].clone()), Some(tabs[1].clone())));

        click(&mut rt, "pane-x-a");
        assert_eq!(state(&mut rt, |s| (s.active.clone(), s.split.is_none())), (Some(tabs[1].clone()), true), "closing the focused pane keeps the other note");
        assert!(rt.find("doc-scroll-2").is_none());

        menu(&mut rt, &tabs[2], "tmn-split-down");
        let (a, b) = (rt.find("doc-scroll").unwrap(), rt.find("doc-scroll-2").unwrap());
        assert!(b.1 < a.1 && (a.0 - b.0).abs() < 1.0, "the other pane is on top: {a:?} {b:?}");
        click(&mut rt, "pane-x-b");
        assert_eq!(state(&mut rt, |s| (s.active.clone(), s.split.is_none())), (Some(tabs[2].clone()), true));

        // Closing the other note's tab ends the split.
        menu(&mut rt, &tabs[0], "tmn-split-right");
        menu(&mut rt, &tabs[2], "tmn-close");
        assert!(state(&mut rt, |s| s.split_view().is_none()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_and_replace_work_on_the_tab_note() {
        let _env = lock();
        let (mut rt, dir, tabs) = start("find");
        rt.update(|s, _| {
            s.set_text(&tabs[1], "Cat cat CAT dog".into());
            s.mode = Mode::Preview;
        });
        menu(&mut rt, &tabs[1], "tmn-find");
        let opened = state(&mut rt, |s| (s.active.clone(), s.mode == Mode::Source, s.find.as_ref().map(|f| f.replace)));
        assert_eq!(opened, (Some(tabs[1].clone()), true, Some(false)), "Find opens on the tab's note in the editor");
        assert!(rt.find("find-bar").is_some() && rt.find("find-rep").is_none());

        let found = state(&mut rt, |s| {
            let mut tx = rune::Tx::default();
            s.find_mut().q = rune::TextEdit::new("cat");
            s.find_from_caret(&mut tx);
            let first = s.cur_edit().range();
            s.find_next(false, &mut tx);
            let second = s.cur_edit().range();
            s.find_next(true, &mut tx);
            s.find_next(true, &mut tx);
            let wrapped = s.cur_edit().range();
            s.toggle_find_case(&mut tx);
            (s.find_matches().len(), first, second, wrapped)
        });
        assert_eq!(found, (1, (0, 3), (4, 7), (8, 11)));

        rt.update(|s, _| s.find.as_mut().unwrap().case = false);
        menu(&mut rt, &tabs[1], "tmn-replace");
        assert!(rt.find("find-rep").is_some(), "Replace shows the replace row");
        let out = state(&mut rt, |s| {
            let mut tx = rune::Tx::default();
            s.find_mut().rep = rune::TextEdit::new("bird");
            s.cur_edit().set_selection(0, 0);
            s.find_from_caret(&mut tx);
            s.replace_one(&mut tx);
            let one = s.active_text().to_string();
            let n = s.replace_all(&mut tx);
            (one, n, s.active_text().to_string())
        });
        assert_eq!(out, ("bird cat CAT dog".to_string(), 2, "bird bird bird dog".to_string()));
        click(&mut rt, "find-close");
        assert!(state(&mut rt, |s| s.find.is_none()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn copy_path_offers_both_forms() {
        let _env = lock();
        let (mut rt, dir, tabs) = start("copy");
        open_menu(&mut rt, &tabs[1]);
        let (x, y) = rt.find("tmn-copy").unwrap();
        rt.pointer_move(x, y);
        frames(&mut rt);
        assert!(rt.find("tmn-copy-abs").is_some(), "hovering Copy path opens its submenu");
        click(&mut rt, "tmn-copy-md");
        assert!(rt.find("tab-menu").is_none(), "the menu closes after a copy");
        let (abs, link, root) = state(&mut rt, |s| (s.path_text(&tabs[1], false), s.path_text(&tabs[1], true), s.vault.as_ref().unwrap().path.clone()));
        assert_eq!(abs, root.join(&tabs[1]).to_string_lossy());
        assert!(std::path::Path::new(&abs).is_absolute());
        assert_eq!(link, md_link(&tabs[1]));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_file_moves_the_note_to_the_trash() {
        let _env = lock();
        let (mut rt, dir, tabs) = start("delete");
        // The trash keeps the folder in the name: `Docs - Sync protocol.md`.
        let name = tabs[1].replace('/', " - ");
        menu(&mut rt, &tabs[1], "tmn-delete");
        let (gone, root) = state(&mut rt, |s| {
            s.writer.wait(std::time::Duration::from_secs(5));
            (!s.files().contains_key(&tabs[1]) && !s.tabs.contains(&tabs[1]), s.vault.as_ref().unwrap().path.clone())
        });
        assert!(gone, "the note and its tab are gone");
        assert!(!root.join(&tabs[1]).exists());
        assert!(root.join(".trash").join(name).exists(), "the file is in .trash");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn markdown_links_escape_what_breaks_a_link() {
        assert_eq!(md_link("Ideas.md"), "[Ideas](Ideas.md)");
        assert_eq!(md_link("Journal/On slow software.md"), "[On slow software](Journal/On%20slow%20software.md)");
        assert_eq!(md_link("a (b) [c].md"), "[a (b) \\[c\\]](a%20%28b%29%20[c].md)");
    }
}
