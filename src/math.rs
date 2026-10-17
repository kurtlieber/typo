//! Math: LaTeX is converted to Typst by mitex and typeset by typst, in
//! process, into an image the editor shows in place of the source. mitex's
//! Typst output calls the functions in third_party/mitex, which this module
//! serves to typst as files, along with the converted formula.

use std::borrow::Cow;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};

use regex::{Captures, Regex};
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::layout::{Abs, Frame, FrameItem};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::{LazyHash, Scalar};
use typst::{Library, LibraryExt, World};

/// What an empty math block shows, as Typora does.
const EMPTY: &str = r"<Empty \space Math \space Block>";

/// The point size formulas are set at; `render` scales the result to pixels.
const SIZE_PT: f64 = 10.0;

/// mitex's Typst functions, which converted formulas call.
pub const SPECS: [(&str, &str); 3] = [
    ("/specs/mod.typ", include_str!("../third_party/mitex/specs/mod.typ")),
    ("/specs/prelude.typ", include_str!("../third_party/mitex/specs/prelude.typ")),
    ("/specs/latex/standard.typ", include_str!("../third_party/mitex/specs/latex/standard.typ")),
];

/// A typeset formula: premultiplied RGBA pixels, row by row.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// How far down the formula's baseline is, in pixels, for inline math
    /// to sit on the line around it.
    pub baseline: f64,
    /// For display math of several rows, where each row's baseline is, as
    /// a part of the height, for its number to sit level with it.
    pub rows: Vec<f64>,
}

/// What follows inline math so its baseline can be found: a dot no one
/// sees, which takes no room.
const MARK: &str = "#box(width: 0pt, text(fill: rgb(0, 0, 0, 0))[.])";

/// What starts each row of a copy of a formula to find the rows'
/// baselines: a character no formula has, smashed so it adds no height.
const ROW_MARK: char = '\u{E000}';

/// Typeset `tex` in `color` (`rrggbb`), with an em of `em_px` pixels; display
/// math is centred on its own line, inline math sits in text. The error is
/// what is wrong with the formula, in one line.
pub fn render(tex: &str, display: bool, em_px: f64, color: &str) -> Result<Image, String> {
    let page = typeset(tex, display, color)?;
    let scale = Scalar::new((em_px / SIZE_PT).clamp(0.1, 20.0));
    let pixmap = typst_render::render(&page, &typst_render::RenderOptions { pixel_per_pt: scale, render_bleed: false });
    let baseline = baseline(&page, display).to_pt() * scale.get();
    let rows = if display { rows(tex, &page) } else { Vec::new() };
    Ok(Image { width: pixmap.width(), height: pixmap.height(), rgba: pixmap.take(), baseline, rows })
}

/// Where each row of `tex`, typeset as `page`, has its baseline, as a part
/// of the page's height; empty for one row, or when they can't be found.
/// Rows of a height can't be assumed: a fraction makes its row taller.
fn rows(tex: &str, page: &typst_layout::Page) -> Vec<f64> {
    let starts = crate::eqnum::row_starts(tex);
    let mut marked = tex.to_string();
    for &at in starts.iter().rev() {
        marked.insert_str(at, &format!("\\smash{{\\text{{{ROW_MARK}}}}}"));
    }
    let height = page.frame.height();
    // The marks move columns sideways, but must leave rows where they are.
    let Some(copy) = typeset(&marked, true, "000000").ok().filter(|c| (c.frame.height() - height).abs() < Abs::pt(0.1))
    else {
        return Vec::new();
    };
    let mut ys = Vec::new();
    marks(&copy.frame, Abs::zero(), &mut ys);
    if ys.len() != starts.len() {
        return Vec::new();
    }
    ys.sort_by(f64::total_cmp);
    ys.iter().map(|y| y / height.to_pt()).collect()
}

/// How far down `frame` each row mark is, in points.
fn marks(frame: &Frame, top: Abs, ys: &mut Vec<f64>) {
    for (pos, item) in frame.items() {
        match item {
            FrameItem::Text(text) if text.text.starts_with(ROW_MARK) => ys.push((top + pos.y).to_pt()),
            FrameItem::Group(group) => marks(&group.frame, top + pos.y, ys),
            _ => {}
        }
    }
}

/// A formula as SVG for export, sized in ems so it scales with the text
/// around it; inline math is lowered to sit on the line.
pub fn svg(tex: &str, display: bool, color: &str) -> Result<String, String> {
    let page = typeset(tex, display, color)?;
    let svg = typst_svg::svg(&page, &typst_svg::SvgOptions::default());
    let em = |abs: Abs| format!("{:.3}em", abs.to_pt() / SIZE_PT);
    let size = page.frame.size();
    let lower = size.y - baseline(&page, display);
    let style = if display { String::new() } else { format!(" style=\"vertical-align: -{}\"", em(lower)) };
    // typst gives the size in points; the page's text sets the em.
    let open = svg.find("<svg").ok_or("no drawing")?;
    let close = open + svg[open..].find('>').ok_or("no drawing")?;
    let tag = &svg[open..close];
    let tag = SIZE_ATTR.replace_all(tag, "");
    Ok(format!("{tag} width=\"{}\" height=\"{}\"{style}{}", em(size.x), em(size.y), &svg[close..]))
}

/// Whether formulas use the physics package, as Preferences say.
static PHYSICS: AtomicBool = AtomicBool::new(false);

pub fn set_physics(on: bool) {
    PHYSICS.store(on, Ordering::Relaxed);
}

static SIZE_ATTR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\s(width|height)="[^"]*""#).unwrap());

/// `tex` as the Typst math mitex makes of it, which calls the functions in
/// `SPECS`.
pub fn convert(tex: &str) -> Result<String, String> {
    let tex = if tex.trim().is_empty() { EMPTY } else { tex };
    let tex = &if PHYSICS.load(Ordering::Relaxed) { crate::physics::expand(tex) } else { tex.to_string() };
    // mitex says "error: unknown command: \foo"; the block says what's wrong.
    mitex::convert_math(&known_environments(tex, false), None).map_err(|e| e.trim_start_matches("error: ").to_string())
}

/// What is wrong with a formula, if anything, in one line.
pub fn check(tex: &str, display: bool) -> Result<(), String> {
    typeset(tex, display, "000000").map(|_| ())
}

/// Lay out a formula on a page of its own, as small as the formula.
fn typeset(tex: &str, display: bool, color: &str) -> Result<typst_layout::Page, String> {
    let formula = convert(tex)?;
    let color: String = color.chars().filter(char::is_ascii_hexdigit).take(8).collect();
    // Inline math is a line of its own, as tall as its ink, not the cap
    // height to baseline a line of text would be.
    let mark = if display { "" } else { MARK };
    let main = format!(
        "#import \"/specs/mod.typ\": mitex-scope\n\
         #set page(width: auto, height: auto, margin: 1pt, fill: none)\n\
         #set text(size: {SIZE_PT}pt, fill: rgb(\"#{color}\"), top-edge: \"bounds\", bottom-edge: \"bounds\")\n\
         #math.equation(block: {display}, eval(\"$\" + read(\"/formula\") + \"$\", scope: mitex-scope)){mark}\n"
    );
    let world = Formula { main: Source::new(id("/main.typ"), main), formula };
    let result = typst::compile::<typst_layout::PagedDocument>(&world).output;
    // Forget what recent compiles memoised, or memory grows with every edit.
    typst::comemo::evict(30);
    let doc = result.map_err(|errors| errors.first().map(|e| e.message.to_string()).unwrap_or_default())?;
    doc.pages().first().cloned().ok_or_else(|| "nothing to show".to_string())
}

/// How far down the page the formula's baseline is: at the mark after
/// inline math, the bottom for display math.
fn baseline(page: &typst_layout::Page, display: bool) -> Abs {
    match last_text(&page.frame, Abs::zero()) {
        Some((y, ".")) if !display => y,
        _ => page.frame.height(),
    }
}

static ENVIRONMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\\(begin|end)\s*\{(gather\*|multline\*?|flalign\*?|x{0,2}alignat\*?|eqnarray\*?)\}(\s*\{\d+\})?").unwrap()
});

static EQNARRAY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)\\begin\s*\{eqnarray(\*?)\}(.*?)\\end\s*\{eqnarray\*?\}").unwrap());

static AMPERSAND: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(^|[^\\])&").unwrap());

static LABEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\\label\s*\{[^{}]*\}").unwrap());

/// `tex` with the AMS environments mitex doesn't know written as the
/// nearest it does, laid out alike: rows centred (`gather`) or aligned at
/// `&` (`align`). An `eqnarray`'s right, centre and left columns become
/// `align`'s right and left, the relation joining the left. For MathML,
/// only those math-core lacks are rewritten, starred ones staying starred.
fn known_environments(tex: &str, mathml: bool) -> Cow<'_, str> {
    let tex = EQNARRAY.replace_all(tex, |c: &Captures| {
        let rows: Vec<String> = c[2]
            .split("\\\\")
            .map(|row| {
                let mut n = 0;
                AMPERSAND
                    .replace_all(row, |a: &Captures| {
                        n += 1;
                        if n == 2 { a[1].to_string() } else { a[0].to_string() }
                    })
                    .into_owned()
            })
            .collect();
        let star = if mathml { &c[1] } else { "" };
        format!("\\begin{{align{star}}}{}\\end{{align{star}}}", rows.join("\\\\"))
    });
    match ENVIRONMENT.replace_all(&tex, |c: &Captures| {
        let (verb, env) = (&c[1], &c[2]);
        if mathml && (env.starts_with("gather") || env.starts_with("multline")) {
            return c[0].to_string();
        }
        // alignat's column count isn't content; anything else is.
        let after = if env.contains("alignat") { "" } else { c.get(3).map_or("", |m| m.as_str()) };
        let env = match env {
            _ if env.starts_with("gather") || env.starts_with("multline") => "gather",
            _ if mathml && env.ends_with('*') => "align*",
            _ => "align",
        };
        format!("\\{verb}{{{env}}}{after}")
    }) {
        Cow::Borrowed(_) => tex,
        Cow::Owned(s) => Cow::Owned(s),
    }
}

/// `tex` as MathML, as Copy as MathML gives it: display math with what
/// each row shows at its right, `shown` (as `eqnum::Tags` has it).
pub fn mathml(tex: &str, display: bool, shown: &[String]) -> Result<String, String> {
    static CONVERTER: LazyLock<math_core::LatexToMathML> = LazyLock::new(|| {
        let config = math_core::MathCoreConfig {
            pretty_print: math_core::PrettyPrint::Always,
            xml_namespace: true,
            ..Default::default()
        };
        math_core::LatexToMathML::new(config).expect("no macros to fail")
    });
    // math-core numbers equations itself, from one in each formula; the
    // document's numbers go in as tags instead. Inline math has no number
    // for a label to name.
    let tex = if display { crate::eqnum::tagged(tex, shown) } else { LABEL.replace_all(tex, "").into_owned() };
    let display = if display { math_core::MathDisplay::Block } else { math_core::MathDisplay::Inline };
    CONVERTER
        .convert_with_local_state(&known_environments(&tex, true), display)
        .map(|r| r.mathml)
        .map_err(|e| e.to_string())
}

/// Where the last run of text in `frame` sits, and what it says.
fn last_text(frame: &Frame, top: Abs) -> Option<(Abs, &str)> {
    frame.items().rev().find_map(|(pos, item)| match item {
        FrameItem::Text(text) => Some((top + pos.y, text.text.as_str())),
        FrameItem::Group(group) => last_text(&group.frame, top + pos.y),
        _ => None,
    })
}

fn id(path: &str) -> FileId {
    FileId::new(RootedPath::new(VirtualRoot::Project, VirtualPath::new(path).expect("valid path")))
}

static LIBRARY: LazyLock<LazyHash<Library>> = LazyLock::new(|| LazyHash::new(Library::default()));

static FONTS: LazyLock<Vec<Font>> =
    LazyLock::new(|| typst_assets::fonts().flat_map(|data| Font::iter(Bytes::new(data))).collect());

static BOOK: LazyLock<LazyHash<FontBook>> = LazyLock::new(|| LazyHash::new(FontBook::from_fonts(FONTS.iter())));

/// One formula's compile: the page around it, the formula, and mitex's specs.
struct Formula {
    main: Source,
    formula: String,
}

impl World for Formula {
    fn library(&self) -> &LazyHash<Library> {
        &LIBRARY
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &BOOK
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            return Ok(self.main.clone());
        }
        let path = id.vpath().get_with_slash();
        match SPECS.iter().find(|(p, _)| *id.root() == VirtualRoot::Project && *p == path) {
            Some((_, text)) => Ok(Source::new(id, text.to_string())),
            None => Err(FileError::NotFound(path.into())),
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if *id.root() == VirtualRoot::Project && id.vpath().get_with_slash() == "/formula" {
            return Ok(Bytes::from_string(self.formula.clone()));
        }
        self.source(id).map(Bytes::from_string)
    }

    fn font(&self, index: usize) -> Option<Font> {
        FONTS.get(index).cloned()
    }

    fn today(&self, _: Option<Duration>) -> Option<Datetime> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(tex: &str) -> (u32, u32) {
        let image = render(tex, true, 20.0, "cdd6f4").unwrap_or_else(|e| panic!("{tex}: {e}"));
        assert_eq!(image.rgba.len(), (image.width * image.height * 4) as usize);
        (image.width, image.height)
    }

    #[test]
    fn typesets_latex() {
        let (w, h) = size(r"\frac{a}{b} + \sqrt{x^2+1}");
        assert!(w > 80 && h > 30, "{w}x{h}");
        // A fraction stands taller than a lone letter.
        assert!(size("x").1 < h);
        size(r"\begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}");
        size(r"f(x) = \begin{cases} 1 & x > 0 \\ 0 & \text{otherwise} \end{cases}");
        size(r"\begin{aligned} a &= b \\ c &= d \end{aligned}");
        // mitex's command table matches the vendored specs: \color needs both.
        size(r"\color{red}{x} + \textcolor{blue}{y}");
        // AMS environments mitex lacks.
        size(r"\begin{multline} a \\ b \end{multline}");
        size(r"\begin{gather*} a \\ b \end{gather*}");
        size(r"\begin{alignat*}{2} a &= b \end{alignat*}");
        size(r"\begin{flalign} a &= b \end{flalign}");
        size(r"\begin{eqnarray} a &=& b \\ c &=& d \end{eqnarray}");
    }

    #[test]
    fn converts_to_mathml() {
        let mml = mathml(r"\frac{a}{b}", true, &[]).unwrap();
        assert!(mml.starts_with("<math xmlns=\"http://www.w3.org/1998/Math/MathML\" display=\"block\">"), "{mml}");
        assert!(mml.contains("<mfrac>"), "{mml}");
        assert!(!mathml("x", false, &[]).unwrap().contains("display=\"block\""));
        assert!(mathml(r"\frac{a}{", true, &[]).is_err());
        // The document's numbers, not math-core's, and labels anywhere.
        let shown = |rows: &[&str]| rows.iter().map(|r| r.to_string()).collect::<Vec<_>>();
        let mml = mathml(r"x^2 \label{sq}", true, &shown(&["(3)"])).unwrap();
        assert!(mml.contains("<mtext>(3)</mtext>") && mml.contains("id=\"sq\""), "{mml}");
        let mml = mathml(r"\begin{eqnarray} a &=& b \\ c &=& d \notag \end{eqnarray}", true, &shown(&["(4)", ""])).unwrap();
        assert!(mml.contains("<mtext>(4)</mtext>") && !mml.contains("(1)") && !mml.contains("(2)"), "{mml}");
        assert!(mathml(r"\begin{alignat}{2} a &= b \end{alignat}", true, &[]).unwrap().contains("<mi>b</mi>"));
        assert!(mathml(r"y \label{a}", false, &[]).is_ok());
    }

    #[test]
    fn rewrites_unknown_environments() {
        assert_eq!(known_environments(r"\begin{alignat}{2} a \end{alignat}", false), r"\begin{align} a \end{align}");
        assert_eq!(
            known_environments(r"\begin{eqnarray*}a &=& b \\ c & \& & d\end{eqnarray*}", false),
            r"\begin{align}a &= b \\ c & \&  d\end{align}"
        );
        assert_eq!(known_environments(r"\begin{gather*}{2}\end{gather*}", false), r"\begin{gather}{2}\end{gather}");
        assert_eq!(known_environments(r"\begin{align*}x\end{align*}", false), r"\begin{align*}x\end{align*}");
        // math-core knows more, and numbers what isn't starred.
        assert_eq!(known_environments(r"\begin{eqnarray*}a &=& b\end{eqnarray*}", true), r"\begin{align*}a &= b\end{align*}");
        assert_eq!(known_environments(r"\begin{flalign*}a\end{flalign*}", true), r"\begin{align*}a\end{align*}");
        assert_eq!(known_environments(r"\begin{multline*}a\end{multline*}", true), r"\begin{multline*}a\end{multline*}");
    }

    #[test]
    fn scales_with_the_em() {
        let small = render("x+y", false, 10.0, "000000").unwrap();
        let large = render("x+y", false, 40.0, "000000").unwrap();
        assert!(large.width > 3 * small.width);
    }

    #[test]
    fn inline_math_sits_on_the_baseline() {
        let inline = |tex| render(tex, false, 20.0, "000000").unwrap();
        // A letter barely reaches below the line; a fraction's denominator
        // does, and nothing is cut off.
        let x = inline("x");
        assert!(x.baseline > 0.7 * x.height as f64 && x.baseline < x.height as f64, "{} of {}", x.baseline, x.height);
        let frac = inline(r"\frac{a}{b}");
        assert!(frac.height as f64 > 1.6 * x.height as f64, "{} vs {}", frac.height, x.height);
        assert!(frac.baseline < 0.7 * frac.height as f64, "{} of {}", frac.baseline, frac.height);
        // The mark takes no room.
        assert!(inline("x").width == render("x", true, 20.0, "000000").unwrap().width);
    }

    #[test]
    fn finds_each_rows_baseline() {
        let rows = |tex| render(tex, true, 20.0, "000000").unwrap().rows;
        let gaps = |r: Vec<f64>| {
            assert!(r.len() == 3 && r[0] > 0.0 && r[2] < 1.0, "{r:?}");
            (r[2] - r[1]) - (r[1] - r[0])
        };
        // Rows are as tall as what is in them: a fraction's is taller.
        assert!(gaps(rows(r"\begin{align} a &= b \\ b &= a \\ a &= b \end{align}")).abs() < 0.01);
        assert!(gaps(rows(r"\begin{align} a &= b \\ \frac{c}{d} &= e \\ f &= g \end{align}")) > 0.03);
        assert!(rows("x").is_empty() && rows(r"\begin{align} x \end{align}").is_empty());
        assert!(rows(r"\begin{gather} x \\ y \end{gather}").len() == 2);
        assert!(render(r"\begin{align} a \\ b \end{align}", false, 20.0, "000000").unwrap().rows.is_empty());
    }

    #[test]
    fn draws_svg_in_ems() {
        let s = svg(r"\frac{a}{b}", false, "333333").unwrap();
        assert!(s.starts_with("<svg") && s.contains("em\" height=") && s.contains("vertical-align: -"), "{}", &s[..200]);
        assert!(!svg("x", true, "000000").unwrap().contains("vertical-align"));
        assert!(svg(r"\foo", true, "000000").is_err());
    }

    #[test]
    fn an_empty_block_says_so() {
        assert!(size("").0 > size("x").0 * 5);
    }

    #[test]
    fn reports_errors() {
        let error = |tex| render(tex, true, 20.0, "000000").err().unwrap_or_default();
        assert_eq!(error(r"x + \foo"), r"unknown command: \foo");
        assert!(!error("a}").is_empty());
        // Unfinished input still shows what there is, as MathJax does.
        assert_eq!(error(r"\frac{a}{"), "");
    }
}

