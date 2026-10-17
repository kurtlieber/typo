//! Diagrams: ```mermaid, ```flow and ```sequence fences, drawn as Typora
//! draws them but without a browser. merman parses, lays out and draws
//! Mermaid in process; flowchart.js (`flow`) and js-sequence-diagrams'
//! simple theme (`sequence`) are laid out and drawn here, as they draw
//! them. Colours come from the Omarchy palette, through Mermaid's `base`
//! theme for Mermaid, as Typora feeds its own theme's colours to Mermaid.

mod flow;
mod sequence;

use std::fmt;
use std::ops::Range;
use std::sync::{Arc, LazyLock};

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

/// Draw the `lang` diagram `src` as a PNG, `scale` pixels to the point, a
/// gantt chart `width` points wide (Mermaid's 1200 for 0), as Mermaid
/// draws one to its panel. The error is Typora's: `ERROR: [Mermaid] …`,
/// `[Flowchart]` or `[Sequence]`, with the bytes of a Mermaid fence's code
/// it is about.
pub fn render(lang: &str, src: &str, colors: &Colors, scale: f64, width: f64) -> Result<Vec<u8>, (String, Option<Range<usize>>)> {
    if let Some(svg) = own_svg(lang, src, colors) {
        return svg.and_then(|svg| png(&svg, scale)).map_err(|e| (e, None));
    }
    let (mermaid, renderer) = prepare(src, colors, width);
    let output = renderer
        .render(RenderRequest::png(
            &mermaid,
            OperationControl::new(),
            PngRequest { svg: svg_request(), options: RasterOptions::default().with_scale(scale.clamp(0.25, 8.0) as f32) },
        ))
        .map_err(|e| {
            let span = match &e {
                RenderError::Parse(d) => d.terminal_diagnostic_details().span.map(|s| widen(src, s.start..s.end)),
                _ => None,
            };
            (error(e), span)
        })?;
    match output {
        RenderOutput::Png(Some(png)) => Ok(png.bytes),
        _ => Err((error("No diagram type detected"), None)),
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
    if let Some(svg) = own_svg(lang, src, colors) {
        return svg;
    }
    let (mermaid, renderer) = prepare(src, colors, 0.0);
    let output = renderer.render(RenderRequest::svg(&mermaid, OperationControl::new(), svg_request())).map_err(error)?;
    match output {
        RenderOutput::Svg(Some(svg)) => Ok(svg.svg().to_string()),
        _ => Err(error("No diagram type detected")),
    }
}

/// A ```flow or ```sequence fence's SVG, or its error; `None` for Mermaid.
fn own_svg(lang: &str, src: &str, colors: &Colors) -> Option<Result<String, String>> {
    match lang.trim().to_lowercase().as_str() {
        "sequence" => Some(sequence::parse(src).map(|d| sequence::svg(&d, colors)).map_err(|e| format!("ERROR: [Sequence] {e}"))),
        "flow" => Some(flow::svg(src, colors).map_err(|e| format!("ERROR: [Flowchart] {e}"))),
        _ => None,
    }
}

fn error(e: impl fmt::Display) -> String {
    format!("ERROR: [Mermaid] {e}")
}

/// A length, to the hundredth: the layout's sums of tenths would otherwise
/// print as 175.20000000000002.
struct N(f64);

impl fmt::Display for N {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", (self.0 * 100.0).round() / 100.0)
    }
}

/// The system's fonts, which our own diagrams are measured and drawn in.
static FONT_DB: LazyLock<Arc<resvg::usvg::fontdb::Database>> = LazyLock::new(|| {
    let mut fonts = resvg::usvg::fontdb::Database::new();
    fonts.load_system_fonts();
    Arc::new(fonts)
});

/// SVG as a PNG, `scale` pixels to the point, in the system's fonts.
fn png(svg: &str, scale: f64) -> Result<Vec<u8>, String> {
    use resvg::{tiny_skia, usvg};
    let tree = usvg::Tree::from_str(svg, &usvg::Options { fontdb: FONT_DB.clone(), ..Default::default() }).map_err(|e| e.to_string())?;
    let scale = scale.clamp(0.25, 8.0) as f32;
    let size = tree.size().scale_by(scale).ok_or("No diagram to draw")?.to_int_size();
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("No diagram to draw")?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| e.to_string())
}

/// The Mermaid to draw, and a renderer coloured like the page.
fn prepare(src: &str, colors: &Colors, width: f64) -> (String, Renderer) {
    // Typora draws an empty Mermaid fence as the words "Empty Diagram".
    let mermaid = if src.trim().is_empty() { "flowchart TD\n  empty[\"Empty Diagram\"]" } else { src };
    let mut config = serde_json::json!({
        "theme": "base",
        "darkMode": colors.dark(),
        "fontFamily": FONTS,
        "themeVariables": colors.theme(),
        "sequence": { "diagramMarginX": 8, "diagramMarginY": 8, "boxMargin": 8 },
        "gantt": { "leftPadding": 75, "rightPadding": 20 },
    });
    if width > 0.0 {
        config["gantt"]["useWidth"] = width.into();
    }
    let config = MermaidConfig::from_value(config);
    (mermaid.to_string(), Renderer::new().with_engine(Engine::new().with_site_config(config)))
}

/// Mermaid's own pages are white; this one is the page's.
fn svg_request() -> SvgRequest {
    SvgRequest {
        pipeline: Some(SvgPipeline::resvg_safe().with_postprocessor(RootBackgroundPostprocessor::new("transparent"))),
        ..SvgRequest::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// flowchart.js's own demo, more or less.
    const FLOW: &str = "st=>start: Start:>http://www.google.com[blank]\n\
                        e=>end:>http://www.google.com\n\
                        op1=>operation: My Operation\n\
                        sub1=>subroutine: My Subroutine\n\
                        cond=>condition: Yes\n\
                        or No?:>http://www.google.com\n\
                        io=>inputoutput: catch something...\n\
                        para=>parallel: parallel tasks\n\
                        st->op1->cond\n\
                        cond(yes)->io->e\n\
                        cond(no)->para\n\
                        para(path1, bottom)->sub1(right)->op1\n\
                        para(path2, top)->op1\n\
                        op1@>cond({\"stroke\":\"red\",\"stroke-dasharray\":\"- \"})";

    #[test]
    fn renders_png() {
        let colors = Colors::parse("1a1b26,c0caf5,7aa2f7,414868");
        let zenuml = "zenuml\n  title Order\n  Client->Server.post() {\n    if(ok) {\n      return done\n    }\n  }";
        for (name, lang, src) in [
            ("mermaid", "mermaid", "graph TD\n  A[Start] --> B{Ok?}\n  B -->|yes| C[Done]"),
            ("zenuml", "mermaid", zenuml),
            ("flow", "flow", FLOW),
            ("sequence", "sequence", "A->B: hi"),
        ] {
            let png = render(lang, src, &colors, 1.0, 0.0).unwrap();
            assert_eq!(&png[1..4], b"PNG", "{name}");
            if let Ok(dir) = std::env::var("TYPO_DIAGRAM_OUT") {
                std::fs::write(format!("{dir}/{name}.png"), &png).unwrap();
            }
        }
        let (error, span) = render("mermaid", "graph TD\n  A -->", &colors, 1.0, 0.0).unwrap_err();
        assert!(error.starts_with("ERROR: [Mermaid] "), "{error}");
        assert_eq!(span, Some(11..16));
        assert_eq!(render("sequence", "A->B no colon", &colors, 1.0, 0.0).unwrap_err().1, None);
        let gantt = |width| {
            let png = render("mermaid", "gantt\n  section A\n  Task :a1, 2024-01-01, 3d", &colors, 1.0, width).unwrap();
            u32::from_be_bytes(png[16..20].try_into().unwrap())
        };
        assert!(gantt(600.0) < gantt(0.0), "{} {}", gantt(600.0), gantt(0.0));
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
