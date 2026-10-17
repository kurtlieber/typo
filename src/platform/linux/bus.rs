//! Messages between typo's windows. Each window is its own process, so File ▸
//! Quit (or Save All) broadcasts a signal on the session bus and every window
//! that shares this one's settings closes itself, asking about unsaved changes
//! as it goes (or saves).
//! Sharing settings is what makes windows one app: a test run with its own
//! settings directory never closes the user's windows.

use std::path::PathBuf;

use zbus::blocking::{Connection, MessageIterator};
use zbus::{MatchRule, message};

const PATH: &str = "/io/github/kurtlieber/Typo";
const IFACE: &str = "io.github.kurtlieber.Typo";

/// The settings directory, which names the group of windows.
fn group() -> String {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Show `path` selected in the file manager, through the freedesktop
/// FileManager1 interface Nautilus and others offer.
pub fn show_item(path: &std::path::Path) -> Result<(), String> {
    let conn = Connection::session().map_err(|e| e.to_string())?;
    let uri = crate::image::file_url(path);
    conn.call_method(
        Some("org.freedesktop.FileManager1"),
        "/org/freedesktop/FileManager1",
        Some("org.freedesktop.FileManager1"),
        "ShowItems",
        &(vec![uri], ""),
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Ask every window in the group, this one included, to close ("Quit") or
/// save ("Save").
pub fn broadcast(what: &str) -> Result<(), String> {
    let conn = Connection::session().map_err(|e| e.to_string())?;
    conn.emit_signal(None::<&str>, PATH, IFACE, what, &group()).map_err(|e| e.to_string())
}

/// Call `heard` on a background thread with each request made of the group:
/// "Quit" or "Save".
pub fn on_broadcast(heard: impl Fn(&str) + Send + 'static) {
    std::thread::spawn(move || {
        let Ok(conn) = Connection::session() else { return };
        let Ok(rule) = MatchRule::builder()
            .msg_type(message::Type::Signal)
            .interface(IFACE)
            .and_then(|b| b.path(PATH))
            .map(|b| b.build())
        else {
            return;
        };
        let Ok(messages) = MessageIterator::for_match_rule(rule, &conn, None) else { return };
        let mine = group();
        for msg in messages.flatten() {
            let header = msg.header();
            if let Some(what) = header.member()
                && msg.body().deserialize::<String>().is_ok_and(|g| g == mine)
            {
                heard(what.as_str());
            }
        }
    });
}
