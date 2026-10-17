//! The windows open when typo last closed, which Preferences ▸ Files ▸ On
//! Launch can bring back. Each window keeps a file of its own, holding its
//! document and its folder, named for its process: `<pid>` for the
//! process's first window, `<pid>.<n>` for the Mac's later ones (one
//! process has them all there). No two windows write the same file. A
//! window that closes while others stay open removes its file, so what is
//! left when the last one goes (or when Quit closes them all) is the
//! session.
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

/// The file name of this process's window `n`.
pub fn key(n: u32) -> String {
    let pid = std::process::id();
    if n == 0 { pid.to_string() } else { format!("{pid}.{n}") }
}

/// Note window `key`'s document and folder.
pub fn record(dir: &Path, key: &str, path: &str, folder: &str) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    file::write_atomic(&dir.join(key), format!("{path}\n{folder}\n").as_bytes())
}

/// Forget window `key`, as it closes while others stay.
pub fn forget(dir: &Path, key: &str) {
    let _ = fs::remove_file(dir.join(key));
}

/// The ids of the processes with a file in `dir` and the files, oldest
/// first; a process with several windows comes up once for each.
pub fn entries(dir: &Path) -> Vec<(u32, PathBuf)> {
    let Ok(read) = fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<(std::time::SystemTime, u32, PathBuf)> = read
        .flatten()
        .filter_map(|e| {
            let pid = e.file_name().to_str()?.split('.').next()?.parse().ok()?;
            let time = e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
            Some((time, pid, e.path()))
        })
        .collect();
    out.sort();
    out.into_iter().map(|(_, pid, path)| (pid, path)).collect()
}

/// Whether any window but `key` is open.
pub fn others_open(dir: &Path, key: &str, alive: impl Fn(u32) -> bool) -> bool {
    let me = std::process::id();
    entries(dir).iter().any(|(pid, path)| path.file_name().is_some_and(|f| f != key) && (*pid == me || alive(*pid)))
}

/// The process after `me` among the live windows in `dir`, oldest first and
/// round again: Switch Between Opened Documents. None when `me` is alone.
pub fn next_process(dir: &Path, me: u32, alive: impl Fn(u32) -> bool) -> Option<u32> {
    let pids: Vec<u32> = entries(dir).into_iter().map(|(pid, _)| pid).filter(|&pid| pid == me || alive(pid)).collect();
    let i = pids.iter().position(|&pid| pid == me)?;
    Some(pids[(i + 1) % pids.len()]).filter(|&pid| pid != me)
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
        record(&sess, &key(0), "", "").unwrap();
        assert!(!others_open(&sess, &key(0), |_| false));
        assert!(others_open(&sess, &key(0), |pid| pid == 1));
        let got = take(&sess, |_| false);
        assert_eq!(got, vec![Window { path: doc.display().to_string(), folder: d.display().to_string() }]);
        // Taken once; this window's own file stays.
        assert!(take(&sess, |_| false).is_empty());
        assert!(sess.join(std::process::id().to_string()).exists());
        forget(&sess, &key(0));
        assert!(!sess.join(std::process::id().to_string()).exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn a_process_keeps_a_file_per_window() {
        let d = scratch("windows");
        record(&d, &key(0), "", &d.display().to_string()).unwrap();
        record(&d, &key(3), "", "").unwrap();
        assert!(d.join(format!("{}.3", std::process::id())).exists());
        assert!(others_open(&d, &key(0), |_| false) && others_open(&d, &key(3), |_| false));
        // Its own windows are never the last session.
        assert!(take(&d, |_| false).is_empty());
        forget(&d, &key(3));
        assert!(!others_open(&d, &key(0), |_| false));
        // An earlier run's later window comes back.
        fs::write(d.join("1.2"), format!("\n{}\n", d.display())).unwrap();
        assert_eq!(take(&d, |_| false).len(), 1);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn switches_to_the_next_live_window() {
        let d = scratch("next");
        for pid in [7, 8, 9] {
            fs::write(d.join(pid.to_string()), "\n\n").unwrap();
        }
        assert_eq!(next_process(&d, 8, |_| true), Some(9));
        assert_eq!(next_process(&d, 9, |_| true), Some(7));
        assert_eq!(next_process(&d, 8, |pid| pid != 9), Some(7));
        assert_eq!(next_process(&d, 8, |_| false), None);
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
}
