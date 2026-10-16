//! The document as a tree for export: the flat blocks, each with the
//! quotes, list items and footnotes it sits in, nested into those
//! containers, as HTML and Typst both write them. Footnote definitions
//! leave the flow and are listed apart, numbered in order of first use, as
//! Typora puts them all at the end.

use std::collections::HashMap;

use crate::md::inline::{Options, Refs};
use crate::md::query::{self, footnote_key};
use crate::md::{Block, BlockKind, ContainerKind, Document, ListMarker};

#[derive(Debug)]
pub enum Node<'a> {
    /// Block `index` of the document, without the first `skip` bytes of
    /// its text (an alert's `[!NOTE]` line).
    Block { index: usize, block: &'a Block, skip: usize },
    /// A quote, or a GitHub alert when `alert` names its kind.
    Quote { alert: Option<&'static str>, children: Vec<Node<'a>> },
    List(List<'a>),
}

#[derive(Debug)]
pub struct List<'a> {
    /// The first item's marker, as written.
    pub marker: ListMarker,
    /// The first number of an ordered list; `None` for bullets.
    pub start: Option<u64>,
    /// Loose lists keep their paragraphs apart; tight ones write them bare.
    pub loose: bool,
    pub items: Vec<Item<'a>>,
}

#[derive(Debug)]
pub struct Item<'a> {
    /// `Some(checked)` for a task.
    pub task: Option<bool>,
    pub children: Vec<Node<'a>>,
}

#[derive(Debug)]
pub struct Footnote<'a> {
    pub label: String,
    pub number: usize,
    pub children: Vec<Node<'a>>,
}

#[derive(Debug)]
pub struct Tree<'a> {
    pub body: Vec<Node<'a>>,
    /// Defined footnotes, by number.
    pub footnotes: Vec<Footnote<'a>>,
    /// Each footnote's number, by `footnote_key`.
    pub numbers: HashMap<String, usize>,
    /// Each heading's anchor, by block index, as links to `#anchor` find it.
    pub anchors: HashMap<usize, String>,
}

impl Tree<'_> {
    /// The number of footnote `label`, when it is defined.
    pub fn footnote_number(&self, label: &str) -> Option<usize> {
        self.numbers.get(&footnote_key(label)).copied()
    }

    /// The anchor of the heading in block `index`.
    pub fn anchor(&self, index: usize) -> Option<&str> {
        self.anchors.get(&index).map(String::as_str)
    }
}

pub fn build<'a>(doc: &'a Document, refs: &Refs, opts: &Options) -> Tree<'a> {
    let blocks: Vec<(usize, &Block)> = doc.blocks.iter().enumerate().collect();
    let mut defs: Vec<(String, Vec<Node<'a>>)> = Vec::new();
    let body = nodes(&blocks, 0, opts.alerts, &mut defs);

    // Numbers as Typora's export gives them: in order of first reference,
    // table cells included; a footnote's text is written at its first
    // reference, so the references in it are numbered then. Footnotes
    // never referred to from the body are left out.
    let notes = query::footnotes(doc, refs, opts);
    let mut all: Vec<(&query::NoteRef, String)> = notes.iter().flat_map(|f| f.refs.iter().map(move |r| (r, footnote_key(&f.label)))).collect();
    all.sort_by_key(|(r, _)| (r.block, r.cell, r.range.start));
    fn number(key: &str, all: &[(&query::NoteRef, String)], notes: &[query::Footnote], numbers: &mut HashMap<String, usize>) {
        let Some((first, last)) = notes.iter().find(|f| footnote_key(&f.label) == key).and_then(|f| f.def) else { return };
        if numbers.contains_key(key) {
            return;
        }
        numbers.insert(key.to_string(), numbers.len() + 1);
        for (_, k) in all.iter().filter(|(r, _)| (first..=last).contains(&r.block)) {
            number(k, all, notes, numbers);
        }
    }
    let mut numbers = HashMap::new();
    let in_note = |b: usize| doc.blocks[b].containers.iter().any(|c| matches!(c.kind, ContainerKind::Footnote { .. }));
    for (_, k) in all.iter().filter(|(r, _)| !in_note(r.block)) {
        number(k, &all, &notes, &mut numbers);
    }
    let mut footnotes: Vec<Footnote> = Vec::new();
    for (label, children) in defs {
        let key = footnote_key(&label);
        // The first definition counts, as in GFM.
        if footnotes.iter().any(|f| footnote_key(&f.label) == key) {
            continue;
        }
        let Some(&number) = numbers.get(&key) else { continue };
        footnotes.push(Footnote { label, number, children });
    }
    footnotes.sort_by_key(|f| f.number);

    let mut anchors = HashMap::new();
    let mut seen = HashMap::<String, usize>::new();
    for h in query::outline(doc, refs, opts) {
        let base = query::slug(&h.title);
        let n = seen.entry(base.clone()).or_insert(0);
        anchors.insert(h.block, if *n == 0 { base } else { format!("{base}-{n}") });
        *n += 1;
    }
    Tree { body, footnotes, numbers, anchors }
}

/// Reference `k` (from 0) to footnote `number`, as Typora's export names
/// its anchors (`ref-footnote-ID` at a reference, `dfref-footnote-ID` at
/// its ↩): "1", "1-1", "1-2"…
pub fn ref_id(number: usize, k: usize) -> String {
    if k == 0 { number.to_string() } else { format!("{number}-{k}") }
}

/// The nodes for `blocks`, all of which share their first `depth`
/// containers.
fn nodes<'a>(blocks: &[(usize, &'a Block)], depth: usize, alerts: bool, defs: &mut Vec<(String, Vec<Node<'a>>)>) -> Vec<Node<'a>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < blocks.len() {
        let (index, block) = blocks[i];
        let Some(c) = block.containers.get(depth) else {
            out.push(Node::Block { index, block, skip: 0 });
            i += 1;
            continue;
        };
        // The run of blocks inside this container.
        let run = |from: usize, id| from + blocks[from..].iter().take_while(|(_, b)| b.containers.get(depth).is_some_and(|d| d.id == id)).count();
        match &c.kind {
            ContainerKind::Quote { .. } => {
                let end = run(i, c.id);
                let mut children = nodes(&blocks[i..end], depth + 1, alerts, defs);
                let alert = if alerts { alert_of(&mut children, block, depth) } else { None };
                out.push(Node::Quote { alert, children });
                i = end;
            }
            ContainerKind::Footnote { label } => {
                let end = run(i, c.id);
                let children = nodes(&blocks[i..end], depth + 1, alerts, defs);
                defs.push((label.clone(), children));
                i = end;
            }
            ContainerKind::ListItem(first) => {
                let mut list = List {
                    marker: first.marker.clone(),
                    start: match first.marker {
                        ListMarker::Ordered { start, .. } => Some(start),
                        ListMarker::Bullet(_) => None,
                    },
                    loose: false,
                    items: Vec::new(),
                };
                // Items of the same list follow one another.
                while let Some(item) = blocks.get(i).and_then(|(_, b)| b.containers.get(depth)).and_then(|c| c.list_item().map(|it| (c.id, it))) {
                    let (id, it) = item;
                    if it.list != first.list {
                        break;
                    }
                    let end = run(i, id);
                    list.loose |= it.loose;
                    list.items.push(Item { task: it.task, children: nodes(&blocks[i..end], depth + 1, alerts, defs) });
                    i = end;
                }
                out.push(Node::List(list));
            }
        }
    }
    out
}

/// A quote whose first paragraph opens with `[!NOTE]` on its own line is
/// an alert: that line is dropped from what it shows.
fn alert_of(children: &mut [Node], first: &Block, depth: usize) -> Option<&'static str> {
    if first.containers.len() != depth + 1 || first.kind != BlockKind::Paragraph {
        return None;
    }
    let (kind, len) = query::alert_of(&first.text)?;
    if let Some(Node::Block { skip, .. }) = children.first_mut() {
        *skip = (len + 1).min(first.text.len());
    }
    Some(kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::{inline, parse};

    fn tree(src: &str) -> String {
        let doc = parse(src);
        let refs = inline::refs_of(&doc);
        let t = build(&doc, &refs, &Options::all());
        fn show(nodes: &[Node], out: &mut String) {
            for n in nodes {
                match n {
                    Node::Block { block, skip, .. } => out.push_str(&format!("[{}]", &block.text[*skip..])),
                    Node::Quote { alert, children } => {
                        out.push_str(&format!("q{}(", alert.map(|a| format!(":{a}")).unwrap_or_default()));
                        show(children, out);
                        out.push(')');
                    }
                    Node::List(l) => {
                        out.push_str(&format!("{}{}(", l.start.map_or("ul".into(), |s| format!("ol{s}")), if l.loose { "~" } else { "" }));
                        for it in &l.items {
                            out.push_str(match it.task {
                                Some(true) => "x(",
                                Some(false) => "o(",
                                None => "li(",
                            });
                            show(&it.children, out);
                            out.push(')');
                        }
                        out.push(')');
                    }
                }
            }
        }
        let mut out = String::new();
        show(&t.body, &mut out);
        for f in &t.footnotes {
            out.push_str(&format!(" fn{}:{}=", f.number, f.label));
            show(&f.children, &mut out);
        }
        out
    }

    #[test]
    fn nests_containers() {
        assert_eq!(tree("# T\n\n> a\n>\n> - b\n> - c\n\nd\n"), "[T]q([a]ul(li([b])li([c])))[d]");
        assert_eq!(tree("3. a\n4. b\n\n   c\n"), "ol3~(li([a])li([b][c]))");
        assert_eq!(tree("- [x] a\n- [ ] b\n"), "ul(x([a])o([b]))");
        // Two lists side by side stay two.
        assert_eq!(tree("- a\n+ b\n"), "ul(li([a]))ul(li([b]))");
        assert_eq!(tree("- a\n  - b\n"), "ul(li([a]ul(li([b]))))");
    }

    #[test]
    fn finds_alerts() {
        assert_eq!(tree("> [!TIP]\n> Use it.\n"), "q:tip([Use it.])");
        assert_eq!(tree("> [!TIP]\n\n> plain\n"), "q:tip([])q([plain])");
        assert_eq!(tree("> a\n> [!TIP]\n"), "q([a\n[!TIP]])");
    }

    #[test]
    fn numbers_footnotes_by_first_use() {
        assert_eq!(tree("a[^z] b[^y]\n\n[^y]: Y.\n[^z]: Z.\n"), "[a[^z] b[^y]] fn1:z=[Z.] fn2:y=[Y.]");
        // Undefined references get no number; unused definitions are left out.
        assert_eq!(tree("a[^q]\n\n[^u]: U.\n"), "[a[^q]]");
        // A footnote's own references are numbered at its first reference.
        assert_eq!(
            tree("a[^x] b[^y]\n\n[^y]: Y.\n\n[^x]: X[^z].\n\n[^z]: Z.\n"),
            "[a[^x] b[^y]] fn1:x=[X[^z].] fn2:z=[Z.] fn3:y=[Y.]"
        );
        // References in table cells count where the table is.
        assert!(tree("| a[^t] |\n| --- |\n\nb[^s]\n\n[^s]: S.\n\n[^t]: T.\n").ends_with(" fn1:t=[T.] fn2:s=[S.]"));
    }

    #[test]
    fn anchors_headings() {
        let doc = parse("# Hello World\n\n## Hello World\n\ntext\n");
        let refs = inline::refs_of(&doc);
        let t = build(&doc, &refs, &Options::all());
        assert_eq!(t.anchor(0), Some("hello-world"));
        assert_eq!(t.anchor(1), Some("hello-world-1"));
        assert_eq!(t.anchor(2), None);
    }
}
