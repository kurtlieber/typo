//! HTML from the clipboard to markdown, for a rich paste. The output follows
//! what Typora writes: `**` and `*` for emphasis, `-` bullets, fenced code
//! with the language from its class, GFM tables and task items. Markdown
//! characters in the text are escaped so they paste as written.

use scraper::{ElementRef, Html, Node};

use super::inline::Options;

/// Markdown for an HTML fragment (or whole page).
pub fn to_markdown(html: &str, opts: &Options) -> String {
    let doc = Html::parse_fragment(html);
    Conv { opts }.blocks(doc.root_element(), Ctx::default()).join("\n\n")
}

/// The inline styles already open, so nested tags don't double markers.
#[derive(Clone, Copy, Default)]
struct Ctx {
    bold: bool,
    italic: bool,
    strike: bool,
    link: bool,
}

struct Conv<'o> {
    opts: &'o Options,
}

const BLOCK: &[&str] = &[
    "address", "article", "aside", "blockquote", "body", "center", "dd", "details", "div", "dl", "dt", "fieldset",
    "figcaption", "figure", "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "header", "hr", "html", "li", "main",
    "nav", "ol", "p", "pre", "section", "summary", "table", "ul",
];

const SKIP: &[&str] = &[
    "button", "head", "input", "link", "meta", "noscript", "script", "select", "style", "svg", "template", "textarea",
    "title",
];

fn is_block(name: &str) -> bool {
    BLOCK.contains(&name)
}

/// The value of CSS property `prop` in an element's `style`.
fn style(e: ElementRef, prop: &str) -> Option<String> {
    e.value().attr("style")?.split(';').find_map(|decl| {
        let (k, v) = decl.split_once(':')?;
        (k.trim().eq_ignore_ascii_case(prop)).then(|| v.trim().to_ascii_lowercase())
    })
}

fn heavy(weight: &str) -> bool {
    weight == "bold" || weight == "bolder" || weight.parse::<u32>().is_ok_and(|w| w >= 600)
}

/// Appends an inline piece, folding the spaces where two pieces meet.
fn append(acc: &mut String, piece: &str) {
    if acc.ends_with([' ', '\n']) && piece.starts_with(' ') {
        acc.push_str(&piece[1..]);
    } else {
        acc.push_str(piece);
    }
}

/// A paragraph's text: lines trimmed, and characters that would start a
/// block at a line start escaped.
fn paragraph(text: &str) -> Option<String> {
    let lines: Vec<String> = text
        .split('\n')
        .map(|l| l.trim_matches(' '))
        .filter(|l| !l.is_empty())
        .map(escape_line_start)
        .collect();
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn escape_line_start(line: &str) -> String {
    let b = line.as_bytes();
    let first = b[0];
    let hashes = b.iter().take_while(|&&c| c == b'#').count();
    if (1..=6).contains(&hashes) && b.get(hashes).is_none_or(|&c| c == b' ') {
        return format!("\\{line}");
    }
    if first == b'>' || ((first == b'-' || first == b'+') && b.get(1).is_none_or(|&c| c == b' ')) {
        return format!("\\{line}");
    }
    if line.chars().all(|c| c == '=' || c == '-') {
        return format!("\\{line}");
    }
    let digits = b.iter().take_while(|c| c.is_ascii_digit()).count();
    if (1..=9).contains(&digits)
        && matches!(b.get(digits), Some(b'.') | Some(b')'))
        && b.get(digits + 1).is_none_or(|&c| c == b' ')
    {
        return format!("{}\\{}", &line[..digits], &line[digits..]);
    }
    line.to_string()
}

impl Conv<'_> {
    /// The blocks inside a container: runs of inline content become
    /// paragraphs between its block children.
    fn blocks(&self, el: ElementRef, ctx: Ctx) -> Vec<String> {
        let mut out = Vec::new();
        let mut para = String::new();
        for child in el.children() {
            match child.value() {
                Node::Text(t) => append(&mut para, &self.text(t)),
                Node::Element(e) if SKIP.contains(&e.name()) => {}
                Node::Element(e) if is_block(e.name()) => {
                    out.extend(paragraph(&std::mem::take(&mut para)));
                    out.extend(self.block(ElementRef::wrap(child).unwrap(), ctx));
                }
                Node::Element(_) => append(&mut para, &self.inline(ElementRef::wrap(child).unwrap(), ctx)),
                _ => {}
            }
        }
        out.extend(paragraph(&para));
        out
    }

    fn block(&self, e: ElementRef, ctx: Ctx) -> Vec<String> {
        let name = e.value().name();
        match name {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = (name.as_bytes()[1] - b'0') as usize;
                let text = self.inline_children(e, ctx).replace('\n', " ");
                let text = text.trim();
                if text.is_empty() { vec![] } else { vec![format!("{} {}", "#".repeat(level), text)] }
            }
            "pre" => vec![self.code_block(e)],
            "blockquote" => {
                let inner = self.blocks(e, ctx).join("\n\n");
                if inner.is_empty() {
                    return vec![];
                }
                vec![inner.lines().map(|l| if l.is_empty() { ">".to_string() } else { format!("> {l}") }).collect::<Vec<_>>().join("\n")]
            }
            "ul" | "ol" => vec![self.list(e, ctx)].into_iter().filter(|l| !l.is_empty()).collect(),
            "hr" => vec!["---".to_string()],
            "table" => self.table(e, ctx).into_iter().collect(),
            _ => self.blocks(e, ctx),
        }
    }

    fn text(&self, t: &str) -> String {
        let mut collapsed = String::with_capacity(t.len());
        for c in t.chars() {
            let c = if c.is_ascii_whitespace() || c == '\u{a0}' { ' ' } else { c };
            if !(c == ' ' && collapsed.ends_with(' ')) {
                collapsed.push(c);
            }
        }
        self.escape(&collapsed)
    }

    fn escape(&self, s: &str) -> String {
        let chars: Vec<char> = s.chars().collect();
        let mut out = String::with_capacity(s.len());
        for (i, &c) in chars.iter().enumerate() {
            let prev = i.checked_sub(1).map(|j| chars[j]);
            let next = chars.get(i + 1).copied();
            let escape = match c {
                '\\' | '*' | '`' | '[' | ']' => true,
                '_' => !(prev.is_some_and(char::is_alphanumeric) && next.is_some_and(char::is_alphanumeric)),
                '<' => next.is_some_and(|n| n.is_ascii_alphabetic() || "/!?".contains(n)),
                '~' => self.opts.subscript || next == Some('~') || prev == Some('~'),
                '$' => self.opts.math,
                '^' => self.opts.superscript,
                '=' => self.opts.highlight && (next == Some('=') || prev == Some('=')),
                '&' => {
                    let rest: String = chars[i + 1..].iter().take(12).collect();
                    let name = rest.split(';').next().unwrap_or("");
                    rest.contains(';') && !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '#')
                }
                _ => false,
            };
            if escape {
                out.push('\\');
            }
            out.push(c);
        }
        out
    }

    fn inline_children(&self, e: ElementRef, ctx: Ctx) -> String {
        let mut out = String::new();
        for child in e.children() {
            match child.value() {
                Node::Text(t) => append(&mut out, &self.text(t)),
                Node::Element(_) => append(&mut out, &self.inline(ElementRef::wrap(child).unwrap(), ctx)),
                _ => {}
            }
        }
        out
    }

    /// Wraps the children in markers, keeping edge spaces outside them.
    fn wrap(&self, e: ElementRef, ctx: Ctx, open: &str, close: &str) -> String {
        let inner = self.inline_children(e, ctx);
        let core = inner.trim_matches(' ');
        if core.is_empty() {
            return inner;
        }
        let lead = if inner.starts_with(' ') { " " } else { "" };
        let trail = if inner.ends_with(' ') { " " } else { "" };
        format!("{lead}{open}{core}{close}{trail}")
    }

    fn inline(&self, e: ElementRef, ctx: Ctx) -> String {
        let name = e.value().name();
        if SKIP.contains(&name) {
            return String::new();
        }
        let weight = style(e, "font-weight");
        let normal_weight = weight.as_deref().is_some_and(|w| !heavy(w));
        match name {
            "br" => "\n".to_string(),
            "strong" | "b" if !ctx.bold && !normal_weight => self.wrap(e, Ctx { bold: true, ..ctx }, "**", "**"),
            "em" | "i" | "cite" | "dfn" | "var" if !ctx.italic => self.wrap(e, Ctx { italic: true, ..ctx }, "*", "*"),
            "del" | "s" | "strike" if !ctx.strike => self.wrap(e, Ctx { strike: true, ..ctx }, "~~", "~~"),
            "code" | "kbd" | "samp" | "tt" => code_span(&e.text().collect::<String>()),
            "u" | "ins" => self.wrap(e, ctx, "<u>", "</u>"),
            "sup" if self.opts.superscript => self.wrap(e, ctx, "^", "^"),
            "sup" => self.wrap(e, ctx, "<sup>", "</sup>"),
            "sub" if self.opts.subscript => self.wrap(e, ctx, "~", "~"),
            "sub" => self.wrap(e, ctx, "<sub>", "</sub>"),
            "mark" if self.opts.highlight => self.wrap(e, ctx, "==", "=="),
            "mark" => self.wrap(e, ctx, "<mark>", "</mark>"),
            "a" => self.link(e, ctx),
            "img" => image(e),
            _ => {
                // Styled spans, as Google Docs and word processors write them.
                let bold = !ctx.bold && weight.as_deref().is_some_and(heavy);
                let italic = !ctx.italic && style(e, "font-style").is_some_and(|s| s == "italic");
                let strike = !ctx.strike && style(e, "text-decoration").is_some_and(|s| s.contains("line-through"));
                let inner = Ctx { bold: ctx.bold || bold, italic: ctx.italic || italic, strike: ctx.strike || strike, ..ctx };
                let mut text = self.inline_children(e, inner);
                for (on, mark) in [(strike, "~~"), (italic, "*"), (bold, "**")] {
                    if on {
                        let core = text.trim_matches(' ');
                        if !core.is_empty() {
                            let lead = if text.starts_with(' ') { " " } else { "" };
                            let trail = if text.ends_with(' ') { " " } else { "" };
                            text = format!("{lead}{mark}{core}{mark}{trail}");
                        }
                    }
                }
                // A block inside inline content still separates words.
                if is_block(name) && !text.is_empty() { format!(" {text} ") } else { text }
            }
        }
    }

    fn link(&self, e: ElementRef, ctx: Ctx) -> String {
        let text = self.inline_children(e, Ctx { link: true, ..ctx });
        let href = e.value().attr("href").unwrap_or("").trim();
        if ctx.link || href.is_empty() || href.to_ascii_lowercase().starts_with("javascript:") {
            return text;
        }
        let core = text.trim_matches(' ');
        if core.is_empty() {
            return text;
        }
        let dest = if href.contains([' ', '(', ')', '<', '>']) { format!("<{}>", href.replace(' ', "%20")) } else { href.to_string() };
        let title = e.value().attr("title").filter(|t| !t.is_empty()).map(|t| format!(" \"{}\"", t.replace('"', "\\\""))).unwrap_or_default();
        let lead = if text.starts_with(' ') { " " } else { "" };
        let trail = if text.ends_with(' ') { " " } else { "" };
        format!("{lead}[{core}]({dest}{title}){trail}")
    }

    fn code_block(&self, pre: ElementRef) -> String {
        let mut code = String::new();
        collect_pre(pre, &mut code);
        let code = code.trim_end_matches('\n');
        let lang = std::iter::once(pre)
            .chain(pre.children().filter_map(ElementRef::wrap))
            .chain(pre.parent().and_then(ElementRef::wrap))
            .find_map(language)
            .unwrap_or_default();
        let ticks = longest_run(code, '`').max(2) + 1;
        let fence = "`".repeat(ticks);
        format!("{fence}{lang}\n{code}\n{fence}")
    }

    fn list(&self, e: ElementRef, ctx: Ctx) -> String {
        let ordered = e.value().name() == "ol";
        let mut n: u64 = e.value().attr("start").and_then(|s| s.trim().parse().ok()).unwrap_or(1);
        let mut items: Vec<String> = Vec::new();
        for child in e.children().filter_map(ElementRef::wrap) {
            match child.value().name() {
                "li" => {
                    let marker = if ordered { format!("{n}. ") } else { "- ".to_string() };
                    n += 1;
                    items.push(self.item(child, ctx, &marker));
                }
                // A list nested straight inside a list belongs to the item
                // before it.
                "ul" | "ol" => {
                    let nested = self.list(child, ctx);
                    if nested.is_empty() {
                        continue;
                    }
                    let indent = if ordered { format!("{}. ", n.saturating_sub(1)).len() } else { 2 };
                    let nested = indent_lines(&nested, indent);
                    match items.last_mut() {
                        Some(last) => {
                            last.push('\n');
                            last.push_str(&nested);
                        }
                        None => items.push(nested),
                    }
                }
                _ => {}
            }
        }
        let loose = items.iter().any(|i| i.contains("\n\n"));
        items.join(if loose { "\n\n" } else { "\n" })
    }

    fn item(&self, li: ElementRef, ctx: Ctx, marker: &str) -> String {
        let check = li
            .descendants()
            .filter_map(ElementRef::wrap)
            .take_while(|d| !matches!(d.value().name(), "ul" | "ol"))
            .find(|d| d.value().name() == "input" && d.value().attr("type").is_some_and(|t| t.eq_ignore_ascii_case("checkbox")))
            .map(|d| if d.value().attr("checked").is_some() { "[x] " } else { "[ ] " });
        let blocks = self.blocks(li, ctx);
        let head = format!("{marker}{}", check.unwrap_or(""));
        if blocks.is_empty() {
            return head.trim_end().to_string();
        }
        let mut out = String::new();
        for (i, b) in blocks.iter().enumerate() {
            if i == 0 {
                out.push_str(&head);
                out.push_str(&indent_lines(b, marker.len())[marker.len()..]);
                continue;
            }
            // A nested list stays tight under its item's text.
            out.push_str(if starts_list(b) { "\n" } else { "\n\n" });
            out.push_str(&indent_lines(b, marker.len()));
        }
        out
    }

    fn table(&self, table: ElementRef, ctx: Ctx) -> Option<String> {
        let mut rows: Vec<Vec<(String, Option<String>)>> = Vec::new();
        let mut sections = vec![table];
        sections.extend(
            table.children().filter_map(ElementRef::wrap).filter(|c| matches!(c.value().name(), "thead" | "tbody" | "tfoot")),
        );
        for section in sections {
            for tr in section.children().filter_map(ElementRef::wrap).filter(|c| c.value().name() == "tr") {
                let cells = tr
                    .children()
                    .filter_map(ElementRef::wrap)
                    .filter(|c| matches!(c.value().name(), "td" | "th"))
                    .map(|c| {
                        let text = self.blocks(c, ctx).join("<br>").replace('\n', "<br>").replace('|', "\\|");
                        let align = c.value().attr("align").map(str::to_ascii_lowercase).or_else(|| style(c, "text-align"));
                        (text, align)
                    })
                    .collect::<Vec<_>>();
                if !cells.is_empty() {
                    rows.push(cells);
                }
            }
        }
        let cols = rows.iter().map(Vec::len).max()?;
        let line = |cells: Vec<String>| format!("| {} |", cells.join(" | "));
        let mut out = Vec::new();
        let cell = |row: &Vec<(String, Option<String>)>, i: usize| row.get(i).map(|c| c.0.clone()).unwrap_or_default();
        out.push(line((0..cols).map(|i| cell(&rows[0], i)).collect()));
        out.push(line(
            (0..cols)
                .map(|i| match rows[0].get(i).and_then(|c| c.1.as_deref()) {
                    Some("left") => ":---".to_string(),
                    Some("center") => ":---:".to_string(),
                    Some("right") => "---:".to_string(),
                    _ => "---".to_string(),
                })
                .collect(),
        ));
        for row in &rows[1..] {
            out.push(line((0..cols).map(|i| cell(row, i)).collect()));
        }
        Some(out.join("\n"))
    }
}

fn starts_list(block: &str) -> bool {
    let digits = block.bytes().take_while(u8::is_ascii_digit).count();
    block.starts_with("- ") || block == "-" || (digits > 0 && block[digits..].starts_with(". "))
}

fn indent_lines(s: &str, n: usize) -> String {
    let pad = " ".repeat(n);
    s.lines().map(|l| if l.is_empty() { String::new() } else { format!("{pad}{l}") }).collect::<Vec<_>>().join("\n")
}

fn longest_run(s: &str, c: char) -> usize {
    let (mut best, mut run) = (0, 0);
    for x in s.chars() {
        run = if x == c { run + 1 } else { 0 };
        best = best.max(run);
    }
    best
}

fn code_span(text: &str) -> String {
    let text: String = text.chars().map(|c| if c == '\n' || c == '\u{a0}' { ' ' } else { c }).collect();
    if text.is_empty() {
        return text;
    }
    let ticks = "`".repeat(longest_run(&text, '`') + 1);
    let pad = if text.starts_with('`') || text.ends_with('`') { " " } else { "" };
    format!("{ticks}{pad}{text}{pad}{ticks}")
}

fn image(e: ElementRef) -> String {
    let Some(src) = e.value().attr("src").map(str::trim).filter(|s| !s.is_empty()) else { return String::new() };
    let alt = e.value().attr("alt").unwrap_or("").replace('[', "\\[").replace(']', "\\]");
    let src = if src.contains([' ', '(', ')']) { format!("<{}>", src.replace(' ', "%20")) } else { src.to_string() };
    let title = e.value().attr("title").filter(|t| !t.is_empty()).map(|t| format!(" \"{}\"", t.replace('"', "\\\""))).unwrap_or_default();
    format!("![{alt}]({src}{title})")
}

/// A code block's language from `class="language-x"` (or `lang-x`,
/// GitHub's `highlight-source-x`) or `data-lang`.
fn language(e: ElementRef) -> Option<String> {
    if let Some(l) = e.value().attr("data-lang").filter(|l| !l.is_empty()) {
        return Some(l.to_string());
    }
    e.value().classes().find_map(|c| {
        ["language-", "lang-", "highlight-source-"].iter().find_map(|p| c.strip_prefix(p)).filter(|l| !l.is_empty()).map(str::to_string)
    })
}

/// The text of a `pre`, with `<br>` as line breaks.
fn collect_pre(e: ElementRef, out: &mut String) {
    for child in e.children() {
        match child.value() {
            Node::Text(t) => out.push_str(&t.replace('\u{a0}', " ")),
            Node::Element(el) if el.name() == "br" => out.push('\n'),
            Node::Element(_) => collect_pre(ElementRef::wrap(child).unwrap(), out),
            _ => {}
        }
    }
}
