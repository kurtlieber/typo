//! HTML blocks as the editor shows them. Typora lets the browser draw a
//! block's HTML; typo draws it with Qt's rich text, which knows a subset of
//! HTML 4. This module rewrites a block into that subset: tags Qt doesn't
//! know become the nearest it does (`<mark>` a coloured span, `<details>` a
//! summary that opens and closes, inline `<svg>` an image), what can't show
//! at all (video, frames) becomes a link to it, and scripts, styles and
//! forms are dropped, as Typora's sanitizer drops them.

use std::collections::HashSet;

use base64::Engine;
use ego_tree::NodeRef;
use scraper::{ElementRef, Html, Node};

/// How a block shows when the caret is elsewhere.
#[derive(Debug, PartialEq, Eq)]
pub enum Look {
    /// Drawn as HTML.
    Rendered,
    /// Its source, faint: the tags Typora won't draw (`<script>`, `<style>`…).
    Source,
    /// Its source in the comment colour: nothing but `<!-- … -->`.
    Comment,
}

/// Blocks whose first tag is one of these show their source.
const HIDDEN: &[&str] = &["script", "style", "meta", "noscript", "input", "template", "head", "canvas", "link"];

pub fn look(src: &str) -> Look {
    let s = src.trim();
    if is_comments(s) {
        return Look::Comment;
    }
    let name: String = s.strip_prefix('<').unwrap_or("").chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
    if HIDDEN.contains(&name.to_ascii_lowercase().as_str()) { Look::Source } else { Look::Rendered }
}

/// `src` when it is one `<video>` or `<audio>` and nothing else, which
/// plays in place: its tag name, source (its own or its first
/// `<source>`'s) and the attributes the player takes, as JSON for QML.
pub fn lone_media(src: &str) -> Option<String> {
    // Most text isn't, so look before parsing.
    let head = src.trim_start().get(..6).unwrap_or("");
    if !head.eq_ignore_ascii_case("<video") && !head.eq_ignore_ascii_case("<audio") {
        return None;
    }
    let doc = Html::parse_fragment(src);
    let mut nodes = doc.root_element().children().filter(|n| !matches!(n.value(), Node::Text(t) if t.trim().is_empty()));
    let e = nodes.next().and_then(ElementRef::wrap)?;
    let name = e.value().name();
    if nodes.next().is_some() || name != "video" && name != "audio" {
        return None;
    }
    let source = e.value().attr("src").or_else(|| {
        e.descendants().filter_map(ElementRef::wrap).find(|d| d.value().name() == "source").and_then(|d| d.value().attr("src"))
    })?;
    let attr = |a: &str| serde_json::to_string(e.value().attr(a).unwrap_or("")).unwrap_or_default();
    let has = |a: &str| e.value().attr(a).is_some();
    Some(format!(
        "{{\"kind\":\"{name}\",\"src\":{},\"width\":{},\"height\":{},\"autoplay\":{},\"loop\":{},\"muted\":{}}}",
        serde_json::to_string(source).unwrap_or_default(),
        attr("width"),
        attr("height"),
        has("autoplay"),
        has("loop"),
        has("muted")
    ))
}

fn is_comments(mut s: &str) -> bool {
    if !s.starts_with("<!--") {
        return false;
    }
    while let Some(rest) = s.strip_prefix("<!--") {
        let Some(end) = rest.find("-->") else { return false };
        s = rest[end + 3..].trim_start();
    }
    s.is_empty()
}

/// What the rich text takes from the theme and the page.
pub struct Page<'a> {
    /// Colours as `#rrggbb`.
    pub border: &'a str,
    pub code_background: &'a str,
    pub mark: &'a str,
    pub faint: &'a str,
    pub link: &'a str,
    pub text: &'a str,
    /// The page's width in pixels, for percentages and wide pictures.
    pub room: f64,
    /// The `<details>` the reader has opened or closed, by order in the
    /// block, which flips what their `open` attribute says.
    pub toggled: &'a HashSet<usize>,
    /// The URL to load for an image source, and its file when it is local.
    pub resolve: &'a dyn Fn(&str) -> (String, Option<std::path::PathBuf>),
}

/// The link a `<details>` summary carries, to open or close it.
pub const DETAILS: &str = "typo-details:";

/// `src` as rich text for Qt.
pub fn rich_text(src: &str, page: &Page) -> String {
    let doc = Html::parse_fragment(src);
    let mut out = Out { s: String::new(), page, details: 0 };
    out.children(*doc.root_element());
    out.s
}

struct Out<'p, 'a> {
    s: String,
    page: &'p Page<'a>,
    details: usize,
}

/// Tags Qt's rich text knows, written as they are.
const KNOWN: &[&str] = &[
    "a", "abbr", "b", "big", "blockquote", "br", "caption", "center", "cite", "code", "dd", "del", "dfn", "div", "dl",
    "dt", "em", "font", "h1", "h2", "h3", "h4", "h5", "h6", "hr", "i", "ins", "li", "ol", "p", "pre", "q", "s", "small",
    "span", "strike", "strong", "sub", "sup", "table", "tbody", "td", "tfoot", "th", "thead", "tr", "u", "ul", "var",
];

/// Tags that take their own lines, for those Qt doesn't know.
const BLOCKS: &[&str] = &[
    "address", "article", "aside", "dialog", "fieldset", "figcaption", "figure", "footer", "form", "header", "hgroup",
    "left", "legend", "main", "map", "nav", "right", "section", "summary",
];

/// Dropped with everything in them.
const DROP: &[&str] = &[
    "button", "head", "input", "link", "meta", "noscript", "script", "select", "style", "template", "textarea", "title",
    "canvas", "wbr",
];

const VOID: &[&str] = &["br", "hr"];

/// Attributes kept on the tags written out.
const ATTRS: &[&str] = &[
    "align", "valign", "style", "color", "face", "size", "width", "height", "colspan", "rowspan", "start", "type", "dir",
    "title", "bgcolor", "border", "cellpadding", "cellspacing", "name",
];

impl Out<'_, '_> {
    fn children(&mut self, n: NodeRef<Node>) {
        for c in n.children() {
            self.node(c);
        }
    }

    fn node(&mut self, n: NodeRef<Node>) {
        match n.value() {
            Node::Text(t) => self.s.push_str(&html_escape::encode_text(&**t)),
            Node::Element(_) => {
                if let Some(e) = ElementRef::wrap(n) {
                    self.element(e);
                }
            }
            _ => {}
        }
    }

    fn element(&mut self, e: ElementRef) {
        let name = e.value().name().to_ascii_lowercase();
        let name = name.as_str();
        match name {
            _ if DROP.contains(&name) => {}
            "mark" => self.wrap(e, &format!("<span style=\"background-color:{}\">", self.page.mark), "</span>"),
            "kbd" | "samp" | "tt" => {
                self.wrap(e, &format!("<code style=\"background-color:{}\">", self.page.code_background), "</code>")
            }
            "details" => self.details(e),
            "img" => self.image(e),
            "svg" => self.svg(e),
            "picture" => {
                if let Some(img) = e.descendants().filter_map(ElementRef::wrap).find(|d| d.value().name() == "img") {
                    self.image(img);
                }
            }
            "video" | "audio" | "iframe" | "embed" | "object" => self.media(e, name),
            "a" => {
                // Qt draws links in its own blue; the theme's, unless the
                // link says otherwise.
                let href = e.value().attr("href").unwrap_or("");
                let style = e.value().attr("style").unwrap_or("");
                let color = if declares(style, "color") {
                    String::new()
                } else {
                    format!("color:{};", self.page.link)
                };
                let open = format!(
                    "<a href=\"{}\" style=\"{color}{}\">",
                    html_escape::encode_double_quoted_attribute(href),
                    html_escape::encode_double_quoted_attribute(style)
                );
                self.wrap(e, &open, "</a>");
            }
            "table" => {
                let open = format!(
                    "<table{} border=\"1\" cellspacing=\"0\" cellpadding=\"6\" style=\"border-collapse:collapse;border-color:{}\">",
                    attrs(e, &["align", "width", "bgcolor"]),
                    self.page.border
                );
                self.wrap(e, &open, "</table>");
            }
            _ if VOID.contains(&name) => {
                self.s.push_str(&format!("<{name}{}>", attrs(e, ATTRS)));
            }
            _ if KNOWN.contains(&name) => {
                let open = format!("<{name}{}>", attrs(e, ATTRS));
                self.wrap(e, &open, &format!("</{name}>"));
            }
            _ if BLOCKS.contains(&name) => {
                let open = format!("<div{}>", attrs(e, ATTRS));
                self.wrap(e, &open, "</div>");
            }
            _ => {
                let open = format!("<span{}>", attrs(e, ATTRS));
                self.wrap(e, &open, "</span>");
            }
        }
    }

    fn wrap(&mut self, e: ElementRef, open: &str, close: &str) {
        self.s.push_str(open);
        self.children(*e);
        self.s.push_str(close);
    }

    /// A summary line that opens and closes, as a browser draws it, and
    /// the rest when open.
    fn details(&mut self, e: ElementRef) {
        let i = self.details;
        self.details += 1;
        let open = e.value().attr("open").is_some() != self.page.toggled.contains(&i);
        let summary = e.children().filter_map(ElementRef::wrap).find(|c| c.value().name() == "summary");
        self.s.push_str(&format!("<div><a href=\"{DETAILS}{i}\" style=\"text-decoration:none;color:{}\">", self.page.text));
        self.s.push_str(if open { "▼ " } else { "▶ " });
        match summary {
            Some(s) => self.children(*s),
            None => self.s.push_str("Details"),
        }
        self.s.push_str("</a></div>");
        if open {
            self.s.push_str("<div>");
            for c in e.children() {
                if summary.is_none_or(|s| s.id() != c.id()) {
                    self.node(c);
                }
            }
            self.s.push_str("</div>");
        }
    }

    fn image(&mut self, e: ElementRef) {
        let el = e.value();
        let src = el.attr("src").unwrap_or("");
        let (url, file) = (self.page.resolve)(src);
        let natural = file.and_then(|f| imagesize::size(f).ok()).map(|s| (s.width as f64, s.height as f64));
        let (w, h) = size(length(el.attr("width"), el, "width", self.page.room), length(el.attr("height"), el, "height", 0.0), natural, self.page.room);
        let mut out = format!("<img src=\"{}\"", html_escape::encode_double_quoted_attribute(&url));
        if let Some(w) = w {
            out.push_str(&format!(" width=\"{}\"", w.round()));
        }
        if let Some(h) = h {
            out.push_str(&format!(" height=\"{}\"", h.round()));
        }
        if let Some(align) = el.attr("align") {
            out.push_str(&format!(" align=\"{}\"", html_escape::encode_double_quoted_attribute(align)));
        }
        out.push('>');
        self.s.push_str(&out);
    }

    /// Inline SVG as a picture of itself.
    fn svg(&mut self, e: ElementRef) {
        let mut xml = e.html();
        if !xml.contains("xmlns=") {
            xml = xml.replacen("<svg", "<svg xmlns=\"http://www.w3.org/2000/svg\"", 1);
        }
        let el = e.value();
        let px = |v: Option<&str>| v.and_then(|v| v.trim().trim_end_matches("px").parse::<f64>().ok());
        let data = base64::engine::general_purpose::STANDARD.encode(xml.as_bytes());
        let mut out = format!("<img src=\"data:image/svg+xml;base64,{data}\"");
        if let Some(w) = px(el.attr("width")) {
            out.push_str(&format!(" width=\"{}\"", w.min(self.page.room).round()));
        }
        if let Some(h) = px(el.attr("height")) {
            out.push_str(&format!(" height=\"{}\"", h.round()));
        }
        out.push('>');
        self.s.push_str(&out);
    }

    /// What Qt can't play or frame, as a link to it.
    fn media(&mut self, e: ElementRef, name: &str) {
        let src = e.value().attr("src").or(e.value().attr("data")).map(str::to_string).or_else(|| {
            e.descendants().filter_map(ElementRef::wrap).find_map(|d| d.value().attr("src").map(str::to_string))
        });
        let src = src.unwrap_or_default();
        let label = match name {
            "video" => "▶ Video",
            "audio" => "▶ Audio",
            _ => "↗ Embedded page",
        };
        let shown = src.rsplit('/').find(|s| !s.is_empty()).unwrap_or("");
        self.s.push_str(&format!(
            "<p><a href=\"{}\" style=\"color:{}\">{label}{}{}</a></p>",
            html_escape::encode_double_quoted_attribute(&src),
            self.page.faint,
            if shown.is_empty() { "" } else { ": " },
            html_escape::encode_text(shown)
        ));
    }
}

/// An element's attributes from `keep`, written out.
fn attrs(e: ElementRef, keep: &[&str]) -> String {
    let mut out = String::new();
    for (k, v) in e.value().attrs() {
        let k = k.to_ascii_lowercase();
        if keep.contains(&k.as_str()) {
            out.push_str(&format!(" {k}=\"{}\"", html_escape::encode_double_quoted_attribute(v)));
        }
    }
    out
}

/// Whether CSS declarations `style` set property `prop`.
fn declares(style: &str, prop: &str) -> bool {
    style.split(';').any(|d| d.split_once(':').is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case(prop)))
}

/// A length from an attribute or the `style` property of the same name:
/// pixels, or a percentage of `of`.
fn length(attr: Option<&str>, el: &scraper::node::Element, prop: &str, of: f64) -> Option<f64> {
    let style = el.attr("style").and_then(|s| {
        s.split(';').find_map(|d| {
            let (k, v) = d.split_once(':')?;
            k.trim().eq_ignore_ascii_case(prop).then(|| v.trim().to_string())
        })
    });
    let v = style.or(attr.map(str::to_string))?;
    let v = v.trim();
    if let Some(p) = v.strip_suffix('%') {
        return p.trim().parse::<f64>().ok().filter(|_| of > 0.0).map(|p| p * of / 100.0);
    }
    v.trim_end_matches("px").trim().parse().ok()
}

/// The size to draw a picture at: as its tag says, keeping its proportions
/// when only one side is given, and never wider than `room`.
fn size(w: Option<f64>, h: Option<f64>, natural: Option<(f64, f64)>, room: f64) -> (Option<f64>, Option<f64>) {
    let (mut w, mut h) = match (w, h, natural) {
        (Some(w), Some(h), _) => (Some(w), Some(h)),
        (Some(w), None, Some((nw, nh))) if nw > 0.0 => (Some(w), Some(w * nh / nw)),
        (None, Some(h), Some((nw, nh))) if nh > 0.0 => (Some(h * nw / nh), Some(h)),
        (None, None, Some((nw, nh))) => (Some(nw), Some(nh)),
        (w, h, _) => (w, h),
    };
    if let Some(width) = w
        && room > 0.0
        && width > room
    {
        h = h.map(|h| h * room / width);
        w = Some(room);
    }
    (w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rich(src: &str, toggled: &[usize]) -> String {
        let toggled: HashSet<usize> = toggled.iter().copied().collect();
        let resolve = |s: &str| (format!("file:///doc/{s}"), None);
        let page = Page {
            border: "#111111",
            code_background: "#222222",
            mark: "#333333",
            faint: "#444444",
            link: "#0000ee",
            text: "#000000",
            room: 500.0,
            toggled: &toggled,
            resolve: &resolve,
        };
        rich_text(src, &page)
    }

    #[test]
    fn looks() {
        assert_eq!(look("<div>x</div>"), Look::Rendered);
        assert_eq!(look("<style>\np {}\n</style>"), Look::Source);
        assert_eq!(look("<SCRIPT src=x></SCRIPT>"), Look::Source);
        assert_eq!(look("<!-- a -->\n<!-- b -->"), Look::Comment);
        assert_eq!(look("<!-- a --><p>x</p>"), Look::Rendered);
    }

    #[test]
    fn lone_media_plays() {
        let m = |s| lone_media(s).map(|j| serde_json::from_str::<serde_json::Value>(&j).unwrap());
        let v = m("<video src=\"a b.mp4\" width=\"50%\" controls loop></video>").unwrap();
        assert_eq!((v["kind"].as_str(), v["src"].as_str(), v["width"].as_str()), (Some("video"), Some("a b.mp4"), Some("50%")));
        assert_eq!((v["loop"].as_bool(), v["autoplay"].as_bool()), (Some(true), Some(false)));
        let a = m(" <audio controls>\n  <source src=\"s.ogg\">\n  Not supported.\n</audio>\n").unwrap();
        assert_eq!((a["kind"].as_str(), a["src"].as_str()), (Some("audio"), Some("s.ogg")));
        assert_eq!(m("<video src=a.mp4></video> and text"), None);
        assert_eq!(m("<video></video>"), None);
        assert_eq!(m("<div><video src=a.mp4></video></div>"), None);
    }

    #[test]
    fn rewrites_into_what_qt_knows() {
        assert_eq!(
            rich("<div align=\"center\" onclick=\"x()\"><b>B</b> <mark>m</mark> <kbd>K</kbd><script>bad()</script></div>", &[]),
            "<div align=\"center\"><b>B</b> <span style=\"background-color:#333333\">m</span> <code style=\"background-color:#222222\">K</code></div>"
        );
        assert_eq!(rich("<section><custom-tag>x</custom-tag></section>", &[]), "<div><span>x</span></div>");
        assert!(rich("<table><tr><td>1</td></tr></table>", &[]).starts_with("<table border=\"1\" cellspacing=\"0\""));
        assert_eq!(rich("a<br>b<hr>", &[]), "a<br>b<hr>");
    }

    #[test]
    fn details_open_and_close() {
        let src = "<details><summary>More</summary>Hidden</details><details open>Shown</details>";
        let closed = rich(src, &[]);
        assert!(closed.contains("typo-details:0\"") && closed.contains("▶ More") && !closed.contains("Hidden"), "{closed}");
        assert!(closed.contains("▼ Details") && closed.contains("Shown"), "{closed}");
        let toggled = rich(src, &[0, 1]);
        assert!(toggled.contains("Hidden") && !toggled.contains("Shown"), "{toggled}");
    }

    #[test]
    fn pictures() {
        assert_eq!(rich("<img src=\"a.png\" width=\"50%\">", &[]), "<img src=\"file:///doc/a.png\" width=\"250\">");
        assert_eq!(rich("<img src=\"a.png\" style=\"width:900px;height:300px\">", &[]), "<img src=\"file:///doc/a.png\" width=\"500\" height=\"167\">");
        let svg = rich("<svg width=\"10\" height=\"20\"><rect width=\"10\" height=\"20\"/></svg>", &[]);
        assert!(svg.starts_with("<img src=\"data:image/svg+xml;base64,") && svg.ends_with(" width=\"10\" height=\"20\">"), "{svg}");
        assert_eq!(
            rich("<video src=\"clips/a.mp4\" controls></video>", &[]),
            "<p><a href=\"clips/a.mp4\" style=\"color:#444444\">▶ Video: a.mp4</a></p>"
        );
    }
}
