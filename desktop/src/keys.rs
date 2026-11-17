//! Shortcut labels and the window's shortcuts.

use rune::{Key, KeyEvent, Tx};

use crate::app::{App, Screen};

pub fn is_mac() -> bool {
    cfg!(target_os = "macos")
}

/// A shortcut label as the platform writes it: `⌘⇧F` on macOS, `Ctrl+Shift+F` elsewhere.
pub fn k(keys: &[&str]) -> String {
    let mac = is_mac();
    let parts: Vec<String> = keys
        .iter()
        .map(|x| match (*x, mac) {
            ("Mod", true) => "⌘".into(),
            ("Shift", true) => "⇧".into(),
            ("Alt", true) => "⌥".into(),
            ("Mod", false) => "Ctrl".into(),
            (s, _) if s.chars().count() == 1 => s.to_uppercase(),
            (s, _) => s.into(),
        })
        .collect();
    if mac { parts.concat() } else { parts.join("+") }
}

/// The Replace shortcut's label.
pub fn replace_keys() -> String {
    if is_mac() { k(&["Mod", "Alt", "F"]) } else { k(&["Mod", "H"]) }
}

pub fn mod_click() -> String {
    format!("{}-click", if is_mac() { "⌘" } else { "Ctrl" })
}

/// The window's shortcuts (`AppSpec::on_key`). Keys reach this when the focused control
/// does not take them.
pub fn on_key(s: &mut App, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if ev.key == Key::Escape {
        return s.escape(tx);
    }
    // macOS quits from its app menu.
    if !cfg!(target_os = "macos") && ev.mods.cmd && !ev.mods.shift && ev.key == Key::Command('q') {
        s.quit(tx);
        return true;
    }
    if !ev.mods.cmd || s.screen != Screen::Vault {
        return false;
    }
    let shift = ev.mods.shift;
    let Key::Command(c) = ev.key else { return false };
    // A note window has one note: no palette, search, settings or new notes.
    if s.lean.is_some() && !matches!((c, shift), ('f' | 'ƒ' | 'h' | 'b' | 'i' | 'k' | 'e', false)) {
        return false;
    }
    match (c, shift) {
        // Replace is ⌥⌘F on macOS, where ⌘H hides the app.
        ('f' | 'ƒ', false) if ev.mods.alt => s.open_find(true, tx),
        ('f', false) => s.open_find(false, tx),
        ('h', false) if !is_mac() => s.open_find(true, tx),
        ('f', true) => s.open_search(tx),
        ('b', false) => s.format(crate::format::Fmt::Bold, tx),
        ('i', false) => s.format(crate::format::Fmt::Italic, tx),
        ('k', false) => s.format(crate::format::Fmt::Link, tx),
        ('i', true) => s.pick_image(tx),
        ('o', false) => s.open_palette(crate::app::PalMode::Files, tx),
        ('p', false) => s.open_palette(crate::app::PalMode::Commands, tx),
        ('e', false) => s.toggle_mode(tx),
        (',', false) => s.toggle_settings(tx),
        ('.', false) => s.toggle_focus(tx),
        ('\\', false) => s.toggle_left(),
        ('n', false) => s.new_note(None, tx),
        _ => return false,
    }
    true
}
