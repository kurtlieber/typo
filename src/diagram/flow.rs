//! flowchart.js (```flow), parsed, laid out and drawn as Typora's copy of
//! flowchart.js 1.x does it, quirks and error messages included: symbols
//! set out from the start a line-length apart, lines routed round them and
//! hopping where they cross, in 14px Arial, lines and text in the text
//! colour. Typora fills symbols white in every theme; here they take the
//! page's colour, so dark themes stay readable.

use std::collections::HashMap;
use std::fmt::Write;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::{Colors, N};

const LINE_WIDTH: f64 = 3.0;
const LINE_LENGTH: f64 = 50.0;
const MARGIN: f64 = 10.0;
const SIZE: f64 = 14.0;
/// Raphaël's 1.2em between lines.
const LEADING: f64 = 16.8;
const FONTS: &str = "Arial, Liberation Sans, Nimbus Sans, sans-serif";
/// About how deep the browser's stack lets flowchart.js walk a chart.
const DEPTH: usize = 1000;

const TOP: usize = 0;
const RIGHT: usize = 1;
const BOTTOM: usize = 2;
const LEFT: usize = 3;
const SIDES: [&str; 4] = ["top", "right", "bottom", "left"];
const PATHS: [&str; 3] = ["path1", "path2", "path3"];

/// The font the SVG asks for, which its text is measured in.
static FONT: LazyLock<Option<(&'static [u8], u32)>> = LazyLock::new(|| {
    use resvg::usvg::fontdb::{Family, Query};
    let families = [Family::Name("Arial"), Family::Name("Liberation Sans"), Family::Name("Nimbus Sans"), Family::SansSerif];
    let id = super::FONT_DB.query(&Query { families: &families, ..Default::default() })?;
    let (data, index) = super::FONT_DB.with_face_data(id, |data, index| (data.to_vec(), index))?;
    Some((Box::leak(data.into_boxed_slice()), index))
});

fn face() -> Option<rustybuzz::Face<'static>> {
    FONT.and_then(|(data, index)| rustybuzz::Face::from_slice(data, index))
}

/// A line's ascent and descent at 14px, each rounded to the pixel, as the
/// browser rounds them.
fn metrics() -> (f64, f64) {
    match face() {
        Some(face) => {
            let scale = SIZE / face.units_per_em() as f64;
            ((face.ascender() as f64 * scale).round(), (-face.descender() as f64 * scale).round())
        }
        None => (13.0, 3.0),
    }
}

/// A line's width at 14px, kerned; a character the font lacks is half
/// an em a column.
fn advance(line: &str) -> f64 {
    let Some(face) = face() else { return line.width() as f64 * 7.0 };
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(line);
    let glyphs = rustybuzz::shape(&face, &[], buffer);
    let scale = SIZE / face.units_per_em() as f64;
    let missing = |cluster: u32| line[cluster as usize..].chars().next().and_then(|c| c.width()).unwrap_or(0) as f64 * 7.0;
    glyphs
        .glyph_infos()
        .iter()
        .zip(glyphs.glyph_positions())
        .map(|(info, at)| if info.glyph_id == 0 { missing(info.cluster) } else { at.x_advance as f64 * scale })
        .sum()
}

/// The text's lines as the browser sets them: spaces run together, and an
/// empty line takes no room.
fn lines(text: &str) -> Vec<String> {
    text.split('\n')
        .map(|line| line.split([' ', '\t']).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect()
}

/// The text's box: its widest line, and a line's height and 1.2em more
/// for each line after the first.
fn size(lines: &[String]) -> (f64, f64) {
    if lines.is_empty() {
        return (0.0, 0.0);
    }
    let (ascent, descent) = metrics();
    (lines.iter().map(|l| advance(l)).fold(0.0, f64::max), ascent + descent + (lines.len() - 1) as f64 * LEADING)
}

/// Text centred on `y`, as Raphaël centres it, from `x` (or to it, `end`).
fn text(out: &mut String, x: f64, y: f64, end: bool, lines: &[String], ink: &str) {
    let (ascent, _) = metrics();
    let top = y - size(lines).1 / 2.0;
    let anchor = if end { r#" text-anchor="end""# } else { "" };
    for (i, line) in lines.iter().enumerate() {
        let _ = write!(
            out,
            r#"<text x="{}" y="{}"{anchor} fill="{ink}" xml:space="preserve">{}</text>"#,
            N(x),
            N(top + ascent + i as f64 * LEADING),
            html_escape::encode_text(line)
        );
    }
}

/// JavaScript's `s.indexOf(c)`.
fn index(s: &str, c: char) -> isize {
    s.find(c).map_or(-1, |i| i as isize)
}

/// JavaScript's `s.substring(a, b)`: each end kept to the string, the two
/// swapped when backwards. Only ever cut at ASCII, so bytes do.
fn substring(s: &str, a: isize, b: isize) -> &str {
    let clamp = |i: isize| (i.max(0) as usize).min(s.len());
    let (a, b) = (clamp(a), clamp(b));
    &s[a.min(b)..a.max(b)]
}

/// What `(…)` holds, if there is a `)`.
fn parens(s: &str) -> Option<&str> {
    let close = index(s, ')');
    (close >= 0).then(|| substring(s, index(s, '(') + 1, close))
}

/// A symbol as written, `key(params)=>type: text|state:>link`, and where
/// flows send it: `next`, `yes`, `no`, `path1`… to a symbol, a direction.
#[derive(Default)]
struct Symbol {
    key: String,
    kind: String,
    text: Option<String>,
    params: HashMap<String, String>,
    styles: HashMap<String, Value>,
    links: HashMap<String, usize>,
    dirs: HashMap<String, String>,
    yes_label: Option<String>,
    no_label: Option<String>,
}

#[derive(Default)]
struct Parsed {
    symbols: Vec<Symbol>,
    keys: HashMap<String, usize>,
    start: Option<usize>,
}

/// flowchart.js's parser. A line is a symbol (`=>`), a flow (`->`) or a
/// line style (`@>`); a line with none of them goes on the one before, a
/// `\` at a line's end joins the next, and the first line alone is dropped
/// if it is neither.
fn parse(src: &str) -> Result<Parsed, String> {
    let src = src.trim();
    let bytes = src.as_bytes();
    let mut pieces: Vec<String> = Vec::new();
    let mut from = 0;
    for o in 1..bytes.len() {
        if bytes[o] == b'\n' && bytes[o - 1] != b'\\' {
            pieces.push(src[from..o].replace("\\\n", "\n"));
            from = o + 1;
        }
    }
    if from < src.len() {
        pieces.push(src[from..].to_string());
    }
    let arrow = |p: &str| p.contains("->") || p.contains("=>") || p.contains("@>");
    let mut v = 1;
    while v < pieces.len() {
        if arrow(&pieces[v]) {
            v += 1;
        } else {
            let piece = pieces.remove(v);
            pieces[v - 1] = format!("{}\n{piece}", pieces[v - 1]);
        }
    }
    let mut d = Parsed::default();
    for p in &pieces {
        let p = p.trim();
        if p.contains("=>") {
            d.symbol(p);
        } else if p.contains("->") {
            d.flow(p)?;
        } else if p.contains("@>") {
            d.style(p)?;
        }
    }
    Ok(d)
}

impl Parsed {
    /// The symbol a flow names: `key`, or `key(…)` when there is a `)`.
    fn get(&self, t: &str) -> Option<usize> {
        let key = if t.contains(')') { substring(t, 0, index(t, '(')) } else { t };
        self.keys.get(key).copied()
    }

    fn symbol(&mut self, p: &str) {
        static PARAMS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\((.*)\)").unwrap());
        let mut g = p.split("=>");
        let (head, mut kind) = (g.next().unwrap_or(""), g.next().unwrap_or("").to_string());
        let mut s = Symbol::default();
        match PARAMS.captures(head) {
            Some(c) => {
                let all = c.get(0).unwrap();
                s.key = format!("{}{}", &head[..all.start()], &head[all.end()..]);
                for param in c[1].split(',') {
                    if let [k, v] = param.split('=').collect::<Vec<_>>()[..] {
                        s.params.insert(k.into(), v.into());
                    }
                }
            }
            None => s.key = head.to_string(),
        }
        let before = |s: &str, sep: &str| s.split_once(sep).map_or(s, |(a, _)| a).to_string();
        let mut text = None;
        if let Some((k, t)) = kind.split_once(": ") {
            (text, kind) = (Some(t.to_string()), k.to_string());
        }
        // `:$function` or `:>link` follows, and only the first is cut.
        let has = |t: &Option<String>, sep| t.as_deref().is_some_and(|t| t.contains(sep));
        if has(&text, ":$") {
            text = text.map(|t| before(&t, ":$"));
        } else if kind.contains(":$") {
            kind = before(&kind, ":$");
        } else if has(&text, ":>") {
            text = text.map(|t| before(&t, ":>"));
        } else if kind.contains(":>") {
            kind = before(&kind, ":>");
        }
        kind = before(&kind, "\n");
        // `|state` ends the text.
        if let Some(t) = text.as_deref().and_then(|t| t.rsplit_once('|')).map(|(t, _)| t.to_string()) {
            text = Some(t);
        }
        (s.kind, s.text) = (kind, text);
        self.keys.insert(s.key.clone(), self.symbols.len());
        self.symbols.push(s);
    }

    /// `a->b(yes, right)->c`: each symbol sends a branch, `next` unless
    /// named, to the one after, in a direction. `c(yes@Label)` labels it.
    fn flow(&mut self, p: &str) -> Result<(), String> {
        let mut inner = substring(p, index(p, '(') + 1, index(p, ')'));
        if let Some(comma) = inner.find(',').filter(|&c| c > 0) {
            inner = &inner[..comma];
        }
        let mut label = inner.split('@').nth(1).filter(|l| index(p, ')') >= 0 && !l.is_empty()).map(str::to_string);
        let p = match &label {
            Some(l) => p.replacen(&format!("@{l}"), "", 1),
            None => p.to_string(),
        };
        let steps: Vec<&str> = p.split("->").collect();
        for (b, step) in steps.iter().enumerate() {
            let mut a = step.to_string();
            if matches!(parens(&a), Some("true" | "false")) {
                a = a.replacen("true", "yes", 1).replacen("false", "no", 1);
            }
            let mut branch = match parens(&a) {
                Some(e) if !e.contains(',') && e != "yes" && e != "no" => format!("next, {e}"),
                Some(e) => e.to_string(),
                None => "next".into(),
            };
            let from = self.get(&a);
            let mut dir = None;
            if let Some((first, rest)) = branch.split_once(',') {
                let rest = rest.split(',').next().unwrap_or("").trim().to_string();
                (branch, dir) = (first.to_string(), Some(rest));
            }
            if let Some(l) = label.take() {
                let yes = branch == "yes" || branch == "true";
                let field = if yes { "yes_annotation" } else { "no_annotation" };
                let s = &mut self.symbols[from.ok_or_else(|| format!("Cannot set properties of undefined (setting '{field}')"))?];
                *if yes { &mut s.yes_label } else { &mut s.no_label } = Some(l);
            }
            if self.start.is_none() {
                self.start = from;
            }
            if b + 1 < steps.len() {
                let s = from.ok_or_else(|| format!("Cannot set properties of undefined (setting '{branch}')"))?;
                let to = self.get(steps[b + 1]);
                let s = &mut self.symbols[s];
                match to {
                    Some(to) => s.links.insert(branch.clone(), to),
                    None => s.links.remove(&branch),
                };
                match dir.filter(|d| !d.is_empty()) {
                    Some(dir) => s.dirs.insert(branch, dir),
                    None => s.dirs.remove(&branch),
                };
            }
        }
        Ok(())
    }

    /// `a@>b({"stroke":"red"})`: how the line from a to b is drawn.
    fn style(&mut self, p: &str) -> Result<(), String> {
        let steps: Vec<&str> = p.split("@>").collect();
        for pair in steps.windows(2) {
            let from = self.get(pair[0]).ok_or("Cannot read properties of undefined (reading 'lineStyle')")?;
            let to = self.get(pair[1]).ok_or("Cannot read properties of undefined (reading 'key')")?;
            let style = json(parens(pair[1]).unwrap_or("{}"))?;
            let key = self.symbols[to].key.clone();
            self.symbols[from].styles.insert(key, style);
        }
        Ok(())
    }
}

/// `JSON.parse`, in V8's words when it fails.
fn json(s: &str) -> Result<Value, String> {
    serde_json::from_str(s).map_err(|e| {
        let at = s.lines().nth(e.line().saturating_sub(1)).and_then(|l| l.chars().nth(e.column().saturating_sub(1)));
        match at {
            Some(c) if !e.is_eof() => format!("Unexpected token '{c}', \"{s}\" is not valid JSON"),
            _ => "Unexpected end of JSON input".into(),
        }
    })
}

#[derive(Clone, Copy, PartialEq, Default, Debug)]
enum Kind {
    Start,
    End,
    Operation,
    InputOutput,
    Subroutine,
    Condition,
    Parallel,
    /// flowchart.js's `new Error("Wrong symbol type!")`, which it goes on
    /// with as if it were a symbol.
    #[default]
    Error,
}

/// The branches a condition or parallel takes, as flows name them.
fn branches(kind: Kind) -> &'static [&'static str] {
    match kind {
        Kind::Condition => &["yes", "no"],
        Kind::Parallel => &PATHS,
        _ => &[],
    }
}

/// A symbol as drawn.
#[derive(Default)]
struct Sym {
    kind: Kind,
    /// The parsed symbol it was made from.
    of: usize,
    text: Vec<String>,
    text_x: f64,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    positioned: bool,
    path_ok: bool,
    handled: bool,
    next: Option<usize>,
    next_dir: Option<String>,
    /// A condition's yes and no, a parallel's three paths: each its
    /// symbol, direction and label.
    out: [Option<usize>; 3],
    dirs: [String; 3],
    labels: [String; 3],
    /// Whether a symbol set beside this moves past ones below it.
    align: bool,
    /// How many lines leave or reach each side.
    lines: [usize; 4],
    left_end: bool,
}

impl Sym {
    fn new(kind: Kind, of: usize, p: &mut Symbol) -> Sym {
        let mut s = Sym { kind, of, x: LINE_WIDTH, y: LINE_WIDTH, align: true, ..Default::default() };
        let text = match kind {
            Kind::Error => return s,
            Kind::Start => p.text.as_deref().filter(|t| !t.is_empty()).unwrap_or("Start"),
            Kind::End => p.text.as_deref().filter(|t| !t.is_empty()).unwrap_or("End"),
            // Raphaël writes no text as "null".
            _ => p.text.as_deref().unwrap_or("null"),
        };
        s.text = lines(text);
        let (tw, th) = size(&s.text);
        (s.w, s.h, s.text_x) = match kind {
            Kind::Subroutine => (tw + 4.0 * MARGIN, th + 2.0 * MARGIN, 2.0 * MARGIN),
            Kind::InputOutput => (tw + 6.0 * MARGIN, th + 2.0 * MARGIN, 3.0 * MARGIN),
            Kind::Condition => {
                let w = (tw + 3.0 * MARGIN) * 1.5;
                (w, f64::max(w / 2.0, (th + 2.0 * MARGIN) * 1.5), w / 4.0 + MARGIN / 2.0)
            }
            _ => (tw + 2.0 * MARGIN, th + 2.0 * MARGIN, MARGIN),
        };
        match kind {
            Kind::Condition => {
                let (mut yes, mut no) = (p.dirs.get("yes").map(String::as_str), p.dirs.get("no").map(String::as_str));
                if no.is_none() && yes == Some("right") {
                    no = Some("bottom");
                } else if yes.is_none() && no == Some("bottom") {
                    yes = Some("right");
                }
                s.dirs = [yes.unwrap_or("bottom").into(), no.unwrap_or("right").into(), String::new()];
                s.labels = [p.yes_label.clone().unwrap_or("yes".into()), p.no_label.clone().unwrap_or("no".into()), String::new()];
            }
            Kind::Parallel => {
                // `p(path1)->x` sends path1 to x.
                if let Some(path) = p.dirs.get("next").filter(|d| PATHS.contains(&d.as_str())).cloned() {
                    if let Some(&next) = p.links.get("next").filter(|_| !p.links.contains_key(&path)) {
                        p.links.insert(path, next);
                    }
                }
                let d = PATHS.map(|k| p.dirs.get(k).map(String::as_str));
                let all = PATHS.iter().all(|k| p.links.contains_key(*k));
                let only = |k: usize| all && (0..3).all(|j| d[j].is_some() == (j == k));
                let dirs = if only(0) {
                    match d[0] {
                        Some("right") => ["right", "bottom", "top"],
                        Some("top") => ["top", "right", "bottom"],
                        Some("left") => ["left", "right", "bottom"],
                        _ => ["bottom", "right", "top"],
                    }
                } else if only(1) {
                    match d[1] {
                        Some("right") => ["bottom", "right", "top"],
                        Some("left") => ["bottom", "left", "right"],
                        _ => ["right", "bottom", "top"],
                    }
                } else if only(2) {
                    // flowchart.js looks at path2's direction here, never set.
                    ["right", "bottom", "top"]
                } else {
                    [0, 1, 2].map(|k| d[k].unwrap_or(["bottom", "right", "top"][k]))
                };
                s.dirs = dirs.map(String::from);
            }
            _ => s.next_dir = p.dirs.get("next").filter(|_| p.links.contains_key("next")).cloned(),
        }
        if matches!(kind, Kind::Condition | Kind::Parallel) {
            s.align = p.params.get("align-next").is_none_or(|v| v != "no");
        }
        s
    }

    fn centre(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    fn top(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y)
    }

    fn bottom(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h)
    }

    /// An input/output's sides are its slant, a margin in.
    fn left(&self) -> (f64, f64) {
        let inset = if self.kind == Kind::InputOutput { MARGIN } else { 0.0 };
        (self.x + inset, self.y + self.h / 2.0)
    }

    fn right(&self) -> (f64, f64) {
        let inset = if self.kind == Kind::InputOutput { MARGIN } else { 0.0 };
        (self.x + self.w - inset, self.y + self.h / 2.0)
    }
}

/// A line: its path (a point for the move and each line, three for a
/// hop's curve), how it is stroked, and its label.
struct Line {
    path: Vec<Vec<f64>>,
    stroke: Option<String>,
    width: f64,
    dash: Option<&'static [f64]>,
    arrow: String,
    label: Option<(f64, f64, bool, Vec<String>)>,
}

impl Line {
    /// The `@>` style's stroke, width, dashes and arrow, as Raphaël takes them.
    fn style(&mut self, style: &serde_json::Map<String, Value>) {
        if let Some(Value::String(stroke)) = style.get("stroke") {
            self.stroke = Some(stroke.clone());
        }
        let width = match style.get("stroke-width") {
            Some(Value::Number(n)) => n.as_f64(),
            Some(Value::String(s)) => s.trim().parse().ok(),
            _ => None,
        };
        self.width = width.unwrap_or(self.width);
        if let Some(Value::String(dash)) = style.get("stroke-dasharray") {
            self.dash = match dash.to_lowercase().as_str() {
                "-" => Some(&[3.0, 1.0]),
                "." => Some(&[1.0, 1.0]),
                "-." => Some(&[3.0, 1.0, 1.0, 1.0]),
                "-.." => Some(&[3.0, 1.0, 1.0, 1.0, 1.0, 1.0]),
                ". " => Some(&[1.0, 3.0]),
                "- " => Some(&[4.0, 3.0]),
                "--" => Some(&[8.0, 3.0]),
                "- ." => Some(&[4.0, 3.0, 1.0, 3.0]),
                "--." => Some(&[8.0, 3.0, 1.0, 3.0]),
                "--.." => Some(&[8.0, 3.0, 1.0, 3.0, 1.0, 3.0]),
                _ => None,
            };
        }
        if let Some(Value::String(arrow)) = style.get("arrow-end") {
            self.arrow = arrow.clone();
        }
    }

    /// The line, stopped short where its arrowhead's tip would overshoot,
    /// the head (Raphaël's marker, drawn in place), then the label.
    fn draw(&self, out: &mut String, ink: &str) {
        let stroke = self.stroke.as_deref().map_or(ink.to_string(), |s| html_escape::encode_double_quoted_attribute(s).into_owned());
        // `type-width-length`, the first of each that is given counting.
        let (mut kind, mut w, mut h, mut t) = ("classic", 3.0, 3.0, 5.0);
        for part in self.arrow.to_lowercase().split('-').rev() {
            match part {
                "block" => kind = "block",
                "classic" => kind = "classic",
                "oval" => kind = "oval",
                "diamond" => kind = "diamond",
                "open" => kind = "open",
                "none" => kind = "none",
                "wide" => h = 5.0,
                "narrow" => h = 2.0,
                "long" => w = 5.0,
                "short" => w = 2.0,
                _ => {}
            }
        }
        let open = kind == "open";
        if open {
            (w, h, t) = (w + 2.0, h + 2.0, t + 2.0);
        }
        let (ref_x, short) = if open { (4.0, 1.0) } else { (w / 2.0, w / 2.0) };
        let short = if matches!(kind, "diamond" | "oval" | "none") { 0.0 } else { short * self.width };
        let mut path = self.path.clone();
        let last = path.len() - 1;
        let end = (path[last][path[last].len() - 2], path[last][path[last].len() - 1]);
        let from = match path[last].len() {
            6 => (path[last][2], path[last][3]),
            _ if last > 0 => (path[last - 1][path[last - 1].len() - 2], path[last - 1][path[last - 1].len() - 1]),
            _ => end,
        };
        let (dx, dy) = (end.0 - from.0, end.1 - from.1);
        let length = dx.hypot(dy);
        let tip = if length > 0.0 {
            let cut = short.min(length) / length;
            (end.0 - dx * cut, end.1 - dy * cut)
        } else {
            end
        };
        let n = path[last].len();
        (path[last][n - 2], path[last][n - 1]) = tip;
        let mut d = String::new();
        for (i, segment) in path.iter().enumerate() {
            let command = if i == 0 { "M" } else if segment.len() == 6 { " C" } else { " L" };
            let points: Vec<String> = segment.chunks(2).map(|p| format!("{},{}", N(p[0]), N(p[1]))).collect();
            let _ = write!(d, "{command}{}", points.join(" "));
        }
        let dash = self.dash.map_or(String::new(), |dash| {
            let dash: Vec<String> = dash.iter().map(|v| N(v * self.width).to_string()).collect();
            format!(r#" stroke-dasharray="{}""#, dash.join(","))
        });
        let _ = write!(out, r#"<path d="{d}" fill="none" stroke="{stroke}" stroke-width="{}"{dash}/>"#, N(self.width));
        let head = match kind {
            "block" => "M5,0 0,2.5 5,5z",
            "classic" => "M5,0 0,2.5 5,5 3.5,3 3.5,2z",
            "diamond" => "M2.5,0 5,2.5 2.5,5 0,2.5z",
            "open" => "M6,1 1,3.5 6,6",
            "oval" => "M2.5,0A2.5,2.5,0,0,1,2.5,5 2.5,2.5,0,0,1,2.5,0z",
            _ => "",
        };
        if !head.is_empty() {
            let paint = if open {
                format!(r#"fill="none" stroke="{stroke}" stroke-width="{}" stroke-linecap="round""#, N(1.0 / ((w / t + h / t) / 2.0)))
            } else {
                format!(r#"fill="{stroke}""#)
            };
            let _ = write!(
                out,
                r#"<path d="{head}" transform="translate({},{}) rotate({}) scale({}) translate({},{}) rotate(180 {} {}) scale({},{})" {paint}/>"#,
                N(tip.0),
                N(tip.1),
                N(dy.atan2(dx).to_degrees()),
                N(self.width),
                N(-ref_x),
                N(-h / 2.0),
                N(w / 2.0),
                N(h / 2.0),
                w / t,
                h / t
            );
        }
        if let Some((x, y, end, lines)) = &self.label {
            text(out, *x, *y, *end, lines, ink);
        }
    }
}

/// Where segment (x1,y1)–(x2,y2) crosses (x3,y3)–(x4,y4), inside both.
#[allow(clippy::too_many_arguments)]
fn cross(x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, x4: f64, y4: f64) -> Option<(f64, f64)> {
    let v = (y4 - y3) * (x2 - x1) - (x4 - x3) * (y2 - y1);
    if v == 0.0 {
        return None;
    }
    let (dy, dx) = (y1 - y3, x1 - x3);
    let ua = ((x4 - x3) * dy - (y4 - y3) * dx) / v;
    let ub = ((x2 - x1) * dy - (y2 - y1) * dx) / v;
    (ua > 0.0 && ua < 1.0 && ub > 0.0 && ub < 1.0).then(|| (x1 + ua * (x2 - x1), y1 + ua * (y2 - y1)))
}

/// The path's box, its curves' bulges included.
fn bounds(path: &[Vec<f64>]) -> (f64, f64, f64, f64) {
    let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    let mut add = |x: f64, y: f64| b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
    // Where a cubic's derivative is 0, inside it.
    let turns = |p0: f64, p1: f64, p2: f64, p3: f64| -> Vec<f64> {
        let (a, b, c) = (3.0 * (-p0 + 3.0 * p1 - 3.0 * p2 + p3), 6.0 * (p0 - 2.0 * p1 + p2), 3.0 * (p1 - p0));
        let roots = if a.abs() < 1e-12 {
            if b.abs() < 1e-12 { vec![] } else { vec![-c / b] }
        } else {
            let disc = b * b - 4.0 * a * c;
            if disc < 0.0 { vec![] } else { vec![(-b + disc.sqrt()) / (2.0 * a), (-b - disc.sqrt()) / (2.0 * a)] }
        };
        roots.into_iter().filter(|t| *t > 0.0 && *t < 1.0).collect()
    };
    let at = |p0: f64, p1: f64, p2: f64, p3: f64, t: f64| {
        let u = 1.0 - t;
        u * u * u * p0 + 3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t * p3
    };
    let mut from = (path[0][0], path[0][1]);
    for s in path {
        if s.len() == 6 {
            let (x, y) = ([from.0, s[0], s[2], s[4]], [from.1, s[1], s[3], s[5]]);
            for t in turns(x[0], x[1], x[2], x[3]).into_iter().chain(turns(y[0], y[1], y[2], y[3])) {
                add(at(x[0], x[1], x[2], x[3], t), at(y[0], y[1], y[2], y[3], t));
            }
        }
        from = (s[s.len() - 2], s[s.len() - 1]);
        add(from.0, from.1);
    }
    b
}

#[derive(Default)]
struct Chart {
    parsed: Parsed,
    syms: Vec<Sym>,
    /// Symbols made, by key.
    made: HashMap<String, usize>,
    /// The chart's symbols, in the order they joined it.
    order: Vec<usize>,
    start: Option<usize>,
    lines: Vec<Line>,
    min_x: Option<f64>,
    /// How far right a line reaches, roughly; 0 is unset.
    max_x: f64,
}

impl Chart {
    /// flowchart.js's walk from the start: each symbol made once per key,
    /// joined to the one it was reached from unless that one has all its
    /// branches; then on to its own.
    fn visit(&mut self, e: usize, parent: Option<(usize, usize)>, depth: usize) -> Result<(), String> {
        if depth > DEPTH {
            return Err("Maximum call stack size exceeded".into());
        }
        let n = self.make(e);
        if Some(e) == self.parsed.start {
            self.start = Some(n);
            self.handle(n);
        } else if let Some((i, r)) = parent.filter(|&(i, _)| !self.syms[i].path_ok) {
            match self.syms[i].kind {
                kind @ (Kind::Condition | Kind::Parallel) => {
                    for (k, name) in branches(kind).iter().enumerate() {
                        if self.parsed.symbols[r].links.get(*name) == Some(&e) {
                            self.syms[i].out[k] = Some(n);
                            if self.syms[i].out[(k + 1) % branches(kind).len()].is_some() {
                                self.syms[i].path_ok = true;
                            }
                            self.handle(n);
                        }
                    }
                }
                Kind::Error if !self.syms[i].handled => return Err("i.then is not a function".into()),
                _ => {
                    (self.syms[i].next, self.syms[i].path_ok) = (Some(n), true);
                    self.handle(n);
                }
            }
        }
        if !self.syms[n].path_ok {
            let kind = self.syms[n].kind;
            let names = if branches(kind).is_empty() { &["next"][..] } else { branches(kind) };
            for name in names {
                if let Some(&c) = self.parsed.symbols[e].links.get(*name) {
                    self.visit(c, Some((n, e)), depth + 1)?;
                }
            }
        }
        Ok(())
    }

    fn make(&mut self, e: usize) -> usize {
        let key = self.parsed.symbols[e].key.clone();
        if let Some(&n) = self.made.get(&key) {
            return n;
        }
        let kind = match self.parsed.symbols[e].kind.as_str() {
            "start" => Kind::Start,
            "end" => Kind::End,
            "operation" => Kind::Operation,
            "inputoutput" => Kind::InputOutput,
            "subroutine" => Kind::Subroutine,
            "condition" => Kind::Condition,
            "parallel" => Kind::Parallel,
            _ => Kind::Error,
        };
        self.syms.push(Sym::new(kind, e, &mut self.parsed.symbols[e]));
        let n = self.syms.len() - 1;
        if kind != Kind::Error {
            self.made.insert(key, n);
        }
        n
    }

    fn handle(&mut self, n: usize) {
        if !self.order.contains(&n) {
            self.order.push(n);
        }
        self.syms[n].handled = true;
    }

    /// Centre the symbols in the widest and tallest one's box, set them out
    /// from the start, then draw their lines.
    fn layout(&mut self) -> Result<(), String> {
        if self.order.iter().any(|&n| self.syms[n].kind == Kind::Error) {
            return Err("h.shiftX is not a function".into());
        }
        let (w, h) = self.order.iter().fold((0.0, 0.0), |(w, h), &n| (f64::max(w, self.syms[n].w), f64::max(h, self.syms[n].h)));
        for &n in &self.order {
            let s = &mut self.syms[n];
            s.x += (w - s.w) / 2.0 + LINE_WIDTH;
            s.y += (h - s.h) / 2.0 + LINE_WIDTH;
        }
        if let Some(start) = self.start {
            self.render(start);
        }
        for i in 0..self.order.len() {
            self.render_lines(self.order[i]);
        }
        Ok(())
    }

    /// Set out what `s` leads to: below it, unless a direction says beside
    /// (or, for a parallel, above).
    fn render(&mut self, s: usize) {
        let kind = self.syms[s].kind;
        if branches(kind).is_empty() {
            if let Some(t) = self.syms[s].next {
                let side = match self.syms[s].next_dir.as_deref() {
                    Some("right") => RIGHT,
                    Some("left") => LEFT,
                    _ => BOTTOM,
                };
                self.place(s, t, side);
            }
            return;
        }
        let mut slot = [None; 4];
        for k in 0..branches(kind).len() {
            if let Some(side) = SIDES.iter().position(|&d| d == self.syms[s].dirs[k]) {
                slot[side] = self.syms[s].out[k];
            }
        }
        let order: &[usize] = if kind == Kind::Condition { &[BOTTOM, RIGHT, LEFT] } else { &[BOTTOM, TOP, LEFT, RIGHT] };
        for &side in order {
            if let Some(t) = slot[side] {
                self.place(s, t, side);
            }
        }
    }

    /// Put `t` a line-length off `s`'s side, unless it has a place.
    fn place(&mut self, s: usize, t: usize, side: usize) {
        if self.syms[t].positioned {
            return;
        }
        let from = &self.syms[s];
        let (x, y, w, h, align) = (from.x, from.y, from.w, from.h, from.align);
        let (anchor, top) = (if side == LEFT { from.left() } else { from.right() }, from.top());
        let to = &mut self.syms[t];
        match side {
            BOTTOM => (to.y, to.x) = (to.y + y + h + LINE_LENGTH, x + w / 2.0 - to.w / 2.0),
            TOP => (to.y, to.x) = (to.y + y - to.h - LINE_LENGTH, top.0 + to.w),
            RIGHT => (to.y, to.x) = (anchor.1 - to.h / 2.0, to.x + x + w + LINE_LENGTH),
            _ => (to.y, to.x) = (anchor.1 - to.h / 2.0, to.x - (x + w + LINE_LENGTH)),
        }
        if (side == LEFT || side == RIGHT) && align {
            // Past any symbol below it in its column, but an end.
            while self.syms[t].kind != Kind::End {
                let ((cx, cy), w) = (self.syms[t].centre(), self.syms[t].w);
                let below = self.order.iter().copied().find(|&n| {
                    let (nx, ny) = self.syms[n].centre();
                    ny > cy && (nx - cx).abs() <= w / 2.0
                });
                let Some(n) = below else { break };
                self.syms[t].x = self.syms[n].x + self.syms[n].w + LINE_LENGTH;
            }
        }
        self.syms[t].positioned = true;
        self.render(t);
    }

    fn render_lines(&mut self, s: usize) {
        let kind = self.syms[s].kind;
        if branches(kind).is_empty() {
            if let Some(t) = self.syms[s].next {
                let dir = self.syms[s].next_dir.clone();
                self.line_to(s, t, "", dir.as_deref());
            }
        }
        for k in 0..branches(kind).len() {
            if let Some(t) = self.syms[s].out[k] {
                let (label, dir) = (self.syms[s].labels[k].clone(), self.syms[s].dirs[k].clone());
                self.line_to(s, t, &label, Some(&dir));
            }
        }
    }

    /// flowchart.js's `drawLineTo`: a line from `s` to `t`, leaving by
    /// `dir` (any side that suits, unset), routed by where `t` lies, a
    /// little higher for each line already at the sides it joins.
    fn line_to(&mut self, s: usize, t: usize, label: &str, dir: Option<&str>) {
        let (a, h, v, l) = (self.syms[s].right(), self.syms[s].bottom(), self.syms[s].top(), self.syms[s].left());
        let (f, p, d) = (self.syms[t].top(), self.syms[t].right(), self.syms[t].left());
        let ((sx, sy), (tx, ty)) = (self.syms[s].centre(), self.syms[t].centre());
        let (same_col, same_row, below, above) = (sx == tx, sy == ty, sy < ty, sy > ty || s == t);
        let (t_left, t_right) = (sx > tx, sx < tx);
        let is = |side: &str| dir.is_none_or(|d| d == side);
        let [st, sr, sb, sl] = self.syms[s].lines;
        let [tt, tr, _, tl] = self.syms[t].lines;
        let gap = |a: usize, b: usize| 10.0 * a.max(b) as f64;
        let m = LINE_LENGTH;
        let around = |n: f64| {
            let x = h.0 + (h.0 - f.0) / 2.0;
            vec![h, (h.0, h.1 + m / 2.0 - n), (x, h.1 + m / 2.0 - n), (x, f.1 - m / 2.0 - n), (f.0, f.1 - m / 2.0 - n), f]
        };
        let over = |n: f64| vec![h, (h.0, f.1 - m / 2.0 - n), (f.0, f.1 - m / 2.0 - n), f];
        let route = if is("bottom") && same_col && below {
            let pts = if tt == 0 && sb == 0 { vec![h, f] } else { vec![h, (f.0, f.1 - gap(tt, sb)), f] };
            Some((pts, BOTTOM, TOP, h.0))
        } else if is("right") && same_row && t_right {
            self.syms[t].left_end = true;
            let n = gap(tl, sr);
            let pts = if tl == 0 && sr == 0 { vec![a, d] } else { vec![a, (a.0, a.1 - n), (a.0, d.1 - n), (d.0, d.1 - n), d] };
            Some((pts, RIGHT, LEFT, d.0))
        } else if is("left") && same_row && t_left {
            let n = gap(tr, sl);
            // flowchart.js starts the raised line from the right.
            let pts = if tr == 0 && sl == 0 { vec![l, p] } else { vec![a, (a.0, a.1 - n), (a.0, p.1 - n), (p.0, p.1 - n), p] };
            Some((pts, LEFT, RIGHT, p.0))
        } else if is("right") && same_col && (above || below) {
            let n = gap(tt, sr);
            Some((vec![a, (a.0 + m / 2.0, a.1 - n), (a.0 + m / 2.0, f.1 - m / 2.0 - n), (f.0, f.1 - m / 2.0 - n), f], RIGHT, TOP, a.0 + m / 2.0))
        } else if is("bottom") && t_left {
            let n = gap(tt, sb);
            Some((if self.syms[s].left_end && above { around(n) } else { over(n) }, BOTTOM, TOP, h.0 + (h.0 - f.0) / 2.0))
        } else if is("bottom") && t_right && below {
            Some((over(gap(tt, sb)), BOTTOM, TOP, h.0.max(f.0)))
        } else if is("bottom") && t_right {
            Some((around(gap(tt, sb)), BOTTOM, TOP, h.0 + (h.0 - f.0) / 2.0))
        } else if dir == Some("right") && t_left {
            let n = gap(tt, sr);
            Some((vec![a, (a.0 + m / 2.0, a.1), (a.0 + m / 2.0, f.1 - m / 2.0 - n), (f.0, f.1 - m / 2.0 - n), f], RIGHT, TOP, a.0 + m / 2.0))
        } else if dir == Some("right") && t_right {
            let n = gap(tt, sr);
            Some((vec![a, (f.0, a.1 - n), (f.0, f.1 - n)], RIGHT, TOP, a.0 + m / 2.0))
        } else if dir == Some("bottom") && same_col && above {
            let n = gap(tt, sb);
            let x = a.0 + m / 2.0;
            Some((vec![h, (h.0, h.1 + m / 2.0 - n), (x, h.1 + m / 2.0 - n), (x, f.1 - m / 2.0 - n), (f.0, f.1 - m / 2.0 - n), f], BOTTOM, TOP, h.0 + m / 2.0))
        } else if dir == Some("left") && same_col && above {
            let (g, n) = (d.0.min(l.0) - m / 2.0, gap(tt, sl));
            Some((vec![l, (g, l.1 - n), (g, f.1 - m / 2.0 - n), (f.0, f.1 - m / 2.0 - n), f], LEFT, TOP, l.0))
        } else if dir == Some("left") {
            let (x, n) = (f.0 + (l.0 - f.0) / 2.0, gap(tt, sl));
            Some((vec![l, (x, l.1), (x, f.1 - m / 2.0 - n), (f.0, f.1 - m / 2.0 - n), f], LEFT, TOP, l.0))
        } else if dir == Some("top") {
            let n = gap(tt, st);
            Some((vec![v, (v.0, f.1 - m / 2.0 - n), (f.0, f.1 - m / 2.0 - n), f], TOP, TOP, v.0))
        } else {
            None
        };
        let c = route.as_ref().map_or(0.0, |r| r.3);
        if let Some((pts, from, to, _)) = route {
            self.syms[s].lines[from] += 1;
            self.syms[t].lines[to] += 1;
            // The label sits by the line's start, above it or beside it.
            let label = lines(label);
            let label = (!label.is_empty()).then(|| {
                let (e, l) = (pts[0], pts[1]);
                if e.1 == l.1 {
                    let end = e.0 > l.0;
                    (if end { e.0 - MARGIN / 2.0 } else { e.0 + MARGIN / 2.0 }, e.1 - MARGIN, end, label)
                } else {
                    (e.0 + MARGIN / 2.0, if e.1 > l.1 { e.1 - MARGIN } else { e.1 + MARGIN }, false, label)
                }
            });
            let path = pts.iter().map(|&(x, y)| vec![x, y]).collect();
            let mut line = Line { path, stroke: None, width: LINE_WIDTH, dash: None, arrow: "block".into(), label };
            let key = &self.parsed.symbols[self.syms[t].of].key;
            if let Some(Value::Object(style)) = self.parsed.symbols[self.syms[s].of].styles.get(key) {
                line.style(style);
            }
            self.hop(&mut line.path);
            self.lines.push(line);
            self.min_x = Some(self.min_x.map_or(l.0, |x| x.min(l.0)));
        }
        if self.max_x == 0.0 || c > self.max_x {
            self.max_x = c;
        }
    }

    /// A hop over each earlier line the new one crosses. flowchart.js
    /// counts the new line's segments once per earlier segment, before its
    /// hops add any, and takes a curve's first control point for its end.
    fn hop(&self, b: &mut Vec<Vec<f64>>) {
        let w = LINE_WIDTH;
        for k in &self.lines {
            for seg in k.path.windows(2) {
                let (c, f, r, i) = (seg[0][0], seg[0][1], seg[1][0], seg[1][1]);
                let z = b.len() - 1;
                let mut e = 0;
                while e < z {
                    let (o, p, j, m) = (b[e][0], b[e][1], b[e + 1][0], b[e + 1][1]);
                    if let Some((x, y)) = cross(c, f, r, i, o, p, j, m) {
                        let (to, curve) = if p == m {
                            let s = if o > j { 1.0 } else { -1.0 };
                            (vec![x + s * 2.0 * w, p], vec![x + s * 2.0 * w, p, x, p - 4.0 * w, x - s * 2.0 * w, p])
                        } else {
                            let s = if p > m { 1.0 } else { -1.0 };
                            (vec![o, y + s * 2.0 * w], vec![o, y + s * 2.0 * w, o + 4.0 * w, y, o, y - s * 2.0 * w])
                        };
                        b.insert(e + 1, to);
                        b.insert(e + 2, curve);
                        e += 2;
                    }
                    e += 1;
                }
            }
        }
    }

    /// The SVG: sized to the symbols and lines from 0, 0, as flowchart.js
    /// sizes its paper, every symbol made (those never joined stay where
    /// they were made), then the lines.
    fn draw(&self, colors: &Colors) -> String {
        let (ink, page) = (colors.foreground.hex(), colors.background.hex());
        let (mut left, mut top, mut right, mut bottom) = (0.0f64, 0.0f64, self.max_x, 0.0f64);
        for &n in &self.order {
            let s = &self.syms[n];
            (left, right, bottom) = (f64::min(left, s.x), f64::max(right, s.x + s.w), f64::max(bottom, s.y + s.h));
        }
        for line in &self.lines {
            let (x1, y1, x2, y2) = bounds(&line.path);
            (left, top, right, bottom) = (left.min(x1), top.min(y1), right.max(x2), bottom.max(y2));
        }
        left = self.min_x.map_or(left, |x| left.min(x));
        if left < 0.0 {
            left -= LINE_WIDTH;
        }
        if top < 0.0 {
            top -= LINE_WIDTH;
        }
        let (width, height) = (N(right + LINE_WIDTH - left), N(bottom + LINE_WIDTH - top));
        let mut out = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="{} {} {width} {height}" font-family="{FONTS}" font-size="{SIZE}">"#,
            N(left),
            N(top)
        );
        let paint = format!(r#"fill="{page}" stroke="{ink}" stroke-width="{LINE_WIDTH}""#);
        for s in self.syms.iter().filter(|s| s.kind != Kind::Error) {
            let _ = write!(out, r#"<g transform="translate({},{})">"#, N(s.x), N(s.y));
            let (w, h) = (s.w, s.h);
            let rect = |x: f64, w: f64, round: &str| format!(r#"<rect x="{}" y="0" width="{}" height="{}"{round} {paint}/>"#, N(x), N(w), N(h));
            let path = |points: &[(f64, f64)]| {
                let points: Vec<String> = points.iter().map(|p| format!("{},{}", N(p.0), N(p.1))).collect();
                format!(r#"<path d="M{}" {paint}/>"#, points.join(" L"))
            };
            out += &match s.kind {
                Kind::Start | Kind::End => rect(0.0, w, r#" rx="20" ry="20""#),
                Kind::Subroutine => rect(0.0, w, "") + &rect(MARGIN, w - 2.0 * MARGIN, ""),
                Kind::InputOutput => path(&[(MARGIN, h / 2.0), (0.0, h), (w - 2.0 * MARGIN, h), (w, 0.0), (2.0 * MARGIN, 0.0), (MARGIN, h / 2.0)]),
                Kind::Condition => path(&[(w / 4.0, h / 4.0), (0.0, h / 2.0), (w / 2.0, h), (w, h / 2.0), (w / 2.0, 0.0), (0.0, h / 2.0)]),
                _ => rect(0.0, w, ""),
            };
            text(&mut out, s.text_x, h / 2.0, false, &s.text, &ink);
            out += "</g>";
        }
        for line in &self.lines {
            line.draw(&mut out, &ink);
        }
        out + "</svg>"
    }
}

/// The ```flow fence `src` as SVG, or flowchart.js's error, as Typora
/// draws it: an empty fence is a start and an end, "Empty Diagram".
pub fn svg(src: &str, colors: &Colors) -> Result<String, String> {
    let src = src.replace('\r', "");
    let src = if src.trim().is_empty() { "st=>start: Empty Diagram\ne=>end\nst->e" } else { &src };
    let mut chart = Chart { parsed: parse(src)?, ..Default::default() };
    let start = chart.parsed.start.ok_or("Cannot read properties of null (reading 'key')")?;
    chart.visit(start, None, 0)?;
    chart.layout()?;
    Ok(chart.draw(colors))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_as_flowchart_js() {
        let src = "st=>start: Begin|past:>http://example.com[blank]\n\
                   c(align-next=no)=>condition: Ok?\n\
                   op=>operation: Do\\\nit\n\
                   st->c(yes, right)->op\n\
                   c(no@Nope)->st";
        let d = parse(src).unwrap();
        let get = |k: &str| &d.symbols[d.keys[k]];
        assert_eq!((get("st").kind.as_str(), get("st").text.as_deref()), ("start", Some("Begin")));
        assert_eq!(get("c").params["align-next"], "no");
        assert_eq!(get("op").text.as_deref(), Some("Do\nit"));
        assert_eq!(get("st").links["next"], d.keys["c"]);
        assert_eq!((get("c").links["yes"], get("c").dirs["yes"].as_str()), (d.keys["op"], "right"));
        assert_eq!((get("c").links["no"], get("c").no_label.as_deref()), (d.keys["st"], Some("Nope")));
        assert_eq!(d.start, Some(d.keys["st"]));
        // `a=>operation:` has no ": ", so its type is "operation:".
        assert_eq!(parse("a=>operation:").unwrap().symbols[0].kind, "operation:");
    }

    #[test]
    fn errs_as_flowchart_js() {
        let colors = Colors::parse("ffffff,333333,4183c4,dddddd");
        let err = |src| svg(src, &colors).unwrap_err();
        assert_eq!(err("a->b"), "Cannot set properties of undefined (setting 'next')");
        assert_eq!(err("a=>start"), "Cannot read properties of null (reading 'key')");
        assert_eq!(err("a=>blob: x\nb=>end\na->b"), "h.shiftX is not a function");
        assert_eq!(err("a=>start\nb=>end\na->b\na@>c"), "Cannot read properties of undefined (reading 'key')");
        assert_eq!(err("a=>start\nb=>end\na->b\na@>b({\"stroke\":x})"), "Unexpected token 'x', \"{\"stroke\":x}\" is not valid JSON");
        assert_eq!(err("a=>start\nb=>end\na->b\na@>b({)"), "Unexpected end of JSON input");
    }

    #[test]
    fn lays_out_as_flowchart_js() {
        let colors = Colors::parse("ffffff,333333,4183c4,dddddd");
        // Two 36-high symbols, a line-length apart, the line stopping
        // short of its arrowhead's tip.
        let svg = svg("st=>start: Start\ne=>end: End\nst->e", &colors).unwrap();
        assert!(svg.contains(r#"height="137""#), "{svg}");
        assert!(svg.contains(",6)\">") && svg.contains(",98)\">"), "{svg}");
        assert!(svg.contains(",42 L") && svg.contains(",93.5\" fill=\"none\""), "{svg}");
        assert!(super::svg("", &colors).unwrap().contains(">Empty Diagram</text>"));
        // A line crossing an earlier one hops it, three line widths high.
        let line = Line { path: vec![vec![50.0, 0.0], vec![50.0, 100.0]], stroke: None, width: 3.0, dash: None, arrow: "block".into(), label: None };
        let chart = Chart { lines: vec![line], ..Default::default() };
        let mut path = vec![vec![0.0, 50.0], vec![100.0, 50.0]];
        chart.hop(&mut path);
        assert_eq!(path, [vec![0.0, 50.0], vec![44.0, 50.0], vec![44.0, 50.0, 50.0, 38.0, 56.0, 50.0], vec![100.0, 50.0]]);
    }
}
