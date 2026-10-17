//! Diagrams: ```mermaid, ```flow and ```sequence fences, drawn as Typora
//! draws them but without a browser. merman parses, lays out and draws
//! Mermaid in process; Typora's other two kinds, flowchart.js (`flow`) and
//! js-sequence-diagrams (`sequence`), are translated to Mermaid first.
//! Colours come from the Omarchy palette through Mermaid's `base` theme,
//! as Typora feeds its own theme's colours to Mermaid.

use std::collections::HashMap;
use std::ops::Range;

use merman::svg::export::RasterOptions;
use merman::svg::{RootBackgroundPostprocessor, SvgPipeline};
use merman::{Engine, MermaidConfig, OperationControl, PngRequest, RenderError, RenderOutput, RenderRequest, Renderer, SvgRequest};

use crate::palette::Rgb;

/// The fence languages that are diagrams, as Typora names them.
pub const LANGUAGES: [&str; 3] = ["mermaid", "flow", "sequence"];

/// Mermaid's own fonts, then Arial's metric twin, which Omarchy has: the
/// renderer has no fontconfig, so a bare `sans-serif` would take whichever
/// font it found first.
const FONTS: &str = "\"trebuchet ms\", verdana, arial, \"Liberation Sans\", \"Nimbus Sans\", sans-serif";

/// What a diagram is drawn in: the page, its text, the accent and lines.
pub struct Colors {
    pub background: Rgb,
    pub foreground: Rgb,
    pub accent: Rgb,
    pub border: Rgb,
}

impl Colors {
    /// `rrggbb,rrggbb,rrggbb,rrggbb`: background, text, accent, border.
    pub fn parse(s: &str) -> Colors {
        let mut it = s.split(',').map(|c| Rgb::parse(&format!("#{c}")));
        let mut next = |or| it.next().flatten().unwrap_or(or);
        Colors {
            background: next(Rgb(255, 255, 255)),
            foreground: next(Rgb(51, 51, 51)),
            accent: next(Rgb(65, 131, 196)),
            border: next(Rgb(221, 221, 221)),
        }
    }

    fn dark(&self) -> bool {
        self.background.luminance() < 0.2
    }

    /// Mermaid's `base` theme, coloured like the page: nodes a tint of the
    /// accent with accent borders, as Typora's themes give `--node-fill`.
    fn theme(&self) -> serde_json::Value {
        let (bg, fg, accent) = (self.background, self.foreground, self.accent);
        let node = bg.mix(accent, if self.dark() { 0.22 } else { 0.12 });
        let cluster = bg.mix(fg, 0.06);
        let line = bg.mix(fg, 0.7);
        serde_json::json!({
            "darkMode": self.dark(),
            "background": bg.hex(),
            "fontFamily": FONTS,
            "primaryColor": node.hex(),
            "primaryTextColor": fg.hex(),
            "primaryBorderColor": accent.hex(),
            "secondaryColor": cluster.hex(),
            "secondaryTextColor": fg.hex(),
            "secondaryBorderColor": self.border.hex(),
            "tertiaryColor": cluster.hex(),
            "tertiaryTextColor": fg.hex(),
            "tertiaryBorderColor": self.border.hex(),
            "lineColor": line.hex(),
            "textColor": fg.hex(),
            "mainBkg": node.hex(),
            "nodeBorder": accent.hex(),
            "clusterBkg": cluster.hex(),
            "clusterBorder": self.border.hex(),
            "titleColor": fg.hex(),
            "edgeLabelBackground": bg.hex(),
            "noteBkgColor": cluster.hex(),
            "noteTextColor": fg.hex(),
            "noteBorderColor": self.border.hex(),
            "actorBkg": node.hex(),
            "actorBorder": accent.hex(),
            "actorTextColor": fg.hex(),
            "actorLineColor": line.hex(),
            "signalColor": fg.hex(),
            "signalTextColor": fg.hex(),
            "labelBoxBkgColor": node.hex(),
            "labelBoxBorderColor": accent.hex(),
            "labelTextColor": fg.hex(),
            "loopTextColor": fg.hex(),
            "sequenceNumberColor": bg.hex(),
        })
    }
}

/// Draw the `lang` diagram `src` as a PNG, `scale` pixels to the point.
/// The error is Typora's: `ERROR: [Mermaid] …`, `[Flowchart]` or
/// `[Sequence]`, with the bytes of a Mermaid fence's code it is about.
pub fn render(lang: &str, src: &str, colors: &Colors, scale: f64) -> Result<Vec<u8>, (String, Option<Range<usize>>)> {
    let (mermaid, renderer, error) = prepare(lang, src, colors).map_err(|e| (e, None))?;
    let output = renderer
        .render(RenderRequest::png(
            &mermaid,
            OperationControl::new(),
            PngRequest { svg: svg_request(), options: RasterOptions::default().with_scale(scale.clamp(0.25, 8.0) as f32) },
        ))
        .map_err(|e| {
            // flowchart.js and sequence code is translated first, so only
            // Mermaid's own code has the spans.
            let span = match &e {
                RenderError::Parse(d) if mermaid == src => d.terminal_diagnostic_details().span.map(|s| widen(src, s.start..s.end)),
                _ => None,
            };
            (error(e.to_string()), span)
        })?;
    match output {
        RenderOutput::Png(Some(png)) => Ok(png.bytes),
        _ => Err((error("No diagram type detected".into()), None)),
    }
}

/// The code an error marks: merman's span, or, when that is empty or
/// blank, its line from the first word to the last (the nearest line
/// before with any, past the end), as Typora's lint marks a line.
fn widen(src: &str, span: Range<usize>) -> Range<usize> {
    if !src.get(span.clone()).unwrap_or("").trim().is_empty() {
        return span;
    }
    let line = |at: usize| src[..at].rfind('\n').map_or(0, |i| i + 1)..src[at..].find('\n').map_or(src.len(), |i| at + i);
    let mut l = line(if src.is_char_boundary(span.start) { span.start } else { src.len() });
    while src[l.clone()].trim().is_empty() && l.start > 0 {
        l = line(l.start - 1);
    }
    let text = &src[l.clone()];
    let start = l.start + text.len() - text.trim_start().len();
    start..start + text.trim().len()
}

/// Draw the `lang` diagram `src` as SVG markup, for export.
pub fn svg(lang: &str, src: &str, colors: &Colors) -> Result<String, String> {
    let (mermaid, renderer, error) = prepare(lang, src, colors)?;
    let output =
        renderer.render(RenderRequest::svg(&mermaid, OperationControl::new(), svg_request())).map_err(|e| error(e.to_string()))?;
    match output {
        RenderOutput::Svg(Some(svg)) => Ok(svg.svg().to_string()),
        _ => Err(error("No diagram type detected".into())),
    }
}

type Error = Box<dyn Fn(String) -> String>;

/// The Mermaid to draw, a renderer coloured like the page, and how to word
/// an error.
fn prepare(lang: &str, src: &str, colors: &Colors) -> Result<(String, Renderer, Error), String> {
    let lang = lang.trim().to_lowercase();
    let (name, mermaid) = match lang.as_str() {
        "flow" => ("Flowchart", flow_to_mermaid(src)),
        "sequence" => ("Sequence", sequence_to_mermaid(src)),
        _ => ("Mermaid", Ok(src.to_string())),
    };
    let error: Error = Box::new(move |message: String| format!("ERROR: [{name}] {message}"));
    let mermaid = mermaid.map_err(&error)?;
    // Typora draws an empty Mermaid fence as the words "Empty Diagram".
    let mermaid = if mermaid.trim().is_empty() { "flowchart TD\n  empty[\"Empty Diagram\"]".to_string() } else { mermaid };
    let config = MermaidConfig::from_value(serde_json::json!({
        "theme": "base",
        "darkMode": colors.dark(),
        "fontFamily": FONTS,
        "themeVariables": colors.theme(),
        "sequence": { "diagramMarginX": 8, "diagramMarginY": 8, "boxMargin": 8 },
    }));
    Ok((mermaid, Renderer::new().with_engine(Engine::new().with_site_config(config)), error))
}

/// Mermaid's own pages are white; this one is the page's.
fn svg_request() -> SvgRequest {
    SvgRequest {
        pipeline: Some(SvgPipeline::resvg_safe().with_postprocessor(RootBackgroundPostprocessor::new("transparent"))),
        ..SvgRequest::default()
    }
}

/// Mermaid text: `#` starts an entity and `;` ends a statement, so both
/// are written as entities; flowchart.js and js-sequence-diagrams take a
/// literal `\n` for a line break.
fn text(s: &str) -> String {
    let mut out = String::new();
    for c in s.trim().replace("\\n", "\n").chars() {
        match c {
            '#' => out.push_str("#35;"),
            ';' => out.push_str("#59;"),
            '"' => out.push_str("#quot;"),
            '\n' => out.push_str("<br/>"),
            c => out.push(c),
        }
    }
    out
}

/// flowchart.js to Mermaid. Symbols are `name=>type: label|state:>url`,
/// flows are `a->b(yes, right)->c`; conditions branch on `yes`/`no` and
/// parallels on `path1`…`path3`, which become edge labels. Directions
/// (`left`, `right`, …), states and links have no Mermaid counterpart.
pub fn flow_to_mermaid(src: &str) -> Result<String, String> {
    let src = if src.trim().is_empty() { "st=>start: Empty Diagram\ne=>end\nst->e" } else { src };
    let mut ids: HashMap<String, String> = HashMap::new();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let id = |name: &str, ids: &mut HashMap<String, String>| {
        let n = ids.len();
        ids.entry(name.to_string()).or_insert_with(|| format!("n{n}")).clone()
    };
    for (i, line) in src.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((name, rest)) = line.split_once("=>") {
            let (kind, label) = match rest.split_once(':') {
                Some((kind, label)) => (kind.trim(), label),
                None => (rest.trim(), ""),
            };
            // `|state` and `:>link[target]` follow the label.
            let label = label.split(":>").next().unwrap_or_default();
            let label = match label.rfind('|') {
                Some(bar) => &label[..bar],
                None => label,
            };
            // flowchart.js names a bare start and end; other symbols are blank.
            let label = match (label.trim(), kind) {
                ("", "start") => "Start",
                ("", "end") => "End",
                _ => label,
            };
            let (open, close) = match kind {
                "start" | "end" => ("([", "])"),
                "operation" | "parallel" => ("[", "]"),
                "subroutine" => ("[[", "]]"),
                "condition" => ("{", "}"),
                "inputoutput" => ("[/", "/]"),
                _ => return Err(format!("Parse error on line {}: unknown symbol type \"{kind}\"", i + 1)),
            };
            let node = id(name.trim(), &mut ids);
            // An empty label would be Mermaid's error, not a blank symbol.
            let label = if label.trim().is_empty() { "#32;".to_string() } else { text(label) };
            nodes.push(format!("    {node}{open}\"{label}\"{close}"));
        } else if line.contains("->") {
            let steps: Vec<&str> = line.split("->").collect();
            for pair in steps.windows(2) {
                let (from, args) = match pair[0].trim().split_once('(') {
                    Some((name, args)) => (name.trim(), args.trim_end_matches(')')),
                    None => (pair[0].trim(), ""),
                };
                let to = pair[1].trim().split('(').next().unwrap_or_default().trim();
                if from.is_empty() || to.is_empty() {
                    return Err(format!("Parse error on line {}: a flow needs a symbol on each side of ->", i + 1));
                }
                // The branch taken, if any; a direction alone is no label.
                let branch = args
                    .split(',')
                    .map(str::trim)
                    .find(|a| matches!(*a, "yes" | "no" | "true" | "false") || a.starts_with("path"));
                let (a, b) = (id(from, &mut ids), id(to, &mut ids));
                edges.push(match branch {
                    Some(label) => format!("    {a} -->|{}| {b}", text(label)),
                    None => format!("    {a} --> {b}"),
                });
            }
        } else {
            return Err(format!("Parse error on line {}: expected a symbol (name=>type: text) or a flow (a->b)", i + 1));
        }
    }
    // A flow may name a symbol that was never defined: it shows its name.
    let defined: Vec<String> = nodes.iter().map(|n| n.trim().split(['(', '[', '{']).next().unwrap_or_default().to_string()).collect();
    let mut names: Vec<(&String, &String)> = ids.iter().collect();
    names.sort_by_key(|(_, id)| id[1..].parse::<usize>().unwrap_or(0));
    for (name, node) in names {
        if !defined.contains(node) {
            nodes.push(format!("    {node}[\"{}\"]", text(name)));
        }
    }
    Ok(format!("flowchart TD\n{}\n{}\n", nodes.join("\n"), edges.join("\n")))
}

/// js-sequence-diagrams to Mermaid. Lines are `Title: …`, `participant A`
/// or `participant "Long name" as A`, `Note left of|right of|over A[,B]: …`
/// and messages `A->B: …`, where `-` is a solid line, `--` a dashed one, `>`
/// a filled arrow and `>>` an open one. Participants may have spaces in
/// their names, so each gets an id.
pub fn sequence_to_mermaid(src: &str) -> Result<String, String> {
    let src = if src.trim().is_empty() { "title: Empty Diagram" } else { src };
    let mut out = vec!["sequenceDiagram".to_string()];
    let mut actors: HashMap<String, String> = HashMap::new();
    let mut order: Vec<(String, String)> = Vec::new();
    let mut body = Vec::new();
    let mut actor = |name: &str, label: Option<&str>| -> String {
        let name = name.trim().to_string();
        if let Some(id) = actors.get(&name) {
            return id.clone();
        }
        let id = format!("a{}", actors.len());
        actors.insert(name.clone(), id.clone());
        order.push((id.clone(), label.unwrap_or(&name).to_string()));
        id
    };
    for (i, line) in src.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parse_error = |what: &str| format!("Parse error on line {}:\n{line}\n{what}", i + 1);
        let lower = line.to_lowercase();
        if lower.starts_with("title") && line[5..].trim_start().starts_with(':') {
            let title = line[5..].trim_start()[1..].trim();
            out.push(format!("    title {}", text(title)));
        } else if lower.starts_with("participant ") {
            let rest = line["participant ".len()..].trim();
            match rest.strip_prefix('"').and_then(|r| r.split_once('"')) {
                Some((label, alias)) => {
                    let alias = alias.trim().strip_prefix("as").map(str::trim).filter(|a| !a.is_empty());
                    actor(alias.unwrap_or(label), Some(label));
                }
                None => {
                    actor(rest, None);
                }
            }
        } else if lower.starts_with("note ") {
            let (head, message) = line.split_once(':').ok_or_else(|| parse_error("Expecting ':'"))?;
            let head = head[5..].trim();
            let lower = head.to_lowercase();
            let (place, who) = ["left of", "right of", "over"]
                .iter()
                .find_map(|p| lower.starts_with(p).then(|| (*p, &head[p.len()..])))
                .ok_or_else(|| parse_error("Expecting 'left of', 'right of' or 'over'"))?;
            let who: Vec<String> = who.split(',').map(|a| actor(a, None)).collect();
            body.push(format!("    Note {place} {}: {}", who.join(","), text(message)));
        } else if let Some(arrow) = line.find('-').filter(|&a| line[a..].trim_start_matches('-').starts_with('>')) {
            let (from, rest) = line.split_at(arrow);
            let dashes = rest.len() - rest.trim_start_matches('-').len();
            let rest = &rest[dashes..];
            let heads = rest.len() - rest.trim_start_matches('>').len();
            let (to, message) = rest[heads..].split_once(':').ok_or_else(|| parse_error("Expecting ':'"))?;
            if from.trim().is_empty() || to.trim().is_empty() {
                return Err(parse_error("Expecting a participant on each side of the arrow"));
            }
            let mermaid_arrow = match (dashes > 1, heads > 1) {
                (false, false) => "->>",
                (true, false) => "-->>",
                (false, true) => "-)",
                (true, true) => "--)",
            };
            let (a, b) = (actor(from, None), actor(to, None));
            body.push(format!("    {a}{mermaid_arrow}{b}: {}", text(message)));
        } else {
            return Err(parse_error("Expecting a message (A->B: text), a note or a participant"));
        }
    }
    out.extend(order.iter().map(|(id, label)| format!("    participant {id} as {}", text(label))));
    out.extend(body);
    Ok(out.join("\n") + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_translates() {
        let src = "st=>start: Start|past:>http://example.com[blank]\n\
                   e=>end: End\n\
                   op=>operation: Do it|current\n\
                   c=>condition: Yes or No?\n\
                   io=>inputoutput: read; #1\n\
                   st->op(right)->c\n\
                   c(yes, right)->io->e\n\
                   c(no)->op\n";
        assert_eq!(
            flow_to_mermaid(src).unwrap(),
            "flowchart TD\n    n0([\"Start\"])\n    n1([\"End\"])\n    n2[\"Do it\"]\n    n3{\"Yes or No?\"}\n    n4[/\"read#59; #35;1\"/]\n\
             \x20   n0 --> n2\n    n2 --> n3\n    n3 -->|yes| n4\n    n4 --> n1\n    n3 -->|no| n2\n"
        );
        // Undefined symbols show their names; bad lines are errors.
        assert!(flow_to_mermaid("a->b").unwrap().contains("n0[\"a\"]"));
        assert!(flow_to_mermaid("x=>blob: no").unwrap_err().contains("line 1"));
        assert!(flow_to_mermaid("hello").is_err());
        assert!(flow_to_mermaid("").unwrap().contains("Empty Diagram"));
        // A bare start or end is named, as flowchart.js names them.
        assert!(flow_to_mermaid("e=>end\no=>operation").unwrap().contains("n0([\"End\"])\n    n1[\"#32;\"]"));
    }

    #[test]
    fn sequence_translates() {
        let src = "Title: Hi there\n\
                   participant \"Long name\" as L\n\
                   Alice Smith->Bob: Hello\\nworld\n\
                   Bob-->Alice Smith: dashed\n\
                   Bob->>L: open\n\
                   L-->>Bob: dashed open\n\
                   Note right of Bob: thinks\n\
                   Note over Alice Smith,Bob: both\n";
        assert_eq!(
            sequence_to_mermaid(src).unwrap(),
            "sequenceDiagram\n    title Hi there\n    participant a0 as Long name\n    participant a1 as Alice Smith\n    participant a2 as Bob\n\
             \x20   a1->>a2: Hello<br/>world\n    a2-->>a1: dashed\n    a2-)a0: open\n    a0--)a2: dashed open\n\
             \x20   Note right of a2: thinks\n    Note over a1,a2: both\n"
        );
        assert!(sequence_to_mermaid("A->B no colon").unwrap_err().starts_with("Parse error on line 1"));
        assert!(sequence_to_mermaid("").unwrap().contains("title Empty Diagram"));
    }

    #[test]
    fn renders_png() {
        let colors = Colors::parse("1a1b26,c0caf5,7aa2f7,414868");
        for (lang, src) in [("mermaid", "graph TD\n  A[Start] --> B{Ok?}\n  B -->|yes| C[Done]"), ("flow", "a=>start: A\nb=>end: B\na->b"), ("sequence", "A->B: hi")] {
            let png = render(lang, src, &colors, 1.0).unwrap();
            assert_eq!(&png[1..4], b"PNG", "{lang}");
            if let Ok(dir) = std::env::var("TYPO_DIAGRAM_OUT") {
                std::fs::write(format!("{dir}/{lang}.png"), &png).unwrap();
            }
        }
        let (error, span) = render("mermaid", "graph TD\n  A -->", &colors, 1.0).unwrap_err();
        assert!(error.starts_with("ERROR: [Mermaid] "), "{error}");
        assert_eq!(span, Some(11..16));
        assert_eq!(render("sequence", "A->B no colon", &colors, 1.0).unwrap_err().1, None);
    }

    #[test]
    fn widens_empty_spans_to_their_line() {
        let src = "classDiagram\n  class A {\n";
        assert_eq!(widen(src, 25..25), 15..24);
        assert_eq!(widen("x\n  A-x\n", 7..8), 4..7);
        assert_eq!(widen("graph TD\n  A -- B\n", 16..18), 16..18);
        assert!(widen(" \n", 2..2).is_empty());
    }
}
