//! The windows open when typo last closed, which Preferences ▸ Files ▸ On
//! Launch can bring back. Each window is a process of its own, so each
//! keeps a file of its own, named for its process id, holding its document
//! and its folder; no two processes ever write the same file. A window
//! that closes while others stay open removes its file, so what is left
//! when the last one goes (or when Quit closes them all) is the session.
//! No Qt.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::file;

/// A window to bring back: its document and its folder, either empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub path: String,
    pub folder: String,
}

/// Where the windows' files are.
pub fn dir() -> PathBuf {
    file::config_dir().join("session")
}

/// Note this window's document and folder.
pub fn record(dir: &Path, path: &str, folder: &str) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    file::write_atomic(&dir.join(std::process::id().to_string()), format!("{path}\n{folder}\n").as_bytes())
}

/// Forget this window, as it closes while others stay.
pub fn forget(dir: &Path) {
    let _ = fs::remove_file(dir.join(std::process::id().to_string()));
}

/// The ids of the processes with a file in `dir`, oldest file first.
fn entries(dir: &Path) -> Vec<(u32, PathBuf)> {
    let Ok(read) = fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<(std::time::SystemTime, u32, PathBuf)> = read
        .flatten()
        .filter_map(|e| {
            let pid = e.file_name().to_str()?.parse().ok()?;
            let time = e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
            Some((time, pid, e.path()))
        })
        .collect();
    out.sort();
    out.into_iter().map(|(_, pid, path)| (pid, path)).collect()
}

/// Whether process `pid` is a running typo: the same program as this one,
/// so a reused id isn't taken for a window.
pub fn alive(pid: u32) -> bool {
    let name = |p: &str| fs::read_to_string(format!("/proc/{p}/comm")).ok();
    name(&pid.to_string()).is_some_and(|n| Some(n) == name("self"))
}

/// Whether any window but this one is open.
pub fn others_open(dir: &Path, alive: impl Fn(u32) -> bool) -> bool {
    let me = std::process::id();
    entries(dir).iter().any(|(pid, _)| *pid != me && alive(*pid))
}

/// The windows of the last session, oldest first, forgetting them: the
/// files of processes no longer running. A document that is gone is left
/// out, and a folder that is gone too; a window left with neither is
/// dropped.
pub fn take(dir: &Path, alive: impl Fn(u32) -> bool) -> Vec<Window> {
    let me = std::process::id();
    let mut out = Vec::new();
    for (pid, path) in entries(dir) {
        if pid == me || alive(pid) {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_default();
        let _ = fs::remove_file(&path);
        let mut lines = text.lines();
        let mut window = Window { path: lines.next().unwrap_or_default().into(), folder: lines.next().unwrap_or_default().into() };
        if !Path::new(&window.path).is_file() {
            window.path.clear();
        }
        if !Path::new(&window.folder).is_dir() {
            window.folder.clear();
        }
        if !window.path.is_empty() || !window.folder.is_empty() {
            out.push(window);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("typo-session-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn records_and_takes_the_last_session() {
        let d = scratch("take");
        let doc = d.join("a.md");
        fs::write(&doc, "# A\n").unwrap();
        let sess = d.join("session");
        // Two windows of an earlier run (ids 1 and 2, not running) and
        // this one.
        fs::create_dir_all(&sess).unwrap();
        fs::write(sess.join("1"), format!("{}\n{}\n", doc.display(), d.display())).unwrap();
        fs::write(sess.join("2"), format!("{}\n\n", d.join("gone.md").display())).unwrap();
        record(&sess, "", "").unwrap();
        assert!(!others_open(&sess, |_| false));
        assert!(others_open(&sess, |pid| pid == 1));
        let got = take(&sess, |_| false);
        assert_eq!(got, vec![Window { path: doc.display().to_string(), folder: d.display().to_string() }]);
        // Taken once; this window's own file stays.
        assert!(take(&sess, |_| false).is_empty());
        assert!(sess.join(std::process::id().to_string()).exists());
        forget(&sess);
        assert!(!sess.join(std::process::id().to_string()).exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn running_windows_are_not_taken() {
        let d = scratch("running");
        fs::write(d.join("7"), format!("\n{}\n", d.display())).unwrap();
        assert!(take(&d, |pid| pid == 7).is_empty());
        assert_eq!(take(&d, |_| false).len(), 1);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn this_process_is_alive() {
        assert!(alive(std::process::id()));
        assert!(!alive(u32::MAX));
    }
}
