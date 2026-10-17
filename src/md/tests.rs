use std::path::Path;

use pulldown_cmark::{Options, Parser};

use super::*;
use super::{html, inline};

// ---------------------------------------------------------------------------
// Helpers.

fn kinds(doc: &Document) -> Vec<&'static str> {
    doc.blocks.iter().map(|b| b.kind.name()).collect()
}

fn texts(doc: &Document) -> Vec<&str> {
    doc.blocks.iter().map(|b| b.text.as_str()).collect()
}

/// Container kinds per block, e.g. `"q,li"`.
fn paths(doc: &Document) -> Vec<String> {
    doc.blocks
        .iter()
        .map(|b| {
            b.containers
                .iter()
                .map(|c| match c.kind {
                    ContainerKind::Quote { .. } => "q",
                    ContainerKind::ListItem(_) => "li",
                    ContainerKind::Footnote { .. } => "fn",
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .collect()
}

fn html_of(md: &str) -> String {
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, Parser::new_ext(md, opts));
    out
}

/// The document as the serializer generates it from the model alone.
fn regenerated(doc: &Document, keep_spacing: bool) -> String {
    let mut doc = doc.clone();
    for (i, b) in doc.blocks.iter_mut().enumerate() {
        b.clear_raw();
        if !keep_spacing && i > 0 {
            b.blank_before = None;
        }
    }
    serialize(&doc)
}

#[track_caller]
fn assert_round_trip(src: &str) {
    let out = serialize(&parse(src));
    assert_eq!(out, src, "round trip changed the document");
}

/// Regenerating every block keeps the meaning (same HTML from an
/// independent CommonMark implementation) and is stable under re-parsing.
#[track_caller]
fn assert_equivalent(src: &str) {
    let doc = parse(src);
    for keep_spacing in [true, false] {
        let out = regenerated(&doc, keep_spacing);
        assert_eq!(html_of(&out), html_of(src), "meaning changed (keep_spacing={keep_spacing}):\n{out}");
        let again = regenerated(&parse(&out), keep_spacing);
        assert_eq!(again, out, "not a fixed point (keep_spacing={keep_spacing})");
    }
}

// ---------------------------------------------------------------------------
// Corpus: documents in the style Typora writes, which must round-trip byte
// for byte.

const CANONICAL: &[&str] = &[
    "",
    "\n",
    "Hello\n",
    "Hello",
    "# Title\n\nSome *text* here.\n\n## Sub\n\nMore.\n",
    "Line one\nline two  \nline three\n",
    "Setext\n======\n\nOther\n---\n",
    "# Closed #\n",
    "- a\n- b\n- c\n",
    "* a\n* b\n",
    "1. one\n2. two\n3. three\n",
    "3) three\n4) four\n",
    "1. same\n1. same\n1. same\n",
    "- a\n\n- b\n\n- c\n",
    "- a\n  - b\n    - c\n- d\n",
    "- a\n\n  second paragraph\n\n- b\n",
    "- [ ] todo\n- [x] done\n",
    "1. item\n\n   ```rust\n   let x = 1;\n   ```\n\n2. next\n",
    "> quote\n> more\n",
    "> quote\n>\n> second\n",
    "> outer\n>\n> > inner\n",
    "> - a\n> - b\n",
    "- > quoted item\n- plain\n",
    "```\ncode\n```\n",
    "```js\nlet a = 1;\n\nlet b = 2;\n```\n",
    "~~~~\n```\nnested\n```\n~~~~\n",
    "Para\n\n    indented code\n    more\n\nAfter\n",
    "$$\nE = mc^2\n$$\n",
    "| a | b |\n| --- | :-: |\n| 1 | 2 |\n",
    "| Left | Right |\n|:-----|------:|\n| x    |     y |\n",
    "---\ntitle: Doc\ntags: [a, b]\n---\n\n# Body\n",
    "[toc]\n\n# One\n",
    "***\n\n___\n\n- - -\n",
    "<div>\n<p>html</p>\n</div>\n\nafter\n",
    "<!-- comment -->\n\ntext\n",
    "Text[^1].\n\n[^1]: The note.\n",
    "[^long]: First line\n    continued.\n",
    "[ref]: https://example.com \"Title\"\n\nSee [ref].\n",
    "a\n\n\n\nb\n",
    "a\n\n",
    "\n\nleading blanks\n",
    "# Title\r\n\r\nWindows line endings.\r\n",
    "- a\n-\n- c\n",
    "10. ten\n11. eleven\n    continued\n",
    "- a\n    - four-space nested\n",
    "Para\n- list right after\n",
    "Para\n> quote right after\n",
    "Para\n# heading right after\n",
    "Para\n```\nfence right after\n```\n",
    "> lazy\ncontinuation\n",
    "> > nested lazy\n> line\n",
    "* Outline\n{:toc}\n## Next\n",
    "> a\n>\nb\n",
    "> quote\n>\n\nafter\n",
    "* a\n  > b\n  >\n* c\n",
    "-\n\n  foo\n",
    "    chunk1\n      \n      chunk2\n",
];

/// Documents written in other styles: they may be normalized on save but
/// must keep their meaning.
const NONCANONICAL: &[&str] = &[
    "> lazy\ncontinuation\n",
    ">no space\n",
    "- a\nlazy\n",
    "-\tTabbed\n",
    "  - indented item\n",
    "1. a\n3. b\n7. c\n",
    "| a | b |\n|---|---|\n| only one |\n| x | y | z |\n",
    "|a|b|\n|-|-|\n|`x\\|y`|z|\n",
    "| 中文 | b |\n|---|---|\n| 表格 | ok |\n",
    "```\nunclosed fence\n",
    "<script>\nlet x = '<div>';\n</script>\ntext\n",
    "Text\n***\n",
    "  # indented heading\n",
    "# Heading ##   \n",
    "- a\n\n\n- b\n",
    "> a\n\n> b\n",
    "- a\n  - b\n\n  c\n",
    "- one\n\n  ```\n  code\n  ```\n- two\n",
    "> ```\n> code in quote\n> ```\n",
    "1. a\n   > q\n2. b\n",
    "Title\n===\nbody\n",
    "- [ ] task\n  continued\n- [X] Done upper\n",
];

#[test]
fn canonical_documents_round_trip() {
    for src in CANONICAL {
        assert_round_trip(src);
    }
}

#[test]
fn blocks_know_their_source_lines() {
    let src = "---\ntitle: x\n---\n\n# Head\n\nPara one\nstill\n\n```\ncode\n```\n\n- a\n- b\n\n> quote\n";
    let doc = parse(src);
    let (out, starts) = serialize::serialize_with_starts(&doc);
    assert_eq!(out, src);
    assert_eq!(starts.len(), doc.blocks.len());
    let at = |line| &doc.blocks[serialize::block_at_line(&doc, line)];
    assert_eq!(at(1).kind.name(), "frontmatter");
    assert_eq!(at(4).text, "Head");
    assert_eq!(at(7).text, "Para one\nstill");
    assert_eq!(at(10).text, "code");
    assert_eq!(at(14).text, "b");
    assert_eq!(at(16).text, "quote");
    assert_eq!(at(99).text, "quote");
}

#[test]
fn documents_keep_their_meaning_when_regenerated() {
    for src in CANONICAL.iter().chain(NONCANONICAL) {
        assert_equivalent(src);
    }
}

/// The `.md` files in the directories `TYPO_REFERENCE_DIRS` lists (a path
/// list, like `PATH`): real documents kept outside the repo. None if unset.
fn reference_files() -> Vec<std::path::PathBuf> {
    let dirs = std::env::var_os("TYPO_REFERENCE_DIRS").unwrap_or_default();
    let mut files: Vec<_> = Vec::new();
    for dir in std::env::split_paths(&dirs) {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        files.extend(entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "md")));
    }
    files.sort();
    files
}

#[test]
fn reference_documents() {
    for path in reference_files() {
        let src = std::fs::read_to_string(&path).unwrap();
        let out = serialize(&parse(&src));
        assert_eq!(out, src, "round trip changed {}", path.display());
        let regen = regenerated(&parse(&src), true);
        assert_eq!(html_of(&regen), html_of(&src), "meaning changed in {}", path.display());
    }
}

// ---------------------------------------------------------------------------
// Structure.

#[test]
fn headings_and_paragraphs() {
    let doc = parse("# One\n\nText *a*\nmore\n\nTwo\n---\n");
    assert_eq!(kinds(&doc), ["heading", "paragraph", "heading"]);
    assert_eq!(texts(&doc), ["One", "Text *a*\nmore", "Two"]);
    assert!(matches!(doc.blocks[0].kind, BlockKind::Heading { level: 1, setext: None, .. }));
    assert!(matches!(doc.blocks[2].kind, BlockKind::Heading { level: 2, setext: Some(_), .. }));
    assert_eq!(doc.blocks[1].blank_before, Some(vec![0]));
}

#[test]
fn list_structure() {
    let doc = parse("- a\n  - b\n- c\n\n1. x\n");
    assert_eq!(texts(&doc), ["a", "b", "c", "x"]);
    assert_eq!(paths(&doc), ["li", "li,li", "li", "li"]);
    let list = |b: usize, d: usize| doc.blocks[b].containers[d].list_item().unwrap().list;
    assert_eq!(list(0, 0), list(2, 0), "a and c are in one list");
    assert_ne!(list(0, 0), list(3, 0), "a bullet and an ordered list differ");
    assert_eq!(doc.blocks[0].containers[0].id, doc.blocks[1].containers[0].id, "b is nested in a's item");
    assert_ne!(doc.blocks[0].containers[0].id, doc.blocks[2].containers[0].id);
}

#[test]
fn tight_and_loose_lists() {
    let loose = |src: &str| parse(src).blocks[0].containers[0].list_item().unwrap().loose;
    assert!(!loose("- a\n- b\n"));
    assert!(loose("- a\n\n- b\n"));
    assert!(loose("- a\n\n  more\n"));
    assert!(!loose("- a\n  - b\n\n  - c\n"), "a blank inside the nested list only");
    let doc = parse("- a\n  - b\n\n  - c\n");
    assert!(doc.blocks[1].containers[1].list_item().unwrap().loose);
    assert!(loose("- a\n  - b\n\n  c\n"), "item a holds a list and a paragraph with a blank");
}

#[test]
fn task_items() {
    let doc = parse("- [ ] todo\n- [x] done\n- [] not a task\n");
    let task = |b: usize| doc.blocks[b].containers[0].list_item().unwrap().task;
    assert_eq!(task(0), Some(false));
    assert_eq!(task(1), Some(true));
    assert_eq!(task(2), None);
    assert_eq!(texts(&doc), ["todo", "done", "[] not a task"]);
}

#[test]
fn ordered_list_start_and_numbering() {
    let doc = parse("3. a\n4. b\n");
    match &doc.blocks[1].containers[0].list_item().unwrap().marker {
        ListMarker::Ordered { start, delim, same_numbers } => {
            assert_eq!((*start, *delim, *same_numbers), (3, '.', false));
        }
        m => panic!("{m:?}"),
    }
    // Paragraph interruption: only a list starting at 1 may interrupt.
    assert_eq!(kinds(&parse("Text\n2. no\n")), ["paragraph"]);
    assert_eq!(paths(&parse("Text\n1. yes\n")), ["", "li"]);
}

#[test]
fn quotes() {
    let doc = parse("> a\n> > b\n> c\n\nd\n");
    assert_eq!(texts(&doc), ["a", "b\nc", "d"]);
    assert_eq!(paths(&doc), ["q", "q,q", ""]);
    let doc = parse("> a\n\n> b\n");
    assert_ne!(doc.blocks[0].containers[0].id, doc.blocks[1].containers[0].id, "a blank line ends a quote");
}

#[test]
fn lazy_continuation() {
    let doc = parse("> a\nb\n");
    assert_eq!(texts(&doc), ["a\nb"]);
    let doc = parse("- a\nb\n");
    assert_eq!(texts(&doc), ["a\nb"]);
    let doc = parse("> a\n---\n");
    assert_eq!(kinds(&doc), ["paragraph", "hr"]);
}

#[test]
fn code_blocks() {
    let doc = parse("```rust title\nfn main() {}\n\n```\n\n    indented\n\n~~~\nopen");
    assert_eq!(kinds(&doc), ["code", "code", "code"]);
    assert_eq!(texts(&doc), ["fn main() {}\n", "indented", "open"]);
    match &doc.blocks[0].kind {
        BlockKind::CodeFence { info, fence, closed } => {
            assert_eq!((info.as_str(), fence.as_str(), *closed), ("rust title", "```", true));
        }
        k => panic!("{k:?}"),
    }
    assert!(matches!(doc.blocks[2].kind, BlockKind::CodeFence { closed: false, .. }));
    // Fence content inside a list item loses the item's indentation.
    let doc = parse("- x\n\n  ```\n  code\n  ```\n");
    assert_eq!(texts(&doc), ["x", "code"]);
}

#[test]
fn math_html_and_extensions() {
    let doc = parse("---\na: 1\n---\n\n[TOC]\n\n$$\nx^2\n$$\n\n<div>\nhi\n</div>\n\n[x]: /u\n\n[^n]: note\n");
    assert_eq!(kinds(&doc), ["frontmatter", "toc", "math", "html", "linkref", "paragraph"]);
    assert_eq!(texts(&doc), ["a: 1", "[TOC]", "x^2", "<div>\nhi\n</div>", "[x]: /u", "note"]);
    assert_eq!(paths(&doc)[5], "fn");
}

#[test]
fn latex_delimited_math_blocks() {
    let src = "a\n\n\\[\nx^2\n\\]\n\n$$\ny\n$$\n";
    let mut doc = parse(src);
    assert_eq!(kinds(&doc), ["paragraph", "math", "math"]);
    assert!(matches!(doc.blocks[1].kind, BlockKind::Math { brackets: true, .. }));
    assert_eq!(texts(&doc)[1], "x^2");
    // Edited, it keeps its delimiters.
    doc.blocks[1].set_text("z");
    assert_eq!(serialize(&doc), "a\n\n\\[\nz\n\\]\n\n$$\ny\n$$\n");
    // `\]` doesn't close a `$$` block.
    assert_eq!(texts(&parse("$$\na\n\\]\n$$\n")), ["a\n\\]"]);
    // Without LaTeX delimiters it is a paragraph, and reads back the same.
    let off = super::parse_with(src, false, false);
    assert_eq!(kinds(&off), ["paragraph", "paragraph", "math"]);
    assert_eq!(serialize(&off), src);
}

#[test]
fn math_blocks_need_a_closing_line_and_keep_tags() {
    // As Typora: without a closing line further on, `$$` is text.
    assert_eq!(kinds(&parse("$$\nx\n\ny\n")), ["paragraph", "paragraph"]);
    assert_eq!(kinds(&parse("a\n\n$$\nx\n\ny\n$$\n")), ["paragraph", "math"]);
    assert_eq!(kinds(&parse("$$ {}\nx\n$$\n$$$\n")), ["paragraph"]);
    let src = "$$ {#eq:one}\nx\n$$ {(1)}\n";
    let mut doc = parse(src);
    assert!(matches!(&doc.blocks[0].kind, BlockKind::Math { before, after, .. } if before == "#eq:one" && after == "(1)"));
    doc.blocks[0].set_text("y");
    assert_eq!(serialize(&doc), "$$ {#eq:one}\ny\n$$ {(1)}\n");
}

#[test]
fn front_matter_needs_a_closing_line() {
    assert_eq!(kinds(&parse("---\nno close\n")), ["hr", "paragraph"]);
}

#[test]
fn tables() {
    let doc = parse("| a | b |\n|:--|--:|\n| 1 | 2 |\n| 3 |\n\nafter\n");
    assert_eq!(kinds(&doc), ["table", "paragraph"]);
    match &doc.blocks[0].kind {
        BlockKind::Table { align, rows } => {
            assert_eq!(align, &[Align::Left, Align::Right]);
            assert_eq!(rows, &[vec!["a", "b"], vec!["1", "2"], vec!["3", ""]]);
        }
        k => panic!("{k:?}"),
    }
    // The header must match the delimiter row's cell count.
    assert_eq!(kinds(&parse("| a | b |\n|---|\n")), ["paragraph"]);
    // A table splits off the end of a paragraph.
    let doc = parse("intro\n| a |\n| - |\n");
    assert_eq!(kinds(&doc), ["paragraph", "table"]);
    assert_eq!(doc.blocks[1].blank_before, Some(vec![]));
}

#[test]
fn table_generation_pads_columns() {
    let mut doc = parse("| a | long header |\n|:-:|--:|\n| wide cell | 1 |\n");
    doc.blocks[0].raw = None;
    assert_eq!(
        serialize(&doc),
        "|     a     | long header |\n| :-------: | ----------: |\n| wide cell |           1 |\n"
    );
}

#[test]
fn html_block_kinds() {
    assert_eq!(kinds(&parse("<span>inline</span> text\n")), ["paragraph"]);
    assert_eq!(kinds(&parse("<custom-tag>\n\ntext\n")), ["html", "paragraph"]);
    assert_eq!(kinds(&parse("Para\n<custom-tag>\n")), ["paragraph"], "kind 7 cannot interrupt");
    assert_eq!(kinds(&parse("<!-- a -->\ntext\n")), ["html", "paragraph"]);
}

#[test]
fn empty_containers_hold_an_empty_paragraph() {
    let doc = parse("- a\n-\n- c\n");
    assert_eq!(texts(&doc), ["a", "", "c"]);
    assert_eq!(paths(&doc), ["li", "li", "li"]);
    let doc = parse(">\n");
    assert_eq!(paths(&doc), ["q"]);
}

// ---------------------------------------------------------------------------
// Serializing edited documents.

#[test]
fn edited_blocks_are_regenerated() {
    let mut doc = parse("#  Title  \n\n*  item\n");
    doc.blocks[0].set_text("New");
    doc.blocks[1].set_text("changed");
    assert_eq!(serialize(&doc), "# New\n\n*  changed\n");
}

#[test]
fn new_blocks_get_default_spacing() {
    let mut doc = parse("- a\n- b\n");
    let mut c = doc.blocks[1].clone();
    c.id = next_id();
    c.containers[0] = Container::new(c.containers[0].kind.clone());
    c.blank_before = None;
    c.set_text("c");
    doc.blocks.push(c);
    doc.blocks.push(Block::paragraph("after"));
    assert_eq!(serialize(&doc), "- a\n- b\n- c\n\nafter\n");
}

#[test]
fn required_blank_lines_are_added() {
    // Two paragraphs written with no blank line between them would merge.
    let mut doc = parse("a\n");
    let mut b = Block::paragraph("b");
    b.blank_before = Some(vec![]);
    doc.blocks.push(b);
    assert_eq!(serialize(&doc), "a\n\nb\n");
    // Two adjacent quotes need a blank line to stay apart.
    let mut doc = parse("> a\n");
    let mut b = Block::new(BlockKind::Paragraph, "b", vec![Container::quote()]);
    b.blank_before = Some(vec![1]);
    doc.blocks.push(b);
    assert_eq!(serialize(&doc), "> a\n\n> b\n");
}

#[test]
fn renumbering_after_edits() {
    let mut doc = parse("1. a\n2. b\n3. c\n");
    doc.blocks.remove(1);
    assert_eq!(serialize(&doc), "1. a\n2. c\n");
}

#[test]
fn code_fence_grows_past_backticks_in_code() {
    let mut doc = parse("```\nx\n```\n");
    doc.blocks[0].set_text("```\ninner\n```");
    assert_eq!(serialize(&doc), "````\n```\ninner\n```\n````\n");
}

// ---------------------------------------------------------------------------
// The CommonMark spec and GFM examples, read from the pulldown-cmark crate
// sources in the cargo registry when they are present.

fn spec_examples() -> Vec<(String, String)> {
    let home = std::env::var("CARGO_HOME")
        .unwrap_or_else(|_| format!("{}/.cargo", std::env::var("HOME").unwrap_or_default()));
    let Ok(indexes) = std::fs::read_dir(Path::new(&home).join("registry/src")) else { return Vec::new() };
    let suite = indexes
        .flatten()
        .map(|e| e.path().join("pulldown-cmark-0.13.4/tests/suite"))
        .find(|p| p.is_dir());
    let Some(suite) = suite else { return Vec::new() };
    let mut out = Vec::new();
    for file in ["spec.rs", "gfm_table.rs", "table.rs", "gfm_tasklist.rs", "footnotes.rs"] {
        let Ok(src) = std::fs::read_to_string(suite.join(file)) else { continue };
        let mut rest = src.as_str();
        let mut n = 0;
        while let Some(start) = rest.find("let original = r##\"") {
            rest = &rest[start + "let original = r##\"".len()..];
            let end = rest.find("\"##;").unwrap();
            n += 1;
            out.push((format!("{file}#{n}"), rest[..end].to_string()));
            rest = &rest[end..];
        }
    }
    out
}

/// Examples where pulldown-cmark reads tables and footnotes differently from
/// cmark-gfm (whose reading typo follows): a table interrupting a paragraph
/// without a leading pipe, a lone `|` row, a footnote whose content starts
/// on the line after its label.
const ORACLE_DISAGREES: &[&str] = &["table.rs#13", "table.rs#19", "table.rs#20", "footnotes.rs#5"];

#[test]
fn spec_examples_keep_their_meaning() {
    let examples = spec_examples();
    let mut exact = 0;
    for (name, src) in &examples {
        let doc = parse(src);
        if serialize(&doc) == *src {
            exact += 1;
        }
        if ORACLE_DISAGREES.contains(&name.as_str()) {
            continue;
        }
        let regen = regenerated(&doc, true);
        assert_eq!(html_of(&regen), html_of(src), "{name}: {src:?} regenerated as {regen:?}");
    }
    // The rest normalize container prefixes (`>foo`, odd list indents, tabs).
    assert!(examples.is_empty() || exact >= 693, "only {exact} of {} round-trip exactly", examples.len());
}

/// Survey: `SHOW=1 cargo test --bin typo spec_survey -- --ignored --nocapture`.
#[test]
#[ignore]
fn spec_survey() {
    let examples = spec_examples();
    let (mut exact, mut same) = (0, 0);
    for (name, src) in &examples {
        let doc = parse(src);
        if serialize(&doc) == *src {
            exact += 1;
        }
        let regen = regenerated(&doc, true);
        if html_of(&regen) == html_of(src) {
            same += 1;
        } else if std::env::var("SHOW").is_ok() {
            println!("--- {name}\n{src:?}\n=> {regen:?}\n{}\nvs\n{}", html_of(src), html_of(&regen));
        }
    }
    println!("{} examples: {exact} round-trip exactly, {same} keep their meaning", examples.len());
}

#[test]
fn blank_quote_lines_do_not_loosen_the_outer_list() {
    let doc = parse("* a\n  > b\n  >\n* c\n");
    assert!(!doc.blocks[0].containers[0].list_item().unwrap().loose);
}

#[test]
fn generated_lines_that_would_start_blocks_are_escaped() {
    let mut doc = Document::default();
    doc.blocks[0].set_text("# not a heading\n- not a list\n1. not ordered\n===");
    assert_eq!(serialize(&doc), "\\# not a heading\n\\- not a list\n1\\. not ordered\n\\===\n");
    let mut doc = parse("# x\n");
    doc.blocks[0].set_text("C #");
    assert_eq!(serialize(&doc), "# C \\#\n");
    let mut doc = parse("| a |\n| - |\n");
    doc.blocks[0].set_kind(BlockKind::Table { align: vec![Align::None], rows: vec![vec!["x|y".into()]] });
    assert_eq!(serialize(&doc), "| x\\|y |\n| ---- |\n");
}

// ---------------------------------------------------------------------------
// Inline parser and HTML renderer, checked against pulldown-cmark with
// Typora's extensions off.

fn rendered(md: &str) -> String {
    html::render(&parse(md), &inline::Options::commonmark())
}

/// Newlines next to tags are layout, not meaning.
fn normalized(html: &str) -> String {
    html.replace(">\n", ">").replace("\n<", "<")
}

/// Whether the oracle knows all the syntax in a document, and reads it as
/// Typora does: it takes an `<img>` alone on a line for an HTML block,
/// Typora for a paragraph with an image in it.
fn oracle_reads(md: &str) -> bool {
    !md.contains("$$") && !md.to_lowercase().contains("[toc]") && !md.lines().any(|l| l.trim_start().starts_with("<img"))
}

#[test]
fn corpus_renders_like_the_oracle() {
    for src in CANONICAL.iter().chain(NONCANONICAL).filter(|s| oracle_reads(s)) {
        assert_eq!(normalized(&rendered(src)), normalized(&html_of(src)), "{src:?}");
    }
    for path in reference_files() {
        let src = std::fs::read_to_string(&path).unwrap();
        if oracle_reads(&src) {
            assert_eq!(normalized(&rendered(&src)), normalized(&html_of(&src)), "{}", path.display());
        }
    }
}

#[test]
fn spec_examples_render_like_the_oracle() {
    let examples = spec_examples();
    let same = examples.iter().filter(|(_, src)| normalized(&rendered(src)) == normalized(&html_of(src))).count();
    // Typora's extensions must cope with anything.
    for (_, src) in &examples {
        html::render(&parse(src), &inline::Options::all());
    }
    assert!(examples.is_empty() || same >= RENDER_SAME, "only {same} of {} render like the oracle", examples.len());
}

/// The rest are multi-line link reference definitions (not supported yet),
/// the oracle disagreements above plus `- | -` read as a delimiter row,
/// footnote definitions inside paragraphs and other footnotes, multi-line
/// labels, `---\n---` (front matter to Typora) and `&ngE;` (two code points).
const RENDER_SAME: usize = 698;

/// Survey: `SHOW=1 cargo test --bin typo render_survey -- --ignored --nocapture`.
#[test]
#[ignore]
fn render_survey() {
    let examples = spec_examples();
    let mut same = 0;
    for (name, src) in &examples {
        let (ours, theirs) = (normalized(&rendered(src)), normalized(&html_of(src)));
        if ours == theirs {
            same += 1;
        } else if std::env::var("SHOW").is_ok() {
            println!("--- {name}\n{src:?}\nours:   {ours:?}\ntheirs: {theirs:?}");
        }
    }
    println!("{} examples: {same} render like the oracle", examples.len());
}

/// Inline nodes as `kind(content)` or `kind[children]`.
fn shape(src: &str, opts: &inline::Options) -> String {
    fn walk(src: &str, nodes: &[inline::Node], out: &mut Vec<String>) {
        for n in nodes {
            let name = format!("{:?}", n.kind);
            let name = name.split([' ', '(', '{']).next().unwrap().to_lowercase();
            if n.children.is_empty() {
                out.push(format!("{name}({})", &src[n.content.clone()]));
            } else {
                let mut inner = Vec::new();
                walk(src, &n.children, &mut inner);
                out.push(format!("{name}[{}]", inner.join(" ")));
            }
        }
    }
    let mut out = Vec::new();
    walk(src, &inline::parse(src, &inline::Refs::new(), opts), &mut out);
    out.join(" ")
}

#[test]
fn inline_nodes_keep_marker_and_content_ranges() {
    let nodes = inline::parse("a **b** [c](d)", &inline::Refs::new(), &inline::Options::default());
    assert_eq!((nodes[1].range.clone(), nodes[1].content.clone()), (2..7, 4..5));
    assert_eq!((nodes[3].range.clone(), nodes[3].content.clone()), (8..14, 9..10));
    assert_eq!(nodes[3].kind, inline::Kind::Link { dest: "d".into(), title: String::new() });
}

#[test]
fn full_width_punctuation_remaps_when_asked() {
    let doc = super::parse_with("》 quoted\n\n- 【x】 done\n", true, true);
    assert_eq!(serialize(&doc), "> quoted\n\n- [x] done\n");
    let doc = super::parse_with("》 quoted\n", true, false);
    assert_eq!(serialize(&doc), "》 quoted\n");
    let opts = inline::Options { remap: true, ..inline::Options::default() };
    let nodes = inline::parse("[a](b ’t’)", &inline::Refs::new(), &opts);
    assert_eq!(nodes[0].kind, inline::Kind::Link { dest: "b".into(), title: "t".into() });
    let nodes = inline::parse("[a](b “t”)", &inline::Refs::new(), &opts);
    assert_eq!(nodes[0].kind, inline::Kind::Link { dest: "b".into(), title: "t".into() });
}

#[test]
fn destinations_with_spaces_read_as_typora_reads_them() {
    let kind = |src: &str| {
        let nodes = inline::parse(src, &inline::Refs::new(), &inline::Options::default());
        nodes.into_iter().find(|n| !matches!(n.kind, inline::Kind::Text)).map(|n| (n.kind, n.range))
    };
    let image = |dest: &str, title: &str| inline::Kind::Image { dest: dest.into(), title: title.into() };
    assert_eq!(kind("![a](my cat.png) x"), Some((image("my cat.png", ""), 0..16)));
    assert_eq!(kind("![a](<my cat.png>)"), Some((image("my cat.png", ""), 0..18)));
    assert_eq!(kind("![a](my cat.png \"Cat\")"), Some((image("my cat.png", "Cat"), 0..22)));
    assert_eq!(kind("![a](a (1).png)"), Some((image("a (1).png", ""), 0..15)));
    assert_eq!(kind("[a](b c)").map(|k| k.0), Some(inline::Kind::Link { dest: "b c".into(), title: String::new() }));
    // CommonMark's reading comes first.
    assert_eq!(kind("![a](b \"t\")"), Some((image("b", "t"), 0..11)));
    assert_eq!(kind("![a](b c"), None);
}

#[test]
fn typora_inline_extensions() {
    let all = inline::Options::all();
    assert_eq!(shape("==hi== x^2^ H~2~O ~~no~~", &all), "highlight[text(hi)] text( x) sup[text(2)] text( H) sub[text(2)] text(O ) strike[text(no)]");
    assert_eq!(shape("$x^2$ costs $5 and $6", &all), "math(x^2) text( costs $5 and $6)");
    assert_eq!(shape(":smile: <u>under</u> [^1]", &all), "emoji(smile) text( ) tag[text(under)] text( ) footnoteref(1)");
    assert_eq!(shape("see https://typora.io/a_b.", &all), "text(see ) autolink(https://typora.io/a_b) text(.)");
    assert_eq!(shape(r"X~long\ text~ ~a b~ x^a b^", &all), r"text(X) sub[text(long\ text)] text( ) strike[text(a b)] text( x^a b^)");
    // Off by default, as in Typora.
    let typora = inline::Options::default();
    assert_eq!(shape("==hi== x^2^ $x$ ~a~", &typora), "text(==hi== x^2^ $x$ ) strike[text(a)]");
    // LaTeX's delimiters are on, on one line, with something between.
    assert_eq!(shape(r"a \(x^2\) b \[y\]", &typora), "text(a ) math(x^2) text( b ) math(y)");
    assert_eq!(shape(r"\(\) \(a", &typora), "escape(() escape()) text( ) escape(() text(a)");
    assert_eq!(shape("\\(a\nb\\)", &typora), "escape(() text(a) softbreak() text(b) escape())");
    assert_eq!(shape(r"\(x\]\)", &typora), r"math(x\])");
    let off = inline::Options { latex_delimiters: false, ..typora };
    assert_eq!(shape(r"\(x\)", &off), "escape(() text(x) escape())");
}

#[test]
fn typora_extensions_render_to_html() {
    let doc = parse("==hi== H~2~O $x$ :smile: [^n]\n\n[^n]: note\n");
    let html = html::render(&doc, &inline::Options::all());
    assert_eq!(
        html,
        "<p><mark>hi</mark> H<sub>2</sub>O <span class=\"math math-inline\">x</span> 😄 \
         <sup class=\"footnote-reference\"><a href=\"#n\">1</a></sup></p>\n\
         <div class=\"footnote-definition\" id=\"n\"><sup class=\"footnote-definition-label\">1</sup>\n<p>note</p>\n</div>\n"
    );
}

// ---------------------------------------------------------------------------
// Edit operations: the markdown after the edit, and the caret.

use super::edit::{self, ListKind, Pos, Style};

/// Apply `f` to the parsed `src`; the result must be stable when re-parsed.
#[track_caller]
fn edited(src: &str, f: impl FnOnce(&mut Document) -> Pos) -> (String, Pos) {
    let mut doc = parse(src);
    let pos = f(&mut doc);
    let out = serialize(&doc);
    assert_eq!(serialize(&parse(&out)), out, "edit result is not stable");
    (out, pos)
}

#[track_caller]
fn edited_doc(src: &str, f: impl FnOnce(&mut Document)) -> String {
    edited(src, |d| {
        f(d);
        Pos::default()
    })
    .0
}

#[test]
fn enter_splits_blocks() {
    assert_eq!(edited("foo bar\n", |d| edit::split(d, Pos::new(0, 4))), ("foo\n\nbar\n".into(), Pos::new(1, 0)));
    assert_eq!(edited("# Title\n", |d| edit::split(d, Pos::new(0, 2))), ("# Ti\n\ntle\n".into(), Pos::new(1, 0)));
    let (out, pos) = edited("# Title\n", |d| edit::split(d, Pos::new(0, 0)));
    assert_eq!((out.as_str(), pos), ("# Title\n", Pos::new(1, 0)));
}

#[test]
fn enter_in_a_list_item_starts_the_next_item() {
    assert_eq!(edited("- a\n- b\n", |d| edit::split(d, Pos::new(0, 1))).0, "- a\n-\n- b\n");
    assert_eq!(edited("1. ab\n2. c\n", |d| edit::split(d, Pos::new(0, 1))).0, "1. a\n2. b\n3. c\n");
    let doc = { let mut d = parse("- [x] a\n"); edit::split(&mut d, Pos::new(0, 1)); d };
    assert_eq!(doc.blocks[1].containers[0].list_item().unwrap().task, Some(false));
}

#[test]
fn enter_on_an_empty_item_or_quote_line_leaves_it() {
    let mut doc = parse("- a\n  - b\n");
    let p = edit::split(&mut doc, Pos::new(1, 1));
    edit::split(&mut doc, p);
    assert_eq!(serialize(&doc), "- a\n  - b\n-\n");
    edit::split(&mut doc, Pos::new(2, 0));
    assert_eq!(paths(&doc), ["li", "li,li", ""]);
    let mut doc = parse("> a\n");
    let p = edit::split(&mut doc, Pos::new(0, 1));
    assert_eq!(paths(&doc), ["q", "q"]);
    edit::split(&mut doc, p);
    assert_eq!(paths(&doc), ["q", ""]);
}

#[test]
fn backspace_at_block_start() {
    assert_eq!(edited("# T\n", |d| edit::backspace(d, 0)).0, "T\n");
    assert_eq!(edited("a\n\nb\n", |d| edit::backspace(d, 1)), ("ab\n".into(), Pos::new(0, 1)));
    assert_eq!(edited("- a\n- b\n", |d| edit::backspace(d, 1)).0, "- a\n\n  b\n");
    assert_eq!(edited("- a\n", |d| edit::backspace(d, 0)).0, "a\n");
    assert_eq!(edited("> a\n", |d| edit::backspace(d, 0)).0, "a\n");
    assert_eq!(edited("a\n\n---\n\nb\n", |d| edit::backspace(d, 2)), ("a\n\nb\n".into(), Pos::new(1, 0)));
    // Into the end of a code block.
    assert_eq!(edited("```\ncode\n```\n\nx\n", |d| edit::backspace(d, 1)), ("```\ncode\n```\n\nx\n".into(), Pos::new(0, 4)));
}

#[test]
fn delete_at_block_end_and_across_blocks() {
    assert_eq!(edited("a\n\nb\n", |d| edit::delete_forward(d, 0)), ("ab\n".into(), Pos::new(0, 1)));
    let out = edited("one\n\ntwo\n\nthree\n", |d| edit::delete_range(d, Pos::new(0, 1), Pos::new(2, 2)));
    assert_eq!(out, ("oree\n".into(), Pos::new(0, 1)));
    let out = edited("one\n\n---\n\nthree\n", |d| edit::delete_range(d, Pos::new(2, 2), Pos::new(0, 3)));
    assert_eq!(out, ("oneree\n".into(), Pos::new(0, 3)));
}

#[test]
fn typing_over_a_selection() {
    let out = edited("one\n\ntwo\n", |d| edit::replace_range(d, Pos::new(0, 1), Pos::new(1, 2), "*"));
    assert_eq!(out, ("o*o\n".into(), Pos::new(0, 2)));
}

#[test]
fn heading_anchors() {
    let doc = parse("# Hello, World!\n\ntext\n\n## Hello World\n\n## *Hello* World\n");
    let (refs, opts) = (inline::Refs::new(), inline::Options::default());
    assert_eq!(query::slug("Hello, World!"), "hello-world");
    assert_eq!(query::heading_with_anchor(&doc, &refs, &opts, "hello-world"), Some(0));
    assert_eq!(query::heading_with_anchor(&doc, &refs, &opts, "Hello-World-1"), Some(2));
    assert_eq!(query::heading_with_anchor(&doc, &refs, &opts, "hello-world-2"), Some(3));
    assert_eq!(query::heading_with_anchor(&doc, &refs, &opts, "nope"), None);
}

#[test]
fn slices_for_the_clipboard() {
    let doc = parse("# Title\n\nsome **bold** text\n\n- a\n- b\n\n---\n\nend\n");
    let refs = inline::Refs::new();
    let opts = inline::Options::default();
    let cut = |a: (usize, usize), z: (usize, usize)| {
        serialize(&edit::slice(&doc, Pos::new(a.0, a.1), Pos::new(z.0, z.1), &refs, &opts))
    };
    assert_eq!(cut((1, 5), (1, 13)), "**bold**");
    assert_eq!(cut((0, 2), (1, 4)), "# tle\n\nsome");
    assert_eq!(cut((1, 10), (3, 1)), "**d** text\n\n- a\n- b");
    // Spans cut through keep their markers.
    assert_eq!(cut((1, 0), (1, 9)), "some **bo**");
    assert_eq!(cut((1, 6), (1, 9)), "**bo**");
    // A horizontal rule at the end counts once the selection passes it.
    assert_eq!(cut((3, 0), (4, 0)), "- b");
    assert_eq!(cut((3, 0), (4, 1)), "- b\n\n---");
    // Selected backwards, the same.
    assert_eq!(cut((1, 13), (1, 5)), "**bold**");
    let doc = parse("a [link *text*](http://x) b\n");
    let cut = |s: usize, e: usize| serialize(&edit::slice(&doc, Pos::new(0, s), Pos::new(0, e), &refs, &opts));
    assert_eq!(cut(0, 5), "a [li](http://x)");
    assert_eq!(cut(10, 27), "[*ext*](http://x) b");
}

#[test]
fn headings_quotes_and_lists_toggle() {
    assert_eq!(edited_doc("a\n", |d| edit::set_heading(d, 0, 2)), "## a\n");
    assert_eq!(edited_doc("## a\n", |d| edit::set_heading(d, 0, 0)), "a\n");
    assert_eq!(edited_doc("a\n\nb\n", |d| edit::toggle_quote(d, 0, 1)), "> a\n>\n> b\n");
    assert_eq!(edited_doc("> a\n>\n> b\n", |d| edit::toggle_quote(d, 1, 1)), "a\n\nb\n");
    assert_eq!(edited_doc("a\n\nb\n", |d| edit::toggle_list(d, 0, 1, ListKind::Bullet)), "- a\n- b\n");
    assert_eq!(edited_doc("- a\n- b\n", |d| edit::toggle_list(d, 0, 0, ListKind::Ordered)), "1. a\n2. b\n");
    assert_eq!(edited_doc("1. a\n2. b\n", |d| edit::toggle_list(d, 0, 1, ListKind::Ordered)), "a\n\nb\n");
    assert_eq!(edited_doc("a\n", |d| edit::toggle_list(d, 0, 0, ListKind::Task)), "- [ ] a\n");
    assert_eq!(edited_doc("- [ ] a\n", |d| edit::toggle_task(d, 0)), "- [x] a\n");
    assert_eq!(edited_doc("- [x] a\n", |d| edit::set_task(d, 0, Some(true))), "- [x] a\n");
    assert_eq!(edited_doc("- [x] a\n", |d| edit::set_task(d, 0, Some(false))), "- [ ] a\n");
    // Quoting part of a list quotes whole items.
    assert_eq!(edited_doc("- a\n\n  b\n- c\n", |d| edit::toggle_quote(d, 1, 2)), "> - a\n>\n>   b\n>\n> - c\n");
}

#[test]
fn list_indentation() {
    assert_eq!(edited_doc("- a\n- b\n", |d| assert!(edit::indent(d, 1))), "- a\n  - b\n");
    assert_eq!(edited_doc("- a\n- b\n", |d| assert!(!edit::indent(d, 0))), "- a\n- b\n");
    assert_eq!(edited_doc("1. a\n2. b\n3. c\n", |d| assert!(edit::indent(d, 1))), "1. a\n   1. b\n2. c\n");
    assert_eq!(edited_doc("- a\n  - b\n- c\n", |d| assert!(edit::indent(d, 2))), "- a\n  - b\n  - c\n");
    assert_eq!(edited_doc("- a\n  - b\n  - c\n", |d| { edit::outdent(d, 1); }), "- a\n- b\n  - c\n");
    assert_eq!(edited_doc("- a\n- b\n- c\n", |d| { edit::outdent(d, 1); }), "- a\n\nb\n\n- c\n");
}

#[test]
fn new_blocks_follow_the_syntax_preferences() {
    use edit::{HeadingStyle, Syntax};
    let with = |syntax: Syntax, src: &str, f: &dyn Fn(&mut Document)| {
        edit::set_syntax(syntax);
        let out = edited_doc(src, |d| f(d));
        edit::set_syntax(Syntax::default());
        out
    };
    let plain = Syntax { bullet: '*', ordered_same: true, indent: 4, ..Syntax::default() };
    assert_eq!(with(plain.clone(), "a\n\nb\n", &|d| edit::toggle_list(d, 0, 1, ListKind::Bullet)), "* a\n* b\n");
    assert_eq!(with(plain.clone(), "a\n\nb\n", &|d| edit::toggle_list(d, 0, 1, ListKind::Ordered)), "1. a\n1. b\n");
    // Nested by the indent size.
    assert_eq!(with(plain.clone(), "- a\n- b\n", &|d| assert!(edit::indent(d, 1))), "- a\n    - b\n");
    // Pretty indentation pads the markers out to it.
    let pretty = Syntax { indent: 4, pretty: true, ..Syntax::default() };
    assert_eq!(with(pretty.clone(), "a\n\nb\n", &|d| edit::toggle_list(d, 0, 1, ListKind::Bullet)), "-   a\n-   b\n");
    assert_eq!(with(pretty.clone(), "a\n", &|d| edit::toggle_list(d, 0, 0, ListKind::Ordered)), "1.  a\n");
    assert_eq!(with(pretty.clone(), "a\n\nb\n", &|d| edit::toggle_quote(d, 0, 1)), ">   a\n>\n>   b\n");
    // Headings: setext for levels 1 and 2 outside lists, closed ATX.
    let setext = Syntax { heading: HeadingStyle::Setext, ..Syntax::default() };
    assert_eq!(with(setext.clone(), "Title\n", &|d| edit::set_heading(d, 0, 1)), "Title\n===\n");
    assert_eq!(with(setext.clone(), "Title\n", &|d| edit::set_heading(d, 0, 3)), "### Title\n");
    assert_eq!(with(setext, "- Title\n", &|d| edit::set_heading(d, 0, 2)), "- ## Title\n");
    let wide = Syntax { heading: HeadingStyle::SetextWide, ..Syntax::default() };
    assert_eq!(with(wide, "Sub title\n", &|d| edit::set_heading(d, 0, 2)), "Sub title\n---------\n");
    let closed = Syntax { heading: HeadingStyle::AtxClosed, ..Syntax::default() };
    assert_eq!(with(closed, "Title\n", &|d| edit::set_heading(d, 0, 2)), "## Title ##\n");
    // A closing sequence read from a file stays when the heading changes.
    assert_eq!(edited_doc("# Title #\n", |d| d.blocks[0].set_text("Name")), "# Name #\n");
    assert_eq!(edited_doc("# C#\n", |d| d.blocks[0].set_text("C#!")), "# C#!\n");
}

#[test]
fn input_rules_convert_typed_markers() {
    let typed = |text: &str, caret: usize| {
        let mut doc = Document::default();
        doc.blocks[0].set_text(text);
        let pos = edit::input_rule(&mut doc, Pos::new(0, caret));
        (serialize(&doc), pos)
    };
    assert_eq!(typed("## Title", 3), ("## Title\n".into(), Some(Pos::new(0, 0))));
    assert_eq!(typed("- x", 2), ("- x\n".into(), Some(Pos::new(0, 0))));
    assert_eq!(typed("3) x", 3), ("3) x\n".into(), Some(Pos::new(0, 0))));
    assert_eq!(typed("> x", 2), ("> x\n".into(), Some(Pos::new(0, 0))));
    assert_eq!(typed("[^n]: note", 6), ("[^n]: note\n".into(), Some(Pos::new(0, 0))));
    assert_eq!(typed("#x", 2).1, None);
    assert_eq!(typed("1.5 x", 4).1, None);
    // `[ ] ` at the start of an item makes it a task.
    let mut doc = parse("- [ ] x\n");
    doc.blocks[0].containers[0] = Container::new(ContainerKind::ListItem(ListItem { task: None, ..doc.blocks[0].containers[0].list_item().unwrap().clone() }));
    doc.blocks[0].set_text("[ ] x");
    assert!(edit::input_rule(&mut doc, Pos::new(0, 4)).is_some());
    assert_eq!(serialize(&doc), "- [ ] x\n");
    // A new item right after a list joins it.
    let mut doc = parse("- a\n");
    doc.blocks.push(Block::paragraph("- c"));
    edit::input_rule(&mut doc, Pos::new(1, 2));
    assert_eq!(serialize(&doc), "- a\n- c\n");
}

#[test]
fn enter_rules_complete_block_syntax() {
    // The paragraph as typed, before Enter.
    let entered = |before: &[&str], typed: &str| {
        let mut doc = Document { blocks: before.iter().map(|t| Block::paragraph(*t)).collect(), ..Document::default() };
        doc.blocks.push(Block::paragraph(typed));
        let pos = edit::enter_rule(&mut doc, before.len(), true);
        (kinds(&doc), pos, serialize(&doc))
    };
    assert_eq!(entered(&[], "```js"), (vec!["code"], Some(Pos::new(0, 0)), "```js\n```\n".into()));
    assert_eq!(entered(&[], "$$").0, ["math"]);
    assert_eq!(entered(&[], r"\[").2, "\\[\n\\]\n");
    let (kinds, pos, _) = entered(&["a"], "***");
    assert_eq!((kinds, pos), (vec!["paragraph", "hr", "paragraph"], Some(Pos::new(2, 0))));
    assert_eq!(entered(&[], "| a | b |").2, "| a   | b   |\n| --- | --- |\n|     |     |\n");
    // The caret goes to the first cell under the header.
    assert_eq!(entered(&[], "| a | b |").1, Some(crate::md::table::caret(0, (1, 0))));
    assert_eq!(entered(&[], "[toc]").0, ["toc", "paragraph"]);
    assert_eq!(entered(&[], "---").0, ["frontmatter"]);
    assert_eq!(entered(&["a"], "---").0, ["paragraph", "hr", "paragraph"]);
}

#[test]
fn paste_markdown() {
    assert_eq!(edited("ab\n", |d| edit::insert_markdown(d, Pos::new(0, 1), "x", true)), ("axb\n".into(), Pos::new(0, 2)));
    assert_eq!(edited("ab\n", |d| edit::insert_markdown(d, Pos::new(0, 1), "x\n\ny", true)), ("ax\n\nyb\n".into(), Pos::new(1, 1)));
    let (out, pos) = edited("ab\n", |d| edit::insert_markdown(d, Pos::new(0, 1), "- i\n- j\n", true));
    assert_eq!((out.as_str(), pos), ("a\n\n- i\n- j\n\nb\n", Pos::new(2, 1)));
    let (out, _) = edited("- ab\n", |d| edit::insert_markdown(d, Pos::new(0, 1), "> q\n\nz", true));
    assert_eq!(out, "- a\n  > q\n\n  zb\n");
}

#[test]
fn inline_style_toggles() {
    let refs = inline::Refs::new();
    assert_eq!(edit::toggle_style("a b c", 2..3, Style::Strong, &refs), ("a **b** c".into(), 4..5));
    assert_eq!(edit::toggle_style("a **b** c", 4..5, Style::Strong, &refs), ("a b c".into(), 2..3));
    assert_eq!(edit::toggle_style("ab", 1..1, Style::Strong, &refs), ("a****b".into(), 3..3));
    assert_eq!(edit::toggle_style("**bold**", 3..3, Style::Strong, &refs), ("bold".into(), 1..1));
    assert_eq!(edit::toggle_style("x", 0..1, Style::Underline, &refs), ("<u>x</u>".into(), 3..4));
    assert_eq!(edit::toggle_style("<u>x</u>", 3..4, Style::Underline, &refs), ("x".into(), 0..1));
    assert_eq!(edit::toggle_style("a b ", 0..4, Style::Strong, &refs), ("**a b** ".into(), 2..5));
    assert_eq!(edit::toggle_style("==hi==", 2..4, Style::Highlight, &refs), ("hi".into(), 0..2));
    assert_eq!(edit::toggle_link("go here", 3..7, &refs, ""), ("go [here]()".into(), 10..10));
    assert_eq!(edit::toggle_link("go [here](x)", 5..5, &refs, ""), ("go here".into(), 4..4));
    let footnote = |t: &str, sel| edit::text_command(t, sel, "footnote", "", &refs).unwrap();
    assert_eq!(footnote("a b", 1..1), ("a[^] b".into(), 3..3));
    assert_eq!(footnote("a note ", 2..7), ("a [^note] ".into(), 4..8));
    assert_eq!(footnote("a[^note] b", 4..4), ("anote b".into(), 1..5));
    assert_eq!(edit::clear_format("**a** *b* c", 0..11, &refs, &inline::Options::default()), ("a b c".into(), 0..5));
}

#[test]
fn zooms_and_switches_images() {
    let refs = inline::Refs::new();
    let zoom = |text: &str, at: usize, z: &str| edit::zoom_image(text, at..at, &refs, z);
    // A markdown image becomes a tag with a zoom, selected.
    let (out, sel) = zoom("a ![cat](c.png) b", 4, "50%").unwrap();
    assert_eq!(out, r#"a <img src="c.png" alt="cat" style="zoom:50%;" /> b"#);
    assert_eq!(&out[sel], r#"<img src="c.png" alt="cat" style="zoom:50%;" />"#);
    // Again on the tag, and back to 100%, which leaves the tag.
    let (out, _) = zoom(&out, 4, "200%").unwrap();
    assert_eq!(out, r#"a <img src="c.png" alt="cat" style="zoom: 200%;" /> b"#);
    let (out, _) = zoom(&out, 4, "100%").unwrap();
    assert_eq!(out, r#"a <img src="c.png" alt="cat" /> b"#);
    assert_eq!(zoom("![cat](c.png)", 1, "100%"), None);
    assert_eq!(zoom("no image", 1, "50%"), None);
    let switch = |text: &str, at: usize, html: bool| edit::switch_image_syntax(text, at..at, &refs, html).map(|r| r.0);
    assert_eq!(switch("![a *b*](x.png \"t\")", 2, true).as_deref(), Some(r#"<img src="x.png" alt="a b" title="t" />"#));
    assert_eq!(switch(r#"<img src="x.png" alt="a" style="zoom:50%" />"#, 3, false).as_deref(), Some("![a](x.png)"));
    assert_eq!(switch("![a](x.png)", 2, false), None);
}

#[test]
fn writes_front_matter_properties() {
    let mut d = parse("text\n");
    assert!(!edit::set_property(&mut d, "typora-root-url", None));
    assert!(edit::set_property(&mut d, "typora-root-url", Some("./")));
    assert_eq!(serialize(&d), "---\ntypora-root-url: ./\n---\n\ntext\n");
    assert!(edit::set_property(&mut d, "typora-copy-images-to", Some("./assets")));
    assert!(edit::set_property(&mut d, "Typora-Root-URL", Some("../img")));
    assert_eq!(d.blocks[0].text, "Typora-Root-URL: ../img\ntypora-copy-images-to: ./assets");
    assert!(!edit::set_property(&mut d, "typora-copy-images-to", Some("./assets")));
    assert!(edit::set_property(&mut d, "typora-root-url", None));
    assert_eq!(d.blocks[0].text, "typora-copy-images-to: ./assets");
}

#[test]
fn finds_and_repoints_images() {
    let mut refs = inline::Refs::new();
    refs.insert("r".into(), inline::LinkRef { dest: "r.png".into(), title: String::new() });
    let text = r#"a ![x](x.png) [![y](y.png)](l) <img src="z.png"> ![r][r]"#;
    let found: Vec<String> = edit::images(text, &refs).into_iter().map(|(r, src)| format!("{}={src}", &text[r])).collect();
    assert_eq!(found, ["![x](x.png)=x.png", "![y](y.png)=y.png", r#"<img src="z.png">=z.png"#, "![r][r]=r.png"]);
    let at = |pos: usize| edit::image_at(text, pos..pos, &refs).map(|(_, src, syntax)| (src, syntax));
    assert_eq!(at(4), Some(("x.png".into(), "markdown")));
    assert_eq!(at(2), Some(("x.png".into(), "markdown")));
    assert_eq!(at(35), Some(("z.png".into(), "html")));
    assert_eq!(at(text.len() - 1), Some(("r.png".into(), "ref")));
    assert_eq!(at(0), None);
    let point = |text: &str, pos: usize, src: &str| edit::set_image_src(text, pos..pos, &refs, src).map(|r| r.0);
    assert_eq!(point("![x](x.png \"t\")", 3, "a b.png").as_deref(), Some("![x](<a b.png> \"t\")"));
    assert_eq!(point("![r][r]", 3, "s.png").as_deref(), Some("![r](s.png)"));
    assert_eq!(point("<img src=z.png alt=z>", 3, "w.png").as_deref(), Some(r#"<img src="w.png" alt=z>"#));
    // In a table cell, through the cell's commands.
    let cell = |name, arg| edit::text_command("a ![x](x.png) b", 4..4, name, arg, &refs);
    assert_eq!(cell("imageSrc", "y.png"), Some(("a ![x](y.png) b".into(), 2..13)));
    assert_eq!(cell("deleteImage", ""), Some(("a  b".into(), 2..2)));
}

#[test]
fn links_and_images_take_their_address() {
    let refs = inline::Refs::new();
    // A URL selected is its own address; otherwise the clipboard's, if any.
    assert_eq!(edit::toggle_link("see x.org now", 4..9, &refs, "").0, "see [x.org]() now");
    assert_eq!(edit::toggle_link("a http://x.y", 2..12, &refs, ""), ("a [http://x.y](http://x.y)".into(), 25..25));
    assert_eq!(edit::toggle_link("a b", 2..3, &refs, " https://c.d \n"), ("a [b](https://c.d)".into(), 17..17));
    assert_eq!(edit::toggle_link("a b", 2..3, &refs, "just words").0, "a [b]()");
    // Spaces at the ends of the selection stay outside.
    assert_eq!(edit::toggle_link("a  b ", 1..5, &refs, "").0, "a  [b]() ");
    // With nothing selected the caret goes between the brackets, and a
    // path or file name on the clipboard will do.
    assert_eq!(edit::toggle_link("ab", 1..1, &refs, "notes/today.md"), ("a[](notes/today.md)b".into(), 2..2));
    assert_eq!(edit::toggle_link("ab", 1..1, &refs, "two words"), ("a[]()b".into(), 2..2));
    // Images want an image's address.
    assert_eq!(edit::toggle_image("x", 1..1, &refs, ""), ("x![]()".into(), 3..3));
    assert_eq!(edit::toggle_image("x", 1..1, &refs, "https://a.b/c.PNG?s=1"), ("x![](https://a.b/c.PNG?s=1)".into(), 3..3));
    assert_eq!(edit::toggle_image("x", 1..1, &refs, "https://a.b/c.html").0, "x![]()");
    assert_eq!(edit::toggle_image("a cat", 2..5, &refs, "img/cat.jpg"), ("a ![cat](img/cat.jpg)".into(), 20..20));
    assert_eq!(edit::toggle_image("pic.gif", 0..7, &refs, ""), ("![pic.gif](pic.gif)".into(), 18..18));
    // Again, the image goes back to its alt text, selected.
    assert_eq!(edit::toggle_image("a ![cat](c.png) b", 9..9, &refs, ""), ("a cat b".into(), 2..5));
    assert!(edit::looks_like_url("mailto:a@b.c") && edit::looks_like_url("www.x") && !edit::looks_like_url("x.org"));
}

// ---------------------------------------------------------------------------
// History and queries.

use super::history::{History, Merge, Sel};
use super::query::{self, FindOptions};

#[test]
fn undo_and_redo_restore_blocks_and_selection() {
    let mut doc = parse("a\n\nb\n");
    let mut h = History::default();
    let old = doc.blocks.clone();
    let caret = edit::split(&mut doc, Pos::new(0, 1));
    let s = h.record(&old, &doc, Sel::caret(Pos::new(0, 1)), Sel::caret(caret), Merge::Never).unwrap();
    // "a" is untouched; "b" changed its spacing.
    assert_eq!((s.at, s.removed, s.inserted), (1, 1, 2));
    assert!(h.modified());
    let (_, sel) = h.undo(&mut doc).unwrap();
    assert_eq!((serialize(&doc), sel.caret), ("a\n\nb\n".into(), Pos::new(0, 1)));
    assert!(!h.modified());
    h.redo(&mut doc);
    assert_eq!((doc.blocks.len(), serialize(&doc)), (3, "a\n\nb\n".into()));
}

#[test]
fn typing_in_one_block_coalesces() {
    let mut doc = parse("a\n");
    let mut h = History::default();
    let id = doc.blocks[0].id;
    for text in ["ab", "abc", "abcd"] {
        let old = doc.blocks.clone();
        doc.blocks[0].set_text(text);
        h.record(&old, &doc, Sel::default(), Sel::default(), Merge::Typing(id));
    }
    h.mark_saved();
    let old = doc.blocks.clone();
    doc.blocks[0].set_text("abcde");
    h.record(&old, &doc, Sel::default(), Sel::default(), Merge::Typing(id));
    assert!(h.modified(), "coalesced typing after a save still modifies");
    h.undo(&mut doc);
    assert_eq!(doc.blocks[0].text, "a");
    assert!(!h.can_undo());
}

/// Reparse `src` as Source Code Mode does and record it.
fn source_edit(doc: &mut Document, h: &mut History, src: &str) -> super::history::Splice {
    let old = doc.blocks.clone();
    let mut new = parse(src);
    super::history::adopt_ids(&old, &mut new.blocks);
    doc.blocks = new.blocks;
    h.record(&old, doc, Sel::default(), Sel::default(), Merge::Source).unwrap()
}

#[test]
fn source_edits_keep_ids_and_make_one_step() {
    let mut doc = parse("# A\n\n- one\n- two\n\nend\n");
    let mut h = History::default();
    let ids: Vec<_> = doc.blocks.iter().map(|b| b.id).collect();
    // Only the edited item changes; its list keeps its id, so the items
    // stay one list.
    let s = source_edit(&mut doc, &mut h, "# A\n\n- one\n- two!\n\nend\n");
    assert_eq!((s.at, s.removed, s.inserted), (2, 1, 1));
    assert_eq!((doc.blocks[0].id, doc.blocks[1].id, doc.blocks[3].id), (ids[0], ids[1], ids[3]));
    let list = |i: usize| doc.blocks[i].containers[0].list_item().unwrap().list;
    assert_eq!(list(1), list(2));
    assert_eq!(serialize(&doc), "# A\n\n- one\n- two!\n\nend\n");
    // More edits join the same step; undo goes back to before the visit.
    source_edit(&mut doc, &mut h, "# A\n\n- one\n- two!\n\nThe end\n");
    h.seal();
    let old = doc.blocks.clone();
    doc.blocks[0].set_text("B");
    h.record(&old, &doc, Sel::default(), Sel::default(), Merge::Never);
    h.undo(&mut doc);
    h.undo(&mut doc);
    assert_eq!(serialize(&doc), "# A\n\n- one\n- two\n\nend\n");
    assert!(!h.can_undo() && !h.modified());
    // A visit that comes back to the start leaves no step.
    let mut h = History::default();
    source_edit(&mut doc, &mut h, "# A\n\n- one\n- two\n\nend!\n");
    let old = doc.blocks.clone();
    let mut new = parse("# A\n\n- one\n- two\n\nend\n");
    super::history::adopt_ids(&old, &mut new.blocks);
    doc.blocks = new.blocks;
    h.record(&old, &doc, Sel::default(), Sel::default(), Merge::Source);
    assert!(!h.can_undo() && !h.modified());
}

#[test]
fn adopted_ids_keep_split_lists_apart() {
    // Splitting a list in two: the second half must not take the first
    // half's list id.
    let mut doc = parse("- a\n- b\n- c\n");
    let mut h = History::default();
    source_edit(&mut doc, &mut h, "- a\n\nx\n\n- c\n");
    let list = |i: usize| doc.blocks[i].containers[0].list_item().unwrap().list;
    assert_ne!(list(0), list(2));
    assert_eq!(serialize(&doc), "- a\n\nx\n\n- c\n");
}

#[test]
fn link_ref_def_parts() {
    let line = "[Typora]:  https://typora.io  \"The editor\"";
    let [label, dest, title] = inline::link_ref_def_parts(line).unwrap();
    assert_eq!((&line[label], &line[dest], &line[title]), ("Typora", "https://typora.io", "\"The editor\""));
    let line = "[a b]: <x y>";
    let [label, dest, title] = inline::link_ref_def_parts(line).unwrap();
    assert_eq!((&line[label], &line[dest], title.is_empty()), ("a b", "<x y>", true));
    assert!(inline::link_ref_def_parts("[a]: ").is_none());
    // Backspace at its start makes a paragraph of it, escaped to stay one.
    let out = edited("[a]: /u\n", |d| edit::backspace(d, 0));
    assert_eq!(out.0, "\\[a]: /u\n");
}

#[test]
fn alerts() {
    assert_eq!(query::alert_of("[!NOTE]\nUseful."), Some(("note", 7)));
    assert_eq!(query::alert_of("[!caution]  "), Some(("caution", 10)));
    assert_eq!(query::alert_of("[!NOTE] text"), None);
    assert_eq!(query::alert_of("[!FOO]\nx"), None);
    assert_eq!(query::alert_of("Plain"), None);
    // Paragraph ▸ Link Reference splits the paragraph at the caret, or
    // takes the selection as its URL or label.
    let def = |src: &str, a: Pos, z: Pos| edited(src, |d| edit::insert_link_ref(d, a, z));
    assert_eq!(def("ab\n", Pos::new(0, 1), Pos::new(0, 1)), ("a\n\n[]: \n\nb\n".to_string(), Pos::new(1, 1)));
    assert_eq!(def("ab\n", Pos::new(0, 2), Pos::new(0, 2)), ("ab\n\n[]: \n".to_string(), Pos::new(1, 1)));
    assert_eq!(def("ab\n\nc\n", Pos::new(0, 2), Pos::new(0, 2)).0, "ab\n\n[]: \n\nc\n");
    assert_eq!(def("see docs/a.md now\n", Pos::new(0, 4), Pos::new(0, 13)), ("see\n\n[]: docs/a.md\n\nnow\n".to_string(), Pos::new(1, 1)));
    assert_eq!(def("word\n", Pos::new(0, 0), Pos::new(0, 4)), ("[word]: \n".to_string(), Pos::new(0, 1)));
    // Paragraph ▸ Alert, with the caret at offset 1 of block `b`.
    let alert = |src: &str, b: usize, kind: &str| {
        edited(src, |d| edit::toggle_alert(d, [Pos::new(b, 1); 2], kind)[0])
    };
    assert_eq!(alert("Text\n", 0, "tip"), ("> [!TIP]\n> Text\n".to_string(), Pos::new(0, 8)));
    assert_eq!(alert("> [!TIP]\n> Text\n", 0, "warning"), ("> [!WARNING]\n> Text\n".to_string(), Pos::new(0, 5)));
    assert_eq!(alert("> [!TIP]\n> Text\n", 0, "tip"), ("Text\n".to_string(), Pos::new(0, 0)));
    assert_eq!(alert("> [!NOTE]\n>\n> Text\n", 1, "note"), ("Text\n".to_string(), Pos::new(0, 1)));
    assert_eq!(alert("> a\n", 0, "note"), ("> [!NOTE]\n> a\n".to_string(), Pos::new(0, 9)));
    assert_eq!(alert("> a\n>\n> b\n", 1, "note").0, "> a\n>\n> > [!NOTE]\n> > b\n");
    assert_eq!(alert("# Head\n", 0, "note"), ("> [!NOTE]\n>\n> # Head\n".to_string(), Pos::new(1, 1)));
    // A selected TOC goes as a whole.
    let out = edited("a\n\n[TOC]\n\nb\n", |d| edit::delete_range(d, Pos::new(1, 0), Pos::new(1, 1)));
    assert_eq!(out, ("a\n\nb\n".to_string(), Pos::new(1, 0)));
    let refs = inline::Refs::new();
    let opts = inline::Options::default();
    assert_eq!(html::inline_html("Two **bold** `c`", &refs, &opts), "Two <strong>bold</strong> <code>c</code>");
}

#[test]
fn footnote_index() {
    let doc = parse("A[^n] and [^N] and [^x].\n\n[^n]: First\n\n    more [link]\n\nB\n\n[^n]: Again\n\n[link]: /u\n");
    let (refs, opts) = (inline::refs_of(&doc), inline::Options::default());
    let notes = query::footnotes(&doc, &refs, &opts);
    let shown: Vec<_> = notes.iter().map(|f| (f.label.as_str(), f.refs.iter().map(|n| (n.block, n.range.clone())).collect::<Vec<_>>(), f.def)).collect();
    assert_eq!(shown, [("n", vec![(0, 1..5), (0, 10..14)], Some((1, 2))), ("x", vec![(0, 19..23)], None)]);
    let sub = query::footnote_doc(&doc, &refs, &opts, "N").unwrap();
    assert_eq!(html::render(&sub, &opts), "<p>First</p>\n<p>more <a href=\"/u\">link</a></p>\n");
    assert!(query::footnote_doc(&doc, &refs, &opts, "x").is_none());
    let (out, at) = edited("a[^x]\n", |d| edit::add_footnote_def(d, "x"));
    assert_eq!((out.as_str(), at), ("a[^x]\n\n[^x]:\n", Pos::new(1, 0)));
    // References in table cells count, with their cell.
    let doc = parse("| h | a[^t] |\n| --- | --- |\n| [^t] | b |\n\n[^t]: T\n");
    let notes = query::footnotes(&doc, &inline::refs_of(&doc), &opts);
    let cells: Vec<_> = notes[0].refs.iter().map(|n| (n.block, n.cell, n.range.clone(), n.text(&doc).to_string())).collect();
    assert_eq!(cells, [(0, Some((0, 1)), 1..5, "a[^t]".to_string()), (0, Some((1, 0)), 0..4, "[^t]".to_string())]);
}

#[test]
fn outline_and_counts() {
    let doc = parse("# One *two*\n\nsome `code` and **words**\n\n## 中文标题\n\n```\nlet x\n```\n");
    let refs = inline::Refs::new();
    let opts = inline::Options::default();
    let titles: Vec<_> = query::outline(&doc, &refs, &opts).into_iter().map(|h| (h.level, h.title)).collect();
    assert_eq!(titles, [(1, "One two".to_string()), (2, "中文标题".to_string())]);
    assert_eq!(query::words("it's a well-known fact, 3.5 times"), 6);
    let src = serialize(&doc);
    let s = query::stats(&doc, &src, &refs, &opts);
    // one two | some code and words | 4 CJK | let x; the final line break
    // starts a tenth line.
    assert_eq!((s.words, s.lines, s.characters), (2 + 4 + 4 + 2, 10, src.encode_utf16().count()));
    assert_eq!(s.minutes(240), 0);
    assert_eq!(query::Stats { words: 120, ..s }.minutes(240), 1);
}

fn typora_words(src: &str) -> usize {
    let doc = parse(src);
    let refs = inline::refs_of(&doc);
    query::stats(&doc, src, &refs, &inline::Options::default()).words
}

#[test]
fn counts_words_as_typora() {
    // What Typora's status bar shows for each document.
    let cases = [
        ("A paragraph with **strong**, *emphasis*, `code`, ~~strike~~, a [link](https://omarchy.org) and a footnote[^1].\n\n[^1]: The footnote text.\n", 21),
        ("```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n", 5),
        ("| Name | Value |\n| ---- | ----: |\n| one  |     1 |\n| two  |     2 |\n", 6),
        ("$$\nE = mc^2\n$$\n", 3),
        ("- Bullet one\n- Bullet two\n  - Nested bullet\n1. First\n2. Second\n\n- [x] Done task\n- [ ] Open task\n", 12),
        ("> A quote that runs long enough to wrap onto a second line in a window of ordinary width, to see the bar.\n\n---\n\n# Quality check\n", 24),
        ("Visit https://omarchy.org now and [text](http://x.org/a-b) here.\n", 9),
        ("It's e.g. well-known 3.5 a_b a/b x--y.\n", 8),
        ("[link](https://omarchy.org)\n", 3),
        ("a footnote[^1].\n", 4),
        ("[^1]: The footnote text.\n", 4),
        ("**strong**, *emphasis*, `code`, ~~strike~~\n", 4),
        // A definition shows a ↩ for each reference to it.
        ("a footnote[^1].\n\n[^1]: The footnote text.\n", 9),
        ("x[^a] q[^b]\n\n[^a]: y\n\n[^b]: z\n", 10),
        ("![alt text](a.png \"Title\") and <span>html</span> &amp; :smile: H~2~O x^2^ ==mark==\n", 18),
        ("<div>\nhello world\n</div>\n\n[ref]: http://x.org \"T\"\n\n[toc]\n\n---\ntitle: Front\n---\n", 11),
        ("---\ntitle: Front matter here\n---\n\nbody\n", 5),
        ("| A | B |\n| - | - |\n| c d | e |\n", 5),
        ("Text :smile: end\n", 5),
        ("p ![alt here](a.png) q\n", 5),
        ("p <b>bold</b> q &amp; r\n", 7),
    ];
    for (src, n) in cases {
        assert_eq!(typora_words(src), n, "{src:?}");
    }
    assert_eq!(query::words(""), 0);
    assert_eq!(query::words(" \u{3000}"), 0);
    // Punctuation alone, and an ideographic space as either.
    assert_eq!(query::words("a - b -- c"), 3);
    assert_eq!(query::words("中文。\u{3000}b"), 3);
    assert_eq!(query::words("x:smile:y 😀"), 5);
}

#[test]
fn find_and_replace() {
    let mut doc = parse("Foo foo food\n\n| foo |\n| --- |\n| x   |\n");
    let plain = FindOptions::default();
    assert_eq!(query::find(&doc, "foo", &plain).unwrap().len(), 4);
    let whole = FindOptions { whole_word: true, case_sensitive: true, ..plain };
    assert_eq!(query::find(&doc, "foo", &whole).unwrap().len(), 2);
    let re = FindOptions { regex: true, ..plain };
    assert_eq!(query::replace_all(&mut doc, r"f(o+)d", "m${1}d", &re).unwrap(), 1);
    assert_eq!(query::replace_all(&mut doc, "foo", "bar", &whole).unwrap(), 2);
    assert_eq!(serialize(&doc), "Foo bar mood\n\n| bar |\n| --- |\n| x   |\n");
    assert!(query::find(&doc, "(", &re).is_err());
    // A TOC's `[toc]` isn't text anyone sees.
    let doc = parse("[toc]\n\ntoc\n");
    assert_eq!(query::find(&doc, "toc", &plain).unwrap().len(), 1);
}

// ---------------------------------------------------------------------------
// Auto-pairing.

/// Types `keys` one at a time into `src` (`|` marks the caret) through the
/// pairing rules for a block of `kind`.
fn pairing(kind: &BlockKind, src: &str, keys: &str, markdown: bool) -> String {
    let (refs, opts) = (inline::Refs::new(), inline::Options::default());
    let mut at = src.find('|').unwrap();
    let mut text = src.replacen('|', "", 1);
    for ch in keys.chars() {
        match pair::typed(kind, &text, at, ch, markdown, &refs, &opts) {
            pair::Typed::Insert => {
                text.insert(at, ch);
                at += ch.len_utf8();
            }
            pair::Typed::Skip => at += ch.len_utf8(),
            pair::Typed::Pair(c) => {
                text.insert(at, c);
                text.insert(at, ch);
                at += ch.len_utf8();
            }
        }
    }
    format!("{}|{}", &text[..at], &text[at..])
}

#[test]
fn pairs_brackets_and_quotes() {
    let p = |src, keys| pairing(&BlockKind::Paragraph, src, keys, false);
    assert_eq!(p("|", "("), "(|)");
    assert_eq!(p("|", "(x)"), "(x)|");
    assert_eq!(p("|abc", "("), "(|abc");
    assert_eq!(p("a |", "[b"), "a [b|]");
    assert_eq!(p("|)", "{"), "{|})");
    assert_eq!(p("\\|", "("), "\\(|");
    assert_eq!(p("|", "\""), "\"|\"");
    assert_eq!(p("say |", "\"hi\""), "say \"hi\"|");
    assert_eq!(p("don|", "'t"), "don't|");
    assert_eq!(p("x |a", "'"), "x '|a");
    assert_eq!(p("end.|", "\""), "end.\"|");
    assert_eq!(p("\"open |", "\""), "\"open \"|");
    assert_eq!(p("it's |", "'"), "it's '|'");
    // Markdown symbols pair only when asked to.
    assert_eq!(p("|", "*"), "*|");
}

#[test]
fn pairs_markdown_symbols() {
    let p = |src, keys| pairing(&BlockKind::Paragraph, src, keys, true);
    assert_eq!(p("|", "*"), "*|*");
    assert_eq!(p("|", "**b**"), "**b**|");
    assert_eq!(p("|", "*i*"), "*i*|");
    assert_eq!(p("|", "```"), "```|");
    assert_eq!(p("a |", "`c`"), "a `c`|");
    assert_eq!(p("word|", "*"), "word*|");
    assert_eq!(p("(|", "_"), "(_|_");
    assert_eq!(p("|x", "*"), "*|x");
    assert_eq!(p("\\|", "*"), "\\*|");
    assert_eq!(p("`co|de`", "*"), "`co*|de`");
    assert_eq!(p("snake_|", "_"), "snake__|");
}

#[test]
fn pairs_in_code_like_codemirror() {
    let code = BlockKind::CodeFence { info: String::new(), fence: "```".into(), closed: true };
    let p = |src, keys| pairing(&code, src, keys, true);
    assert_eq!(p("|", "f(x)"), "f(x)|");
    assert_eq!(p("|", "("), "(|)");
    assert_eq!(p("|x", "("), "(|x");
    assert_eq!(p("a = |", "'s'"), "a = 's'|");
    assert_eq!(p("x|", "'"), "x'|");
    assert_eq!(p("|", "*"), "*|");
    assert_eq!(p("a\n|", "["), "a\n[|]");
}

#[test]
fn backspace_removes_empty_pairs() {
    let opts = inline::Options::default();
    let para = BlockKind::Paragraph;
    assert!(pair::paired(&para, "()", 1, false, &opts));
    assert!(pair::paired(&para, "a \"\" b", 3, false, &opts));
    assert!(!pair::paired(&para, "(x)", 1, false, &opts));
    assert!(!pair::paired(&para, "**", 1, false, &opts));
    assert!(pair::paired(&para, "**", 1, true, &opts));
    assert!(!pair::paired(&para, "==", 1, true, &opts));
    assert!(!pair::paired(&para, ")", 0, false, &opts));
    assert!(pair::paired(&BlockKind::Math { brackets: false, before: String::new(), after: String::new() }, "{}", 1, false, &opts));
}

// ---------------------------------------------------------------------------
// HTML to markdown, for a rich paste.

fn from_html(html: &str) -> String {
    from_html::to_markdown(html, &inline::Options::default())
}

#[test]
fn converts_inline_html() {
    assert_eq!(from_html("<p>a <b>bold</b> and <em>em</em> and <code>x`y</code></p>"), "a **bold** and *em* and ``x`y``");
    assert_eq!(from_html("<p>a<strong> spaced </strong>b</p>"), "a **spaced** b");
    assert_eq!(from_html("<p><a href=\"https://x.y/a b\" title=\"T\">link</a> <img src=\"i.png\" alt=\"pic\"></p>"),
        "[link](<https://x.y/a%20b> \"T\") ![pic](i.png)");
    assert_eq!(from_html("<p><del>gone</del> <u>under</u> <sup>2</sup> <mark>m</mark></p>"),
        "~~gone~~ <u>under</u> <sup>2</sup> <mark>m</mark>");
    assert_eq!(from_html("<p>line one<br>line two</p>"), "line one\nline two");
    assert_eq!(from_html("<p>  lots   of\n  space  </p>"), "lots of space");
}

#[test]
fn escapes_markdown_in_html_text() {
    assert_eq!(from_html("<p>2 * 3 = [x] snake_case _it_ &lt;div&gt;</p>"), "2 \\* 3 = \\[x\\] snake_case \\_it\\_ \\<div>");
    assert_eq!(from_html("<p># not a heading</p><p>- nor a list</p><p>1. nor this</p>"),
        "\\# not a heading\n\n\\- nor a list\n\n1\\. nor this");
}

#[test]
fn converts_block_html() {
    assert_eq!(from_html("<h2>Title <i>it</i></h2><p>text</p><hr><blockquote><p>q1</p><p>q2</p></blockquote>"),
        "## Title *it*\n\ntext\n\n---\n\n> q1\n>\n> q2");
    assert_eq!(from_html("<pre><code class=\"language-rust\">fn main() {\n    x();\n}\n</code></pre>"),
        "```rust\nfn main() {\n    x();\n}\n```");
    assert_eq!(from_html("<div class=\"highlight highlight-source-js\"><pre>a<br>b</pre></div>"), "```js\na\nb\n```");
    assert_eq!(from_html("<ul><li>a<ul><li>b</li></ul></li><li><input type=checkbox checked> done</li></ul><ol start=3><li>c</li><li><p>d</p><p>e</p></li></ol>"),
        "- a\n  - b\n- [x] done\n\n3. c\n\n4. d\n\n   e");
    assert_eq!(from_html("<table><thead><tr><th>A</th><th align=right>B</th></tr></thead><tbody><tr><td>1|2</td><td><b>3</b></td></tr></tbody></table>"),
        "| A | B |\n| --- | ---: |\n| 1\\|2 | **3** |");
}

#[test]
fn converts_styled_spans() {
    // Google Docs wraps a paste in a <b> that isn't bold.
    let docs = "<meta charset=\"utf-8\"><b style=\"font-weight:normal;\" id=\"docs-internal-guid-1\"><p dir=\"ltr\"><span style=\"font-weight:700\">Bold</span><span> and </span><span style=\"font-style:italic\">it</span></p></b>";
    assert_eq!(from_html(docs), "**Bold** and *it*");
    assert_eq!(from_html("<!--StartFragment--><span>plain</span><!--EndFragment-->"), "plain");
}

#[test]
fn deleting_a_block_moves_to_its_neighbour() {
    assert_eq!(edited("a\n\nb\n\nc\n", |d| edit::delete_block(d, 1)), ("a\n\nc\n".into(), Pos::new(1, 0)));
    assert_eq!(edited("a\n\n```\nx\n```\n", |d| edit::delete_block(d, 1)), ("a\n".into(), Pos::new(0, 1)));
    assert_eq!(edited("---\n", |d| edit::delete_block(d, 0)).1, Pos::new(0, 0));
}

#[test]
fn tab_over_selected_list_items() {
    let tab = |src: &str, a: usize, z: usize| {
        let mut d = parse(src);
        let changed = edit::indent_range(&mut d, a, z);
        (serialize(&d), changed)
    };
    assert_eq!(tab("- a\n- b\n- c\n", 1, 2), ("- a\n  - b\n  - c\n".into(), true));
    // The first item can't be indented, so nothing is.
    assert_eq!(tab("- a\n- b\n", 0, 1), ("- a\n- b\n".into(), false));
    // A nested child moves with its parent.
    assert_eq!(tab("- a\n- b\n  - c\n- d\n", 1, 3).0, "- a\n  - b\n    - c\n  - d\n");
    // Only the nested items are selected: only they move.
    assert_eq!(tab("- a\n  - b\n  - c\n", 2, 2).0, "- a\n  - b\n    - c\n");
    let shift_tab = |src: &str, a: usize, z: usize| {
        let mut d = parse(src);
        edit::outdent_range(&mut d, a, z);
        serialize(&d)
    };
    assert_eq!(shift_tab("- a\n  - b\n  - c\n", 1, 2), "- a\n- b\n- c\n");
    assert_eq!(shift_tab("- a\n- b\n- c\n", 0, 1), "a\n\nb\n\n- c\n");
}

#[test]
fn shift_tab_from_a_nested_item_out_of_the_list() {
    let mut d = parse("- a\n  - b\n- c\n");
    edit::outdent_range(&mut d, 1, 2);
    assert_eq!(serialize(&d), "- a\n- b\n\nc\n");
}

#[test]
fn styles_across_blocks() {
    let styled = |src: &str, a: Pos, z: Pos| {
        let mut d = parse(src);
        let sel = edit::style_range(&mut d, a, z, Style::Strong, &super::inline::Refs::default());
        (serialize(&d), sel)
    };
    // Each block's part, the middle ones whole.
    let (out, sel) = styled("one two\n\nthree\n\nfour five\n", Pos::new(0, 4), Pos::new(2, 4));
    assert_eq!(out, "one **two**\n\n**three**\n\n**four** five\n");
    assert_eq!(sel, (Pos::new(0, 6), Pos::new(2, 6)));
    // All parts strong already: the style comes off.
    assert_eq!(styled("**a**\n\n**b**\n", Pos::new(0, 2), Pos::new(1, 3)).0, "a\n\nb\n");
    // Some strong, some not: the rest become strong too.
    assert_eq!(styled("**a**\n\nb\n", Pos::new(0, 2), Pos::new(1, 1)).0, "**a**\n\n**b**\n");
    // Blocks without inline text are left alone.
    assert_eq!(styled("a\n\n```\nx\n```\n\nb\n", Pos::new(0, 0), Pos::new(2, 1)).0, "**a**\n\n```\nx\n```\n\n**b**\n");
}

#[test]
fn code_language_box_sets_the_info_string() {
    let mut doc = parse("```\nx\n```\n\n    indented\n");
    assert!(edit::set_info(&mut doc, 0, "rust"));
    assert!(!edit::set_info(&mut doc, 0, " rust "));
    // An indented block becomes a fence; a backtick needs tildes.
    assert!(edit::set_info(&mut doc, 1, "a`b"));
    assert_eq!(serialize(&doc), "```rust\nx\n```\n\n~~~a`b\nindented\n~~~\n");
}

#[test]
fn a_paragraph_of_one_image_shows_as_the_image() {
    let lone = |src: &str| {
        let mut refs = inline::Refs::new();
        refs.insert("r".into(), inline::LinkRef { dest: "ref.png".into(), title: String::new() });
        let nodes = inline::parse(src, &refs, &inline::Options::default());
        inline::lone_image(src, &nodes).map(|n| match &n.kind {
            inline::Kind::Image { dest, .. } => dest.clone(),
            _ => unreachable!(),
        })
    };
    assert_eq!(lone("![a](x.png)").as_deref(), Some("x.png"));
    assert_eq!(lone(" ![a *b*](<my x.png> \"t\") ").as_deref(), Some("my x.png"));
    assert_eq!(lone("![a][r]").as_deref(), Some("ref.png"));
    assert_eq!(lone("see ![a](x.png)"), None);
    assert_eq!(lone("![a](x.png)![b](y.png)"), None);
    assert_eq!(lone("[![a](x.png)](https://e.com)"), None);
    assert_eq!(lone("![a](x.png"), None);
}

#[test]
fn emoji_suggestions() {
    let names = |t: &str| query::emoji_suggestions(t).into_iter().map(|p| p.0).collect::<Vec<_>>();
    assert!(names("").is_empty());
    // Shortcodes starting with the token come first, in order, then those
    // with a later word starting with it.
    let smi = names("smi");
    assert_eq!(smi[0], "smile");
    assert!(smi.iter().take_while(|n| n.starts_with("smi")).count() > 3);
    let first_word = smi.iter().position(|n| !n.starts_with("smi")).unwrap();
    assert!(smi[first_word..].iter().all(|n| n.contains("_smi") || n.contains("-smi")));
    assert!(smi.contains(&"slightly_smiling_face"));
    let mut sorted = smi[..first_word].to_vec();
    sorted.sort();
    assert_eq!(sorted, smi[..first_word]);
    // An exact match leads; the emoji comes with it.
    assert_eq!(query::emoji_suggestions("tada")[0], ("tada", "🎉"));
    assert_eq!(names("+1")[0], "+1");
    // Case only matters for the first group.
    assert!(names("Smile").contains(&"smile"));
    assert!(names("zzzz").is_empty());
}

#[test]
fn math_is_drawn_when_copied_as_svg() {
    let doc = parse("$x$\n\n$$\ny\n$$\n");
    let draw = |tex: &str, display: bool| Some(format!("<svg>{tex}|{display}</svg>"));
    let out = html::render_math(&doc, &inline::Options::all(), Some(&draw));
    assert!(out.contains("<span class=\"math math-inline\"><svg>x|false</svg></span>"), "{out}");
    assert!(out.contains("|true</svg></span>"), "{out}");
    assert!(html::render(&doc, &inline::Options::all()).contains("<span class=\"math math-inline\">x</span>"));
}
