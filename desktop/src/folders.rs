//! Folders: new notes inside a folder, new folders, moving notes by drag, and the tree's menu.

use rune::{Drag, Phase, TextEdit, Tx};

use crate::app::{App, Sel};
use crate::md;
use crate::vault::Op;

/// A note being dragged in the file tree.
#[derive(Clone, Debug)]
pub struct DragNote {
    pub path: String,
    /// The pointer, in window points.
    pub x: f32,
    pub y: f32,
    /// Past the threshold: a drag, not a click.
    pub active: bool,
    /// The folder under the pointer (`""` is the root), if the note can go there.
    pub target: Option<String>,
    /// The pointer is over the Pinned group, and the note is not pinned yet.
    pub pin: bool,
}

/// What the tree's menu was opened on.
#[derive(Clone, Debug, PartialEq)]
pub enum MenuOn {
    Folder(String),
    Note(String),
    Root,
}

#[derive(Clone, Debug)]
pub struct TreeMenu {
    pub x: f32,
    pub y: f32,
    pub on: MenuOn,
    /// The menu asks before it deletes the folder it was opened on.
    pub confirm: bool,
}

/// A folder being named in the tree: a new one inside `parent`, or the folder `rename`, whose
/// row the field takes the place of.
pub struct NewFolder {
    pub parent: String,
    pub edit: TextEdit,
    pub rename: Option<String>,
}

/// A tree row key holds its path with `|` for `/`, since `/` separates keys in element paths.
pub fn row_key(kind: &str, path: &str) -> String {
    format!("{kind}:{}", path.replace('/', "|"))
}

/// The folder an element path points into: a folder row is that folder, a note row is its
/// note's folder, and the tree's empty space or the folder name is the root.
pub fn folder_at(path: &str) -> Option<String> {
    for seg in path.split('/').rev() {
        if let Some(p) = seg.strip_prefix("dir:") {
            return Some(p.replace('|', "/"));
        }
        if let Some(p) = seg.strip_prefix("file:") {
            return Some(md::dir(&p.replace('|', "/")).to_string());
        }
    }
    path.split('/').any(|s| matches!(s, "sidebar" | "side-scroll" | "vault-head")).then(String::new)
}

/// Splits `Drafts/Ideas/Name` into a clean folder path and a clean note name.
fn split_path(name: &str) -> (String, String) {
    let parts: Vec<String> = name.split('/').map(md::clean_name).filter(|p| !p.is_empty() && p != "." && p != "..").collect();
    match parts.split_last() {
        Some((last, dirs)) => (dirs.join("/"), last.clone()),
        None => (String::new(), String::new()),
    }
}

impl App {
    /// A new note in `folder` (`None`: the folder from settings). A `name` with slashes, such as
    /// `Drafts/Idea`, makes the note in that folder, creating any folders it names.
    pub fn new_note_in(&mut self, folder: Option<&str>, name: Option<&str>, tx: &mut Tx) {
        if self.vault.is_none() {
            return;
        }
        let mut base_folder = match folder {
            Some(f) => f.to_string(),
            None => {
                let f = self.settings().new_folder.clone();
                if !f.is_empty() && !self.folder_list().contains(&f) { String::new() } else { f }
            }
        };
        let (sub, bn) = match name {
            Some(n) if n.contains('/') => {
                let (d, b) = split_path(n);
                // A path from the root wins over the settings folder.
                base_folder = String::new();
                (d, b)
            }
            Some(n) => (String::new(), md::clean_name(n)),
            None => (String::new(), String::new()),
        };
        let folder = if sub.is_empty() { base_folder } else { md::join(&base_folder, &sub) };
        let bn = if bn.is_empty() { "Untitled".to_string() } else { bn };
        let p = self.unique_path(&folder, &bn);
        let content = if name.is_some() { format!("# {bn}\n\n") } else { String::new() };
        if let Some(v) = &mut self.vault {
            v.files.insert(p.clone(), content.clone());
            v.born.insert(p.clone(), crate::app::now_ns());
            let mut d = folder.clone();
            while !d.is_empty() {
                if !v.folders.contains(&d) {
                    v.folders.push(d.clone());
                }
                d = md::dir(&d).to_string();
            }
            v.folders.sort();
        }
        self.rebuild_names_pub();
        self.mark_dirty_pub(&p);
        self.screen = crate::app::Screen::Vault;
        self.settings_open = false;
        self.tree_menu = None;
        let end = content.len();
        self.open_file(&p, Some(Sel { start: end, end, line: None, force_source: false }), tx);
        if name.is_none() {
            self.mode = crate::app::Mode::Source;
            self.focus = false;
            self.start_rename(tx);
        }
    }

    /// Starts naming a new folder inside `parent` (`""` is the root).
    pub fn start_new_folder(&mut self, parent: &str, tx: &mut Tx) {
        let mut edit = TextEdit::new("");
        edit.focused = true;
        if !parent.is_empty() {
            self.expanded.insert(parent.to_string());
        }
        self.persist.left_open = true;
        self.left_panel = crate::app::Panel::Files;
        self.new_folder = Some(NewFolder { parent: parent.to_string(), edit, rename: None });
        self.tree_menu = None;
        tx.focus("new-folder", false);
    }

    /// Starts renaming `folder` in its own row, with its name selected.
    pub fn start_rename_folder(&mut self, folder: &str, tx: &mut Tx) {
        let name = folder.rsplit('/').next().unwrap_or(folder);
        let mut edit = TextEdit::new(name);
        edit.set_selection(0, name.len());
        edit.focused = true;
        self.persist.left_open = true;
        self.left_panel = crate::app::Panel::Files;
        self.new_folder = Some(NewFolder { parent: md::dir(folder).to_string(), edit, rename: Some(folder.to_string()) });
        self.tree_menu = None;
        tx.focus("new-folder", false);
    }

    pub fn commit_new_folder(&mut self) {
        let Some(nf) = self.new_folder.take() else { return };
        let name = md::clean_name(&nf.edit.text);
        if name.is_empty() || name.starts_with('.') {
            return;
        }
        if let Some(old) = nf.rename {
            self.rename_folder(&old, &name);
            return;
        }
        let path = md::join(&nf.parent, &name);
        let Some(v) = &mut self.vault else { return };
        if v.folders.contains(&path) {
            self.notice = Some(format!("A folder named {name} is already there."));
            return;
        }
        v.folders.push(path.clone());
        v.folders.sort();
        let root = v.path.clone();
        self.writer.send(Op::Mkdir { root, rel: path.clone() });
        self.expanded.insert(path);
        if !nf.parent.is_empty() {
            self.expanded.insert(nf.parent);
        }
        self.persist_later();
        self.sync_after_save();
    }

    /// Renames `folder` to `name` in the same parent, with every note and file in it. Open tabs,
    /// pins, carets and history follow, and so do links that spell out the old folder.
    pub fn rename_folder(&mut self, folder: &str, name: &str) {
        let to = md::join(md::dir(folder), name);
        if folder.is_empty() || to == folder {
            return;
        }
        let Some(v) = &self.vault else { return };
        // A change of case only is the same folder on a disk that ignores case.
        let taken = v.folders.contains(&to) || (v.path.join(&to).exists() && to.to_lowercase() != folder.to_lowercase());
        if taken {
            self.notice = Some(format!("A folder named {name} is already there."));
            return;
        }
        // Pending writes go first, so the files to move are on disk.
        self.flush_notes();
        let pre = format!("{folder}/");
        let moved = |p: &str| -> Option<String> {
            if p == folder {
                Some(to.clone())
            } else {
                p.strip_prefix(&pre).map(|rest| format!("{to}/{rest}"))
            }
        };
        let update = self.settings().update_links;
        let Some(v) = &mut self.vault else { return };
        let root = v.path.clone();
        let notes: Vec<(String, String)> = v.files.keys().filter_map(|p| moved(p).map(|n| (p.clone(), n))).collect();
        for (from, n) in &notes {
            if let Some(t) = v.files.remove(from) {
                v.files.insert(n.clone(), t);
            }
            if let Some(b) = v.born.remove(from) {
                v.born.insert(n.clone(), b);
            }
        }
        for f in v.folders.iter_mut() {
            if let Some(n) = moved(f) {
                *f = n;
            }
        }
        v.folders.sort();
        self.writer.send(Op::Rename { root, from: folder.to_string(), to: to.clone() });
        // Links by note name still find their notes. Links that spell out the folder follow it.
        let mut rewritten = Vec::new();
        if update {
            for (p, t) in v.files.iter_mut() {
                let mut nt = t.clone();
                for (from, n) in &notes {
                    nt = md::rewrite_links(&nt, md::strip_md(from), md::strip_md(n));
                }
                if nt != *t {
                    *t = nt;
                    rewritten.push(p.clone());
                }
            }
        }
        for (from, n) in &notes {
            if let Some(u) = self.unlocked.remove(from) {
                self.unlocked.insert(n.clone(), u);
            }
            if let Some(e) = self.edits.remove(from) {
                self.edits.insert(n.clone(), e);
            }
        }
        for p in rewritten {
            let t = self.files().get(&p).cloned().unwrap_or_default();
            self.set_text(&p, t);
        }
        for t in self.tabs.iter_mut().chain(self.recent.iter_mut()).chain(self.pinned.iter_mut()) {
            if let Some(n) = moved(t) {
                *t = n;
            }
        }
        if let Some(n) = self.active.as_deref().and_then(moved) {
            self.active = Some(n);
        }
        self.retarget(&moved);
        self.expanded = std::mem::take(&mut self.expanded).into_iter().map(|e| moved(&e).unwrap_or(e)).collect();
        self.rebuild_names_pub();
        self.persist_later();
        self.sync_after_save();
    }

    /// Moves a note into `folder` (`""` is the root), keeping its tab, caret and history.
    pub fn move_note(&mut self, from: &str, folder: &str, tx: &mut Tx) {
        if md::dir(from) == folder || !self.files().contains_key(from) {
            return;
        }
        let name = from.rsplit('/').next().unwrap_or(from).to_string();
        let to = md::join(folder, &name);
        let exists = self.files().contains_key(&to) || self.vault.as_ref().is_some_and(|v| v.path.join(&to).exists());
        if exists {
            self.notice = Some(format!("{} already has a note named {}.", if folder.is_empty() { "The folder root" } else { folder }, md::base(&name)));
            return;
        }
        self.flush_notes();
        let update = self.settings().update_links;
        let Some(v) = &mut self.vault else { return };
        let root = v.path.clone();
        let text = v.files.remove(from).unwrap_or_default();
        v.files.insert(to.clone(), text);
        if let Some(b) = v.born.remove(from) {
            v.born.insert(to.clone(), b);
        }
        self.writer.send(Op::Rename { root, from: from.to_string(), to: to.clone() });
        // Links by note name still find it. Links that spell out the old folder follow it.
        let mut rewritten = Vec::new();
        if update {
            let (old_full, new_full) = (md::strip_md(from).to_string(), md::strip_md(&to).to_string());
            if let Some(v) = &mut self.vault {
                for (p, t) in v.files.iter_mut() {
                    let nt = md::rewrite_links(t, &old_full, &new_full);
                    if nt != *t {
                        *t = nt;
                        rewritten.push(p.clone());
                    }
                }
            }
        }
        if let Some(u) = self.unlocked.remove(from) {
            self.unlocked.insert(to.clone(), u);
        }
        if let Some(e) = self.edits.remove(from) {
            self.edits.insert(to.clone(), e);
        }
        for p in rewritten {
            let t = self.files().get(&p).cloned().unwrap_or_default();
            self.set_text(&p, t);
        }
        for t in self.tabs.iter_mut().chain(self.recent.iter_mut()).chain(self.pinned.iter_mut()) {
            if t == from {
                *t = to.clone();
            }
        }
        if self.active.as_deref() == Some(from) {
            self.active = Some(to.clone());
        }
        self.retarget(&|p| (p == from).then(|| to.clone()));
        if !folder.is_empty() {
            self.expanded.insert(folder.to_string());
        }
        self.rebuild_names_pub();
        self.persist_later();
        self.sync_after_save();
        let _ = tx;
    }

    /// A file row's drag: past 6 points it becomes a move, and the folder under the pointer
    /// takes the note when the button comes up.
    pub fn drag_note(&mut self, path: &str, d: &Drag, tx: &mut Tx) {
        if d.cancelled() {
            self.dragging = None;
            return;
        }
        match d.phase {
            Phase::Down => {
                self.dragging = Some(DragNote { path: path.to_string(), x: d.window_point()[0], y: d.window_point()[1], active: false, target: None, pin: false });
            }
            Phase::Move => {
                let Some(dn) = &mut self.dragging else { return };
                dn.x = d.window_point()[0];
                dn.y = d.window_point()[1];
                if !dn.active && d.dx.abs() + d.dy.abs() > 6.0 {
                    dn.active = true;
                }
                if dn.active {
                    let (x, y) = (dn.x, dn.y);
                    tx.element_at(x, y, |s: &mut App, picked, _| {
                        let pin = picked.as_ref().is_some_and(|p| crate::pins::in_group(p.path()));
                        let target = if pin { None } else { picked.and_then(|p| folder_at(p.path())) };
                        let pinned = s.dragging.as_ref().is_some_and(|dn| s.is_pinned(&dn.path));
                        if let Some(dn) = &mut s.dragging {
                            dn.pin = pin && !pinned;
                            dn.target = target.filter(|f| *f != md::dir(&dn.path));
                        }
                    });
                }
            }
            Phase::Up => {
                if let Some(dn) = self.dragging.take() {
                    if dn.active {
                        if dn.pin {
                            self.pin(&dn.path);
                        } else if let Some(t) = dn.target {
                            self.move_note(&dn.path, &t, tx);
                        }
                    }
                }
            }
        }
    }

    pub fn open_tree_menu(&mut self, on: MenuOn, x: f32, y: f32) {
        self.tree_menu = Some(TreeMenu { x, y, on, confirm: false });
        self.vault_menu = false;
    }

    /// The notes inside `folder`, its subfolders included.
    pub fn notes_in(&self, folder: &str) -> Vec<String> {
        let pre = format!("{folder}/");
        self.files().keys().filter(|p| p.starts_with(&pre)).cloned().collect()
    }

    /// Deletes a folder from the menu: an empty folder goes at once, and a folder with notes
    /// asks first.
    pub fn ask_delete_folder(&mut self, folder: &str, x: f32, y: f32) {
        if self.notes_in(folder).is_empty() {
            self.tree_menu = None;
            self.delete_folder(folder);
        } else {
            self.tree_menu = Some(TreeMenu { x, y, on: MenuOn::Folder(folder.to_string()), confirm: true });
        }
    }

    /// Moves a folder, with every file in it, to `.trash` in the notes folder. Unsaved notes
    /// go to disk first, so the trash holds their last text.
    pub fn delete_folder(&mut self, folder: &str) {
        self.remove_folder(folder);
        self.sync_after_save();
    }

    /// `delete_folder` without the sync after it, for a delete that came from a sync.
    pub(crate) fn remove_folder(&mut self, folder: &str) {
        if folder.is_empty() {
            return;
        }
        let pre = format!("{folder}/");
        let inside = |p: &str| p == folder || p.starts_with(&pre);
        self.flush_notes();
        let Some(v) = &mut self.vault else { return };
        let root = v.path.clone();
        let gone: Vec<String> = v.files.keys().filter(|p| inside(p)).cloned().collect();
        for p in &gone {
            v.files.remove(p);
            v.born.remove(p);
        }
        v.folders.retain(|f| !inside(f));
        self.writer.send(Op::DeleteFolder { root, rel: folder.to_string() });
        for p in &gone {
            self.close_tab(p);
            self.edits.remove(p);
        }
        self.recent.retain(|p| !inside(p));
        self.pinned.retain(|p| !inside(p));
        self.unlocked.retain(|p, _| !inside(p));
        self.expanded.retain(|p| !inside(p));
        if self.new_folder.as_ref().is_some_and(|nf| inside(&nf.parent)) {
            self.new_folder = None;
        }
        if self.active.as_deref().is_some_and(inside) {
            self.active = None;
        }
        self.rebuild_names_pub();
        self.persist_later();
    }

    /// Moves a note to `.trash` in the folder, open or not.
    pub fn delete_path(&mut self, p: &str) {
        let was = self.active.clone();
        self.active = Some(p.to_string());
        self.delete_note();
        if was.as_deref() != Some(p) {
            self.active = was.filter(|a| self.files().contains_key(a));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_paths_name_their_folder() {
        assert_eq!(folder_at("root/body/sidebar/side-scroll/side-list/tree/dir:Docs|Specs/#2"), Some("Docs/Specs".into()));
        assert_eq!(folder_at("root/body/sidebar/side-scroll/side-list/tree/file:Journal|2026.md/#1"), Some("Journal".into()));
        assert_eq!(folder_at("root/body/sidebar/side-scroll/side-list"), Some(String::new()));
        assert_eq!(folder_at("root/body/sheet/doc-scroll"), None);
    }

    /// Rename a folder from its menu: the notes, the folder inside it, tabs, pins and links
    /// that spell out the folder all follow, on disk too.
    #[test]
    fn a_folder_renames_with_everything_in_it() {
        use rune::{Env, Runtime};
        let _env = crate::scroll_tests::ENV.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("typo-rename-folder-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // SAFETY: the ENV lock keeps other tests that read these variables out.
        unsafe {
            std::env::set_var("HOME", &dir);
            std::env::set_var("XDG_CONFIG_HOME", dir.join("cfg"));
            std::env::set_var("XDG_DATA_HOME", dir.join("data"));
        }
        let root = dir.join("notes");
        std::fs::create_dir_all(root.join("Work/Old")).unwrap();
        std::fs::create_dir_all(root.join("Workshop")).unwrap();
        std::fs::write(root.join("Work/Plan.md"), "# Plan").unwrap();
        std::fs::write(root.join("Work/Old/Notes.md"), "See [[Work/Plan]] and [[Plan]].").unwrap();
        std::fs::write(root.join("Workshop/Tools.md"), "Not in Work.").unwrap();
        std::fs::write(root.join("Index.md"), "[[Work/Old/Notes|old]] and [[Workshop/Tools]]").unwrap();
        std::fs::write(root.join("Work/photo.png"), [1u8, 2, 3]).unwrap();
        let mut rt = Runtime::new(crate::app::init(), crate::ui::view, (1280.0, 800.0), Env { system_dark: false, scale: 1.0 });
        let r2 = root.clone();
        rt.update(move |s, tx| {
            s.open_vault(r2, tx);
            s.open_file("Work/Plan.md", None, tx);
            s.toggle_pin("Work/Old/Notes.md");
            s.expanded.insert("Work/Old".into());
        });
        rt.frame(1.0 / 60.0);
        rt.update(|s, tx| {
            s.open_tree_menu(MenuOn::Folder("Work".into()), 100.0, 100.0);
            s.start_rename_folder("Work", tx);
            let nf = s.new_folder.as_mut().unwrap();
            assert_eq!(nf.edit.text, "Work");
            nf.edit.replace_text("Projects");
        });
        rt.frame(1.0 / 60.0);
        rt.update(|s, _| {
            s.commit_new_folder();
            s.flush_notes();
            s.writer.wait(std::time::Duration::from_secs(5));
            let files: Vec<&str> = s.files().keys().map(|k| k.as_str()).collect();
            assert_eq!(files, ["Index.md", "Projects/Old/Notes.md", "Projects/Plan.md", "Workshop/Tools.md"]);
            let v = s.vault.as_ref().unwrap();
            assert_eq!(v.folders, ["Projects", "Projects/Old", "Workshop"]);
            assert_eq!(s.active.as_deref(), Some("Projects/Plan.md"));
            assert!(s.tabs.contains(&"Projects/Plan.md".to_string()));
            assert_eq!(s.pinned, ["Projects/Old/Notes.md"]);
            assert!(s.expanded.contains("Projects/Old"));
            assert_eq!(s.files()["Index.md"], "[[Projects/Old/Notes|old]] and [[Workshop/Tools]]");
            assert_eq!(s.files()["Projects/Old/Notes.md"], "See [[Projects/Plan]] and [[Plan]].");
        });
        rt.update(|s, _| {
            s.flush_notes();
            s.writer.wait(std::time::Duration::from_secs(5));
        });
        assert!(!root.join("Work").exists());
        assert_eq!(std::fs::read(root.join("Projects/photo.png")).unwrap(), [1, 2, 3]);
        assert_eq!(std::fs::read_to_string(root.join("Index.md")).unwrap(), "[[Projects/Old/Notes|old]] and [[Workshop/Tools]]");
        assert_eq!(std::fs::read_to_string(root.join("Projects/Old/Notes.md")).unwrap(), "See [[Projects/Plan]] and [[Plan]].");
        // A name that is taken leaves both folders as they are.
        rt.update(|s, _| {
            s.rename_folder("Projects", "Workshop");
            assert!(s.notice.as_deref().is_some_and(|n| n.contains("already there")));
            assert!(s.files().contains_key("Projects/Plan.md"));
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn paths_split_into_folder_and_name() {
        assert_eq!(split_path("Drafts/Ideas/A: b"), ("Drafts/Ideas".into(), "A- b".into()));
        assert_eq!(split_path("../x"), (String::new(), "x".into()));
    }
}
