//! The `Folder` QML singleton: the sidebar's view of the open folder
//! (src/folder.rs) as JSON for QML, the file operations of its menus, and
//! a watch on the folder that says when anything it lists has changed.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        type Folder = super::FolderRust;

        /// Watch `root` and everything in it, in place of what was
        /// watched before; empty stops watching.
        #[qinvokable]
        fn watch(self: Pin<&mut Folder>, root: &QString);
        /// A file or folder the sidebar shows under the watched folder was
        /// added, removed, renamed or changed; once for a burst of them.
        #[qsignal]
        fn changed(self: Pin<&mut Folder>);

        /// Which files the file tree and the articles list (with Quick Open
        /// and the folder search) show, from Preferences ▸ Files: hidden
        /// ones too or not; per view "supported", "all" or "custom", with
        /// its glob patterns. Says `changed`.
        #[qinvokable]
        #[cxx_name = "setFilters"]
        fn set_filters(
            self: Pin<&mut Folder>,
            hidden: bool,
            tree: &QString,
            tree_patterns: &QString,
            library: &QString,
            library_patterns: &QString,
        );

        /// The articles list of `root`, as
        /// `{"more": bool, "items": [{path, folder, name, ext, modified, created}]}`,
        /// `sort` being natural, name, modified or created.
        #[qinvokable]
        fn articles(self: &Folder, root: &QString, sort: &QString, descending: bool, group: bool) -> QString;
        /// The start of the file at `path`, as the articles list shows it.
        #[qinvokable]
        fn summary(self: &Folder, path: &QString) -> QString;
        /// What the file tree of `root` shows in `dir`: `[{path, name, dir}]`,
        /// sorted as the articles list is, folders first with `group`.
        #[qinvokable]
        fn children(self: &Folder, root: &QString, dir: &QString, sort: &QString, descending: bool, group: bool) -> QString;
        /// Every listed file under `root`, as a JSON array of paths.
        #[qinvokable]
        fn files(self: &Folder, root: &QString) -> QString;
        /// The folder search: `{"error": "", "hits": [{path, count, lines:
        /// [{line, text, ranges: [[from, to]]}]}]}`.
        #[qinvokable]
        fn search(self: &Folder, root: &QString, query: &QString, case_sensitive: bool, whole_word: bool, regex: bool) -> QString;
        /// Quick Open's matches for `query` among `candidates` (a JSON
        /// array of paths), best first, as a JSON array.
        #[qinvokable]
        #[cxx_name = "quickOpen"]
        fn quick_open(self: &Folder, candidates: &QString, query: &QString, root: &QString) -> QString;
        /// The name for a new file (with extension `ext`) or folder in `dir`.
        #[qinvokable]
        #[cxx_name = "untitledName"]
        fn untitled_name(self: &Folder, dir: &QString, folder: bool, ext: &QString) -> QString;
        /// Make an empty file (or folder) at `path`; the error, or "".
        #[qinvokable]
        fn create(self: &Folder, path: &QString, folder: bool) -> QString;
        /// Rename `from` to `name` in the same folder; the new path, or
        /// `!` and the error.
        #[qinvokable]
        fn rename(self: &Folder, from: &QString, name: &QString) -> QString;
        /// Move `from` to the path `to`, which must not be there yet
        /// unless `replace`: "moved", "copied" (onto another disk, the
        /// original kept), or `!` and the error.
        #[qinvokable]
        #[cxx_name = "moveFile"]
        fn move_file(self: &Folder, from: &QString, to: &QString, replace: bool) -> QString;
        /// Copy `path` beside itself as `name (copy).ext`; the new path,
        /// or `!` and the error.
        #[qinvokable]
        fn duplicate(self: &Folder, path: &QString) -> QString;
        /// Whether the sidebar lists the file at `path`.
        #[qinvokable]
        #[cxx_name = "isListed"]
        fn is_listed(self: &Folder, path: &QString) -> bool;
    }

    impl cxx_qt::Threading for Folder {}
}

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, RwLock, mpsc};
use std::time::Duration;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::QString;
use notify::{EventKind, RecursiveMode, Watcher};
use serde_json::json;

use crate::folder;
use crate::md::query::FindOptions;

#[derive(Default)]
pub struct FolderRust {
    watcher: Option<notify::RecommendedWatcher>,
    /// Which `watch` call is current, so a watcher that finishes setting
    /// up after the folder changed again is dropped.
    generation: u64,
    /// The file tree's filter, then the articles list's; the watcher reads
    /// them too.
    filters: Arc<RwLock<[folder::Filter; 2]>>,
}

/// Whether a change at `path` can change what the sidebar shows: a file
/// either view lists or what may be a folder, outside hidden folders
/// unless those show.
fn matters(root: &Path, path: &Path, filters: &[folder::Filter]) -> bool {
    let Ok(rel) = path.strip_prefix(root) else { return false };
    let in_hidden = rel.components().any(|c| c.as_os_str().to_string_lossy().starts_with('.'));
    let dir = path.extension().is_none() || path.is_dir();
    filters.iter().any(|f| (f.hidden || !in_hidden) && (dir || f.shows(root, path, false)))
}

impl FolderRust {
    fn tree(&self) -> folder::Filter {
        self.filters.read().map(|f| f[0].clone()).unwrap_or_default()
    }
    fn library(&self) -> folder::Filter {
        self.filters.read().map(|f| f[1].clone()).unwrap_or_default()
    }
}

fn err(e: impl std::fmt::Display) -> QString {
    QString::from(&format!("!{e}"))
}

impl qobject::Folder {
    fn watch(mut self: Pin<&mut Self>, root: &QString) {
        let generation = self.generation + 1;
        let mut rust = self.as_mut().rust_mut();
        rust.generation = generation;
        rust.watcher = None;
        let root = PathBuf::from(root.to_string());
        if root.as_os_str().is_empty() {
            return;
        }
        let thread = self.qt_thread();
        let filters = Arc::clone(&self.filters);
        // Watching a big tree means a watch for each folder in it, which
        // takes a while, so it is set up off the UI thread.
        std::thread::spawn(move || {
            let (tx, rx) = mpsc::channel::<()>();
            let base = root.clone();
            let filters = Arc::clone(&filters);
            let handler = move |event: notify::Result<notify::Event>| {
                let Ok(event) = event else { return };
                if matches!(event.kind, EventKind::Access(_)) {
                    return;
                }
                let Ok(filters) = filters.read() else { return };
                if event.paths.iter().any(|p| matters(&base, p, &filters[..])) {
                    let _ = tx.send(());
                }
            };
            let Ok(mut watcher) = notify::recommended_watcher(handler) else { return };
            if watcher.watch(&root, RecursiveMode::Recursive).is_err() {
                return;
            }
            let queue = thread.clone();
            let installed = thread.queue(move |mut obj| {
                if obj.generation == generation {
                    obj.as_mut().rust_mut().watcher = Some(watcher);
                }
            });
            if installed.is_err() {
                return;
            }
            // A save or a checkout changes many files at once: one signal
            // when they have been quiet for a moment. The channel closes
            // when the watcher is dropped.
            while rx.recv().is_ok() {
                while rx.recv_timeout(Duration::from_millis(250)).is_ok() {}
                if queue.queue(|obj| obj.changed()).is_err() {
                    break;
                }
            }
        });
    }

    fn set_filters(
        mut self: Pin<&mut Self>,
        hidden: bool,
        tree: &QString,
        tree_patterns: &QString,
        library: &QString,
        library_patterns: &QString,
    ) {
        let next = [
            folder::Filter::new(hidden, &tree.to_string(), &tree_patterns.to_string()),
            folder::Filter::new(hidden, &library.to_string(), &library_patterns.to_string()),
        ];
        if let Ok(mut filters) = self.filters.write() {
            if *filters == next {
                return;
            }
            *filters = next;
        }
        self.as_mut().changed();
    }

    fn articles(&self, root: &QString, sort: &QString, descending: bool, group: bool) -> QString {
        let (list, more) = folder::articles(
            Path::new(&root.to_string()),
            &self.library(),
            folder::Sort::parse(&sort.to_string()),
            descending,
            group,
            0,
            folder::LIST_LIMIT,
        );
        let items: Vec<_> = list
            .into_iter()
            .map(|a| {
                let path = Path::new(&a.path);
                let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
                json!({"path": a.path, "folder": a.folder, "name": name, "ext": ext, "modified": a.modified, "created": a.created})
            })
            .collect();
        QString::from(&json!({"more": more, "items": items}).to_string())
    }

    fn summary(&self, path: &QString) -> QString {
        QString::from(&folder::summary_of(Path::new(&path.to_string())))
    }

    fn children(&self, root: &QString, dir: &QString, sort: &QString, descending: bool, group: bool) -> QString {
        let (root, dir, sort) = (root.to_string(), dir.to_string(), folder::Sort::parse(&sort.to_string()));
        let list: Vec<_> = folder::children(Path::new(&root), Path::new(&dir), &self.tree(), sort, descending, group)
            .into_iter()
            .map(|n| json!({"path": n.path, "name": n.name, "dir": n.dir}))
            .collect();
        QString::from(&serde_json::Value::from(list).to_string())
    }

    fn files(&self, root: &QString) -> QString {
        QString::from(&serde_json::Value::from(folder::files(Path::new(&root.to_string()), &self.library())).to_string())
    }

    fn search(&self, root: &QString, query: &QString, case_sensitive: bool, whole_word: bool, regex: bool) -> QString {
        let opts = FindOptions { case_sensitive, whole_word, regex };
        let out = match folder::search(Path::new(&root.to_string()), &self.library(), &query.to_string(), &opts) {
            Ok(hits) => {
                let hits: Vec<_> = hits
                    .into_iter()
                    .map(|h| {
                        let lines: Vec<_> =
                            h.lines.into_iter().map(|l| json!({"line": l.line, "text": l.text, "ranges": l.ranges})).collect();
                        json!({"path": h.path, "count": h.count, "lines": lines})
                    })
                    .collect();
                json!({"error": "", "hits": hits})
            }
            Err(e) => json!({"error": e, "hits": []}),
        };
        QString::from(&out.to_string())
    }

    fn quick_open(&self, candidates: &QString, query: &QString, root: &QString) -> QString {
        let candidates: Vec<String> = serde_json::from_str(&candidates.to_string()).unwrap_or_default();
        let found = folder::quick_open(&candidates, &query.to_string(), &root.to_string());
        QString::from(&serde_json::Value::from(found).to_string())
    }

    fn untitled_name(&self, dir: &QString, is_folder: bool, ext: &QString) -> QString {
        QString::from(&folder::untitled_name(Path::new(&dir.to_string()), is_folder, &ext.to_string()))
    }

    fn create(&self, path: &QString, is_folder: bool) -> QString {
        let path = PathBuf::from(path.to_string());
        let result = if is_folder {
            std::fs::create_dir(&path)
        } else {
            std::fs::OpenOptions::new().write(true).create_new(true).open(&path).map(|_| ())
        };
        match result {
            Ok(()) => QString::default(),
            Err(e) => QString::from(&e.to_string()),
        }
    }

    fn rename(&self, from: &QString, name: &QString) -> QString {
        let from = PathBuf::from(from.to_string());
        // As Typora: no slashes or control characters, nothing around.
        let name: String = name.to_string().chars().filter(|c| *c != '/' && *c != '\\' && !c.is_control()).collect();
        let name = name.trim();
        let Some(dir) = from.parent() else { return err("No such file") };
        if name.is_empty() {
            return QString::from(&from.to_string_lossy().into_owned());
        }
        let to = dir.join(name);
        if to == from {
            return QString::from(&to.to_string_lossy().into_owned());
        }
        // A rename that only changes case is the same file on some disks.
        let same = from.canonicalize().ok().zip(to.canonicalize().ok()).is_some_and(|(a, b)| a == b);
        if to.exists() && !same {
            return err("File with same name already exists at target path.");
        }
        match std::fs::rename(&from, &to) {
            Ok(()) => QString::from(&to.to_string_lossy().into_owned()),
            Err(e) => err(e),
        }
    }

    fn move_file(&self, from: &QString, to: &QString, replace: bool) -> QString {
        match folder::move_path(Path::new(&from.to_string()), Path::new(&to.to_string()), replace) {
            Ok(folder::Moved::Moved) => QString::from("moved"),
            Ok(folder::Moved::Copied) => QString::from("copied"),
            Err(e) => err(e),
        }
    }

    fn duplicate(&self, path: &QString) -> QString {
        let from = PathBuf::from(path.to_string());
        let to = folder::duplicate_path(&from);
        match std::fs::copy(&from, &to) {
            Ok(_) => QString::from(&to.to_string_lossy().into_owned()),
            Err(e) => err(e),
        }
    }

    fn is_listed(&self, path: &QString) -> bool {
        folder::is_listed(Path::new(&path.to_string()))
    }
}
