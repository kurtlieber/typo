//! The document model: a flat list of leaf blocks, each carrying the path of
//! containers (block quotes, list items, footnote definitions) it sits in.
//! See docs/architecture.md for why the tree is flattened this way.

use std::sync::atomic::{AtomicU64, Ordering};

pub type Id = u64;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// A process-unique id for blocks and container instances.
pub fn next_id() -> Id {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListMarker {
    /// `-`, `*` or `+`.
    Bullet(char),
    /// `1.` or `1)`. `start` is the number of the list's first item; when
    /// `same_numbers` is set every item repeats it (`1. 1. 1.`).
    Ordered { start: u64, delim: char, same_numbers: bool },
}

impl ListMarker {
    /// The marker as written for the item at `ordinal` (0-based) in its list.
    pub fn text(&self, ordinal: usize) -> String {
        match self {
            ListMarker::Bullet(c) => c.to_string(),
            ListMarker::Ordered { start, delim, same_numbers } => {
                let n = if *same_numbers { *start } else { start + ordinal as u64 };
                format!("{n}{delim}")
            }
        }
    }

    pub fn is_ordered(&self) -> bool {
        matches!(self, ListMarker::Ordered { .. })
    }

    /// Two markers continue the same list when they are the same kind of
    /// bullet or the same ordered delimiter.
    pub fn compatible(&self, other: &ListMarker) -> bool {
        match (self, other) {
            (ListMarker::Bullet(a), ListMarker::Bullet(b)) => a == b,
            (ListMarker::Ordered { delim: a, .. }, ListMarker::Ordered { delim: b, .. }) => a == b,
            _ => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListItem {
    /// Identity of the list this item belongs to; items of one list share it.
    pub list: Id,
    pub marker: ListMarker,
    /// Columns before the marker, relative to the enclosing content, as written.
    pub indent: usize,
    /// Spaces between the marker and the content, as written.
    pub padding: usize,
    /// `Some(checked)` for task list items.
    pub task: Option<bool>,
    /// Whether the list is loose (blank lines between its items or between
    /// blocks directly inside an item).
    pub loose: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContainerKind {
    /// `padding` spaces after the `>`, one unless pretty indentation made
    /// it more.
    Quote { padding: usize },
    ListItem(ListItem),
    Footnote { label: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Container {
    pub id: Id,
    pub kind: ContainerKind,
}

impl Container {
    pub fn new(kind: ContainerKind) -> Self {
        Container { id: next_id(), kind }
    }

    /// A new quote, `> ` before its lines.
    pub fn quote() -> Self {
        Container::new(ContainerKind::Quote { padding: 1 })
    }

    pub fn list_item(&self) -> Option<&ListItem> {
        match &self.kind {
            ContainerKind::ListItem(item) => Some(item),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    None,
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    /// `setext` holds the underline as written for setext-style headings;
    /// an empty one is drawn as wide as the text on save (Typora's
    /// variable-width setext). `closed` writes an ATX heading with a
    /// closing sequence (`# Title #`).
    Heading { level: u8, setext: Option<String>, closed: bool },
    /// A fenced code block; `fence` is the opening fence (``` or ~~~, any length).
    CodeFence { info: String, fence: String, closed: bool },
    IndentedCode,
    /// A display-math block between `$$` lines, or with `brackets`
    /// between `\[` and `\]` lines, LaTeX's own delimiters, which Typora
    /// also reads.
    Math { brackets: bool },
    Html,
    /// Cells are inline markdown; `rows[0]` is the header row.
    Table { align: Vec<Align>, rows: Vec<Vec<String>> },
    ThematicBreak { marker: String },
    FrontMatter,
    /// `[toc]`; `text` keeps the marker as written (`[TOC]`).
    Toc,
    /// A `[label]: destination "title"` line; `text` holds it verbatim.
    LinkRefDef,
}

impl BlockKind {
    /// Kinds whose `text` is inline markdown edited with live preview.
    pub fn is_inline_text(&self) -> bool {
        matches!(self, BlockKind::Paragraph | BlockKind::Heading { .. })
    }

    pub fn name(&self) -> &'static str {
        match self {
            BlockKind::Paragraph => "paragraph",
            BlockKind::Heading { .. } => "heading",
            BlockKind::CodeFence { .. } | BlockKind::IndentedCode => "code",
            BlockKind::Math { .. } => "math",
            BlockKind::Html => "html",
            BlockKind::Table { .. } => "table",
            BlockKind::ThematicBreak { .. } => "hr",
            BlockKind::FrontMatter => "frontmatter",
            BlockKind::Toc => "toc",
            BlockKind::LinkRefDef => "linkref",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub id: Id,
    pub containers: Vec<Container>,
    pub kind: BlockKind,
    /// Inline markdown for paragraphs and headings; raw content (without
    /// fences or container prefixes) for code, math, html and front matter.
    pub text: String,
    /// The blank lines before this block in the source, each given as how
    /// many of the previous block's containers it sat inside (a blank line
    /// inside a quote is written `>`). `None` for blocks created while
    /// editing, which get the default spacing.
    pub blank_before: Option<Vec<usize>>,
    /// The block's source lines (container prefixes stripped) as parsed.
    /// Serializing an untouched block reproduces them exactly; any edit to
    /// kind or text must clear this (use the setters).
    pub raw: Option<Vec<String>>,
    /// Lazy continuation lines in `raw` (written without some of their
    /// container prefixes), as `(line index, containers matched)`.
    pub lazy: Vec<(usize, usize)>,
}

impl Block {
    pub fn new(kind: BlockKind, text: impl Into<String>, containers: Vec<Container>) -> Self {
        Block { id: next_id(), containers, kind, text: text.into(), blank_before: None, raw: None, lazy: Vec::new() }
    }

    pub fn paragraph(text: impl Into<String>) -> Self {
        Block::new(BlockKind::Paragraph, text, Vec::new())
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if text != self.text {
            self.text = text;
            self.clear_raw();
        }
    }

    pub fn set_kind(&mut self, kind: BlockKind) {
        if kind != self.kind {
            self.kind = kind;
            self.clear_raw();
        }
    }

    /// Forget the source lines so the block is generated from the model.
    pub fn clear_raw(&mut self) {
        self.raw = None;
        self.lazy.clear();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
    pub line_ending: LineEnding,
    /// Whether the source ended with a newline.
    pub final_newline: bool,
    /// Blank lines after the last block, as in `Block::blank_before`.
    pub blank_after: Vec<usize>,
}

impl Default for Document {
    fn default() -> Self {
        Document { blocks: vec![Block::paragraph("")], line_ending: LineEnding::Lf, final_newline: true, blank_after: Vec::new() }
    }
}

/// Depth of the longest common prefix of container ids between two blocks.
pub fn common_depth(a: &Block, b: &Block) -> usize {
    a.containers.iter().zip(&b.containers).take_while(|(x, y)| x.id == y.id).count()
}
