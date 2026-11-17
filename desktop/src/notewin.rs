//! Note windows: one note in a window of its own, with its title, editor and the edit and
//! preview toggle. The main window owns the folder and saves; a note window sends each edit to
//! it, and the main window sends back changes made anywhere else. Note windows are not
//! restored when Typo starts again.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

use rune::{AppHandle, AppSpec, Tx, WindowSpec};

use crate::app::{App, Screen};
use crate::md;
use crate::store::Settings;

thread_local! {
    /// The main window, so a note window can reach it.
    static MAIN: RefCell<Option<AppHandle<App>>> = const { RefCell::new(None) };
}

static NEXT: AtomicU64 = AtomicU64::new(1);

pub fn set_main(h: AppHandle<App>) {
    MAIN.with(|m| *m.borrow_mut() = Some(h));
}

fn main_handle() -> Option<AppHandle<App>> {
    MAIN.with(|m| m.borrow().clone())
}

/// A note window, as the main window keeps track of it.
pub struct NoteWindow {
    pub key: String,
    pub path: String,
    /// None in a headless run, which opens no windows.
    pub handle: Option<AppHandle<App>>,
    /// The note's text as the window has it.
    pub sent: String,
    /// The settings the window has.
    pub settings: Settings,
}

/// What a note window's state knows of the main window.
pub struct Lean {
    pub key: String,
    pub main: Option<AppHandle<App>>,
    pub titled: bool,
}

impl App {
    /// Opens the note in a window of its own, or brings its window to the front. With `move_tab`
    /// its tab here closes.
    pub fn open_note_window(&mut self, p: &str, move_tab: bool) {
        if self.lean.is_some() || !self.files().contains_key(p) {
            return;
        }
        if self.note_locked(p) || self.note_unlocked(p) {
            self.notice = Some("A locked note opens only in the main window.".into());
            return;
        }
        if let Some(w) = self.note_windows.iter().find(|w| w.path == p) {
            if let Some(h) = &w.handle {
                h.focus();
            }
        } else {
            let key = format!("note-{}", NEXT.fetch_add(1, Ordering::Relaxed));
            let sat = self.satellite(p, &key);
            let handle = if cfg!(test) || rune::is_headless() {
                None
            } else {
                let app = note_app(sat);
                let h = app.handle();
                rune::open_window(WindowSpec::from_app(key.clone(), app));
                Some(h)
            };
            let sent = self.files().get(p).cloned().unwrap_or_default();
            self.note_windows.push(NoteWindow { key, path: p.to_string(), handle, sent, settings: self.persist.settings.clone() });
        }
        if move_tab {
            self.close_tab(p);
        }
    }

    /// The state a note window starts from.
    pub(crate) fn satellite(&self, p: &str, key: &str) -> App {
        let mut s = App::blank(self.persist.clone(), self.writer.clone());
        s.screen = Screen::Vault;
        s.vault = self.vault.clone();
        s.names = self.names.clone();
        s.tabs = vec![p.to_string()];
        s.active = Some(p.to_string());
        s.mode = self.mode;
        s.lean = Some(Lean { key: key.to_string(), main: main_handle(), titled: false });
        s.ensure_edit();
        s
    }

    /// A note window: an edit goes to the main window, which saves it.
    pub(crate) fn send_to_main(&mut self, rel: &str, text: String) {
        let Some(l) = &self.lean else { return };
        let Some(m) = &l.main else { return };
        let (key, rel) = (l.key.clone(), rel.to_string());
        m.update(move |m, _| m.window_edit(&key, &rel, text));
    }

    /// A note window: a link opens in the main window.
    pub(crate) fn open_link_in_main(&mut self, target: String) {
        let Some(m) = self.lean.as_ref().and_then(|l| l.main.clone()) else { return };
        m.update(move |m, tx| m.open_link(&target, tx));
        m.focus();
    }

    /// A note window's poll: its title, and images as they load.
    pub(crate) fn poll_lean(&mut self, tx: &mut Tx) -> bool {
        let title = self.active.as_deref().map(md::base).unwrap_or_default().to_string();
        if let Some(l) = &mut self.lean
            && !l.titled
        {
            tx.set_title(if title.is_empty() { "Typo".to_string() } else { title });
            l.titled = true;
        }
        self.images.poll()
    }

    /// A note window closed.
    pub(crate) fn lean_closed(&mut self) {
        let Some(l) = &self.lean else { return };
        let Some(m) = &l.main else { return };
        let key = l.key.clone();
        m.update(move |m, _| m.note_windows.retain(|w| w.key != key));
    }

    /// A note window: the main window has newer text for the note.
    pub(crate) fn take_text(&mut self, rel: &str, text: String) {
        if let Some(e) = self.edits.get_mut(rel)
            && e.text != text
        {
            let (a, b) = (e.anchor.min(text.len()), e.focus.min(text.len()));
            e.replace_text(text.clone());
            e.set_selection(a, b);
        }
        if let Some(v) = &mut self.vault {
            v.files.insert(rel.to_string(), text);
        }
    }

    /// A note window: its note moved.
    pub(crate) fn lean_rename(&mut self, old: &str, new: &str, tx: &mut Tx) {
        if let Some(v) = &mut self.vault
            && let Some(t) = v.files.remove(old)
        {
            v.files.insert(new.to_string(), t);
        }
        if let Some(e) = self.edits.remove(old) {
            self.edits.insert(new.to_string(), e);
        }
        for t in &mut self.tabs {
            if t == old {
                *t = new.to_string();
            }
        }
        if self.active.as_deref() == Some(old) {
            self.active = Some(new.to_string());
        }
        tx.set_title(md::base(new).to_string());
    }

    /// The main window: a note window's edit.
    pub(crate) fn window_edit(&mut self, key: &str, rel: &str, text: String) {
        if let Some(w) = self.note_windows.iter_mut().find(|w| w.key == key) {
            w.sent = text.clone();
        }
        if self.files().get(rel) != Some(&text) {
            self.set_text(rel, text);
        }
    }

    /// The main window: send each note window what changed here. A window whose note is gone
    /// closes. Returns whether the list of windows changed.
    pub(crate) fn poll_windows(&mut self) -> bool {
        let mut gone = Vec::new();
        for w in &mut self.note_windows {
            let Some(text) = self.vault.as_ref().and_then(|v| v.files.get(&w.path)) else {
                gone.push(w.key.clone());
                continue;
            };
            if *text != w.sent {
                w.sent = text.clone();
                if let Some(h) = &w.handle {
                    let (p, t) = (w.path.clone(), text.clone());
                    h.update(move |s, _| s.take_text(&p, t));
                }
            }
            if self.persist.settings != w.settings {
                w.settings = self.persist.settings.clone();
                if let Some(h) = &w.handle {
                    let st = w.settings.clone();
                    h.update(move |s, _| s.persist.settings = st);
                }
            }
        }
        for k in &gone {
            if let Some(w) = self.note_windows.iter().find(|w| &w.key == k)
                && let Some(h) = &w.handle
            {
                h.close();
            }
        }
        self.note_windows.retain(|w| !gone.contains(&w.key));
        !gone.is_empty()
    }

    /// The main window: notes moved, and their windows follow.
    pub(crate) fn retarget_windows(&mut self, moved: &dyn Fn(&str) -> Option<String>) {
        for w in &mut self.note_windows {
            let Some(n) = moved(&w.path) else { continue };
            let old = std::mem::replace(&mut w.path, n.clone());
            if let Some(h) = &w.handle {
                h.update(move |s, tx| s.lean_rename(&old, &n, tx));
            }
        }
    }

    /// The main window closes, and its note windows with it.
    pub fn close_note_windows(&mut self) {
        for w in self.note_windows.drain(..) {
            if let Some(h) = w.handle {
                h.close();
            }
        }
    }
}

fn note_app(sat: App) -> rune::App<App> {
    let spec = AppSpec {
        min_size: Some((360.0, 300.0)),
        fonts: crate::theme::FONTS,
        families: ("Instrument Sans", "IBM Plex Mono"),
        on_key: Some(crate::keys::on_key),
        poll: Some(|s: &mut App, t, tx| s.poll(t, tx)),
        setup: Some(|s: &mut App, waker| s.set_waker(waker)),
        ..AppSpec::new("Typo", (760.0, 860.0), crate::app::init, crate::ui::view)
    };
    rune::App::new(spec)
        .init(move || sat)
        .font_smoothing(crate::smoothing())
        .chrome(rune::Chrome { full_size_content: true, traffic_lights: Some((14.0, 17.0)), drag_regions: true })
        .titlebar(rune::Titlebar::new(42.0))
        .on_close(|s: &mut App, _tx| s.lean_closed())
}
