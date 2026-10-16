//! Images written as HTML, `<img src="a.png" style="zoom:50%;" />`, which
//! Typora draws like `![…](…)` images, and the edits that make them: Zoom
//! Image puts a `zoom` in the tag's style, and Switch Image Syntax turns one
//! form into the other.

use std::sync::LazyLock;

use regex::Regex;

/// What an `<img>` tag says about the image. Sizes are CSS values as
/// written (`200px`, `50%`); a bare number in a `width` or `height`
/// attribute is in pixels, as a browser takes it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImgTag {
    pub src: String,
    pub alt: String,
    pub title: String,
    pub width: String,
    pub height: String,
    pub zoom: String,
}

static ATTR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\s([A-Za-z_:][A-Za-z0-9_.:\-]*)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?"#).unwrap()
});

/// The tag's attributes in order, names lowercased, values as written
/// (still escaped) with where each attribute starts and ends.
fn attrs(tag: &str) -> Vec<(String, String, std::ops::Range<usize>)> {
    // Skip `<img` so the name isn't taken for an attribute.
    let body = tag.find(|c: char| c.is_whitespace()).unwrap_or(tag.len());
    ATTR.captures_iter(&tag[body..])
        .map(|c| {
            let value = c.get(2).or(c.get(3)).or(c.get(4)).map_or("", |m| m.as_str());
            let m = c.get(0).unwrap();
            (c[1].to_ascii_lowercase(), value.to_string(), body + m.start()..body + m.end())
        })
        .collect()
}

/// The value of attribute `name`, its entities decoded.
pub fn attr(tag: &str, name: &str) -> Option<String> {
    attrs(tag).into_iter().find(|(n, ..)| n == name).map(|(_, v, _)| html_escape::decode_html_entities(&v).into_owned())
}

/// The value CSS property `prop` has in a tag's `style` attribute.
pub fn style_of(tag: &str, prop: &str) -> Option<String> {
    declarations(&attr(tag, "style")?).into_iter().rev().find(|(p, _)| p == prop).map(|(_, v)| v)
}

/// The declarations of a `style` attribute, in order.
fn declarations(style: &str) -> Vec<(String, String)> {
    style
        .split(';')
        .filter_map(|d| d.split_once(':'))
        .map(|(p, v)| (p.trim().to_ascii_lowercase(), v.trim().to_string()))
        .filter(|(p, v)| !p.is_empty() && !v.is_empty())
        .collect()
}

/// An `<img>` tag Typora draws as an image: one with a `src`.
pub fn parse(tag: &str) -> Option<ImgTag> {
    let src = attr(tag, "src").filter(|s| !s.trim().is_empty())?;
    let style = declarations(&attr(tag, "style").unwrap_or_default());
    let from_style = |p: &str| style.iter().find(|(n, _)| n == p).map(|(_, v)| v.clone());
    // A size in the style wins over the attribute; a bare number is pixels.
    let size = |p: &str| {
        from_style(p).or_else(|| attr(tag, p).map(|v| v.trim().to_string())).map_or_else(String::new, |v| {
            if v.ends_with(|c: char| c.is_ascii_digit()) { format!("{v}px") } else { v }
        })
    };
    Some(ImgTag {
        src,
        alt: attr(tag, "alt").unwrap_or_default(),
        title: attr(tag, "title").unwrap_or_default(),
        width: size("width"),
        height: size("height"),
        zoom: from_style("zoom").unwrap_or_default(),
    })
}

fn quote(value: &str) -> String {
    value.replace('"', "&quot;")
}

/// The HTML form of an image, as Typora writes it: `<img src alt title />`.
pub fn to_html(src: &str, alt: &str, title: &str) -> String {
    let mut out = format!("<img src=\"{}\"", quote(src));
    if !alt.is_empty() {
        out += &format!(" alt=\"{}\"", quote(alt));
    }
    if !title.is_empty() {
        out += &format!(" title=\"{}\"", quote(title));
    }
    out + " />"
}

/// The markdown form of an image. Typora drops `]` from the alt text and
/// every attribute but the source, alt text and title.
pub fn to_markdown(src: &str, alt: &str, title: &str) -> String {
    let alt = alt.replace(']', "");
    // A source with spaces needs its brackets to stay one.
    let src = if src.contains(char::is_whitespace) { format!("<{src}>") } else { src.to_string() };
    if title.is_empty() {
        format!("![{alt}]({src})")
    } else {
        format!("![{alt}]({src} \"{}\")", title.replace('"', "\\\""))
    }
}

/// The tag with attribute `name` set to `value`, added at the end when it
/// has none.
pub fn with_attr(tag: &str, name: &str, value: &str) -> String {
    let attr = format!(" {name}=\"{}\"", quote(value));
    match attrs(tag).into_iter().find(|(n, ..)| n == name) {
        Some((.., range)) => format!("{}{attr}{}", &tag[..range.start], &tag[range.end..]),
        None => {
            let end = tag.trim_end_matches('>').trim_end_matches('/').trim_end().len();
            format!("{}{attr}{}", &tag[..end], &tag[end..])
        }
    }
}

/// The tag with style property `prop` set to `value`, or taken out when
/// `value` is empty, as a browser writes a style back (`zoom: 50%;`). A
/// style left empty goes, attribute and all.
pub fn with_style(tag: &str, prop: &str, value: &str) -> String {
    let found = attrs(tag).into_iter().find(|(n, ..)| n == "style");
    let Some((_, style, range)) = found else {
        if value.is_empty() {
            return tag.to_string();
        }
        let end = tag.trim_end_matches('>').trim_end_matches('/').trim_end().len();
        let close = if tag.ends_with("/>") { " />" } else { ">" };
        return format!("{} style=\"{prop}:{value};\"{close}", &tag[..end]);
    };
    let mut decls = declarations(&html_escape::decode_html_entities(&style));
    decls.retain(|(p, _)| p != prop);
    if !value.is_empty() {
        decls.push((prop.to_string(), value.to_string()));
    }
    let style: Vec<String> = decls.iter().map(|(p, v)| format!("{p}: {v};")).collect();
    let attr = if style.is_empty() { String::new() } else { format!(" style=\"{}\"", quote(&style.join(" "))) };
    format!("{}{attr}{}", &tag[..range.start], &tag[range.end..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_tag() {
        let t = parse(r#"<img src="a b.png" alt='A &amp; B' style="width: 50%; zoom:33%" height=40 />"#).unwrap();
        assert_eq!(t.src, "a b.png");
        assert_eq!(t.alt, "A & B");
        assert_eq!((t.width.as_str(), t.height.as_str(), t.zoom.as_str()), ("50%", "40px", "33%"));
        assert_eq!(parse("<img src=x.gif width=10>").unwrap().width, "10px");
        assert_eq!(parse(r#"<img alt="no source">"#), None);
        assert_eq!(parse(r#"<img src="">"#), None);
    }

    #[test]
    fn switches_syntax() {
        assert_eq!(to_html("a.png", "cat", ""), r#"<img src="a.png" alt="cat" />"#);
        assert_eq!(to_html("q\".png", "", "t"), r#"<img src="q&quot;.png" title="t" />"#);
        assert_eq!(to_markdown("a.png", "[cat]", ""), "![[cat](a.png)");
        assert_eq!(to_markdown("a.png", "cat", "x"), "![cat](a.png \"x\")");
        assert_eq!(to_markdown("a b.png", "", ""), "![](<a b.png>)");
    }

    #[test]
    fn sets_attributes() {
        assert_eq!(with_attr(r#"<img src="a.png" alt="x" />"#, "src", "b c.png"), r#"<img src="b c.png" alt="x" />"#);
        assert_eq!(with_attr("<img src=a.png>", "src", "b.png"), r#"<img src="b.png">"#);
        assert_eq!(with_attr("<img src=a.png>", "alt", "x"), r#"<img src=a.png alt="x">"#);
    }

    #[test]
    fn zooms() {
        let tag = r#"<img src="a.png" alt="cat" />"#;
        let zoomed = with_style(tag, "zoom", "50%");
        assert_eq!(zoomed, r#"<img src="a.png" alt="cat" style="zoom:50%;" />"#);
        assert_eq!(with_style(&zoomed, "zoom", "200%"), r#"<img src="a.png" alt="cat" style="zoom: 200%;" />"#);
        assert_eq!(with_style(&zoomed, "zoom", ""), tag);
        assert_eq!(with_style("<img src=a.png>", "zoom", "25%"), r#"<img src=a.png style="zoom:25%;">"#);
        assert_eq!(
            with_style(r#"<img src="a" style="width:10px; zoom: 80%">"#, "zoom", "67%"),
            r#"<img src="a" style="width: 10px; zoom: 67%;">"#
        );
        assert_eq!(with_style(tag, "zoom", ""), tag);
    }
}
