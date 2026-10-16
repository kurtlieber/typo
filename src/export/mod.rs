//! File → Export: the document written out as a page of HTML (html.rs),
//! typeset into a PDF or an image, or converted by pandoc. The writers walk
//! the same tree (tree.rs) and draw math, diagrams and code as the editor
//! does, in the colours of a `Look`.

pub mod css;
pub mod html;
pub mod pandoc;
pub mod print;
pub mod settings;
pub mod tree;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::eqnum::{self, Numbering};
use crate::md::inline::{self, Options, Refs};
use crate::md::{Block, BlockKind, Document};
use crate::palette::{self, Palette, Scheme};
use settings::{Fill, Vars};

/// How an export looks: its colours, and the preferences that decide what
/// some blocks are.
pub struct Look {
    pub scheme: Scheme,
    pub numbering: Numbering,
    /// ```math fences are display math.
    pub math_fences: bool,
    /// ```mermaid, ```flow and ```sequence fences are diagrams.
    pub diagrams: bool,
    /// Single line breaks stay line breaks (Preferences ▸ Markdown ▸
    /// Export / Print: preserve).
    pub line_breaks: bool,
    /// HTML without styles writes math as TeX, not as rendered SVG.
    pub math_tex: bool,
    /// Paragraphs' first lines are indented, and list items with them.
    pub indent_first_line: bool,
}

/// The document's equation numbers: what each display formula's rows show
/// at their right, by block index, and what each `\label` refers to.
#[derive(Default)]
pub struct Equations {
    pub rows: HashMap<usize, Vec<String>>,
    pub labels: HashMap<String, String>,
}

impl Look {
    /// Typora's defaults for what the tests leave out.
    #[cfg(test)]
    pub fn test() -> Look {
        Look {
            scheme: crate::palette::Scheme::paper(),
            numbering: eqnum::Numbering::None,
            math_fences: true,
            diagrams: true,
            line_breaks: false,
            math_tex: false,
            indent_first_line: false,
        }
    }

    /// Whether block `kind` is display math.
    pub fn is_math(&self, kind: &BlockKind) -> bool {
        match kind {
            BlockKind::Math { .. } => true,
            BlockKind::CodeFence { info, .. } => self.math_fences && fence_lang(info).eq_ignore_ascii_case("math"),
            _ => false,
        }
    }

    /// Number the display math, as the editor does.
    pub fn equations(&self, doc: &Document) -> Equations {
        let formulas: Vec<(usize, &str)> =
            doc.blocks.iter().enumerate().filter(|(_, b)| self.is_math(&b.kind)).map(|(i, b)| (i, b.text.as_str())).collect();
        let texts: Vec<&str> = formulas.iter().map(|(_, t)| *t).collect();
        let tags = eqnum::number(&texts, self.numbering);
        let rows = formulas.iter().zip(tags.rows).filter(|(_, r)| r.iter().any(|t| !t.is_empty())).map(|((i, _), r)| (*i, r)).collect();
        Equations { rows, labels: tags.labels }
    }
}

/// The formats File → Export writes: its id, then the save dialog's
/// filters, each a name and its extensions; a new file takes the first.
pub const FORMATS: &[(&str, &[(&str, &[&str])])] = &[
    ("pdf", &[("PDF", &["pdf"])]),
    ("html", &[("HTML", &["html", "htm"])]),
    ("html-plain", &[("HTML", &["html", "htm"])]),
    ("image", &[("PNG", &["png"]), ("JPEG", &["jpg", "jpeg"])]),
    ("docx", &[("Word", &["docx"])]),
    ("odt", &[("OpenOffice", &["odt"])]),
    ("rtf", &[("RTF", &["rtf"])]),
    ("epub", &[("Epub", &["epub"])]),
    ("latex", &[("LaTeX", &["tex"])]),
    ("mediawiki", &[("MediaWiki", &["wiki"])]),
    ("rst", &[("reStructuredText", &["rst"])]),
    ("textile", &[("Textile", &["textile"])]),
    ("opml", &[("OPML", &["opml"])]),
];

/// Whether pandoc writes `format`.
pub fn needs_pandoc(format: &str) -> bool {
    !matches!(format, "pdf" | "html" | "html-plain" | "image")
}

/// The save dialog's filters for `format`.
pub fn filters(format: &str) -> &'static [(&'static str, &'static [&'static str])] {
    FORMATS.iter().find(|(f, _)| *f == format).map_or(&[], |(_, f)| f)
}

/// `path`, with `format`'s extension when it has none of its own.
pub fn with_extension(path: &Path, format: &str) -> PathBuf {
    let known = filters(format).iter().flat_map(|(_, exts)| exts.iter());
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase());
    match (ext, filters(format).first()) {
        (Some(e), _) if known.clone().any(|k| *k == e) => path.to_path_buf(),
        (_, Some((_, exts))) => {
            let mut s = path.as_os_str().to_owned();
            s.push(".");
            s.push(exts[0]);
            PathBuf::from(s)
        }
        _ => path.to_path_buf(),
    }
}

/// One export: what the document is, and how and where it goes.
pub struct Job {
    pub doc: Document,
    pub refs: Refs,
    pub opts: Options,
    pub file: Option<PathBuf>,
    pub image_root: Option<PathBuf>,
    /// The theme's look; PDFs and images take paper's under a dark theme.
    pub look: Look,
    pub mono_font: String,
    pub format: String,
    pub path: PathBuf,
    /// The format's options, as Preferences ▸ Export keeps them.
    pub options: Value,
    /// The folder open beside the document.
    pub opened: String,
    /// The side panel's outline is collapsible, as the HTML sidebar is.
    pub collapsible_outline: bool,
    /// Image pixels to the page's pixel at Image Quality ▸ Auto: the
    /// screen's scale, as Typora captures its window.
    pub scale: f64,
    /// The theme's text size in pixels, for an image that keeps it.
    pub theme_font: f64,
    /// The page size and margins the print dialog set up, over the
    /// options'.
    pub page: Option<((f64, f64), [f64; 4])>,
}

/// What an export produced.
pub enum Output {
    Bytes(Vec<u8>),
    /// Premultiplied RGBA, for the caller to encode.
    Pixels { width: u32, height: u32, rgba: Vec<u8> },
    /// pandoc wrote the file itself.
    Written,
}

/// Preferences ▸ Export ▸ PDF ▸ Paper Size, in millimetres.
const PAPER: &[(&str, (f64, f64))] = &[
    ("A5", (148.0, 210.0)),
    ("A4", (210.0, 297.0)),
    ("A3", (297.0, 420.0)),
    ("Letter", (215.9, 279.4)),
    ("Legal", (215.9, 355.6)),
    ("Tabloid", (279.4, 431.8)),
];

/// Typora's margin, all round, unless the options set their own.
const MARGIN: f64 = 16.0;

impl Job {
    /// The format's options with the front matter's over them.
    fn settings(&self, front: &Value) -> Map<String, Value> {
        settings::resolve(&self.format, &self.options, front)
    }

    /// The file's name, without its extension.
    fn file_title(&self) -> String {
        self.file.as_deref().and_then(Path::file_stem).map_or_else(|| "Untitled".into(), |n| n.to_string_lossy().into_owned())
    }

    /// The page's title: the front matter's, or the file's name.
    pub fn title(&self) -> String {
        let front = settings::front_matter(&self.doc);
        match settings::text(&self.settings(&front), "title") {
            "" => self.file_title(),
            t => t.to_string(),
        }
    }

    /// The pages of a PDF, as the options and front matter set them up.
    pub fn pdf_settings(&self) -> print::PdfSettings {
        let front = settings::front_matter(&self.doc);
        let o = self.settings(&front);
        let mut set = print::PdfSettings::default();
        set.page = match settings::text(&o, "paper") {
            "custom" => (settings::number(&o, "paperWidth").unwrap_or(210.0), settings::number(&o, "paperHeight").unwrap_or(297.0)),
            p => PAPER.iter().find(|(name, _)| *name == p).map_or(set.page, |(_, size)| *size),
        };
        if settings::flag(&o, "customMargin") {
            set.margins = ["marginTop", "marginRight", "marginBottom", "marginLeft"].map(|k| settings::number(&o, k).unwrap_or(MARGIN));
        }
        if let Some((page, margins)) = self.page {
            (set.page, set.margins) = (page, margins);
        }
        // The header and footer: `${title}` is the file's name and
        // `${author}` nothing unless something says otherwise.
        let mut page_options = o.clone();
        page_options.entry("title").or_insert_with(|| self.file_title().into());
        page_options.entry("author").or_insert_with(|| "".into());
        let today = settings::today();
        let vars = Vars { front: &front, options: &page_options, output: &self.path, current: self.file.as_deref(), opened: &self.opened, today: &today };
        let numbers = [
            ("pageno", print::PAGE_NO.to_string()),
            ("totalpages", print::PAGE_COUNT.to_string()),
            ("pagecount", print::PAGE_COUNT.to_string()),
        ];
        set.header = vars.expand(settings::text(&o, "header"), &numbers, Fill::Text, None);
        set.footer = vars.expand(settings::text(&o, "footer"), &numbers, Fill::Text, None);
        set.author = settings::text(&o, "author").to_string();
        set.subject = settings::text(&o, "subject").to_string();
        set.creator = settings::text(&o, "creator").to_string();
        // Keywords, or else tags.
        let words = settings::lookup(&front, "keywords").or_else(|| settings::lookup(&front, "tags"));
        set.keywords = match words {
            Some(Value::Array(a)) => a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
            Some(Value::String(s)) => s.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
            _ => Vec::new(),
        };
        set.break_on_h1 = settings::flag(&o, "splitPages");
        set
    }

    /// After Export ▸ Run command, when it is on: the command with its
    /// variables filled in.
    pub fn command(&self) -> Option<String> {
        if !settings::flag(self.options.as_object()?, "runCommand") {
            return None;
        }
        let command = self.options.get("runCommandStr").and_then(Value::as_str).unwrap_or("");
        let front = settings::front_matter(&self.doc);
        let options = Map::new();
        let today = settings::today();
        let vars = Vars { front: &front, options: &options, output: &self.path, current: self.file.as_deref(), opened: &self.opened, today: &today };
        Some(vars.expand(command, &[], Fill::Command, None))
    }

    pub fn run(mut self) -> Result<Output, String> {
        let front = settings::front_matter(&self.doc);
        let options = self.settings(&front);
        let title = match settings::text(&options, "title") {
            "" => self.file_title(),
            t => t.to_string(),
        };
        // Preferences ▸ Export ▸ <format> ▸ Theme.
        if let Some(scheme) = theme(settings::text(&options, "exportTheme")) {
            self.look.scheme = scheme;
        }
        // PDF ▸ Append Extra Content: markup after the document.
        let extra = settings::text(&options, "appendBody");
        if self.format == "pdf" && !extra.trim().is_empty() {
            self.doc.blocks.push(Block::new(BlockKind::Html, extra.trim_end(), Vec::new()));
        }
        let pdf = self.pdf_settings();
        let today = settings::today();
        let vars = Vars { front: &front, options: &options, output: &self.path, current: self.file.as_deref(), opened: &self.opened, today: &today };
        let (doc, refs, opts) = (&self.doc, &self.refs, &self.opts);
        let (file, image_root) = (self.file.as_deref(), self.image_root.as_deref());
        match self.format.as_str() {
            "html" | "html-plain" => {
                // The markup the options and front matter add: Append
                // in <head /> (and its extra), then <body />.
                let parts = |keys: [&str; 2]| {
                    let mut titled = false;
                    let mut out = String::new();
                    for key in keys {
                        let (markup, t) = vars.markup(settings::text(&options, key));
                        titled |= t;
                        if !markup.trim().is_empty() {
                            out.push_str(markup.trim_end());
                            out.push('\n');
                        }
                    }
                    (out, titled)
                };
                let (head, titled) = parts(["appendHead", "appendHeadExtra"]);
                let (body, _) = parts(["appendBody", "appendBodyExtra"]);
                let sidebar = match (self.format == "html" && settings::flag(&options, "sidebar"), self.collapsible_outline) {
                    (false, _) => html::Sidebar::None,
                    (true, false) => html::Sidebar::Flat,
                    (true, true) => html::Sidebar::Collapsible,
                };
                let p = html::Page {
                    title: (!titled).then_some(title.as_str()),
                    styled: self.format == "html",
                    look: &self.look,
                    head: &head,
                    body: &body,
                    file,
                    image_root,
                    dest: self.path.parent(),
                    sidebar,
                };
                Ok(Output::Bytes(html::page(doc, refs, opts, &p).into_bytes()))
            }
            "pdf" | "image" => {
                // Paper is white: a dark theme prints in a light look, as
                // Typora's dark themes do in their print styles.
                if self.look.scheme.dark {
                    self.look.scheme = Scheme::paper();
                }
                let sheet = if self.format == "pdf" {
                    print::Sheet::Pages(&pdf)
                } else {
                    // Image ▸ Width and Font Size.
                    let width = settings::number(&options, "imageWidth").filter(|w| *w > 0.0).unwrap_or(640.0);
                    let font = if settings::flag(&options, "useThemeFontSize") {
                        self.theme_font
                    } else {
                        settings::number(&options, "imageFontSize").filter(|f| *f > 0.0).unwrap_or(24.0)
                    };
                    print::Sheet::Image { width, font }
                };
                let paper = print::Paper { title: &title, look: &self.look, mono_font: &self.mono_font, file, image_root, sheet };
                if self.format == "pdf" {
                    print::pdf(doc, refs, opts, &paper).map(Output::Bytes)
                } else {
                    // Image ▸ Image Quality: Auto draws at the screen's
                    // scale, unless the image is large; Medium, High and
                    // Best at one, two and three pixels to a pixel.
                    let (scale, auto) = match settings::number(&options, "quality").unwrap_or(0.0) as i32 {
                        1 => (1.0, false),
                        2 => (2.0, false),
                        3 => (3.0, false),
                        _ => (self.scale, true),
                    };
                    let (width, height, rgba) = print::image(doc, refs, opts, &paper, scale, auto)?;
                    Ok(Output::Pixels { width, height, rgba })
                }
            }
            f if needs_pandoc(f) && filters(f).is_empty() => Err(format!("Unknown export format {f}")),
            f => {
                // The format's options as pandoc's arguments, as the
                // preferences show them, then its Extra Arguments.
                let mut args: Vec<String> = Vec::new();
                let path = |key: &str| settings::text(&options, key).trim().to_string();
                match f {
                    "docx" if !path("reference-doc").is_empty() => args.extend(["--reference-doc".into(), path("reference-doc")]),
                    "odt" if !path("template").is_empty() => args.extend(["--template".into(), path("template")]),
                    "epub" => {
                        if !path("css").is_empty() {
                            args.extend(["--css".into(), path("css")]);
                        }
                        let depth = settings::number(&options, "tocDepth").or_else(|| settings::number(&options, "toc-depth")).unwrap_or(3.0);
                        args.push(format!("--toc-depth={depth}"));
                    }
                    _ => {}
                }
                let extra = vars.expand(settings::text(&options, "extraArguments"), &[], Fill::Command, None);
                args.extend(pandoc::split_args(&extra));
                let run = pandoc::Run { format: f, file, image_root, out: &self.path, args: &args, meta: &front };
                pandoc::export(doc, refs, opts, &self.look, &run)?;
                Ok(Output::Written)
            }
        }
    }
}

/// Theme `id`'s look, or None for the one in use.
fn theme(id: &str) -> Option<Scheme> {
    if id.is_empty() {
        return None;
    }
    let dir = palette::list_themes(&palette::user_themes_dir(), &palette::system_themes_dir()).into_iter().find(|t| t.id == id)?.dir;
    Some(Scheme::new(Palette::load(&dir)?))
}

/// What After Export ▸ Run command did.
pub struct Ran {
    pub ok: bool,
    /// What it wrote, and on failure what it said went wrong.
    pub output: String,
    pub error: String,
}

/// Run `command` by the shell in folder `dir`, as After Export does.
pub fn run_command(command: &str, dir: &Path) -> Ran {
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c").arg(command).stdin(std::process::Stdio::null());
    if dir.is_dir() {
        cmd.current_dir(dir);
    }
    match cmd.output() {
        Ok(out) => Ran {
            ok: out.status.success(),
            output: String::from_utf8_lossy(&out.stdout).into_owned(),
            error: if out.status.success() {
                String::new()
            } else {
                let err = String::from_utf8_lossy(&out.stderr).trim_end().to_string();
                if err.is_empty() { format!("The command failed ({})", out.status) } else { err }
            },
        },
        Err(e) => Ran { ok: false, output: String::new(), error: e.to_string() },
    }
}

/// A code fence's language: the first word of its info string.
pub fn fence_lang(info: &str) -> String {
    inline::unescape(info.trim()).split_whitespace().next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_the_format_extension() {
        assert_eq!(with_extension(Path::new("/a/b"), "pdf"), PathBuf::from("/a/b.pdf"));
        assert_eq!(with_extension(Path::new("/a/b.PDF"), "pdf"), PathBuf::from("/a/b.PDF"));
        assert_eq!(with_extension(Path::new("/a/b.v1"), "html"), PathBuf::from("/a/b.v1.html"));
        assert_eq!(with_extension(Path::new("/a/b.jpeg"), "image"), PathBuf::from("/a/b.jpeg"));
        assert_eq!(with_extension(Path::new("/a/b"), "image"), PathBuf::from("/a/b.png"));
    }
}
