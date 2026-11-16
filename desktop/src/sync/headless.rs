//! `typo sync`: one sync of a notes folder with no window, for a host without a desktop. A
//! timer runs it. It reads the sync settings from Typo's state file, as the app does, and lays
//! the outcome on the disk with the app's rules: a file that changed while the sync ran keeps
//! its text and the download goes beside it as a conflict copy, and a delete from another device
//! moves only a file that did not change to `.trash`.
//!
//! Don't run it on a folder that Typo has open on the same host: the two would share one
//! last-sync record.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::{Outcome, Remote};
use crate::vault::{Op, apply};

/// Runs `typo sync [--folder <path>]` and returns the exit code.
pub fn main(args: &[String]) -> i32 {
    let mut folder = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match (a.as_str(), it.next()) {
            ("--folder", Some(f)) => folder = Some(PathBuf::from(f)),
            _ => return usage(),
        }
    }
    let folder = folder.unwrap_or_else(crate::store::documents_dir);
    let Ok(root) = folder.canonicalize().map_err(|_| ()).and_then(|p| if p.is_dir() { Ok(p) } else { Err(()) }) else {
        eprintln!("typo sync: {} is not a folder", folder.display());
        return 2;
    };
    let settings = crate::store::load().settings;
    let Some(r) = super::remote_for(&settings.sync, &root) else {
        eprintln!("typo sync: sync is off. Set it up in Typo's settings, or in the state file.");
        return 2;
    };
    match sync(&r, &root, &super::device_name()) {
        Ok(msg) => {
            println!("{msg}");
            0
        }
        Err(e) => {
            eprintln!("typo sync: {e}");
            1
        }
    }
}

fn usage() -> i32 {
    eprintln!("Usage: typo sync [--folder <path>]   (default: the Documents folder)");
    2
}

/// One sync, applied to the disk. A sync that made conflict copies runs again at once, so every
/// device gets both versions.
pub fn sync(r: &Remote, root: &Path, device: &str) -> Result<String, String> {
    let mut msg = String::new();
    for round in 0..2 {
        let out = super::run(r, root, device, &super::base_pins(r, root))?;
        let place = out.place.clone();
        let (down, up) = (out.pulls.len() + out.image_pulls.len(), out.uploaded);
        let conflicts = apply_outcome(root, out)?;
        if round == 0 {
            msg = format!("Synced {} with {place}: {down} down, {up} up.", crate::store::tilde(root));
        }
        if conflicts.is_empty() {
            break;
        }
        msg.push_str(&format!(" Kept both versions of: {}.", conflicts.join(", ")));
    }
    Ok(msg)
}

fn note_hash(p: &Path) -> Option<String> {
    std::fs::read_to_string(p).ok().map(|t| super::sign::sha256_hex(t.replace("\r\n", "\n").as_bytes()))
}

fn file_hash(p: &Path) -> Option<String> {
    std::fs::read(p).ok().map(|b| super::sign::sha256_hex(&b))
}

/// Lays a sync's outcome on the folder at `root` and saves the last-sync record, or deletes it if
/// a write failed. Returns the conflict copies it made.
pub fn apply_outcome(root: &Path, mut out: Outcome) -> Result<Vec<String>, String> {
    let base = out.base.take();
    let mut errors: Vec<String> = Vec::new();
    let mut run = |op: Op| {
        let what = match &op {
            Op::Write { rel, .. } | Op::WriteBytes { rel, .. } | Op::Delete { rel, .. } | Op::DeleteFolder { rel, .. } | Op::Mkdir { rel, .. } => rel.clone(),
            _ => String::new(),
        };
        if let Err(e) = apply(&op) {
            errors.push(format!("could not save {what}: {e}"));
        }
    };
    let mut conflicts = out.conflicts;
    let mut notes: Option<BTreeSet<String>> = None;
    for pl in out.pulls {
        let cur = note_hash(&root.join(&pl.path));
        if cur.as_deref() == Some(super::sign::sha256_hex(pl.text.as_bytes()).as_str()) {
            continue;
        }
        let rel = if cur != pl.seen {
            let taken = notes.get_or_insert_with(|| crate::vault::load(root).map(|v| v.files.into_keys().collect()).unwrap_or_default());
            let c = super::conflict_path(&pl.path, &pl.device, taken);
            taken.insert(c.clone());
            conflicts.push(c.clone());
            c
        } else {
            pl.path
        };
        run(Op::Write { root: root.to_path_buf(), rel, text: pl.text });
    }
    for (p, seen) in out.removes {
        if note_hash(&root.join(&p)).as_deref() == Some(seen.as_str()) {
            run(Op::Delete { root: root.to_path_buf(), rel: p });
        }
    }
    for (path, bytes, seen, device) in out.image_pulls {
        let cur = file_hash(&root.join(&path));
        let rel = if cur.is_some() && cur != seen {
            let taken: BTreeSet<String> = crate::images::list(root).into_iter().collect();
            let c = super::conflict_path(&path, &device, &taken);
            conflicts.push(c.clone());
            c
        } else {
            path
        };
        run(Op::WriteBytes { root: root.to_path_buf(), rel, bytes });
    }
    for (path, seen) in out.image_removes {
        if file_hash(&root.join(&path)).as_deref() == Some(seen.as_str()) {
            run(Op::Delete { root: root.to_path_buf(), rel: path });
        }
    }
    for f in out.mkdirs {
        run(Op::Mkdir { root: root.to_path_buf(), rel: f });
    }
    // A folder keeps a note that came in or stayed here. The next sync puts the folder back.
    for f in out.rmdirs {
        let empty = crate::vault::load(&root.join(&f)).is_ok_and(|v| v.files.is_empty());
        if empty {
            run(Op::DeleteFolder { root: root.to_path_buf(), rel: f });
        }
    }
    if let Some((path, bytes)) = base {
        let r = if errors.is_empty() {
            crate::vault::write_atomic(&path, &bytes)
        } else {
            std::fs::remove_file(&path).or_else(|e| if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e) })
        };
        if let Err(e) = r {
            errors.push(format!("could not save the sync record: {e}"));
        }
    }
    if errors.is_empty() { Ok(conflicts) } else { Err(errors.join("; ")) }
}
