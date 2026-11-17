//! Typo against the GitHub Flavored Markdown spec: every example in `gfm-spec.txt` (GFM 0.29,
//! from github/cmark-gfm, CC-BY-SA 4.0) goes through `parse`, the parse the preview draws, and
//! Typo's blocks are written back as HTML and compared with the spec's.
//!
//! An example that differs must be in `DEVIATIONS` with the reason; an example listed there
//! that now matches fails the test too, so the list stays true. `cargo test gfm_spec` runs it;
//! `TYPO_SPEC=<n>` prints one example's Markdown, the spec's HTML and Typo's.
//!
//! Some differences hold in every example, so `normalize` removes them from both sides instead
//! of listing them: the preview keeps no link or image title, no code block language and no
//! difference between a column with no alignment and one aligned left; it shows each line
//! break as typed, so `<br />` reads as a line end; and it joins a paragraph's last two words
//! with a no-break space. Whitespace between block tags doesn't count, as in cmark's own
//! spec runner.

use std::ops::RangeInclusive;

use super::{Block, Inline, parse};

/// The examples where Typo differs from the spec on purpose, by reason.
#[allow(clippy::single_range_in_vec_init)]
const DEVIATIONS: &[(&str, &[RangeInclusive<usize>])] = &[
    (
        "Raw HTML shows as text, not as HTML, so a note shows what was typed.",
        &[118..=160, 170..=170, 288..=289, 317..=317, 327..=327, 354..=354, 484..=486, 500..=500, 503..=503, 532..=532, 544..=544, 632..=636, 642..=642, 644..=650, 652..=652, 662..=663],
    ),
    ("A note that starts with `---` opens with front matter, shown as properties, as in Obsidian.", &[66..=66]),
    ("`[[…]]` is a wiki link, as in Obsidian.", &[567..=567, 598..=598]),
    (
        "Bare URLs and addresses link, as on GitHub. The spec's core examples run without its autolink extension.",
        &[610..=610, 616..=616, 619..=620],
    ),
    ("A line of a paragraph that holds only images shows as its own block, so the images get the width.", &[595..=595]),
    (
        "comrak nests bold in bold (CommonMark 0.31) where GitHub merges them (GFM 0.29). Both look the same.",
        &[398..=398, 426..=426, 434..=436, 473..=475, 477..=477],
    ),
];

struct Example {
    n: usize,
    section: String,
    md: String,
    html: String,
}

fn examples() -> Vec<Example> {
    let fence = "````````````````````````````````";
    let mut out = Vec::new();
    let mut section = String::new();
    let mut lines = include_str!("gfm-spec.txt").lines();
    while let Some(l) = lines.next() {
        if let Some(h) = l.strip_prefix("## ").or_else(|| l.strip_prefix("# ")) {
            section = h.to_string();
        }
        if !(l.starts_with(fence) && l[fence.len()..].trim_start().starts_with("example")) {
            continue;
        }
        let (mut md, mut html, mut in_html) = (String::new(), String::new(), false);
        for l in lines.by_ref() {
            if l.starts_with(fence) {
                break;
            }
            if l == "." && !in_html {
                in_html = true;
                continue;
            }
            let t = if in_html { &mut html } else { &mut md };
            t.push_str(&l.replace('→', "\t"));
            t.push('\n');
        }
        out.push(Example { n: out.len() + 1, section: section.clone(), md, html });
    }
    out
}

// ---------- Typo's blocks as HTML ----------

fn esc(s: &str) -> String {
    let mut o = String::new();
    let _ = comrak::html::escape(&mut o, s);
    o
}

fn href(s: &str) -> String {
    let mut o = String::new();
    let _ = comrak::html::escape_href(&mut o, s, false);
    o
}

fn inlines(v: &[Inline], o: &mut String) {
    for i in v {
        match i {
            Inline::Text(t) => o.push_str(&esc(t)),
            Inline::Code(t) => o.push_str(&format!("<code>{}</code>", esc(t))),
            Inline::Strong(k) => wrap("strong", k, o),
            Inline::Emph(k) => wrap("em", k, o),
            Inline::Strike(k) => wrap("del", k, o),
            Inline::Link { url, kids } => {
                o.push_str(&format!("<a href=\"{}\">", href(url)));
                inlines(kids, o);
                o.push_str("</a>");
            }
            Inline::Wiki { target, shown } => o.push_str(&esc(&format!("[[{target}|{shown}]]"))),
            Inline::Image(img) => o.push_str(&format!("<img src=\"{}\" alt=\"{}\" />", href(&img.src), esc(&img.alt))),
            Inline::Tag(t) => o.push_str(&esc(&format!("#{t}"))),
            Inline::Break => o.push('\n'),
        }
    }
}

fn wrap(tag: &str, k: &[Inline], o: &mut String) {
    o.push_str(&format!("<{tag}>"));
    inlines(k, o);
    o.push_str(&format!("</{tag}>"));
}

/// `tight`: the blocks of a tight list's item, whose paragraphs have no `<p>`.
fn blocks(v: &[Block], tight: bool, o: &mut String) {
    for b in v {
        match b {
            Block::Props { .. } => o.push_str("<!-- front matter -->"),
            Block::Para { inlines: k, .. } if tight => inlines(k, o),
            Block::Para { inlines: k, .. } => wrap("p", k, o),
            Block::Image { images, .. } => {
                let k: Vec<Inline> = images.iter().map(|i| Inline::Image(i.clone())).collect();
                if tight { inlines(&k, o) } else { wrap("p", &k, o) }
            }
            Block::Heading { level, inlines: k, .. } => wrap(&format!("h{level}"), k, o),
            Block::Pre { text, .. } => {
                let nl = if text.is_empty() { "" } else { "\n" };
                o.push_str(&format!("<pre><code>{}{nl}</code></pre>", esc(text)));
            }
            Block::Rule { .. } => o.push_str("<hr />"),
            Block::Quote { blocks: k, .. } => {
                o.push_str("<blockquote>");
                blocks(k, false, o);
                o.push_str("</blockquote>");
            }
            Block::List { ordered, start, tight, items, .. } => {
                let tag = if *ordered { "ol" } else { "ul" };
                match (*ordered, *start) {
                    (true, s) if s != 1 => o.push_str(&format!("<ol start=\"{s}\">")),
                    _ => o.push_str(&format!("<{tag}>")),
                }
                for it in items {
                    o.push_str("<li>");
                    if let Some(done) = it.task {
                        o.push_str(if done { "<input checked=\"\" disabled=\"\" type=\"checkbox\"> " } else { "<input disabled=\"\" type=\"checkbox\"> " });
                    }
                    blocks(&it.blocks, *tight, o);
                    o.push_str("</li>");
                }
                o.push_str(&format!("</{tag}>"));
            }
            Block::Table { align, rows, .. } => {
                o.push_str("<table><thead>");
                for (r, row) in rows.iter().enumerate() {
                    if r == 1 {
                        o.push_str("<tbody>");
                    }
                    let cell = if r == 0 { "th" } else { "td" };
                    o.push_str("<tr>");
                    for (c, k) in row.iter().enumerate() {
                        match align.get(c) {
                            Some(super::Align::Center) => o.push_str(&format!("<{cell} align=\"center\">")),
                            Some(super::Align::Right) => o.push_str(&format!("<{cell} align=\"right\">")),
                            _ => o.push_str(&format!("<{cell}>")),
                        }
                        inlines(k, o);
                        o.push_str(&format!("</{cell}>"));
                    }
                    o.push_str("</tr>");
                    if r == 0 {
                        o.push_str("</thead>");
                    }
                }
                if rows.len() > 1 {
                    o.push_str("</tbody>");
                }
                o.push_str("</table>");
            }
        }
    }
}

fn typo_html(md: &str) -> String {
    let mut o = String::new();
    blocks(&parse(md).0, false, &mut o);
    o
}

// ---------- Comparing ----------

const BLOCK_TAGS: &[&str] = &["p", "li", "ul", "ol", "blockquote", "h1", "h2", "h3", "h4", "h5", "h6", "hr", "pre", "table", "thead", "tbody", "tr", "th", "td"];

fn tag_name(t: &str) -> &str {
    let t = t.trim_start_matches(['<', '/']);
    let end = t.find(|c: char| !c.is_ascii_alphanumeric()).unwrap_or(t.len());
    &t[..end]
}

/// Drops `attr="…"` from every tag, or only where its value starts with `prefix`.
fn drop_attr(s: &str, attr: &str, prefix: &str) -> String {
    let pat = format!(" {attr}=\"{prefix}");
    let mut o = String::new();
    let mut rest = s;
    while let Some(i) = rest.find(&pat) {
        // Only inside a tag: the last `<` before it comes after the last `>`.
        let head = &rest[..i];
        let in_tag = head.rfind('<').is_some_and(|lt| head.rfind('>').is_none_or(|gt| lt > gt));
        let Some(end) = rest[i + pat.len()..].find('"').map(|e| i + pat.len() + e + 1) else { break };
        o.push_str(head);
        if !in_tag {
            o.push_str(&rest[i..end]);
        }
        rest = &rest[end..];
    }
    o.push_str(rest);
    o
}

fn normalize(s: &str) -> String {
    let s = s.replace("<br />\n", "\n").replace("<br />", "\n").replace('\u{a0}', " ");
    let s = drop_attr(&drop_attr(&drop_attr(&s, "title", ""), "align", "left\""), "class", "language-");
    // Whitespace next to a block tag goes.
    let mut parts: Vec<String> = Vec::new();
    let mut rest = s.as_str();
    while !rest.is_empty() {
        let (part, next) = match rest.find('<') {
            Some(0) => {
                let e = rest.find('>').map_or(rest.len(), |e| e + 1);
                (&rest[..e], &rest[e..])
            }
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, ""),
        };
        parts.push(part.to_string());
        rest = next;
    }
    let block = |p: &str| p.starts_with('<') && BLOCK_TAGS.contains(&tag_name(p));
    let n = parts.len();
    for i in 0..n {
        if parts[i].starts_with('<') {
            continue;
        }
        let before = i > 0 && block(&parts[i - 1]);
        let after = i + 1 < n && block(&parts[i + 1]);
        let in_pre = i > 0 && tag_name(&parts[i - 1]) == "code" && !parts[i - 1].starts_with("</");
        if in_pre {
            continue;
        }
        if before || i == 0 {
            parts[i] = parts[i].trim_start().to_string();
        }
        if after || i + 1 == n {
            parts[i] = parts[i].trim_end().to_string();
        }
    }
    parts.concat()
}

#[test]
fn gfm_spec() {
    let all = examples();
    assert_eq!(all.len(), 672, "the spec file holds 672 examples");
    let listed = |n: usize| DEVIATIONS.iter().find(|(_, rs)| rs.iter().any(|r| r.contains(&n))).map(|(why, _)| *why);
    let only: Option<usize> = std::env::var("TYPO_SPEC").ok().and_then(|v| v.parse().ok());
    let (mut pass, mut known, mut fail, mut stale) = (0, 0, Vec::new(), Vec::new());
    for e in &all {
        let got = typo_html(&e.md);
        let same = normalize(&got) == normalize(&e.html);
        if only == Some(e.n) {
            eprintln!("Example {} ({}):\n{}\n--- spec\n{}\n--- typo\n{got}\n--- normalized\n{}\n{}\n", e.n, e.section, e.md, e.html, normalize(&e.html), normalize(&got));
        }
        match (same, listed(e.n)) {
            (true, None) => pass += 1,
            (false, Some(_)) => known += 1,
            (false, None) => fail.push(e),
            (true, Some(why)) => stale.push(format!("{} ({why})", e.n)),
        }
    }
    eprintln!("GFM spec: {pass} match, {known} differ on purpose, {} differ unlisted, {} listed but match", fail.len(), stale.len());
    for e in &fail {
        eprintln!("--- Example {} ({}):\n{}--- spec: {}\n--- typo: {}\n", e.n, e.section, e.md, normalize(&e.html), normalize(&typo_html(&e.md)));
    }
    let nums: Vec<usize> = fail.iter().map(|e| e.n).collect();
    assert!(nums.is_empty(), "examples that differ from the spec and aren't in DEVIATIONS: {nums:?}");
    assert!(stale.is_empty(), "examples in DEVIATIONS that now match the spec: {stale:?}");
}
