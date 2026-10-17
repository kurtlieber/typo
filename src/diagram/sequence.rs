//! js-sequence-diagrams (```sequence), drawn as Typora's copy of its
//! "simple" theme draws it: actors boxed above and below their lifelines,
//! each message a line with its text above, notes boxed, in 16px
//! Andale Mono, lines and text in the text colour on the page's.

use std::fmt::Write;

use unicode_width::UnicodeWidthStr;

use super::{Colors, N};

pub struct Diagram {
    title: Option<String>,
    actors: Vec<String>,
    items: Vec<Item>,
}

enum Item {
    Signal { from: usize, to: usize, dashed: bool, open: bool, text: String },
    Note { place: Place, actors: Vec<usize>, text: String },
}

#[derive(PartialEq)]
enum Place {
    LeftOf,
    RightOf,
    Over,
}

/// The diagram's text: trimmed, a literal `\n` a line break.
fn unescape(s: &str) -> String {
    let s = s.trim();
    let s = s.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap_or(s);
    s.replace("\\n", "\n")
}

/// Lines are `Title: …`, `participant A`, `participant "Long name" as A`,
/// `Note left of|right of|over A[,B]: …` and messages `A->B: …`, where `-`
/// is a solid line, `--` a dashed one, `>` a filled arrow and `>>` an open
/// one. Actors come in the order they are first named.
pub fn parse(src: &str) -> Result<Diagram, String> {
    let src = if src.trim().is_empty() { "title: Empty Diagram" } else { src };
    let mut d = Diagram { title: None, actors: Vec::new(), items: Vec::new() };
    let mut aliases: Vec<String> = Vec::new();
    let mut actor = |alias: &str, name: Option<&str>, d: &mut Diagram| -> usize {
        let alias = unescape(alias);
        if let Some(i) = aliases.iter().position(|a| *a == alias) {
            return i;
        }
        d.actors.push(name.map(unescape).unwrap_or_else(|| alias.clone()));
        aliases.push(alias);
        aliases.len() - 1
    };
    for (i, line) in src.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parse_error = |what: &str| format!("Parse error on line {}:\n{line}\n{what}", i + 1);
        let lower = line.to_lowercase();
        if lower.starts_with("title") && line[5..].trim_start().starts_with(':') {
            d.title = Some(unescape(&line[5..].trim_start()[1..]));
        } else if lower.starts_with("participant ") {
            let rest = line["participant ".len()..].trim();
            match rest.rsplit_once(" as ").filter(|(_, alias)| !alias.trim().is_empty() && !alias.trim().contains(char::is_whitespace)) {
                Some((name, alias)) => actor(alias, Some(name), &mut d),
                None => actor(rest, None, &mut d),
            };
        } else if lower.starts_with("note ") {
            let (head, message) = line.split_once(':').ok_or_else(|| parse_error("Expecting ':'"))?;
            let head = head[5..].trim();
            let lower = head.to_lowercase();
            let (place, who) = [("left of", Place::LeftOf), ("right of", Place::RightOf), ("over", Place::Over)]
                .into_iter()
                .find_map(|(p, place)| lower.starts_with(p).then(|| (place, &head[p.len()..])))
                .ok_or_else(|| parse_error("Expecting 'left of', 'right of' or 'over'"))?;
            let mut actors: Vec<usize> = who.split(',').map(|a| actor(a, None, &mut d)).collect();
            actors.dedup();
            if place != Place::Over && actors.len() > 1 {
                return Err(parse_error("Expecting ':'"));
            }
            d.items.push(Item::Note { place, actors, text: unescape(message) });
        } else if let Some(arrow) = line.find('-').filter(|&a| line[a..].trim_start_matches('-').starts_with('>')) {
            let (from, rest) = line.split_at(arrow);
            let dashes = rest.len() - rest.trim_start_matches('-').len();
            let rest = &rest[dashes..];
            let heads = rest.len() - rest.trim_start_matches('>').len();
            let (to, message) = rest[heads..].split_once(':').ok_or_else(|| parse_error("Expecting ':'"))?;
            if from.trim().is_empty() || to.trim().is_empty() {
                return Err(parse_error("Expecting a participant on each side of the arrow"));
            }
            let (from, to) = (actor(from, None, &mut d), actor(to, None, &mut d));
            d.items.push(Item::Signal { from, to, dashed: dashes > 1, open: heads > 1, text: unescape(message) });
        } else {
            return Err(parse_error("Expecting a message (A->B: text), a note or a participant"));
        }
    }
    Ok(d)
}

const FONT: f64 = 16.0;
/// A line's box and its baseline in it, about Andale Mono's at 16px, and
/// the 1.2em between lines that Raphaël sets.
const LINE: f64 = 18.4;
const ASCENT: f64 = 14.8;
const LEADING: f64 = 19.2;
const FONTS: &str = "Andale Mono, Liberation Mono, DejaVu Sans Mono, Noto Sans Mono, monospace";

/// The text's size: a monospaced font's 0.6em a column.
// ponytail: columns, not the font's advances; measure the font if a
// proportional one ever stands in.
fn size(text: &str) -> (f64, f64) {
    if text.is_empty() {
        return (0.0, 0.0);
    }
    let lines: Vec<&str> = text.split('\n').collect();
    let columns = lines.iter().map(|l| l.width()).max().unwrap_or(0);
    (columns as f64 * FONT * 0.6, LINE + (lines.len() - 1) as f64 * LEADING)
}

/// The layout of js-sequence-diagrams 1.x, its margins and paddings
/// folded into the sizes: each actor is set right of the one before, and
/// further when a message or note between them needs the room.
pub fn svg(d: &Diagram, colors: &Colors) -> String {
    let n = d.actors.len();
    let title = d.title.as_deref().map(|t| {
        let (w, h) = size(t);
        (t, w + 10.0, h + 10.0)
    });
    let (mut width, mut height) = title.map_or((0.0, 0.0), |t| (t.1, t.2));
    let boxes: Vec<(f64, f64)> = d.actors.iter().map(|a| size(a)).map(|(w, h)| (w + 40.0, h + 40.0)).collect();
    let actors_h = boxes.iter().map(|b| b.1).fold(0.0, f64::max);
    // How far apart actors' centres must be, an actor's room before the
    // first or after the last.
    let mut needs: Vec<(isize, usize, f64)> = Vec::new();
    let mut sizes: Vec<(f64, f64)> = Vec::new();
    for item in &d.items {
        let (text, pad) = match item {
            Item::Signal { text, .. } => (text, 20.0),
            Item::Note { text, .. } => (text, 30.0),
        };
        let (w, h) = size(text);
        let (mut w, h) = (w + pad, h + pad);
        match item {
            Item::Signal { from, to, .. } if from == to => {
                w += 20.0;
                needs.push((*from as isize, from + 1, w));
            }
            Item::Signal { from, to, .. } => needs.push((*from.min(to) as isize, *from.max(to), w)),
            Item::Note { place: Place::LeftOf, actors, .. } => needs.push((actors[0] as isize - 1, actors[0], w + 20.0)),
            Item::Note { place: Place::RightOf, actors, .. } => needs.push((actors[0] as isize, actors[0] + 1, w + 20.0)),
            Item::Note { actors, .. } if actors.len() > 1 => {
                let (a, b) = (actors.iter().min().copied().unwrap_or(0), actors.iter().max().copied().unwrap_or(0));
                needs.push((a as isize, b, w - 40.0));
            }
            Item::Note { actors, .. } => {
                needs.push((actors[0] as isize - 1, actors[0], w / 2.0));
                needs.push((actors[0] as isize, actors[0] + 1, w / 2.0));
            }
        }
        sizes.push((w, h));
    }
    let signals_h: f64 = sizes.iter().map(|s| s.1).sum();
    let mut x = vec![0.0; n];
    let mut apart: Vec<Vec<Option<f64>>> = vec![vec![None; n]; n];
    let mut after = vec![0.0; n];
    for (a, b, room) in needs {
        if a < 0 {
            x[b] = f64::max(x[b], room - boxes[b].0 / 2.0);
        } else if b >= n {
            after[a as usize] = f64::max(after[a as usize], room);
        } else {
            let d = &mut apart[a as usize][b];
            *d = Some(d.unwrap_or(0.0).max(room));
        }
    }
    let mut next = 0.0;
    for i in 0..n {
        x[i] = f64::max(x[i], next);
        for b in 0..n {
            if let Some(room) = apart[i][b] {
                let room = room.max(boxes[i].0 / 2.0).max(boxes[b].0 / 2.0);
                x[b] = f64::max(x[b], x[i] + boxes[i].0 / 2.0 + room - boxes[b].0 / 2.0);
            }
        }
        next = x[i] + boxes[i].0 + after[i];
    }
    width = f64::max(next, width) + 20.0;
    height += 20.0 + 2.0 * actors_h + signals_h;

    let mut svg = Svg { out: String::new(), ink: colors.foreground.hex(), page: colors.background.hex() };
    let (width, height) = (N(width), N(height));
    let _ = write!(svg.out, r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#);
    if let Some((t, w, h)) = title {
        svg.text_box(10.0, 10.0, w, h, 0.0, t);
    }
    let top = 10.0 + title.map_or(0.0, |t| t.2);
    let centre = |i: usize| x[i] + boxes[i].0 / 2.0;
    for (i, name) in d.actors.iter().enumerate() {
        svg.text_box(x[i], top, boxes[i].0, actors_h, 10.0, name);
        svg.text_box(x[i], top + actors_h + signals_h, boxes[i].0, actors_h, 10.0, name);
        svg.line(centre(i), top + actors_h - 10.0, centre(i), top + actors_h + 10.0 + signals_h, false, None);
    }
    let mut y = top + actors_h;
    for (item, &(w, h)) in d.items.iter().zip(&sizes) {
        match item {
            Item::Signal { from, to, dashed, open, text } if from == to => {
                let a = centre(*from);
                svg.text(a + 25.0 + size(text).0 / 2.0, y + h / 2.0, text);
                svg.line(a, y + 5.0, a + 20.0, y + 5.0, *dashed, None);
                svg.line(a + 20.0, y + 5.0, a + 20.0, y + h, *dashed, None);
                svg.line(a + 20.0, y + h, a, y + h, *dashed, Some(*open));
            }
            Item::Signal { from, to, dashed, open, text } => {
                let (a, b) = (centre(*from), centre(*to));
                svg.text((a + b) / 2.0, y + 15.0, text);
                svg.line(a, y + h - 10.0, b, y + h - 10.0, *dashed, Some(*open));
            }
            Item::Note { place, actors, text } => {
                let a = centre(actors[0]);
                let (nx, w) = match place {
                    Place::RightOf => (a + 10.0, w),
                    Place::LeftOf => (a - 10.0 - w, w),
                    Place::Over if actors.len() > 1 => {
                        let (l, r) = actors.iter().map(|&i| centre(i)).fold((f64::MAX, f64::MIN), |(l, r), c| (l.min(c), r.max(c)));
                        (l - 20.0, r + 20.0 - (l - 20.0))
                    }
                    Place::Over => (a - w / 2.0, w),
                };
                svg.text_box(nx, y, w, h, 10.0, text);
            }
        }
        y += h;
    }
    svg.out.push_str("</svg>");
    svg.out
}

struct Svg {
    out: String,
    ink: String,
    page: String,
}

impl Svg {
    /// Text centred on `x`, `y`, a line at a time.
    fn text(&mut self, x: f64, y: f64, text: &str) {
        let top = y - size(text).1 / 2.0;
        for (i, line) in text.split('\n').enumerate() {
            let _ = write!(
                self.out,
                r#"<text x="{}" y="{}" text-anchor="middle" font-family="{FONTS}" font-size="{FONT}" fill="{}" xml:space="preserve">{}</text>"#,
                N(x),
                N(top + ASCENT + i as f64 * LEADING),
                self.ink,
                html_escape::encode_text(line)
            );
        }
    }

    /// A box `margin` inside `w` by `h` at `x`, `y`, its text centred.
    fn text_box(&mut self, x: f64, y: f64, w: f64, h: f64, margin: f64, text: &str) {
        let _ = write!(
            self.out,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" stroke="{}" stroke-width="2"/>"#,
            N(x + margin),
            N(y + margin),
            N(w - 2.0 * margin),
            N(h - 2.0 * margin),
            self.page,
            self.ink
        );
        self.text(x + w / 2.0, y + h / 2.0, text);
    }

    /// A line 2px wide, dashed 6,2 or solid, ending in Raphaël's
    /// `block-wide-long` arrow (`Some(false)`) or `open-wide-long` one.
    /// Arrows are only ever on level lines.
    fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, dashed: bool, arrow: Option<bool>) {
        let dir = if x2 < x1 { -1.0 } else { 1.0 };
        let end = match arrow {
            Some(false) => x2 - 5.0 * dir,
            Some(true) => x2 - 2.0 * dir,
            None => x2,
        };
        let dash = if dashed { r#" stroke-dasharray="6,2""# } else { "" };
        let ink = &self.ink;
        let _ = write!(self.out, r#"<path d="M{},{} L{},{}" fill="none" stroke="{ink}" stroke-width="2"{dash}/>"#, N(x1), N(y1), N(end), N(y2));
        let (y, up, down) = (N(y2), N(y2 - 5.0), N(y2 + 5.0));
        match arrow {
            Some(false) => {
                let (tip, back) = (N(x2), N(x2 - 10.0 * dir));
                let _ = write!(self.out, r#"<path d="M{tip},{y} L{back},{up} L{back},{down} Z" fill="{ink}"/>"#);
            }
            Some(true) => {
                let (tip, back) = (N(x2 + 2.0 * dir), N(x2 - 8.0 * dir));
                let _ = write!(
                    self.out,
                    r#"<path d="M{back},{up} L{tip},{y} L{back},{down}" fill="none" stroke="{ink}" stroke-width="2" stroke-linecap="round"/>"#
                );
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses() {
        let src = "Title: Hi there\n\
                   participant \"Long name\" as L\n\
                   Alice Smith->Bob: Hello\\nworld\n\
                   Bob-->Alice Smith: dashed\n\
                   Bob->>L: open\n\
                   L-->>Bob: dashed open\n\
                   Note right of Bob: thinks\n\
                   Note over Alice Smith,Bob: both\n";
        let d = parse(src).unwrap();
        assert_eq!(d.title.as_deref(), Some("Hi there"));
        assert_eq!(d.actors, ["Long name", "Alice Smith", "Bob"]);
        assert!(matches!(&d.items[0], Item::Signal { from: 1, to: 2, dashed: false, open: false, text } if text == "Hello\nworld"));
        assert!(matches!(&d.items[3], Item::Signal { from: 0, to: 2, dashed: true, open: true, .. }));
        assert!(matches!(&d.items[5], Item::Note { place: Place::Over, actors, .. } if actors == &[1, 2]));
        assert!(parse("A->B no colon").is_err_and(|e| e.starts_with("Parse error on line 1")));
        assert_eq!(parse("").unwrap().title.as_deref(), Some("Empty Diagram"));
        assert_eq!(parse("participant Long Name as L\nL->L: me").unwrap().actors, ["Long Name"]);
    }

    #[test]
    fn lays_out_as_js_sequence() {
        let colors = Colors::parse("ffffff,333333,4183c4,dddddd");
        // Two 49.6-wide actors, side by side; a 38.4 high message.
        let svg = svg(&parse("A->B: hi").unwrap(), &colors);
        assert!(svg.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg" width="119.2" height="175.2""#), "{svg}");
        assert!(svg.contains(r#"<rect x="59.6" y="20" width="29.6" height="38.4""#), "{svg}");
        // A long message moves B over.
        let svg = super::svg(&parse("A->B: a long message").unwrap(), &colors);
        assert!(svg.contains(r#"width="224""#), "{svg}");
    }
}
