//! Omarchy theme palettes. Reads a theme's `colors.toml` and resolves every
//! colour typo uses with the same alias and fallback cascade as Omarchy's own
//! `omarchy-theme-color`, so typo agrees with the rest of the desktop for old
//! (ANSI `colorN`) and new (semantic) palettes alike.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn parse(s: &str) -> Option<Rgb> {
        let h = s.trim().strip_prefix('#')?;
        if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let v = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
        Some(Rgb(v(0)?, v(2)?, v(4)?))
    }

    /// `amount` of the way from `self` to `other`, rounded like Omarchy's
    /// `mix_color`.
    pub fn mix(self, other: Rgb, amount: f64) -> Rgb {
        let a = amount.clamp(0.0, 1.0);
        let m = |x: u8, y: u8| (x as f64 * (1.0 - a) + y as f64 * a + 0.5) as u8;
        Rgb(m(self.0, other.0), m(self.1, other.1), m(self.2, other.2))
    }

    /// WCAG relative luminance.
    pub fn luminance(self) -> f64 {
        let lin = |v: u8| {
            let v = v as f64 / 255.0;
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * lin(self.0) + 0.7152 * lin(self.1) + 0.0722 * lin(self.2)
    }

    // HTML export (phase 8) writes theme colours as CSS.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

const BLACK: Rgb = Rgb(0, 0, 0);
const WHITE: Rgb = Rgb(255, 255, 255);

/// `key = "value"` lines of a colors.toml. Values are taken between the
/// first pair of quotes (so inline comments are ignored) or trimmed.
pub fn parse(text: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        let key: String = key.chars().filter(|c| !matches!(c, '"' | '\'' | ' ' | '\t')).collect();
        if key.is_empty() || key.starts_with('#') || key.starts_with('[') {
            continue;
        }
        let value = match value.find(['"', '\'']) {
            Some(q) => {
                let rest = &value[q + 1..];
                &rest[..rest.find(['"', '\'']).unwrap_or(rest.len())]
            }
            None => value.trim(),
        };
        out.insert(key, value.to_string());
    }
    out
}

/// A fully resolved palette: every key Omarchy's templates may use.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    colors: HashMap<String, Rgb>,
    pub dark: bool,
}

impl Palette {
    pub fn get(&self, key: &str) -> Rgb {
        self.colors.get(key).copied().unwrap_or(if self.dark { Rgb(0xbe, 0xbe, 0xbe) } else { BLACK })
    }

    /// Resolve the raw keys of a colors.toml (`dir` is checked for a
    /// `light.mode` marker, as Omarchy does).
    pub fn resolve(raw: &HashMap<String, String>, dir: Option<&Path>) -> Palette {
        let mut c: HashMap<String, Rgb> = raw.iter().filter_map(|(k, v)| Some((k.clone(), Rgb::parse(v)?))).collect();
        let alias = |c: &mut HashMap<String, Rgb>, key: &str, from: &str| {
            if !c.contains_key(key) {
                if let Some(&v) = c.get(from) {
                    c.insert(key.to_string(), v);
                }
            }
        };
        for (key, short) in [
            ("background", "bg"),
            ("dark_background", "dark_bg"),
            ("darker_background", "darker_bg"),
            ("lighter_background", "lighter_bg"),
            ("foreground", "fg"),
            ("dark_foreground", "dark_fg"),
            ("light_foreground", "light_fg"),
            ("bright_foreground", "bright_fg"),
        ] {
            alias(&mut c, key, short);
        }
        alias(&mut c, "background", "color0");
        alias(&mut c, "foreground", "color7");
        c.entry("background".into()).or_insert(Rgb(0x12, 0x12, 0x12));
        c.entry("foreground".into()).or_insert(Rgb(0xbe, 0xbe, 0xbe));
        let (bg, fg) = (c["background"], c["foreground"]);
        c.insert("color0".into(), bg);
        c.insert("color7".into(), fg);
        for (key, n) in [
            ("red", 1),
            ("green", 2),
            ("yellow", 3),
            ("blue", 4),
            ("magenta", 5),
            ("cyan", 6),
            ("bright_red", 9),
            ("bright_green", 10),
            ("bright_yellow", 11),
            ("bright_blue", 12),
            ("bright_magenta", 13),
            ("bright_cyan", 14),
        ] {
            alias(&mut c, key, &format!("color{n}"));
        }
        alias(&mut c, "magenta", "purple");
        alias(&mut c, "bright_magenta", "bright_purple");
        // A palette missing a base hue falls back to the foreground.
        for key in ["red", "green", "yellow", "blue", "magenta", "cyan"] {
            c.entry(key.into()).or_insert(fg);
        }
        let get = |c: &HashMap<String, Rgb>, k: &str| c.get(k).copied();
        let or = |c: &mut HashMap<String, Rgb>, key: &str, v: Rgb| {
            c.entry(key.to_string()).or_insert(v);
        };
        let v = get(&c, "color7").unwrap_or(fg);
        or(&mut c, "light_foreground", v);
        let v = get(&c, "color15").unwrap_or(fg);
        or(&mut c, "bright_foreground", v);
        let v = c["bright_foreground"];
        c.insert("cursor".into(), v);
        or(&mut c, "lighter_background", bg);
        let v = get(&c, "color8").unwrap_or(fg);
        or(&mut c, "dark_foreground", v);
        let v = get(&c, "color8").unwrap_or(c["dark_foreground"]);
        or(&mut c, "muted", v);
        let v = get(&c, "selection_background").or(get(&c, "color8")).unwrap_or(bg);
        or(&mut c, "selection", v);
        let v = c["selection"];
        or(&mut c, "selection_background", v);
        let v = c["bright_foreground"];
        or(&mut c, "selection_foreground", v);
        let v = c["yellow"];
        or(&mut c, "orange", v);
        let v = c["orange"].mix(BLACK, 0.5);
        or(&mut c, "brown", v);
        or(&mut c, "dark_background", bg.mix(BLACK, 0.25));
        or(&mut c, "darker_background", bg.mix(BLACK, 0.5));
        for hue in ["red", "yellow", "green", "cyan", "blue", "magenta"] {
            let v = c[hue].mix(WHITE, 0.2);
            or(&mut c, &format!("bright_{hue}"), v);
        }

        // Legacy ANSI names for consumers that still use them, as
        // omarchy-theme-color resolves them.
        for (n, key) in [
            "background",
            "red",
            "green",
            "yellow",
            "blue",
            "magenta",
            "cyan",
            "foreground",
            "muted",
            "bright_red",
            "bright_green",
            "bright_yellow",
            "bright_blue",
            "bright_magenta",
            "bright_cyan",
            "bright_foreground",
        ]
        .iter()
        .enumerate()
        {
            let v = c[*key];
            or(&mut c, &format!("color{n}"), v);
        }

        let dark = match raw.get("mode").or(raw.get("theme_type")).map(String::as_str) {
            Some("light") => false,
            Some("dark") => true,
            _ if dir.is_some_and(|d| d.join("light.mode").exists()) => false,
            _ => (bg.0 as u32 + bg.1 as u32 + bg.2 as u32) <= 382,
        };
        Palette { colors: c, dark }
    }

    pub fn load(dir: &Path) -> Option<Palette> {
        let text = fs::read_to_string(dir.join("colors.toml")).ok()?;
        Some(Palette::resolve(&parse(&text), Some(dir)))
    }
}

/// The palette colour of each alert kind (`md::query::ALERTS`), after
/// GitHub's blue, green, purple, yellow and red.
pub const ALERT_COLORS: [&str; 5] = ["blue", "green", "magenta", "yellow", "red"];

/// The colours typo draws with, derived from a palette. Omarchy's template
/// for Obsidian is its own mapping of a palette onto a markdown editor
/// (coloured headings, blue links, cyan code, accent quote bars); typo
/// follows the same mapping so documents look as they do there.
#[derive(Clone, Debug, PartialEq)]
pub struct Scheme {
    pub dark: bool,
    pub background: Rgb,
    pub foreground: Rgb,
    pub accent: Rgb,
    /// Readable text on the accent.
    pub accent_text: Rgb,
    pub selection: Rgb,
    pub selection_text: Rgb,
    pub cursor: Rgb,
    /// Sidebar, menus and popups.
    pub panel: Rgb,
    pub border: Rgb,
    /// Secondary text: hints, counts, revealed markdown markers.
    pub muted: Rgb,
    pub faint: Rgb,
    /// Focus mode's text away from the caret: Typora's `--blur-text-color`,
    /// about two thirds of the way from the text to the page.
    pub blur: Rgb,
    pub headings: [Rgb; 6],
    pub link: Rgb,
    pub code: Rgb,
    pub code_background: Rgb,
    /// TeX source of inline math while it is edited.
    pub math: Rgb,
    pub quote: Rgb,
    pub mark: Rgb,
    pub error: Rgb,
    pub success: Rgb,
    pub palette: Palette,
}

impl Scheme {
    pub fn new(p: Palette) -> Scheme {
        let bg = p.get("background");
        let fg = p.get("foreground");
        let accent = p.get("accent");
        // A theme without an accent uses blue, as its links do.
        let accent = if p.colors.contains_key("accent") { accent } else { p.get("blue") };
        Scheme {
            dark: p.dark,
            background: bg,
            foreground: fg,
            accent,
            accent_text: if accent.luminance() > 0.179 { BLACK } else { WHITE },
            selection: p.get("selection"),
            selection_text: p.get("selection_foreground"),
            cursor: p.get("cursor"),
            panel: p.get("dark_background"),
            border: p.get("muted"),
            muted: fg.mix(bg, 0.3),
            faint: fg.mix(bg, 0.45),
            blur: fg.mix(bg, 0.65),
            headings: ["red", "green", "yellow", "blue", "magenta", "magenta"].map(|k| p.get(k)),
            link: p.get("blue"),
            code: p.get("cyan"),
            code_background: p.get("lighter_background"),
            math: p.get("magenta"),
            quote: accent,
            mark: p.get("yellow").mix(bg, 0.6),
            error: p.get("red"),
            success: p.get("green"),
            palette: p,
        }
    }
}

impl Scheme {
    /// The light page PDFs and images are printed on when the theme is
    /// dark, as Typora prints a dark theme's document in its default light
    /// theme: dark grey text on white, blue links, headings in the text
    /// colour, and GitHub's light colours for code and alerts.
    pub fn paper() -> Scheme {
        let raw: HashMap<String, String> = [
            ("background", "#ffffff"),
            ("foreground", "#333333"),
            ("accent", "#4183c4"),
            ("selection", "#b5d6fc"),
            ("selection_foreground", "#333333"),
            ("red", "#cf222e"),
            ("green", "#116329"),
            ("yellow", "#9a6700"),
            ("blue", "#0969da"),
            ("magenta", "#8250df"),
            ("cyan", "#1b7c83"),
            ("color8", "#6e7781"),
            ("lighter_background", "#f8f8f8"),
            ("dark_background", "#f6f8fa"),
            ("muted", "#dddddd"),
            ("mode", "light"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        let mut s = Scheme::new(Palette::resolve(&raw, None));
        s.headings = [s.foreground; 6];
        s.link = s.accent;
        s.code = s.foreground;
        s.quote = Rgb(0xdf, 0xe2, 0xe5);
        s.mark = Rgb(0xff, 0xf3, 0x80);
        // A ticked box is white on blue, as a browser draws it.
        s.accent_text = WHITE;
        s
    }
}

/// An installed Omarchy theme.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeEntry {
    /// Directory name, e.g. `matte-black`.
    pub id: String,
    /// Display name, e.g. `Matte Black`.
    pub title: String,
    pub dir: PathBuf,
}

/// `matte-black` → `Matte Black`, as `omarchy-theme-list` prints it.
pub fn title_of(id: &str) -> String {
    id.split('-')
        .map(|w| {
            let mut cs = w.chars();
            cs.next().map(|f| f.to_uppercase().chain(cs).collect::<String>()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Themes from the user's theme folder and Omarchy's own; a user theme
/// shadows a built-in one with the same id.
pub fn list_themes(user: &Path, system: &Path) -> Vec<ThemeEntry> {
    let mut out: Vec<ThemeEntry> = Vec::new();
    for root in [user, system] {
        let Ok(entries) = fs::read_dir(root) else { continue };
        for e in entries.flatten() {
            let dir = e.path();
            let id = e.file_name().to_string_lossy().into_owned();
            if dir.join("colors.toml").is_file() && !out.iter().any(|t| t.id == id) {
                out.push(ThemeEntry { title: title_of(&id), id, dir });
            }
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

/// Where Omarchy keeps the active theme.
pub fn current_dir() -> PathBuf {
    home().join(".local/state/omarchy/current/theme")
}

pub fn user_themes_dir() -> PathBuf {
    home().join(".config/omarchy/themes")
}

pub fn system_themes_dir() -> PathBuf {
    std::env::var_os("OMARCHY_PATH").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/usr/share/omarchy")).join("themes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_and_bare_values() {
        let m = parse("mode = \"dark\"\n# note\naccent = '#e68e0d' # inline\nbare = #112233\n[section]\n");
        assert_eq!(m["mode"], "dark");
        assert_eq!(m["accent"], "#e68e0d");
        assert_eq!(m["bare"], "#112233");
        assert_eq!(m.len(), 3);
    }

    #[test]
    fn mixes_like_omarchy() {
        // omarchy-theme-color mixes #121212 25% toward black to #0e0e0e.
        assert_eq!(Rgb(0x12, 0x12, 0x12).mix(BLACK, 0.25).hex(), "#0e0e0e");
        assert_eq!(Rgb(0xff, 0, 0).mix(WHITE, 0.2).hex(), "#ff3333");
    }

    #[test]
    fn resolves_legacy_ansi_palettes() {
        let raw = parse(
            "background = \"#1a1b26\"\nforeground = \"#a9b1d6\"\ncolor1 = \"#f7768e\"\ncolor4 = \"#7aa2f7\"\ncolor8 = \"#444b6a\"\nselection_background = \"#7aa2f7\"\n",
        );
        let p = Palette::resolve(&raw, None);
        assert!(p.dark);
        assert_eq!(p.get("red").hex(), "#f7768e");
        assert_eq!(p.get("blue").hex(), "#7aa2f7");
        assert_eq!(p.get("muted").hex(), "#444b6a");
        assert_eq!(p.get("selection").hex(), "#7aa2f7");
        assert_eq!(p.get("dark_background"), Rgb::parse("#1a1b26").unwrap().mix(BLACK, 0.25));
        // Missing hues fall back to the foreground.
        assert_eq!(p.get("green").hex(), "#a9b1d6");
    }

    #[test]
    fn detects_light_themes() {
        let p = Palette::resolve(&parse("background = \"#ffffff\"\nforeground = \"#000000\""), None);
        assert!(!p.dark);
        let p = Palette::resolve(&parse("mode = \"light\"\nbackground = \"#000000\""), None);
        assert!(!p.dark);
    }

    #[test]
    fn scheme_follows_the_obsidian_mapping() {
        let raw = parse(
            "accent = \"#e68e0d\"\nbackground = \"#121212\"\nforeground = \"#bebebe\"\nred = \"#D35F5F\"\ngreen = \"#FFC107\"\nblue = \"#e68e0d\"\ncyan = \"#bebebe\"\n",
        );
        let s = Scheme::new(Palette::resolve(&raw, None));
        assert_eq!(s.headings[0].hex(), "#d35f5f");
        assert_eq!(s.headings[1].hex(), "#ffc107");
        assert_eq!(s.link.hex(), "#e68e0d");
        assert_eq!(s.accent_text, BLACK);
    }

    #[test]
    fn titles_like_theme_list() {
        assert_eq!(title_of("matte-black"), "Matte Black");
        assert_eq!(title_of("retro-82"), "Retro 82");
    }

    #[test]
    fn installed_themes_resolve() {
        // Every theme on this machine must parse without falling back to
        // the built-in colours.
        for t in list_themes(&user_themes_dir(), &system_themes_dir()) {
            let p = Palette::load(&t.dir).expect("colors.toml readable");
            let raw = parse(&fs::read_to_string(t.dir.join("colors.toml")).unwrap());
            if let Some(bg) = raw.get("background").and_then(|v| Rgb::parse(v)) {
                assert_eq!(p.get("background"), bg, "{}", t.id);
            }
        }
    }
}
