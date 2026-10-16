//! The `Theme` QML singleton: the Omarchy palette as semantic colours.
//! QML polls it once a second; a theme switch rewrites the theme directory,
//! so a changed `colors.toml` (mtime, size or inode) triggers a reload.
//! With no Omarchy theme (as on the Mac), it follows the desktop's light or
//! dark.
//! A theme picked from the Themes menu applies to typo only.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qcolor.h");
        type QColor = cxx_qt_lib::QColor;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(bool, dark)]
        #[qproperty(QColor, background)]
        #[qproperty(QColor, foreground)]
        #[qproperty(QColor, accent)]
        #[qproperty(QColor, accent_text, cxx_name = "accentText")]
        #[qproperty(QColor, selection)]
        #[qproperty(QColor, selection_text, cxx_name = "selectionText")]
        #[qproperty(QColor, cursor)]
        #[qproperty(QColor, panel)]
        #[qproperty(QColor, border)]
        #[qproperty(QColor, muted)]
        #[qproperty(QColor, faint)]
        #[qproperty(QColor, blur)]
        #[qproperty(QColor, h1)]
        #[qproperty(QColor, h2)]
        #[qproperty(QColor, h3)]
        #[qproperty(QColor, h4)]
        #[qproperty(QColor, h5)]
        #[qproperty(QColor, h6)]
        #[qproperty(QColor, link)]
        #[qproperty(QColor, code)]
        #[qproperty(QColor, code_background, cxx_name = "codeBackground")]
        #[qproperty(QColor, quote)]
        #[qproperty(QColor, mark)]
        #[qproperty(QColor, error)]
        #[qproperty(QColor, success)]
        /// Bumped on every reload, for bindings that call `color()`.
        #[qproperty(i32, revision)]
        /// Display name of the theme in use.
        #[qproperty(QString, name)]
        /// Id of the theme picked in typo's Themes menu; empty follows Omarchy.
        #[qproperty(QString, chosen)]
        /// Installed themes as `id|Title` pairs.
        #[qproperty(QStringList, themes)]
        type Theme = super::ThemeRust;

        /// Reload when the active colors.toml changed on disk.
        #[qinvokable]
        fn poll(self: Pin<&mut Self>);

        /// Use theme `id` for typo only (empty to follow Omarchy again).
        #[qinvokable]
        fn choose(self: Pin<&mut Self>, id: &QString);

        /// Any resolved palette key (`red`, `bright_blue`, …).
        #[qinvokable]
        fn color(self: &Self, key: &QString) -> QColor;

        /// An alert kind's colour (`note`, `tip`, … as `query::ALERTS`).
        #[qinvokable]
        #[cxx_name = "alertColor"]
        fn alert_color(self: &Self, kind: &QString) -> QColor;
    }
}

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QColor, QString, QStringList};

use crate::palette::{self, Palette, Rgb, Scheme};

fn qcolor(c: Rgb) -> QColor {
    QColor::from_rgb(c.0 as i32, c.1 as i32, c.2 as i32)
}

pub struct ThemeRust {
    dark: bool,
    background: QColor,
    foreground: QColor,
    accent: QColor,
    accent_text: QColor,
    selection: QColor,
    selection_text: QColor,
    cursor: QColor,
    panel: QColor,
    border: QColor,
    muted: QColor,
    faint: QColor,
    blur: QColor,
    h1: QColor,
    h2: QColor,
    h3: QColor,
    h4: QColor,
    h5: QColor,
    h6: QColor,
    link: QColor,
    code: QColor,
    code_background: QColor,
    quote: QColor,
    mark: QColor,
    error: QColor,
    success: QColor,
    revision: i32,
    name: QString,
    chosen: QString,
    themes: QStringList,
    palette: Option<Palette>,
    stamp: Stamp,
}

impl Default for ThemeRust {
    fn default() -> Self {
        let c = QColor::from_rgb(0, 0, 0);
        let mut t = ThemeRust {
            dark: true,
            background: c.clone(),
            foreground: c.clone(),
            accent: c.clone(),
            accent_text: c.clone(),
            selection: c.clone(),
            selection_text: c.clone(),
            cursor: c.clone(),
            panel: c.clone(),
            border: c.clone(),
            muted: c.clone(),
            faint: c.clone(),
            blur: c.clone(),
            h1: c.clone(),
            h2: c.clone(),
            h3: c.clone(),
            h4: c.clone(),
            h5: c.clone(),
            h6: c.clone(),
            link: c.clone(),
            code: c.clone(),
            code_background: c.clone(),
            quote: c.clone(),
            mark: c.clone(),
            error: c.clone(),
            success: c,
            revision: 0,
            name: QString::default(),
            chosen: QString::default(),
            themes: QStringList::default(),
            palette: None,
            stamp: (None, false),
        };
        // Read the theme before QML first binds to it, so the window never
        // flashes the fallback colours.
        let (scheme, name) = load(&t.chosen.to_string());
        t.apply_fields(&scheme);
        super::shared::set_scheme(&scheme);
        t.palette = Some(scheme.palette.clone());
        t.name = QString::from(&name);
        t.stamp = stamp(&source_dir(""));
        t.themes = theme_list();
        t
    }
}

impl ThemeRust {
    fn apply_fields(&mut self, s: &Scheme) {
        self.dark = s.dark;
        self.background = qcolor(s.background);
        self.foreground = qcolor(s.foreground);
        self.accent = qcolor(s.accent);
        self.accent_text = qcolor(s.accent_text);
        self.selection = qcolor(s.selection);
        self.selection_text = qcolor(s.selection_text);
        self.cursor = qcolor(s.cursor);
        self.panel = qcolor(s.panel);
        self.border = qcolor(s.border);
        self.muted = qcolor(s.muted);
        self.faint = qcolor(s.faint);
        self.blur = qcolor(s.blur);
        [self.h1, self.h2, self.h3, self.h4, self.h5, self.h6] = s.headings.map(qcolor);
        self.link = qcolor(s.link);
        self.code = qcolor(s.code);
        self.code_background = qcolor(s.code_background);
        self.quote = qcolor(s.quote);
        self.mark = qcolor(s.mark);
        self.error = qcolor(s.error);
        self.success = qcolor(s.success);
    }
}

/// The directory whose colors.toml is in use.
fn source_dir(chosen: &str) -> PathBuf {
    if !chosen.is_empty() {
        if let Some(t) = palette::list_themes(&palette::user_themes_dir(), &palette::system_themes_dir())
            .into_iter()
            .find(|t| t.id == chosen)
        {
            return t.dir;
        }
    }
    palette::current_dir()
}

/// What tells a change: colors.toml's mtime, size and inode, and whether
/// the desktop is light.
type Stamp = (Option<(i64, i64, u64, u64)>, bool);

fn stamp(dir: &std::path::Path) -> Stamp {
    let m = fs::metadata(dir.join("colors.toml")).ok();
    (m.map(|m| (m.mtime(), m.mtime_nsec(), m.size(), m.ino())), super::glue::ffi::prefers_light())
}

fn load(chosen: &str) -> (Scheme, String) {
    let dir = source_dir(chosen);
    let scheme = match Palette::load(&dir) {
        Some(p) => Scheme::new(p),
        // Typora's own light page, or a dark one.
        None if super::glue::ffi::prefers_light() => Scheme::paper(),
        None => Scheme::night(),
    };
    let name = if chosen.is_empty() || dir == palette::current_dir() {
        fs::read_to_string(dir.parent().unwrap_or(&dir).join("theme.name"))
            .map(|n| palette::title_of(n.trim()))
            .unwrap_or_else(|_| "Omarchy".into())
    } else {
        palette::title_of(chosen)
    };
    (scheme, name)
}

fn theme_list() -> QStringList {
    palette::list_themes(&palette::user_themes_dir(), &palette::system_themes_dir())
        .iter()
        .map(|t| QString::from(&format!("{}|{}", t.id, t.title)))
        .collect()
}

impl qobject::Theme {
    fn reload(mut self: Pin<&mut Self>) {
        let chosen = self.chosen().to_string();
        let (s, name) = load(&chosen);
        self.as_mut().set_dark(s.dark);
        self.as_mut().set_background(qcolor(s.background));
        self.as_mut().set_foreground(qcolor(s.foreground));
        self.as_mut().set_accent(qcolor(s.accent));
        self.as_mut().set_accent_text(qcolor(s.accent_text));
        self.as_mut().set_selection(qcolor(s.selection));
        self.as_mut().set_selection_text(qcolor(s.selection_text));
        self.as_mut().set_cursor(qcolor(s.cursor));
        self.as_mut().set_panel(qcolor(s.panel));
        self.as_mut().set_border(qcolor(s.border));
        self.as_mut().set_muted(qcolor(s.muted));
        self.as_mut().set_faint(qcolor(s.faint));
        self.as_mut().set_blur(qcolor(s.blur));
        self.as_mut().set_h1(qcolor(s.headings[0]));
        self.as_mut().set_h2(qcolor(s.headings[1]));
        self.as_mut().set_h3(qcolor(s.headings[2]));
        self.as_mut().set_h4(qcolor(s.headings[3]));
        self.as_mut().set_h5(qcolor(s.headings[4]));
        self.as_mut().set_h6(qcolor(s.headings[5]));
        self.as_mut().set_link(qcolor(s.link));
        self.as_mut().set_code(qcolor(s.code));
        self.as_mut().set_code_background(qcolor(s.code_background));
        self.as_mut().set_quote(qcolor(s.quote));
        self.as_mut().set_mark(qcolor(s.mark));
        self.as_mut().set_error(qcolor(s.error));
        self.as_mut().set_success(qcolor(s.success));
        self.as_mut().set_name(QString::from(&name));
        super::shared::set_scheme(&s);
        self.as_mut().rust_mut().palette = Some(s.palette);
        let rev = *self.revision() + 1;
        self.as_mut().set_revision(rev);
    }

    fn poll(mut self: Pin<&mut Self>) {
        let now = stamp(&source_dir(&self.chosen().to_string()));
        if now != self.stamp {
            self.as_mut().rust_mut().stamp = now;
            self.as_mut().set_themes(theme_list());
            self.reload();
        }
    }

    fn choose(mut self: Pin<&mut Self>, id: &QString) {
        self.as_mut().set_chosen(id.clone());
        let now = stamp(&source_dir(&id.to_string()));
        self.as_mut().rust_mut().stamp = now;
        self.reload();
    }

    fn color(&self, key: &QString) -> QColor {
        let p = self.palette.as_ref();
        qcolor(p.map_or(Rgb(0x80, 0x80, 0x80), |p| p.get(&key.to_string())))
    }

    fn alert_color(&self, kind: &QString) -> QColor {
        let kind = kind.to_string();
        let i = crate::md::query::ALERTS.iter().position(|k| *k == kind).unwrap_or(0);
        self.color(&QString::from(palette::ALERT_COLORS[i]))
    }
}
