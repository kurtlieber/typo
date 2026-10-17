//! The `Desktop` QML singleton: calls into the desktop and the window system
//! that QML can't make itself.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("typo_glue.h");

        /// Renders the first window into `path`.
        #[cxx_name = "typoGrabWindow"]
        fn grab_window(path: &QString) -> bool;
        #[cxx_name = "typoSendKey"]
        fn send_key_event(chord: &QString) -> bool;
        #[cxx_name = "typoTypeText"]
        fn type_text_events(text: &QString) -> bool;
        #[cxx_name = "typoClick"]
        fn click_at(x: f64, y: f64, mods: &QString) -> bool;
        #[cxx_name = "typoHover"]
        fn hover_at(x: f64, y: f64) -> bool;
        #[cxx_name = "typoWatchAlt"]
        fn watch_alt_taps();
        #[cxx_name = "typoOpenWindowLater"]
        fn open_window_later(args: &QString);
        #[cxx_name = "typoRaiseAll"]
        fn raise_all_windows();
        #[cxx_name = "typoWindowTitles"]
        fn window_titles_list() -> QString;
        #[cxx_name = "typoFrontWindow"]
        fn front_window_index() -> i32;
        #[cxx_name = "typoRaiseWindow"]
        fn raise_window_at(i: i32) -> bool;
        #[cxx_name = "typoWheel"]
        fn wheel_at(x: f64, y: f64, dy: f64, mods: &QString) -> bool;
        #[cxx_name = "typoDrag"]
        fn drag_between(x1: f64, y1: f64, x2: f64, y2: f64) -> bool;
        #[cxx_name = "typoMouse"]
        fn mouse_at(what: i32, x: f64, y: f64) -> bool;
        #[cxx_name = "typoDropFiles"]
        fn drop_files_at(x: f64, y: f64, paths: &QString) -> bool;
        #[cxx_name = "typoDragFiles"]
        fn drag_files_at(stage: i32, x: f64, y: f64, paths: &QString) -> bool;
        #[cxx_name = "typoReadFile"]
        fn read_file_text(path: &QString) -> QString;
        #[cxx_name = "typoWriteFile"]
        fn write_file_text(path: &QString, text: &QString) -> bool;
        #[cxx_name = "typoSetClipboard"]
        fn set_clipboard_data(text: &QString, html: &QString, markdown: &QString);
        #[cxx_name = "typoClipboardMarkdown"]
        fn clipboard_markdown_data() -> QString;
        #[cxx_name = "typoClipboardText"]
        fn clipboard_plain() -> QString;
        #[cxx_name = "typoClipboardHtml"]
        fn clipboard_rich() -> QString;
        #[cxx_name = "typoHtmlToText"]
        fn html_text(html: &QString) -> QString;
        #[cxx_name = "typoClipboardFiles"]
        fn clipboard_file_paths() -> QString;
        #[cxx_name = "typoSaveClipboardImage"]
        fn save_clipboard_image_in(folder: &QString) -> QString;
        #[cxx_name = "typoSetClipboardFiles"]
        fn set_clipboard_file_paths(paths: &QString);
        #[cxx_name = "typoSetClipboardImage"]
        fn set_clipboard_image_file(path: &QString) -> bool;
        #[cxx_name = "typoMoveToTrash"]
        fn move_file_to_trash(path: &QString) -> bool;
        #[cxx_name = "typoCopyMathImage"]
        fn copy_math_image_data(id: &QString) -> bool;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        type Desktop = super::DesktopRust;

        /// Saves a screenshot of the window, menus and popups included, for
        /// `--screenshot`. QML's grabToImage can't capture the window's root.
        #[qinvokable]
        fn screenshot(self: &Desktop, path: &QString) -> bool;

        // The `--script` test driver.
        /// Press and release a chord such as "Ctrl+S".
        #[qinvokable]
        #[cxx_name = "sendKey"]
        fn send_key(self: &Desktop, chord: &QString) -> bool;
        /// Type text key by key.
        #[qinvokable]
        #[cxx_name = "typeText"]
        fn type_text(self: &Desktop, text: &QString) -> bool;
        /// Left-click at window coordinates, holding `mods` ("Shift",
        /// "Ctrl+Shift"…).
        #[qinvokable]
        fn click(self: &Desktop, x: f64, y: f64, mods: &QString) -> bool;
        /// Move the pointer to a window point, no button held.
        #[qinvokable]
        fn hover(self: &Desktop, x: f64, y: f64) -> bool;
        /// Turn the wheel at a window point, `dy` in eighths of a degree,
        /// holding modifiers ("Ctrl", "Shift"…).
        #[qinvokable]
        fn wheel(self: &Desktop, x: f64, y: f64, dy: f64, mods: &QString) -> bool;
        /// Drag with the left button from one window point to another.
        #[qinvokable]
        fn drag(self: &Desktop, x1: f64, y1: f64, x2: f64, y2: f64) -> bool;
        /// A step of a drag: 0 presses the left button at a window point,
        /// 1 moves there, 2 releases it.
        #[qinvokable]
        fn mouse(self: &Desktop, what: i32, x: f64, y: f64) -> bool;
        /// Drop files (paths one a line) at a window point.
        #[qinvokable]
        #[cxx_name = "dropFiles"]
        fn drop_files(self: &Desktop, x: f64, y: f64, paths: &QString) -> bool;
        /// Files (paths one a line) dragged in from another program: over
        /// a window point (stage 0), dropped there (1), or away (2).
        #[qinvokable]
        #[cxx_name = "dragFiles"]
        fn drag_files(self: &Desktop, stage: i32, x: f64, y: f64, paths: &QString) -> bool;
        #[qinvokable]
        #[cxx_name = "readFile"]
        fn read_file(self: &Desktop, path: &QString) -> QString;
        #[qinvokable]
        #[cxx_name = "writeFile"]
        fn write_file(self: &Desktop, path: &QString, text: &QString) -> bool;

        /// Put `text` on the clipboard, with `html` as its rich form and
        /// `markdown` as typo's own form when not empty.
        #[qinvokable]
        #[cxx_name = "setClipboard"]
        fn set_clipboard(self: &Desktop, text: &QString, html: &QString, markdown: &QString);
        /// The markdown typo put on the clipboard, or empty.
        #[qinvokable]
        #[cxx_name = "clipboardMarkdown"]
        fn clipboard_markdown(self: &Desktop) -> QString;
        #[qinvokable]
        #[cxx_name = "clipboardText"]
        fn clipboard_text(self: &Desktop) -> QString;
        /// The clipboard's HTML, or empty when it has none.
        #[qinvokable]
        #[cxx_name = "clipboardHtml"]
        fn clipboard_html(self: &Desktop) -> QString;
        /// Files copied in a file manager, as paths one a line, or "".
        #[qinvokable]
        #[cxx_name = "clipboardFiles"]
        fn clipboard_files(self: &Desktop) -> QString;
        /// Save the picture on the clipboard in `folder` (made if need
        /// be) as Typora names pasted pictures; its path, or "" when the
        /// clipboard holds none.
        #[qinvokable]
        #[cxx_name = "saveClipboardImage"]
        fn save_clipboard_image(self: &Desktop, folder: &QString) -> QString;
        /// For the test driver: put files (paths one a line) or the
        /// picture in the file at `path` on the clipboard.
        #[qinvokable]
        #[cxx_name = "setClipboardFiles"]
        fn set_clipboard_files(self: &Desktop, paths: &QString);
        #[qinvokable]
        #[cxx_name = "setClipboardImage"]
        fn set_clipboard_image(self: &Desktop, path: &QString) -> bool;
        /// Move the file at `path` to the trash.
        #[qinvokable]
        #[cxx_name = "moveToTrash"]
        fn move_to_trash(self: &Desktop, path: &QString) -> bool;
        /// Show the file at `path` in the file manager, selected, or else
        /// open its folder.
        #[qinvokable]
        #[cxx_name = "showInFolder"]
        fn show_in_folder(self: &Desktop, path: &QString);
        /// The folder pictures pasted into an untitled document, or one
        /// with no image folder, are saved in.
        #[qinvokable]
        #[cxx_name = "pastedImagesFolder"]
        fn pasted_images_folder(self: &Desktop) -> QString;
        /// The text a reader sees in `html`.
        #[qinvokable]
        #[cxx_name = "plainText"]
        fn plain_text(self: &Desktop, html: &QString) -> QString;

        /// Show the desktop's chooser: `kind` is "open", "save", "folder",
        /// "images" (any number, one a line in the answer), "imageCopy" or
        /// "imageMove" (where an image's file goes), "imagesCopy" or
        /// "imagesMove" (a folder for them all), or "imageFolder" or
        /// "imageRoot" (the document's own image folders), "import" (a file
        /// for pandoc to read), or "export:" and
        /// a File → Export format (`name` then gets its extension); `folder` is where it
        /// starts and `name` the suggested file name. The answer comes back
        /// as `fileChosen`, with the kind.
        #[qinvokable]
        #[cxx_name = "chooseFile"]
        fn choose_file(self: Pin<&mut Desktop>, kind: &QString, title: &QString, folder: &QString, name: &QString);
        /// Answer the next chooser with `path` without showing it; an empty
        /// path cancels it (the test driver).
        #[qinvokable]
        #[cxx_name = "fakeChoice"]
        fn fake_choice(self: Pin<&mut Desktop>, path: &QString);
        /// The chooser's answer: the path, empty when cancelled, or an error.
        #[qsignal]
        #[cxx_name = "fileChosen"]
        fn file_chosen(self: Pin<&mut Desktop>, kind: QString, path: QString, error: QString);
        /// On the Mac, `chooseFile` asks QML to open the system's panel:
        /// `mode` is "open", "openMany", "save" or "folder", `folder` and
        /// `file` (the suggested name) are URLs, and `filters` Qt name
        /// filters, one a line. QML answers with `fileChosen`.
        #[qsignal]
        #[cxx_name = "dialogWanted"]
        fn dialog_wanted(self: Pin<&mut Desktop>, kind: QString, title: QString, mode: QString, folder: QString, file: QString, filters: QString);

        /// The code language box's suggestions, one per line.
        #[qinvokable]
        #[cxx_name = "codeLanguages"]
        fn code_languages(self: &Desktop) -> QString;

        /// Why the formula `tex` failed to typeset, once its image has
        /// loaded; empty when it didn't.
        #[qinvokable]
        #[cxx_name = "mathError"]
        fn math_error(self: &Desktop, tex: &QString) -> QString;

        /// Why the `lang` diagram `src` failed to draw, once its image has
        /// loaded, as Typora words it; empty when it didn't.
        #[qinvokable]
        #[cxx_name = "diagramError"]
        fn diagram_error(self: &Desktop, lang: &QString, src: &QString) -> QString;

        /// Where in its code the diagram's error is, as a highlighter's
        /// `hits`, once its image has loaded.
        #[qinvokable]
        #[cxx_name = "diagramErrorMark"]
        fn diagram_error_mark(self: &Desktop, lang: &QString, src: &QString) -> QString;

        /// How far down inline formula `tex` its baseline is, as a part of
        /// its height, once its image has loaded; -1 before.
        #[qinvokable]
        #[cxx_name = "mathBaseline"]
        fn math_baseline(self: &Desktop, tex: &QString) -> f64;

        /// Where each row of display formula `tex` has its baseline, as
        /// parts of its height in a JSON list, once its image has loaded;
        /// empty for one row.
        #[qinvokable]
        #[cxx_name = "mathRows"]
        fn math_rows(self: &Desktop, tex: &QString) -> QString;

        /// Enable physics package: formulas everywhere, export included,
        /// read its commands.
        #[qinvokable]
        #[cxx_name = "setMathPhysics"]
        fn set_math_physics(self: &Desktop, on: bool);

        /// `tex` with the physics package's commands written out, so that
        /// a formula's image changes with the preference.
        #[qinvokable]
        #[cxx_name = "mathPhysics"]
        fn math_physics(self: &Desktop, tex: &QString) -> QString;

        /// The command Upload Image runs for Preferences' `uploader` (and
        /// Custom Command's `custom`) for the document at `doc`, or "".
        #[qinvokable]
        #[cxx_name = "uploaderCommand"]
        fn uploader_command(self: &Desktop, uploader: &QString, custom: &QString, doc: &QString) -> QString;

        /// Test Uploader's image: a small PNG written to the temp folder,
        /// its path.
        #[qinvokable]
        #[cxx_name = "uploadTestImage"]
        fn upload_test_image(self: &Desktop) -> QString;

        /// The online image at `url` downloaded to the temp folder, its
        /// path, or "" when it couldn't be.
        #[qinvokable]
        #[cxx_name = "downloadImage"]
        fn download_image(self: &Desktop, url: &QString) -> QString;

        /// The tests' web server: the files in `folder` served over HTTP
        /// on 127.0.0.1, at the port this returns.
        #[qinvokable]
        #[cxx_name = "serveFolder"]
        fn serve_folder(self: &Desktop, folder: &QString) -> i32;

        /// Copy as Image: formula `tex` typeset in black, `em` pixels to the
        /// em, on the clipboard. False when it doesn't typeset.
        #[qinvokable]
        #[cxx_name = "copyMathImage"]
        fn copy_math_image(self: &Desktop, tex: &QString, display: bool, em: f64) -> bool;

        /// Copy as MathML: formula `tex` in MathML, or "" when it can't be
        /// converted; `tags`, a line for each row, are what display math
        /// shows at the right of its rows.
        #[qinvokable]
        #[cxx_name = "mathML"]
        fn math_ml(self: &Desktop, tex: &QString, display: bool, tags: &QString) -> QString;

        /// Whether pandoc (2.0 or later) is installed, for Import… and the
        /// exports it writes: at `path` (Preferences ▸ Export ▸ General ▸
        /// Pandoc Path), or on PATH when it is empty. Import and export
        /// run that one from then on.
        #[qinvokable]
        #[cxx_name = "hasPandoc"]
        fn has_pandoc(self: &Desktop, path: &QString) -> bool;

        /// Start another typo window, on `path` when it isn't empty.
        #[qinvokable]
        fn spawn(self: &Desktop, path: &QString) -> bool;
        /// Start another typo window on the document `path` and the
        /// folder `folder`, either of them empty, as a restored session
        /// has them.
        #[qinvokable]
        #[cxx_name = "spawnWindow"]
        fn spawn_window(self: &Desktop, path: &QString, folder: &QString) -> bool;
        /// Window ▸ Bring All to Front: every typo window, this one on top.
        #[qinvokable]
        #[cxx_name = "raiseAll"]
        fn raise_all(self: &Desktop);
        /// The Mac's Window menu and ⌘`: the typo windows' titles, a line
        /// each, in the order they opened; the front one's place in that
        /// order (-1 for none); and bringing the one at `i` to the front.
        #[qinvokable]
        #[cxx_name = "windowTitles"]
        fn window_titles(self: &Desktop) -> QString;
        #[qinvokable]
        #[cxx_name = "frontWindow"]
        fn front_window(self: &Desktop) -> i32;
        #[qinvokable]
        #[cxx_name = "raiseWindow"]
        fn raise_window(self: &Desktop, i: i32) -> bool;

        /// The session (src/session.rs): note this window's document and
        /// folder.
        #[qinvokable]
        #[cxx_name = "recordWindow"]
        fn record_window(self: &Desktop, path: &QString, folder: &QString);
        /// Leave this window out of the session, as it closes while others
        /// stay open.
        #[qinvokable]
        #[cxx_name = "forgetWindow"]
        fn forget_window(self: &Desktop);
        /// Whether another typo window is open.
        #[qinvokable]
        #[cxx_name = "otherWindowsOpen"]
        fn other_windows_open(self: &Desktop) -> bool;
        /// Switch Between Opened Documents, where each window is a process.
        #[qinvokable]
        #[cxx_name = "focusNextWindow"]
        fn focus_next_window(self: &Desktop);
        /// Edit ▸ Emoji and Symbols: the system's emoji picker; false when
        /// there is none.
        #[qinvokable]
        #[cxx_name = "showEmojiPicker"]
        fn show_emoji_picker(self: &Desktop) -> bool;
        /// The Mac's File ▸ Get Info…: Finder's info window for `path`.
        #[qinvokable]
        #[cxx_name = "showInfo"]
        fn show_info(self: &Desktop, path: &QString);
        /// The windows open when typo last closed, as JSON `[{path,
        /// folder}]`, oldest first; they are forgotten.
        #[qinvokable]
        #[cxx_name = "takeSession"]
        fn take_session(self: &Desktop) -> QString;
        /// Ask every typo window to close (File ▸ Quit); false when the
        /// session bus can't carry the request.
        #[qinvokable]
        #[cxx_name = "quitAll"]
        fn quit_all(self: &Desktop) -> bool;
        /// Ask every typo window to save (File ▸ Save All), as `quitAll`.
        #[qinvokable]
        #[cxx_name = "saveAll"]
        fn save_all(self: &Desktop) -> bool;
        /// Start passing on quit and save requests as `quitRequested` and
        /// `saveRequested`.
        #[qinvokable]
        #[cxx_name = "listenForQuit"]
        fn listen_for_quit(self: Pin<&mut Desktop>);
        #[qsignal]
        #[cxx_name = "quitRequested"]
        fn quit_requested(self: Pin<&mut Desktop>);
        #[qsignal]
        #[cxx_name = "saveRequested"]
        fn save_requested(self: Pin<&mut Desktop>);
        #[qinvokable]
        fn exists(self: &Desktop, path: &QString) -> bool;
        #[qinvokable]
        #[cxx_name = "isFolder"]
        fn is_folder(self: &Desktop, path: &QString) -> bool;
        /// `path` made absolute and canonical (it may not exist yet).
        #[qinvokable]
        #[cxx_name = "absolutePath"]
        fn absolute_path(self: &Desktop, path: &QString) -> QString;
        /// The advanced settings file, written with the defaults when it
        /// is missing or `reset` asks for them; "" when it can't be.
        #[qinvokable]
        #[cxx_name = "advancedSettings"]
        fn advanced_settings(self: &Desktop, reset: bool) -> QString;
        /// The advanced settings as JSON: the file's over the defaults.
        #[qinvokable]
        fn advanced(self: &Desktop) -> QString;
        /// Call the window's `altTapped` whenever Alt is pressed and
        /// released alone (once is enough).
        #[qinvokable]
        #[cxx_name = "watchAlt"]
        fn watch_alt(self: &Desktop);
        /// The folder Omarchy keeps the user's themes in.
        #[qinvokable]
        #[cxx_name = "themesFolder"]
        fn themes_folder(self: &Desktop) -> QString;
        /// The drafts folder, made if it isn't there.
        #[qinvokable]
        #[cxx_name = "draftsFolder"]
        fn drafts_folder(self: &Desktop) -> QString;
    }

    impl cxx_qt::Threading for Desktop {}
}

use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};

use cxx_qt::{CxxQtType, Threading};

use crate::{export, platform, session};

use cxx_qt_lib::QString;

pub struct DesktopRust {
    /// Answers for the next choosers, from `fakeChoice`.
    fake: Vec<String>,
    listening: bool,
    /// Which of the process's windows this is (each has a Desktop), 0
    /// but on the Mac.
    window: u32,
}

/// The windows this process has opened.
static WINDOWS: AtomicU32 = AtomicU32::new(0);

impl Default for DesktopRust {
    fn default() -> Self {
        Self { fake: Vec::new(), listening: false, window: WINDOWS.fetch_add(1, Ordering::Relaxed) }
    }
}

/// Another window, with `args` after the program's name: a process of its
/// own on Linux, a window of this one on the Mac (one Dock icon).
fn open_window(args: Vec<String>) -> bool {
    if cfg!(target_os = "macos") {
        qobject::open_window_later(&QString::from(&args.join("\0")));
        return true;
    }
    let Ok(exe) = std::env::current_exe() else { return false };
    match std::process::Command::new(exe).args(args).spawn() {
        Ok(mut child) => {
            // Reap it when it exits.
            std::thread::spawn(move || child.wait());
            true
        }
        Err(_) => false,
    }
}

impl qobject::Desktop {
    fn choose_file(mut self: Pin<&mut Self>, kind: &QString, title: &QString, folder: &QString, name: &QString) {
        let kind = kind.to_string();
        if !self.fake.is_empty() {
            let path = self.as_mut().rust_mut().fake.remove(0);
            self.file_chosen(QString::from(&kind), QString::from(&path), QString::default());
            return;
        }
        let (title, folder, mut name) = (title.to_string(), folder.to_string(), name.to_string());
        let export = kind.strip_prefix("export:").map(export::filters);
        if let Some((_, exts)) = export.and_then(|f| f.first()) {
            name = format!("{name}.{}", exts[0]);
        }
        #[cfg(target_os = "macos")]
        {
            let (mode, filters) = platform::dialog(&kind, export);
            let url = |p: PathBuf| if folder.is_empty() { String::new() } else { crate::image::file_url(&p) };
            let (dir, file) = (url(PathBuf::from(&folder)), url(PathBuf::from(&folder).join(&name)));
            self.dialog_wanted(
                QString::from(&kind),
                QString::from(&title),
                QString::from(mode),
                QString::from(&dir),
                QString::from(&file),
                QString::from(&filters.join("\n")),
            );
        }
        #[cfg(target_os = "linux")]
        let thread = self.qt_thread();
        #[cfg(target_os = "linux")]
        std::thread::spawn(move || {
            let folder = (!folder.is_empty()).then(|| PathBuf::from(folder));
            let (path, error) = match platform::choose(&kind, &title, folder.as_deref(), &name, export) {
                Ok(paths) => {
                    let paths: Vec<_> = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
                    (paths.join("\n"), String::new())
                }
                Err(e) => (String::new(), e),
            };
            let _ = thread.queue(move |obj| obj.file_chosen(QString::from(&kind), QString::from(&path), QString::from(&error)));
        });
    }

    fn fake_choice(mut self: Pin<&mut Self>, path: &QString) {
        self.as_mut().rust_mut().fake.push(path.to_string());
    }

    fn exists(&self, path: &QString) -> bool {
        !path.is_empty() && PathBuf::from(path.to_string()).exists()
    }

    fn is_folder(&self, path: &QString) -> bool {
        !path.is_empty() && PathBuf::from(path.to_string()).is_dir()
    }

    fn absolute_path(&self, path: &QString) -> QString {
        let p = PathBuf::from(path.to_string());
        let p = std::fs::canonicalize(&p).or_else(|_| std::path::absolute(&p)).unwrap_or(p);
        QString::from(&p.to_string_lossy().into_owned())
    }

    fn has_pandoc(&self, path: &QString) -> bool {
        export::pandoc::set_program(&path.to_string());
        export::pandoc::available()
    }

    fn spawn(&self, path: &QString) -> bool {
        let path = path.to_string();
        open_window(if path.is_empty() { Vec::new() } else { vec![path] })
    }

    fn spawn_window(&self, path: &QString, folder: &QString) -> bool {
        let (path, folder) = (path.to_string(), folder.to_string());
        let mut args = Vec::new();
        if !folder.is_empty() {
            args.push(format!("--folder={folder}"));
        }
        if !path.is_empty() {
            args.push(path);
        }
        open_window(args)
    }

    fn raise_all(&self) {
        qobject::raise_all_windows();
    }

    fn window_titles(&self) -> QString {
        qobject::window_titles_list()
    }

    fn front_window(&self) -> i32 {
        qobject::front_window_index()
    }

    fn raise_window(&self, i: i32) -> bool {
        qobject::raise_window_at(i)
    }

    fn record_window(&self, path: &QString, folder: &QString) {
        let _ = session::record(&session::dir(), &session::key(self.window), &path.to_string(), &folder.to_string());
    }

    fn forget_window(&self) {
        session::forget(&session::dir(), &session::key(self.window));
    }

    fn focus_next_window(&self) {
        platform::focus_next_window();
    }

    fn show_emoji_picker(&self) -> bool {
        platform::show_emoji_picker()
    }

    fn show_info(&self, path: &QString) {
        platform::show_info(Path::new(&path.to_string()));
    }

    fn other_windows_open(&self) -> bool {
        session::others_open(&session::dir(), &session::key(self.window), platform::alive)
    }

    fn take_session(&self) -> QString {
        let list: Vec<_> = session::take(&session::dir(), platform::alive)
            .into_iter()
            .map(|w| serde_json::json!({"path": w.path, "folder": w.folder}))
            .collect();
        QString::from(&serde_json::Value::from(list).to_string())
    }

    fn code_languages(&self) -> QString {
        QString::from(&crate::code::languages().join("\n"))
    }

    fn math_error(&self, tex: &QString) -> QString {
        QString::from(&super::math::error(&tex.to_string()))
    }

    fn diagram_error(&self, lang: &QString, src: &QString) -> QString {
        QString::from(&super::diagram::error(&lang.to_string(), &src.to_string()))
    }

    fn diagram_error_mark(&self, lang: &QString, src: &QString) -> QString {
        QString::from(&super::diagram::error_mark(&lang.to_string(), &src.to_string()))
    }

    fn math_baseline(&self, tex: &QString) -> f64 {
        super::math::baseline(&tex.to_string())
    }

    fn math_rows(&self, tex: &QString) -> QString {
        QString::from(&serde_json::Value::from(super::math::rows(&tex.to_string())).to_string())
    }

    fn set_math_physics(&self, on: bool) {
        crate::math::set_physics(on);
    }

    fn math_physics(&self, tex: &QString) -> QString {
        QString::from(&crate::physics::expand(&tex.to_string()))
    }

    fn uploader_command(&self, uploader: &QString, custom: &QString, doc: &QString) -> QString {
        let doc = doc.to_string();
        let doc = (!doc.is_empty()).then(|| PathBuf::from(doc));
        QString::from(&crate::upload::command(&uploader.to_string(), &custom.to_string(), doc.as_deref()))
    }

    fn upload_test_image(&self) -> QString {
        // A typo-blue square.
        let rgba: Vec<u8> = [0x3a, 0x7b, 0xd5, 0xff].repeat(64 * 64);
        let png = super::glue::ffi::encode_image(64, 64, &rgba, &QString::from("png"));
        let path = std::env::temp_dir().join("typo-upload-test.png");
        match std::fs::write(&path, png) {
            Ok(()) => QString::from(&*path.to_string_lossy()),
            Err(_) => QString::default(),
        }
    }

    fn download_image(&self, url: &QString) -> QString {
        let dir = std::env::temp_dir().join(format!("typo-{}-downloads", std::process::id()));
        let path = std::fs::create_dir_all(&dir).map_err(|e| e.to_string()).and_then(|_| crate::image::download(&url.to_string(), &dir, false));
        path.map(|p| QString::from(&*p.to_string_lossy())).unwrap_or_default()
    }

    fn serve_folder(&self, folder: &QString) -> i32 {
        use std::io::{Read, Write};
        let folder = PathBuf::from(folder.to_string());
        let Ok(server) = std::net::TcpListener::bind("127.0.0.1:0") else { return 0 };
        let port = server.local_addr().map_or(0, |a| a.port());
        std::thread::spawn(move || {
            // A connection each, as Qt may open one it sends nothing on.
            for mut stream in server.incoming().flatten() {
                let folder = folder.clone();
                std::thread::spawn(move || {
                    let mut request = [0; 4096];
                    let n = stream.read(&mut request).unwrap_or(0);
                    let request = String::from_utf8_lossy(&request[..n]);
                    let path = request.split(' ').nth(1).unwrap_or("/").split('?').next().unwrap_or_default();
                    let file = folder.join(path.trim_start_matches('/'));
                    let body = (!path.contains("..")).then(|| std::fs::read(&file).ok()).flatten();
                    let kind = match file.extension().and_then(|e| e.to_str()) {
                        Some("svg") => "image/svg+xml",
                        Some("png") => "image/png",
                        _ => "application/octet-stream",
                    };
                    let (status, body) = match body {
                        Some(body) => ("200 OK", body),
                        None => ("404 Not Found", Vec::new()),
                    };
                    let head = format!("HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                    let _ = stream.write_all(head.as_bytes()).and_then(|_| stream.write_all(&body));
                });
            }
        });
        port as i32
    }

    fn copy_math_image(&self, tex: &QString, display: bool, em: f64) -> bool {
        // The provider's id; a `%` in the TeX must not read as an escape.
        let id = format!("{}/{em:.2}/000000/{}", if display { "d" } else { "i" }, tex.to_string().replace('%', "%25"));
        qobject::copy_math_image_data(&QString::from(&id))
    }

    fn math_ml(&self, tex: &QString, display: bool, tags: &QString) -> QString {
        let tags: Vec<String> = tags.to_string().lines().map(str::to_string).collect();
        QString::from(&crate::math::mathml(&tex.to_string(), display, &tags).unwrap_or_default())
    }

    fn quit_all(&self) -> bool {
        platform::broadcast("Quit").is_ok()
    }

    fn save_all(&self) -> bool {
        platform::broadcast("Save").is_ok()
    }

    fn listen_for_quit(mut self: Pin<&mut Self>) {
        if self.listening {
            return;
        }
        self.as_mut().rust_mut().listening = true;
        let thread = self.qt_thread();
        platform::on_broadcast(move |what| {
            let _ = match what {
                "Quit" => thread.queue(|obj| obj.quit_requested()),
                "Save" => thread.queue(|obj| obj.save_requested()),
                _ => Ok(()),
            };
        });
    }

    fn screenshot(&self, path: &QString) -> bool {
        qobject::grab_window(path)
    }

    fn send_key(&self, chord: &QString) -> bool {
        qobject::send_key_event(chord)
    }

    fn type_text(&self, text: &QString) -> bool {
        qobject::type_text_events(text)
    }

    fn click(&self, x: f64, y: f64, mods: &QString) -> bool {
        qobject::click_at(x, y, mods)
    }

    fn hover(&self, x: f64, y: f64) -> bool {
        qobject::hover_at(x, y)
    }

    fn wheel(&self, x: f64, y: f64, dy: f64, mods: &QString) -> bool {
        qobject::wheel_at(x, y, dy, mods)
    }

    fn drag(&self, x1: f64, y1: f64, x2: f64, y2: f64) -> bool {
        qobject::drag_between(x1, y1, x2, y2)
    }

    fn mouse(&self, what: i32, x: f64, y: f64) -> bool {
        qobject::mouse_at(what, x, y)
    }

    fn drop_files(&self, x: f64, y: f64, paths: &QString) -> bool {
        qobject::drop_files_at(x, y, paths)
    }

    fn drag_files(&self, stage: i32, x: f64, y: f64, paths: &QString) -> bool {
        qobject::drag_files_at(stage, x, y, paths)
    }

    fn read_file(&self, path: &QString) -> QString {
        qobject::read_file_text(path)
    }

    fn write_file(&self, path: &QString, text: &QString) -> bool {
        qobject::write_file_text(path, text)
    }

    fn set_clipboard(&self, text: &QString, html: &QString, markdown: &QString) {
        qobject::set_clipboard_data(text, html, markdown)
    }

    fn clipboard_markdown(&self) -> QString {
        qobject::clipboard_markdown_data()
    }

    fn clipboard_text(&self) -> QString {
        qobject::clipboard_plain()
    }

    fn clipboard_html(&self) -> QString {
        qobject::clipboard_rich()
    }

    fn clipboard_files(&self) -> QString {
        qobject::clipboard_file_paths()
    }

    fn save_clipboard_image(&self, folder: &QString) -> QString {
        qobject::save_clipboard_image_in(folder)
    }

    fn set_clipboard_files(&self, paths: &QString) {
        qobject::set_clipboard_file_paths(paths)
    }

    fn set_clipboard_image(&self, path: &QString) -> bool {
        qobject::set_clipboard_image_file(path)
    }

    fn move_to_trash(&self, path: &QString) -> bool {
        qobject::move_file_to_trash(path)
    }

    fn show_in_folder(&self, path: &QString) {
        platform::reveal(&PathBuf::from(path.to_string()));
    }

    fn pasted_images_folder(&self) -> QString {
        // Typora keeps them in its settings folder, as typora-user-images.
        QString::from(&*crate::file::config_dir().join("typo-user-images").to_string_lossy())
    }

    fn advanced_settings(&self, reset: bool) -> QString {
        crate::file::advanced_settings(reset).map(|p| QString::from(&*p.to_string_lossy())).unwrap_or_default()
    }

    fn advanced(&self) -> QString {
        QString::from(&crate::file::advanced().to_string())
    }

    fn watch_alt(&self) {
        qobject::watch_alt_taps()
    }

    fn themes_folder(&self) -> QString {
        QString::from(&*crate::palette::user_themes_dir().to_string_lossy())
    }

    fn drafts_folder(&self) -> QString {
        let dir = crate::file::drafts_dir();
        let _ = std::fs::create_dir_all(&dir);
        QString::from(&*dir.to_string_lossy())
    }

    fn plain_text(&self, html: &QString) -> QString {
        qobject::html_text(html)
    }
}
