//! Structural edits, as the editor performs them: Enter, Backspace and Delete
//! at block edges, deleting a selection across blocks, block conversions,
//! list indentation, input rules, paste and inline style toggles.
//!
//! Edits leave untouched blocks alone, so they keep their exact source
//! formatting. Touched blocks lose their raw lines (through the setters) and
//! are written fresh by the serializer. Blocks whose container path changes
//! lose their recorded spacing, as does the block after them, so the
//! serializer applies its defaults there.

use std::ops::Range;
use std::sync::LazyLock;

use regex::Regex;

use super::imgtag;
use super::inline::{self, Kind, Node, Options, Refs};
use super::model::*;
use super::parse::{fence_start, is_thematic_break, math_start, split_row};

/// A caret position: a block index and a byte offset into its text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub block: usize,
    pub offset: usize,
}

impl Pos {
    pub fn new(block: usize, offset: usize) -> Self {
        Pos { block, offset }
    }
}

/// The bullet Typora writes for new lists.
pub const BULLET: char = '-';

/// How the blocks typo makes (from the menus, or by typing their markdown)
/// are written, from Preferences ▸ Editor and ▸ Markdown; blocks read from
/// a file keep what they were written with.
#[derive(Clone, Debug, PartialEq)]
pub struct Syntax {
    /// The bullet of new lists: `-`, `+` or `*`.
    pub bullet: char,
    /// New ordered lists number every item alike (`1. 1. 1.`).
    pub ordered_same: bool,
    /// Indent Size: the columns nested blocks are indented by, 2 to 5.
    pub indent: usize,
    /// Pretty indentation: the marker of a list or quote padded so its
    /// text starts at the indent size.
    pub pretty: bool,
    pub heading: HeadingStyle,
}

/// Preferences ▸ Markdown ▸ Heading Style. Setext applies to levels 1 and
/// 2 outside lists, as Typora does; other headings stay ATX.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadingStyle {
    Atx,
    /// `# Title #`.
    AtxClosed,
    /// `Title` over `===` or `---`.
    Setext,
    /// The underline as wide as the title.
    SetextWide,
}

impl Default for Syntax {
    fn default() -> Self {
        Syntax { bullet: BULLET, ordered_same: false, indent: 2, pretty: false, heading: HeadingStyle::Atx }
    }
}

impl Syntax {
    /// Spaces after a new item's marker, as Typora pads them.
    fn padding(&self, marker: &ListMarker) -> usize {
        let n = self.indent.max(2);
        match (self.pretty, marker) {
            (false, _) => 1,
            (true, ListMarker::Ordered { .. }) => (n - 2).clamp(1, 4),
            (true, ListMarker::Bullet(_)) => (n - 1).min(4),
        }
    }

    fn quote_padding(&self) -> usize {
        if self.pretty { (self.indent.max(2) - 1).min(4) } else { 1 }
    }

    /// The kind of a new heading of `level`, in a list item or not.
    fn heading(&self, level: u8, in_item: bool) -> BlockKind {
        let level = level.min(6);
        let setext = level <= 2 && !in_item;
        match self.heading {
            HeadingStyle::Setext if setext => {
                BlockKind::Heading { level, setext: Some((if level == 1 { "===" } else { "---" }).into()), closed: false }
            }
            HeadingStyle::SetextWide if setext => BlockKind::Heading { level, setext: Some(String::new()), closed: false },
            HeadingStyle::AtxClosed => BlockKind::Heading { level, setext: None, closed: true },
            _ => BlockKind::Heading { level, setext: None, closed: false },
        }
    }
}

thread_local! {
    // The window's preferences. All editing happens on the UI thread, and
    // each test thread keeps the defaults unless it sets its own.
    static SYNTAX: std::cell::RefCell<Syntax> = std::cell::RefCell::new(Syntax::default());
}

/// Use `syntax` for the blocks made from now on.
pub fn set_syntax(syntax: Syntax) {
    SYNTAX.with(|s| *s.borrow_mut() = syntax);
}

thread_local! {
    static REMAP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Read full-width punctuation as ASCII: `》 ` typed makes a quote, as
/// Remap Unicode Punctuation on Parse has it (see `parse::parse_with`).
pub fn set_remap(on: bool) {
    REMAP.with(|r| r.set(on));
}

fn remap() -> bool {
    REMAP.with(|r| r.get())
}

fn syntax() -> Syntax {
    SYNTAX.with(|s| s.borrow().clone())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListKind {
    Bullet,
    Ordered,
    Task,
}

// ---------------------------------------------------------------------------
// Container helpers. A container sits at the same depth in every block it
// holds, and its blocks are contiguous.

fn id_at(b: &Block, depth: usize) -> Option<Id> {
    b.containers.get(depth).map(|c| c.id)
}

/// The blocks inside the container at `depth` of block `i`.
fn span(doc: &Document, i: usize, depth: usize) -> Range<usize> {
    let id = id_at(&doc.blocks[i], depth);
    let mut start = i;
    while start > 0 && id_at(&doc.blocks[start - 1], depth) == id {
        start -= 1;
    }
    let mut end = i + 1;
    while end < doc.blocks.len() && id_at(&doc.blocks[end], depth) == id {
        end += 1;
    }
    start..end
}

fn first_in(doc: &Document, i: usize, depth: usize) -> bool {
    span(doc, i, depth).start == i
}

/// Grow `range` until it covers whole containers at `depth`.
fn whole_containers(doc: &Document, mut range: Range<usize>, depth: usize) -> Range<usize> {
    if doc.blocks[range.start].containers.len() > depth {
        range.start = span(doc, range.start, depth).start;
    }
    if doc.blocks[range.end - 1].containers.len() > depth {
        range.end = span(doc, range.end - 1, depth).end;
    }
    range
}

/// The depth all blocks in `range` share.
fn common_depth_of(doc: &Document, range: Range<usize>) -> usize {
    let first = &doc.blocks[range.start];
    range.map(|j| common_depth(first, &doc.blocks[j])).min().unwrap_or(0)
}

/// Note that the blocks in `range` changed their container path.
fn moved(doc: &mut Document, range: Range<usize>) {
    let end = range.end;
    for b in &mut doc.blocks[range] {
        b.lazy.clear();
        b.blank_before = None;
    }
    if let Some(b) = doc.blocks.get_mut(end) {
        b.blank_before = None;
    }
}

fn new_list_marker(kind: ListKind) -> ListMarker {
    let syntax = syntax();
    match kind {
        ListKind::Ordered => ListMarker::Ordered { start: 1, delim: '.', same_numbers: syntax.ordered_same },
        _ => ListMarker::Bullet(syntax.bullet),
    }
}

fn new_item(list: Id, marker: ListMarker, task: Option<bool>) -> Container {
    let padding = syntax().padding(&marker);
    Container::new(ContainerKind::ListItem(ListItem { list, marker, indent: 0, padding, task, loose: false }))
}

/// Blocks with no text to put a caret in: they are selected and deleted whole.
pub fn is_atomic(kind: &BlockKind) -> bool {
    matches!(kind, BlockKind::Table { .. } | BlockKind::ThematicBreak { .. } | BlockKind::Toc)
}

fn ensure_block(doc: &mut Document) {
    if doc.blocks.is_empty() {
        doc.blocks.push(Block::paragraph(""));
    }
}

fn floor_char(s: &str, mut i: usize) -> usize {
    i = i.min(s.len());
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

// ---------------------------------------------------------------------------
// Enter, Backspace, Delete.

/// Enter at `pos` in a paragraph or heading.
pub fn split(doc: &mut Document, pos: Pos) -> Pos {
    let i = pos.block;
    let b = &doc.blocks[i];
    if !b.kind.is_inline_text() {
        return pos;
    }
    let depth = b.containers.len();
    let paragraph = matches!(b.kind, BlockKind::Paragraph);
    if paragraph && b.text.is_empty() && depth > 0 {
        match b.containers[depth - 1].kind {
            // Enter on an empty item ends the list, or outdents a nested one.
            ContainerKind::ListItem(_) if span(doc, i, depth - 1).len() == 1 => {
                return outdent(doc, i).unwrap_or(pos);
            }
            // Enter on an empty last line of a quote leaves the quote.
            ContainerKind::Quote { .. } if span(doc, i, depth - 1).end == i + 1 => {
                doc.blocks[i].containers.pop();
                moved(doc, i..i + 1);
                return Pos::new(i, 0);
            }
            _ => {}
        }
    }
    let offset = floor_char(&b.text, pos.offset);
    let containers = b.containers.clone();
    if !paragraph && offset == 0 && !b.text.is_empty() {
        // At the start of a heading: a new paragraph above it.
        doc.blocks.insert(i, Block::new(BlockKind::Paragraph, "", containers));
        moved(doc, i..i + 1);
        return Pos::new(i + 1, 0);
    }
    // Spaces before the caret stay, so Backspace can join the halves back
    // as they were; spaces after it would only indent the new paragraph.
    let left = b.text[..offset].to_string();
    let right = b.text[offset..].trim_start().to_string();
    let mut new = Block::new(BlockKind::Paragraph, right, containers);
    if paragraph && depth > 0 {
        if let ContainerKind::ListItem(item) = &doc.blocks[i].containers[depth - 1].kind {
            // Enter in a list item starts the next item, which takes the
            // rest of this one.
            let old = doc.blocks[i].containers[depth - 1].id;
            let next = Container::new(ContainerKind::ListItem(ListItem { task: item.task.map(|_| false), ..item.clone() }));
            new.containers[depth - 1] = next.clone();
            let mut j = i + 1;
            while j < doc.blocks.len() && id_at(&doc.blocks[j], depth - 1) == Some(old) {
                doc.blocks[j].containers[depth - 1] = next.clone();
                j += 1;
            }
            moved(doc, i + 1..j);
        }
    }
    doc.blocks[i].set_text(left);
    doc.blocks.insert(i + 1, new);
    if let Some(b) = doc.blocks.get_mut(i + 2) {
        b.blank_before = None;
    }
    Pos::new(i + 1, 0)
}

/// Backspace with the caret at the start of block `i`.
pub fn backspace(doc: &mut Document, i: usize) -> Pos {
    let b = &doc.blocks[i];
    if let BlockKind::Heading { .. } | BlockKind::LinkRefDef = b.kind {
        doc.blocks[i].set_kind(BlockKind::Paragraph);
        return Pos::new(i, 0);
    }
    if !b.kind.is_inline_text() {
        return Pos::new(i, 0);
    }
    let depth = b.containers.len();
    if depth > 0 && first_in(doc, i, depth - 1) {
        match &b.containers[depth - 1].kind {
            ContainerKind::ListItem(_) => {
                unwrap_item(doc, i, depth - 1);
                return Pos::new(i, 0);
            }
            ContainerKind::Quote { .. } => {
                doc.blocks[i].containers.pop();
                moved(doc, i..i + 1);
                return Pos::new(i, 0);
            }
            ContainerKind::Footnote { .. } => return Pos::new(i, 0),
        }
    }
    if i == 0 {
        return Pos::new(0, 0);
    }
    let prev = &doc.blocks[i - 1];
    if prev.kind.is_inline_text() {
        return join(doc, i - 1);
    }
    // Into a table, the caret goes to the end of its last cell.
    let end = if is_atomic(&prev.kind) { 1 } else { prev.text.len() };
    if matches!(prev.kind, BlockKind::ThematicBreak { .. } | BlockKind::Toc) {
        doc.blocks.remove(i - 1);
        moved(doc, i - 1..i - 1);
        return Pos::new(i - 1, 0);
    }
    if doc.blocks[i].text.is_empty() {
        doc.blocks.remove(i);
        moved(doc, i..i);
    }
    Pos::new(i - 1, end)
}

/// Delete with the caret at the end of block `i`.
pub fn delete_forward(doc: &mut Document, i: usize) -> Pos {
    let end = Pos::new(i, doc.blocks[i].text.len());
    if i + 1 >= doc.blocks.len() || !doc.blocks[i].kind.is_inline_text() {
        return end;
    }
    let next = &doc.blocks[i + 1];
    if next.kind.is_inline_text() {
        return join(doc, i);
    }
    if matches!(next.kind, BlockKind::ThematicBreak { .. } | BlockKind::Toc) {
        doc.blocks.remove(i + 1);
        moved(doc, i + 1..i + 1);
        return end;
    }
    if doc.blocks[i].text.is_empty() {
        doc.blocks.remove(i);
        moved(doc, i..i);
        return Pos::new(i, 0);
    }
    Pos::new(i + 1, 0)
}

/// Append block `i + 1`'s text to block `i` and remove it.
fn join(doc: &mut Document, i: usize) -> Pos {
    let next = doc.blocks.remove(i + 1);
    let b = &mut doc.blocks[i];
    let offset = b.text.len();
    let mut text = next.text;
    if let BlockKind::Heading { .. } = b.kind {
        text = text.replace('\n', " ");
    }
    let joined = format!("{}{}", b.text, text);
    b.set_text(joined);
    moved(doc, i + 1..i + 1);
    Pos::new(i, offset)
}

/// Take the list item at `depth` around block `i` (its first block) out of
/// the list: it joins the previous item, or leaves the list when it is the
/// first one.
fn unwrap_item(doc: &mut Document, i: usize, depth: usize) {
    let items = span(doc, i, depth);
    let prev = i.checked_sub(1).and_then(|p| {
        let c = doc.blocks[p].containers.get(depth)?;
        let ours = doc.blocks[i].containers[depth].list_item()?;
        (c.list_item()?.list == ours.list).then(|| c.clone())
    });
    for j in items.clone() {
        match &prev {
            Some(c) => doc.blocks[j].containers[depth] = c.clone(),
            None => {
                doc.blocks[j].containers.remove(depth);
            }
        }
    }
    moved(doc, items);
}

/// Delete the text between two positions, joining the blocks at either end.
pub fn delete_range(doc: &mut Document, start: Pos, end: Pos) -> Pos {
    let (start, end) = if end < start { (end, start) } else { (start, end) };
    let (a, z) = (start.block, end.block);
    // A block without text goes when it is selected (0..1).
    if a == z && is_atomic(&doc.blocks[a].kind) && start.offset != end.offset {
        return delete_block(doc, a);
    }
    if a == z {
        let b = &mut doc.blocks[a];
        let (s, e) = (floor_char(&b.text, start.offset), floor_char(&b.text, end.offset));
        let mut text = b.text.clone();
        text.replace_range(s..e, "");
        b.set_text(text);
        return Pos::new(a, s);
    }
    let left_end = floor_char(&doc.blocks[a].text, start.offset);
    let right_start = floor_char(&doc.blocks[z].text, end.offset);
    let left = doc.blocks[a].text[..left_end].to_string();
    let right = doc.blocks[z].text[right_start..].to_string();
    let joinable = doc.blocks[a].kind.is_inline_text() && doc.blocks[z].kind.is_inline_text();
    if joinable {
        let mut right = right;
        if let BlockKind::Heading { .. } = doc.blocks[a].kind {
            right = right.replace('\n', " ");
        }
        doc.blocks[a].set_text(left + &right);
        doc.blocks.drain(a + 1..=z);
        moved(doc, a + 1..a + 1);
        return Pos::new(a, left_end);
    }
    // Keep what is left of each end; atomic blocks go when touched.
    let drop_last = is_atomic(&doc.blocks[z].kind) && end.offset > 0;
    let drop_first = is_atomic(&doc.blocks[a].kind) || (start.offset == 0 && !doc.blocks[a].kind.is_inline_text());
    if !is_atomic(&doc.blocks[z].kind) {
        doc.blocks[z].set_text(right);
    }
    if !is_atomic(&doc.blocks[a].kind) {
        doc.blocks[a].set_text(left);
    }
    let last = if drop_last { z + 1 } else { z };
    let first = if drop_first { a } else { a + 1 };
    doc.blocks.drain(first..last);
    moved(doc, first..first);
    ensure_block(doc);
    if drop_first {
        Pos::new(a.min(doc.blocks.len() - 1), 0)
    } else {
        Pos::new(a, left_end)
    }
}

/// Edit ▸ Delete ▸ Paragraph / Block: remove block `i` whole.
/// Delete block `i`. The caret goes to the start of the next block or, at
/// the end of the document, to the end of the one before, as in Typora.
pub fn delete_block(doc: &mut Document, i: usize) -> Pos {
    doc.blocks.remove(i);
    moved(doc, i..i);
    ensure_block(doc);
    if i < doc.blocks.len() || i == 0 {
        return Pos::new(i.min(doc.blocks.len() - 1), 0);
    }
    let b = &doc.blocks[i - 1];
    Pos::new(i - 1, if is_atomic(&b.kind) { 1 } else { b.text.len() })
}

/// Typing over a selection: delete it, then insert `text` as it is.
pub fn replace_range(doc: &mut Document, start: Pos, end: Pos, text: &str) -> Pos {
    let at = delete_range(doc, start, end);
    let b = &mut doc.blocks[at.block];
    if is_atomic(&b.kind) {
        return at;
    }
    let offset = floor_char(&b.text, at.offset);
    let mut t = b.text.clone();
    t.insert_str(offset, text);
    b.set_text(t);
    Pos::new(at.block, offset + text.len())
}

/// The selection from `start` to `end` as a document of its own, for the
/// clipboard: the blocks it touches, cut to the selection, with their
/// containers. Atomic blocks come whole, the last one only when the
/// selection reaches past its start (as `delete_range` does). A span the
/// selection cuts through keeps its markers, so part of `**bold**` copies
/// as `**bo**`, as Typora copies it.
pub fn slice(doc: &Document, start: Pos, end: Pos, refs: &Refs, opts: &Options) -> Document {
    let (start, end) = if end < start { (end, start) } else { (start, end) };
    let mut blocks: Vec<Block> = doc.blocks[start.block..=end.block].to_vec();
    if blocks.len() > 1 && is_atomic(&blocks[blocks.len() - 1].kind) && end.offset == 0 {
        blocks.pop();
    }
    let n = blocks.len();
    let cut_end = end.block - start.block + 1 == n;
    for (k, b) in blocks.iter_mut().enumerate() {
        if is_atomic(&b.kind) || !(k == 0 || (k == n - 1 && cut_end)) {
            continue;
        }
        let s = if k == 0 { floor_char(&b.text, start.offset) } else { 0 };
        let e = if k == n - 1 && cut_end { floor_char(&b.text, end.offset).max(s) } else { b.text.len() };
        let t = if b.kind.is_inline_text() { cut_inline(&b.text, s..e, refs, opts) } else { b.text[s..e].to_string() };
        b.set_text(t);
    }
    blocks[0].blank_before = None;
    Document { blocks, line_ending: LineEnding::Lf, final_newline: false, blank_after: Vec::new() }
}

/// `text[range]` with the markers of any span it cuts through put back.
fn cut_inline(text: &str, range: Range<usize>, refs: &Refs, opts: &Options) -> String {
    fn cut(src: &str, nodes: &[Node], s: usize, e: usize, out: &mut String) {
        let mut at = s;
        for n in nodes {
            if n.range.end <= s || n.range.start >= e {
                continue;
            }
            if n.range.start > at {
                out.push_str(&src[at..n.range.start]);
            }
            let span = matches!(
                n.kind,
                Kind::Code
                    | Kind::Emph
                    | Kind::Strong
                    | Kind::Strike
                    | Kind::Highlight
                    | Kind::Sup
                    | Kind::Sub
                    | Kind::Math
                    | Kind::Link { .. }
                    | Kind::Tag { .. }
            );
            if s <= n.range.start && n.range.end <= e {
                out.push_str(&src[n.range.clone()]);
            } else if matches!(n.kind, Kind::Text) {
                out.push_str(&src[n.range.start.max(s)..n.range.end.min(e)]);
            } else if !span {
                // Anything else cut in two (an image, an entity) comes whole.
                out.push_str(&src[n.range.clone()]);
            } else {
                let cs = s.clamp(n.content.start, n.content.end);
                let ce = e.clamp(cs, n.content.end);
                if cs < ce {
                    out.push_str(&src[n.range.start..n.content.start]);
                    if n.children.is_empty() {
                        out.push_str(&src[cs..ce]);
                    } else {
                        cut(src, &n.children, cs, ce, out);
                    }
                    out.push_str(&src[n.content.end..n.range.end]);
                }
            }
            at = at.max(n.range.end);
        }
        if at < e {
            out.push_str(&src[at..e]);
        }
    }
    let nodes = inline::parse(text, refs, opts);
    let mut out = String::new();
    cut(text, &nodes, range.start, range.end, &mut out);
    out
}

// ---------------------------------------------------------------------------
// Block conversions.

/// Make block `i` a heading of `level`, or a paragraph for level 0.
pub fn set_heading(doc: &mut Document, i: usize, level: u8) {
    let b = &mut doc.blocks[i];
    if !b.kind.is_inline_text() {
        return;
    }
    if level == 0 {
        b.set_kind(BlockKind::Paragraph);
    } else {
        let text = b.text.replace('\n', " ");
        let in_item = b.containers.iter().any(|c| c.list_item().is_some());
        b.set_kind(syntax().heading(level, in_item));
        b.set_text(text);
    }
}

pub fn heading_level(b: &Block) -> u8 {
    match b.kind {
        BlockKind::Heading { level, .. } => level,
        _ => 0,
    }
}

/// Quote the blocks `a..=z`, or unquote them when they are in a quote.
pub fn toggle_quote(doc: &mut Document, a: usize, z: usize) {
    let quote = doc.blocks[a].containers.iter().rposition(|c| matches!(c.kind, ContainerKind::Quote { .. }));
    if let Some(d) = quote {
        let id = doc.blocks[a].containers[d].id;
        if (a..=z).all(|j| id_at(&doc.blocks[j], d) == Some(id)) {
            let whole = span(doc, a, d);
            for j in whole.clone() {
                doc.blocks[j].containers.remove(d);
            }
            moved(doc, whole);
            return;
        }
    }
    let depth = common_depth_of(doc, a..z + 1);
    let range = whole_containers(doc, a..z + 1, depth);
    let q = Container::new(ContainerKind::Quote { padding: syntax().quote_padding() });
    for j in range.clone() {
        doc.blocks[j].containers.insert(depth, q.clone());
    }
    moved(doc, range);
}

fn list_kind_of(item: &ListItem) -> ListKind {
    if item.task.is_some() {
        ListKind::Task
    } else if item.marker.is_ordered() {
        ListKind::Ordered
    } else {
        ListKind::Bullet
    }
}

/// Paragraph ▸ Ordered List / Unordered List / Task List on blocks `a..=z`:
/// make them a list, change the kind of the list they are in, or take them
/// out of it.
pub fn toggle_list(doc: &mut Document, a: usize, z: usize, kind: ListKind) {
    let item_depth = doc.blocks[a].containers.iter().rposition(|c| c.list_item().is_some());
    if let Some(d) = item_depth {
        let list = doc.blocks[a].containers[d].list_item().unwrap().clone();
        let same_list = |b: &Block| b.containers.get(d).and_then(Container::list_item).is_some_and(|i| i.list == list.list);
        if (a..=z).all(|j| same_list(&doc.blocks[j])) {
            if list_kind_of(&list) == kind {
                remove_items(doc, a, z, d);
            } else {
                convert_list(doc, list.list, d, kind);
            }
            return;
        }
    }
    let depth = common_depth_of(doc, a..z + 1);
    let range = whole_containers(doc, a..z + 1, depth);
    // Continue a list of the same kind right above.
    let above = range.start.checked_sub(1).and_then(|p| {
        let prev = &doc.blocks[p];
        let item = prev.containers.get(depth)?.list_item()?;
        let same_path = common_depth(prev, &doc.blocks[range.start]) >= depth;
        (same_path && list_kind_of(item) == kind).then(|| item.clone())
    });
    let (list, marker) = match &above {
        Some(item) => (item.list, item.marker.clone()),
        None => (next_id(), new_list_marker(kind)),
    };
    let task = (kind == ListKind::Task).then_some(false);
    let mut j = range.start;
    while j < range.end {
        // One item per block, or per container at this depth.
        let unit = if doc.blocks[j].containers.len() > depth { span(doc, j, depth) } else { j..j + 1 };
        let mut item = new_item(list, marker.clone(), task);
        if let (Some(above), ContainerKind::ListItem(i)) = (&above, &mut item.kind) {
            i.loose = above.loose;
        }
        for k in unit.clone() {
            doc.blocks[k].containers.insert(depth, item.clone());
        }
        j = unit.end;
    }
    moved(doc, range);
}

/// Take the items holding blocks `a..=z` out of their list at `depth`. Items
/// after them become a list of their own.
fn remove_items(doc: &mut Document, a: usize, z: usize, depth: usize) {
    let first = span(doc, a, depth).start;
    let last = span(doc, z, depth).end;
    let list = doc.blocks[a].containers[depth].list_item().unwrap().list;
    for j in first..last {
        doc.blocks[j].containers.remove(depth);
    }
    let rest = next_id();
    let mut j = last;
    while j < doc.blocks.len() {
        let Some(item) = doc.blocks[j].containers.get_mut(depth) else { break };
        let ContainerKind::ListItem(li) = &mut item.kind else { break };
        if li.list != list {
            break;
        }
        li.list = rest;
        j += 1;
    }
    moved(doc, first..last);
}

/// Give every item of `list` the marker of `kind`.
fn convert_list(doc: &mut Document, list: Id, depth: usize, kind: ListKind) {
    let marker = new_list_marker(kind);
    for b in &mut doc.blocks {
        let Some(c) = b.containers.get_mut(depth) else { continue };
        let ContainerKind::ListItem(item) = &mut c.kind else { continue };
        if item.list != list {
            continue;
        }
        item.marker = marker.clone();
        item.task = match kind {
            ListKind::Task => Some(item.task.unwrap_or(false)),
            _ => None,
        };
    }
}

/// Tab in a list item: make it a child of the item before it.
pub fn indent(doc: &mut Document, i: usize) -> bool {
    let Some(d) = doc.blocks[i].containers.iter().rposition(|c| c.list_item().is_some()) else { return false };
    let items = span(doc, i, d);
    let item = doc.blocks[i].containers[d].clone();
    let li = item.list_item().unwrap().clone();
    let Some(prev) = items.start.checked_sub(1) else { return false };
    let Some(parent) = doc.blocks[prev].containers.get(d).cloned() else { return false };
    if parent.list_item().is_none_or(|p| p.list != li.list) {
        return false;
    }
    // Join a list already nested at the end of the previous item.
    let nested = doc.blocks[prev].containers.get(d + 1).and_then(Container::list_item).cloned();
    let child = match nested {
        Some(n) => ListItem { list: n.list, marker: n.marker, loose: n.loose, indent: n.indent, ..li },
        None => {
            let marker = match li.marker {
                ListMarker::Ordered { delim, same_numbers, .. } => ListMarker::Ordered { start: 1, delim, same_numbers },
                m => m,
            };
            // Nested as deep as the indent size, when that is past the
            // parent's text (Typora's max(marker width, indent size)).
            let p = parent.list_item().unwrap();
            let width = p.marker.text(0).len() + p.padding.max(1);
            let indent = syntax().indent.saturating_sub(width);
            ListItem { list: next_id(), marker, loose: false, indent, ..li }
        }
    };
    let child = Container { id: item.id, kind: ContainerKind::ListItem(child) };
    for j in items.clone() {
        let b = &mut doc.blocks[j];
        b.containers[d] = parent.clone();
        b.containers.insert(d + 1, child.clone());
    }
    moved(doc, items);
    true
}

/// Shift+Tab in a list item: move it out one level. A top-level item
/// becomes plain blocks. Returns the caret, unchanged in place.
pub fn outdent(doc: &mut Document, i: usize) -> Option<Pos> {
    let d = doc.blocks[i].containers.iter().rposition(|c| c.list_item().is_some())?;
    let items = span(doc, i, d);
    let item = doc.blocks[i].containers[d].clone();
    let parent = d.checked_sub(1).and_then(|p| doc.blocks[i].containers.get(p)).filter(|c| c.list_item().is_some()).cloned();
    match parent {
        Some(parent) => {
            // The item becomes the parent's next sibling, and what followed it
            // inside the parent now belongs to it.
            let p = parent.list_item().unwrap();
            let lifted = Container {
                id: item.id,
                kind: ContainerKind::ListItem(ListItem { list: p.list, marker: p.marker.clone(), loose: p.loose, indent: p.indent, ..item.list_item().unwrap().clone() }),
            };
            let parent_end = span(doc, i, d - 1).end;
            for j in items.clone() {
                let b = &mut doc.blocks[j];
                b.containers.remove(d - 1);
                b.containers[d - 1] = lifted.clone();
            }
            for j in items.end..parent_end {
                doc.blocks[j].containers[d - 1] = lifted.clone();
            }
            moved(doc, items.start..parent_end);
        }
        None => {
            let list = item.list_item().unwrap().list;
            for j in items.clone() {
                doc.blocks[j].containers.remove(d);
            }
            // Items after it form a list of their own.
            let rest = next_id();
            let mut j = items.end;
            while let Some(ContainerKind::ListItem(li)) = doc.blocks.get_mut(j).and_then(|b| b.containers.get_mut(d)).map(|c| &mut c.kind) {
                if li.list != list {
                    break;
                }
                li.list = rest;
                j += 1;
            }
            moved(doc, items);
        }
    }
    Some(Pos::new(i, 0))
}

/// The first block of each list item a selection over blocks
/// `first..=last` acts on: the one holding `first` and every one starting
/// after it, less those inside another of them (they move with it).
fn items_in(doc: &Document, first: usize, last: usize) -> Vec<usize> {
    let innermost = |j: usize| {
        let cs = &doc.blocks[j].containers;
        cs.iter().rposition(|c| c.list_item().is_some()).map(|d| (d, cs[d].id))
    };
    let id_at = |j: usize, d: usize| doc.blocks[j].containers.get(d).map(|c| c.id);
    let mut items: Vec<(usize, usize, Id)> = Vec::new();
    for j in first..=last {
        let Some((d, id)) = innermost(j) else { continue };
        let starts = j == 0 || id_at(j - 1, d) != Some(id);
        if j == first || starts {
            let mut k = j;
            while k > 0 && id_at(k - 1, d) == Some(id) {
                k -= 1;
            }
            items.push((k, d, id));
        }
    }
    items
        .iter()
        .filter(|&&(k, d, _)| !doc.blocks[k].containers[..d].iter().any(|c| items.iter().any(|&(_, _, id)| id == c.id)))
        .map(|&(k, _, _)| k)
        .collect()
}

/// Tab over a selection of list items: indent each of them, unless the
/// first can't be (Typora leaves the list alone then).
pub fn indent_range(doc: &mut Document, first: usize, last: usize) -> bool {
    let heads = items_in(doc, first, last);
    let Some((&head, rest)) = heads.split_first() else { return false };
    if !indent(doc, head) {
        return false;
    }
    for &j in rest {
        indent(doc, j);
    }
    true
}

/// Shift+Tab over a selection of list items: outdent each of them.
pub fn outdent_range(doc: &mut Document, first: usize, last: usize) -> bool {
    let heads = items_in(doc, first, last);
    for &j in &heads {
        outdent(doc, j);
    }
    !heads.is_empty()
}

/// Toggle a task item's checkbox.
pub fn toggle_task(doc: &mut Document, i: usize) {
    set_task(doc, i, None);
}

/// Mark the task item holding block `i` done or not done, or flip it when
/// `done` is None. Blocks outside a task item are left alone.
pub fn set_task(doc: &mut Document, i: usize, done: Option<bool>) {
    let Some(d) = doc.blocks[i].containers.iter().rposition(|c| c.list_item().is_some()) else { return };
    let items = span(doc, i, d);
    let mut c = doc.blocks[i].containers[d].clone();
    if let ContainerKind::ListItem(item) = &mut c.kind {
        let Some(was) = item.task else { return };
        item.task = Some(done.unwrap_or(!was));
    }
    for j in items {
        doc.blocks[j].containers[d] = c.clone();
    }
}

/// Set a code block's info string, from the code language box. An indented
/// code block becomes a fence; a backtick in the info needs a tilde fence.
/// False when nothing changes.
pub fn set_info(doc: &mut Document, i: usize, info: &str) -> bool {
    let info = info.replace('\n', " ").trim().to_string();
    let (fence, closed) = match &doc.blocks[i].kind {
        BlockKind::CodeFence { info: old, .. } if *old == info => return false,
        BlockKind::CodeFence { fence, closed, .. } => (fence.clone(), *closed),
        BlockKind::IndentedCode if info.is_empty() => return false,
        BlockKind::IndentedCode => ("```".to_string(), true),
        _ => return false,
    };
    let fence = if fence.starts_with('`') && info.contains('`') { "~".repeat(fence.len()) } else { fence };
    doc.blocks[i].set_kind(BlockKind::CodeFence { info, fence, closed });
    true
}

/// Set the `{tag}`s on a `$$` math block's lines, from the fields beside
/// them. False when nothing changes.
pub fn set_math_tags(doc: &mut Document, i: usize, before: &str, after: &str) -> bool {
    let (before, after) = (before.replace('\n', " "), after.replace('\n', " "));
    match &doc.blocks[i].kind {
        BlockKind::Math { brackets: false, before: b, after: a } if (b, a) != (&before, &after) => {}
        _ => return false,
    }
    doc.blocks[i].set_kind(BlockKind::Math { brackets: false, before, after });
    true
}

/// Insert a new block after block `i` (Paragraph ▸ Code Fences, Math Block,
/// Horizontal Line, Table…), replacing `i` when it is an empty paragraph.
/// Returns the caret in the new block, or after it for a horizontal line.
pub fn insert_block(doc: &mut Document, i: usize, kind: BlockKind) -> Pos {
    let b = &doc.blocks[i];
    let containers = b.containers.clone();
    let replace = matches!(b.kind, BlockKind::Paragraph) && b.text.is_empty();
    let at = if replace { i } else { i + 1 };
    let atomic = is_atomic(&kind);
    let new = Block::new(kind, "", containers.clone());
    if replace {
        doc.blocks[i] = new;
    } else {
        doc.blocks.insert(at, new);
    }
    moved(doc, at..at + 1);
    if atomic && !matches!(doc.blocks[at].kind, BlockKind::Table { .. }) {
        // A paragraph to keep typing in after a rule or TOC.
        doc.blocks.insert(at + 1, Block::new(BlockKind::Paragraph, "", containers));
        moved(doc, at + 1..at + 2);
        return Pos::new(at + 1, 0);
    }
    Pos::new(at, 0)
}

/// Paragraph ▸ Insert Paragraph Before / After.
pub fn insert_paragraph(doc: &mut Document, i: usize, after: bool) -> Pos {
    let at = if after { i + 1 } else { i };
    let containers = doc.blocks[i].containers.clone();
    doc.blocks.insert(at, Block::new(BlockKind::Paragraph, "", containers));
    moved(doc, at..at + 1);
    Pos::new(at, 0)
}

/// An empty definition for footnote `label` at the end of the document, as
/// Typora adds one from a reference's tooltip. The caret goes into it.
pub fn add_footnote_def(doc: &mut Document, label: &str) -> Pos {
    let at = doc.blocks.len();
    let container = Container::new(ContainerKind::Footnote { label: label.to_string() });
    doc.blocks.push(Block::new(BlockKind::Paragraph, "", vec![container]));
    moved(doc, at..at + 1);
    Pos::new(at, 0)
}

pub fn new_table(columns: usize, rows: usize) -> BlockKind {
    BlockKind::Table { align: vec![Align::None; columns], rows: vec![vec![String::new(); columns]; rows.max(1)] }
}

// ---------------------------------------------------------------------------
// Input rules.

/// Block syntax typed at the start of a paragraph turns it into that block
/// as soon as the space after the marker is typed: `# `, `> `, `- `, `1. `,
/// `[ ] ` (in a list item), `[^n]: `. `pos` is the caret after the space.
pub fn input_rule(doc: &mut Document, pos: Pos) -> Option<Pos> {
    let i = pos.block;
    let b = &doc.blocks[i];
    if !matches!(b.kind, BlockKind::Paragraph) || pos.offset > b.text.len() || !b.text.is_char_boundary(pos.offset) {
        return None;
    }
    let typed = &b.text[..pos.offset];
    let marker = typed.strip_suffix(' ')?;
    let rest = b.text[pos.offset..].to_string();
    let depth = b.containers.len();
    if !marker.is_empty() && marker.len() <= 6 && marker.chars().all(|c| c == '#') {
        let level = marker.len() as u8;
        let b = &mut doc.blocks[i];
        let in_item = b.containers.iter().any(|c| c.list_item().is_some());
        b.set_kind(syntax().heading(level, in_item));
        b.set_text(rest.replace('\n', " "));
        return Some(Pos::new(i, 0));
    }
    if marker == ">" || remap() && (marker == "》" || marker == "〉") {
        doc.blocks[i].set_text(rest);
        doc.blocks[i].containers.push(Container::new(ContainerKind::Quote { padding: syntax().quote_padding() }));
        moved(doc, i..i + 1);
        return Some(Pos::new(i, 0));
    }
    let list = match marker {
        "-" | "*" | "+" => Some(ListMarker::Bullet(marker.chars().next().unwrap())),
        _ if marker.is_ascii() && marker.len() >= 2 => {
            let (digits, delim) = marker.split_at(marker.len() - 1);
            let number = digits.bytes().all(|c| c.is_ascii_digit()).then(|| digits.parse::<u64>().ok()).flatten();
            match (delim, number) {
                ("." | ")", Some(start)) if digits.len() <= 9 => {
                    Some(ListMarker::Ordered { start, delim: delim.chars().next().unwrap(), same_numbers: false })
                }
                _ => None,
            }
        }
        _ => None,
    };
    if let Some(marker) = list {
        doc.blocks[i].set_text(rest);
        // Continue a compatible list ending right above.
        let above = i.checked_sub(1).and_then(|p| {
            let prev = &doc.blocks[p];
            let item = prev.containers.get(depth)?.list_item()?;
            (common_depth(prev, &doc.blocks[i]) >= depth && item.marker.compatible(&marker)).then(|| item.clone())
        });
        let item = match above {
            Some(a) => {
                let mut c = new_item(a.list, a.marker, None);
                if let ContainerKind::ListItem(li) = &mut c.kind {
                    li.loose = a.loose;
                }
                c
            }
            None => new_item(next_id(), marker, None),
        };
        doc.blocks[i].containers.push(item);
        moved(doc, i..i + 1);
        return Some(Pos::new(i, 0));
    }
    if let Some(check) = match marker {
        "[ ]" => Some(false),
        "[x]" | "[X]" => Some(true),
        "【 】" if remap() => Some(false),
        "【x】" | "【X】" if remap() => Some(true),
        _ => None,
    } {
        let item = doc.blocks[i].containers.last()?.list_item()?;
        if item.task.is_some() || !first_in(doc, i, depth - 1) {
            return None;
        }
        doc.blocks[i].set_text(rest);
        let items = span(doc, i, depth - 1);
        let mut c = doc.blocks[i].containers[depth - 1].clone();
        if let ContainerKind::ListItem(li) = &mut c.kind {
            li.task = Some(check);
        }
        for j in items {
            doc.blocks[j].containers[depth - 1] = c.clone();
        }
        return Some(Pos::new(i, 0));
    }
    if let Some(label) = marker.strip_prefix("[^").and_then(|m| m.strip_suffix("]:")) {
        if label.is_empty() || label.contains(char::is_whitespace) || label.contains(']') {
            return None;
        }
        let label = label.to_string();
        doc.blocks[i].set_text(rest);
        doc.blocks[i].containers.push(Container::new(ContainerKind::Footnote { label }));
        moved(doc, i..i + 1);
        return Some(Pos::new(i, 0));
    }
    None
}

/// Write `key: value` into the front matter, as Typora's per-file image
/// settings do: in place of the key's line, else after the last line, or
/// in new front matter at the top. With no value the key's line goes.
/// False when nothing changed.
pub fn set_property(doc: &mut Document, key: &str, value: Option<&str>) -> bool {
    let line = value.map(|v| format!("{key}: {v}"));
    if doc.blocks.first().is_none_or(|b| b.kind != BlockKind::FrontMatter) {
        let Some(line) = line else { return false };
        doc.blocks.insert(0, Block::new(BlockKind::FrontMatter, line, Vec::new()));
        moved(doc, 0..1);
        return true;
    }
    let is_key = |l: &&str| l.split_once(':').is_some_and(|(k, _)| k.trim().eq_ignore_ascii_case(key));
    let mut lines: Vec<&str> = doc.blocks[0].text.lines().collect();
    match (lines.iter().position(|l| is_key(&l)), &line) {
        (Some(i), Some(line)) => lines[i] = line,
        (Some(i), None) => {
            lines.remove(i);
        }
        (None, Some(line)) => lines.push(line),
        (None, None) => return false,
    }
    let text = lines.join("\n");
    if text == doc.blocks[0].text {
        return false;
    }
    doc.blocks[0].set_text(text);
    true
}

/// Block syntax completed with Enter: a code fence, `$$` or `\[`, a horizontal rule
/// (or front matter on the first line), a table header row, `[toc]`.
pub fn enter_rule(doc: &mut Document, i: usize, latex: bool) -> Option<Pos> {
    let b = &doc.blocks[i];
    if !matches!(b.kind, BlockKind::Paragraph) || b.text.contains('\n') {
        return None;
    }
    let text = b.text.trim().to_string();
    let containers = b.containers.clone();
    if let Some((fence, info)) = fence_start(&text) {
        let b = &mut doc.blocks[i];
        b.set_kind(BlockKind::CodeFence { info, fence, closed: true });
        b.set_text("");
        return Some(Pos::new(i, 0));
    }
    if let Some((brackets, before)) = math_start(&text, latex) {
        let b = &mut doc.blocks[i];
        b.set_kind(BlockKind::Math { brackets, before, after: String::new() });
        b.set_text("");
        return Some(Pos::new(i, 0));
    }
    if text == "---" && i == 0 && containers.is_empty() {
        let b = &mut doc.blocks[i];
        b.set_kind(BlockKind::FrontMatter);
        b.set_text("");
        return Some(Pos::new(i, 0));
    }
    if is_thematic_break(&text) {
        let b = &mut doc.blocks[i];
        b.set_kind(BlockKind::ThematicBreak { marker: text });
        b.set_text("");
        doc.blocks.insert(i + 1, Block::new(BlockKind::Paragraph, "", containers));
        moved(doc, i + 1..i + 2);
        return Some(Pos::new(i + 1, 0));
    }
    if text.eq_ignore_ascii_case("[toc]") {
        let b = &mut doc.blocks[i];
        b.set_kind(BlockKind::Toc);
        b.set_text("[toc]");
        doc.blocks.insert(i + 1, Block::new(BlockKind::Paragraph, "", containers));
        moved(doc, i + 1..i + 2);
        return Some(Pos::new(i + 1, 0));
    }
    if text.len() > 2 && text.starts_with('|') && text.ends_with('|') && !text.ends_with("\\|") {
        let header = split_row(&text);
        let columns = header.len();
        let b = &mut doc.blocks[i];
        b.set_kind(BlockKind::Table { align: vec![Align::None; columns], rows: vec![header, vec![String::new(); columns]] });
        b.set_text("");
        // The caret goes to the first cell under the header.
        return Some(super::table::caret(i, (1, 0)));
    }
    None
}

// ---------------------------------------------------------------------------
// Paste.

/// Paste markdown at `pos`. A single paragraph goes into the text;
/// anything else splits the block and the pasted blocks go between the
/// halves, joining them where they are paragraphs.
pub fn insert_markdown(doc: &mut Document, pos: Pos, md: &str, latex: bool) -> Pos {
    let i = pos.block;
    let offset = floor_char(&doc.blocks[i].text, pos.offset);
    let mut blocks = super::parse::parse_with(md, latex, remap()).blocks;
    let head = doc.blocks[i].clone();
    let plain = |b: &Block| matches!(b.kind, BlockKind::Paragraph) && b.containers.is_empty();
    if (blocks.len() == 1 && plain(&blocks[0])) || !head.kind.is_inline_text() {
        let text = if head.kind.is_inline_text() { blocks[0].text.clone() } else { md.to_string() };
        let mut t = head.text.clone();
        t.insert_str(offset, &text);
        doc.blocks[i].set_text(t);
        return Pos::new(i, offset + text.len());
    }
    let joins_left = plain(&blocks[0]);
    let joins_right = blocks.len() > 1 && plain(blocks.last().unwrap());
    for b in &mut blocks {
        let mut containers = head.containers.clone();
        containers.append(&mut b.containers);
        b.containers = containers;
        b.lazy.clear();
        b.blank_before = None;
    }
    let (left, right) = head.text.split_at(offset);
    let heading = !matches!(head.kind, BlockKind::Paragraph);
    let mut out: Vec<Block> = Vec::new();
    if joins_left {
        let first = blocks.remove(0);
        let mut b = head.clone();
        let text = if heading { first.text.replace('\n', " ") } else { first.text };
        b.set_text(format!("{left}{text}"));
        out.push(b);
    } else if !left.is_empty() || heading {
        let mut b = head.clone();
        b.set_text(left);
        out.push(b);
    }
    out.extend(blocks);
    let last = out.len() - 1;
    let caret = Pos::new(i + last, out[last].text.len());
    if !right.is_empty() {
        if joins_right {
            let b = &mut out[last];
            let text = format!("{}{right}", b.text);
            b.set_text(text);
        } else {
            out.push(Block::new(BlockKind::Paragraph, right, head.containers.clone()));
        }
    }
    let n = out.len();
    doc.blocks.splice(i..i + 1, out);
    moved(doc, i..i + n);
    caret
}

// ---------------------------------------------------------------------------
// Inline styles.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Strong,
    Emphasis,
    Code,
    Strike,
    Underline,
    Highlight,
    Superscript,
    Subscript,
    Math,
    Comment,
}

impl Style {
    fn markers(self) -> (&'static str, &'static str) {
        match self {
            Style::Strong => ("**", "**"),
            Style::Emphasis => ("*", "*"),
            Style::Code => ("`", "`"),
            Style::Strike => ("~~", "~~"),
            Style::Underline => ("<u>", "</u>"),
            Style::Highlight => ("==", "=="),
            Style::Superscript => ("^", "^"),
            Style::Subscript => ("~", "~"),
            Style::Math => ("$", "$"),
            Style::Comment => ("<!-- ", " -->"),
        }
    }

    fn matches(self, kind: &Kind, src: &str, n: &Node) -> bool {
        match (self, kind) {
            (Style::Strong, Kind::Strong)
            | (Style::Emphasis, Kind::Emph)
            | (Style::Code, Kind::Code)
            | (Style::Strike, Kind::Strike)
            | (Style::Highlight, Kind::Highlight)
            | (Style::Superscript, Kind::Sup)
            | (Style::Subscript, Kind::Sub)
            | (Style::Math, Kind::Math) => true,
            (Style::Underline, Kind::Tag { name, .. }) => name == "u",
            (Style::Comment, Kind::Comment) => true,
            _ => false,
        }
    }
}

/// The innermost node of `style` whose source covers `sel`.
fn enclosing<'n>(src: &str, nodes: &'n [Node], sel: &Range<usize>, style: Style) -> Option<&'n Node> {
    for n in nodes {
        if n.range.start <= sel.start && sel.end <= n.range.end {
            if let Some(inner) = enclosing(src, &n.children, sel, style) {
                return Some(inner);
            }
            if style.matches(&n.kind, src, n) {
                return Some(n);
            }
        }
    }
    None
}

/// Format ▸ Strong and friends: wrap the selection in the style's markers,
/// or remove them when the selection is already inside that style. With no
/// selection the markers go in empty, caret between them. Returns the new
/// text and selection.
pub fn toggle_style(text: &str, sel: Range<usize>, style: Style, refs: &Refs) -> (String, Range<usize>) {
    let sel = floor_char(text, sel.start)..floor_char(text, sel.end);
    // Every syntax on, so a span can be removed even when its preference
    // is off.
    let nodes = inline::parse(text, refs, &Options::all());
    if let Some(n) = enclosing(text, &nodes, &sel, style) {
        let open = n.content.start - n.range.start;
        let out = format!("{}{}{}", &text[..n.range.start], &text[n.content.clone()], &text[n.range.end..]);
        let clamp = |p: usize| p.clamp(n.content.start, n.content.end) - open;
        return (out, clamp(sel.start)..clamp(sel.end));
    }
    let (open, close) = style.markers();
    // Markers hug the text: `** a **` would not be strong.
    let inner = &text[sel.clone()];
    let start = sel.start + (inner.len() - inner.trim_start().len());
    let end = (sel.end - (inner.len() - inner.trim_end().len())).max(start);
    let mut out = String::with_capacity(text.len() + open.len() + close.len());
    out.push_str(&text[..start]);
    out.push_str(open);
    out.push_str(&text[start..end]);
    out.push_str(close);
    out.push_str(&text[end..]);
    (out, start + open.len()..end + open.len())
}

/// The part of each inline-text block a selection from `start` to `end`
/// covers, skipping blank ones.
fn parts(doc: &Document, start: Pos, end: Pos) -> Vec<(usize, Range<usize>)> {
    (start.block..=end.block)
        .filter(|&j| doc.blocks[j].kind.is_inline_text())
        .map(|j| {
            let t = &doc.blocks[j].text;
            let a = if j == start.block { floor_char(t, start.offset) } else { 0 };
            let z = if j == end.block { floor_char(t, end.offset) } else { t.len() };
            (j, a..z.max(a))
        })
        .filter(|(j, r)| !doc.blocks[*j].text[r.clone()].trim().is_empty())
        .collect()
}

/// A style over a selection across blocks, as Typora applies it: each
/// block's part is styled on its own, and the style comes off instead when
/// every part already has it. Returns the new selection.
pub fn style_range(doc: &mut Document, start: Pos, end: Pos, style: Style, refs: &Refs) -> (Pos, Pos) {
    let parts = parts(doc, start, end);
    let has = |doc: &Document, j: usize, r: &Range<usize>| {
        let t = &doc.blocks[j].text;
        enclosing(t, &inline::parse(t, refs, &Options::all()), r, style).is_some()
    };
    let remove = parts.iter().all(|(j, r)| has(doc, *j, r));
    let (mut a, mut z) = (start, end);
    for (j, r) in parts {
        if has(doc, j, &r) != remove {
            continue;
        }
        let (text, sel) = toggle_style(&doc.blocks[j].text, r, style, refs);
        doc.blocks[j].set_text(text);
        if j == start.block {
            a.offset = sel.start;
        }
        if j == end.block {
            z.offset = sel.end;
        }
    }
    (a, z)
}

/// Clear Format over a selection across blocks: each block's part.
pub fn clear_range(doc: &mut Document, start: Pos, end: Pos, refs: &Refs, opts: &Options) -> (Pos, Pos) {
    let (mut a, mut z) = (start, end);
    for (j, r) in parts(doc, start, end) {
        let (text, sel) = clear_format(&doc.blocks[j].text, r, refs, opts);
        doc.blocks[j].set_text(text);
        if j == start.block {
            a.offset = sel.start;
        }
        if j == end.block {
            z.offset = sel.end;
        }
    }
    (a, z)
}

/// Format ▸ Hyperlink: `[selection]()` with the caret in the parentheses, or
/// the link's text alone when the selection is inside a link.
///
/// The address comes from the selection when it is a URL, or else from the
/// clipboard (`clip`). With nothing selected the caret goes between the
/// brackets, as Typora's, and the clipboard is taken for a path too.
pub fn toggle_link(text: &str, sel: Range<usize>, refs: &Refs, clip: &str) -> (String, Range<usize>) {
    toggle_wrap(text, sel, refs, clip, false)
}

/// Format ▸ Image ▸ Insert Image (Ctrl+Shift+I): `![selection](…)`, or the
/// image around the caret back to its alt text, selected. The address comes
/// from the selection or the clipboard when it names an image.
pub fn toggle_image(text: &str, sel: Range<usize>, refs: &Refs, clip: &str) -> (String, Range<usize>) {
    toggle_wrap(text, sel, refs, clip, true)
}

/// The innermost image the selection is in, edges included: Typora's
/// expanded image.
fn image_around<'n>(nodes: &'n [Node], sel: &Range<usize>) -> Option<&'n Node> {
    // A caret between text and an image is on the image.
    nodes
        .iter()
        .filter(|n| n.range.start <= sel.start && sel.end <= n.range.end)
        .find_map(|n| image_around(&n.children, sel).or(n.kind.is_image().then_some(n)))
}

/// An image's source, alt text and title, whichever way it is written.
fn image_parts(text: &str, n: &Node) -> (String, String, String) {
    match &n.kind {
        Kind::ImgTag(tag) => (tag.src.clone(), tag.alt.clone(), tag.title.clone()),
        Kind::Image { dest, title } => {
            let mut alt = String::new();
            inline::plain_text(text, &n.children, &mut alt);
            (dest.clone(), alt, title.clone())
        }
        _ => Default::default(),
    }
}

/// `text` with node `n` replaced by `with`, which ends up selected, as
/// Typora selects an image's new source.
fn replace_node(text: &str, n: &Node, with: &str) -> (String, Range<usize>) {
    let out = format!("{}{with}{}", &text[..n.range.start], &text[n.range.end..]);
    (out, n.range.start..n.range.start + with.len())
}

/// Format ▸ Image ▸ Zoom Image: the image the selection is in as an
/// `<img>` tag with `zoom` (such as "50%") in its style; "100%" takes the
/// zoom out. A markdown image becomes a tag first, unless it stays at 100%.
pub fn zoom_image(text: &str, sel: Range<usize>, refs: &Refs, zoom: &str) -> Option<(String, Range<usize>)> {
    let nodes = inline::parse(text, refs, &Options::default());
    let n = image_around(&nodes, &sel)?;
    let zoom = if zoom == "100%" { "" } else { zoom };
    let tag = match &n.kind {
        Kind::ImgTag(_) => text[n.range.clone()].to_string(),
        _ if zoom.is_empty() => return None,
        _ => {
            let (src, alt, title) = image_parts(text, n);
            imgtag::to_html(&src, &alt, &title)
        }
    };
    let tag = imgtag::with_style(&tag, "zoom", zoom);
    (tag != text[n.range.clone()]).then(|| replace_node(text, n, &tag))
}

/// Format ▸ Image ▸ Switch Image Syntax: the image the selection is in
/// written as an `<img>` tag (`html`) or as `![alt](src)`; the markdown
/// form keeps only the source, alt text and title.
pub fn switch_image_syntax(text: &str, sel: Range<usize>, refs: &Refs, html: bool) -> Option<(String, Range<usize>)> {
    let nodes = inline::parse(text, refs, &Options::default());
    let n = image_around(&nodes, &sel)?;
    let (src, alt, title) = image_parts(text, n);
    let new = match (&n.kind, html) {
        (Kind::ImgTag(_), false) => imgtag::to_markdown(&src, &alt, &title),
        (Kind::Image { .. }, true) => imgtag::to_html(&src, &alt, &title),
        _ => return None,
    };
    Some(replace_node(text, n, &new))
}

/// The image the selection is in: its range, its source as written, and
/// how it is written: "markdown", "html" (an `<img>` tag) or "ref" (with a
/// link reference, which Switch Image Syntax leaves alone).
pub fn image_at(text: &str, sel: Range<usize>, refs: &Refs) -> Option<(Range<usize>, String, &'static str)> {
    let nodes = inline::parse(text, refs, &Options::default());
    let n = image_around(&nodes, &sel)?;
    let syntax = match n.kind {
        Kind::ImgTag(_) => "html",
        _ if text[n.content.end..n.range.end].starts_with("](") => "markdown",
        _ => "ref",
    };
    Some((n.range.clone(), n.kind.image_src()?.to_string(), syntax))
}

/// Every image in `text`, first to last: its range and source as written.
pub fn images(text: &str, refs: &Refs) -> Vec<(Range<usize>, String)> {
    fn walk(nodes: &[Node], out: &mut Vec<(Range<usize>, String)>) {
        for n in nodes {
            match n.kind.image_src() {
                Some(src) => out.push((n.range.clone(), src.to_string())),
                None => walk(&n.children, out),
            }
        }
    }
    let mut out = Vec::new();
    walk(&inline::parse(text, refs, &Options::default()), &mut out);
    out
}

/// The image the selection is in, pointed at `src`, which ends up
/// selected; an image written with a reference becomes an inline one.
pub fn set_image_src(text: &str, sel: Range<usize>, refs: &Refs, src: &str) -> Option<(String, Range<usize>)> {
    let nodes = inline::parse(text, refs, &Options::default());
    let n = image_around(&nodes, &sel)?;
    let new = match &n.kind {
        Kind::ImgTag(_) => imgtag::with_attr(&text[n.range.clone()], "src", src),
        Kind::Image { title, .. } => {
            let dest = if src.contains(char::is_whitespace) { format!("<{src}>") } else { src.to_string() };
            let title = if title.is_empty() { String::new() } else { format!(" \"{}\"", title.replace('"', "\\\"")) };
            format!("{}]({dest}{title})", &text[n.range.start..n.content.end])
        }
        _ => return None,
    };
    Some(replace_node(text, n, &new))
}

/// The edits that work on one line of text, for a paragraph and a table
/// cell alike: Hyperlink, Insert Image (`arg` is the clipboard), Zoom Image
/// (`arg` is the zoom), Switch Image Syntax (`arg` is "html" or
/// "markdown"), and pointing the image at `arg` or deleting it, after a
/// copy or move of its file. None when it doesn't apply or changes nothing.
pub fn text_command(text: &str, sel: Range<usize>, name: &str, arg: &str, refs: &Refs) -> Option<(String, Range<usize>)> {
    let sel = floor_char(text, sel.start)..floor_char(text, sel.end);
    match name {
        "link" => Some(toggle_link(text, sel, refs, arg)),
        "image" => Some(toggle_image(text, sel, refs, arg)),
        "zoomImage" => zoom_image(text, sel, refs, arg),
        "imageSyntax" => switch_image_syntax(text, sel, refs, arg == "html"),
        "imageSrc" => set_image_src(text, sel, refs, arg),
        "deleteImage" => image_at(text, sel, refs).map(|(r, ..)| (format!("{}{}", &text[..r.start], &text[r.end..]), r.start..r.start)),
        "footnote" => Some(toggle_footnote(text, sel, refs)),
        _ => None,
    }
}

/// Paragraph ▸ Footnotes: the selection becomes a footnote's label, or with
/// none, an empty `[^]` takes the caret; in a footnote reference it
/// becomes its label again. Typora writes `[^ ]`, which GFM wouldn't read
/// as a footnote once a label is typed in.
fn toggle_footnote(text: &str, sel: Range<usize>, refs: &Refs) -> (String, Range<usize>) {
    let nodes = inline::parse(text, refs, &Options::default());
    fn find<'n>(nodes: &'n [Node], sel: &Range<usize>) -> Option<&'n Node> {
        nodes.iter().find(|n| n.range.start <= sel.start && sel.end <= n.range.end).and_then(|n| {
            find(&n.children, sel).or(matches!(n.kind, Kind::FootnoteRef { .. }).then_some(n))
        })
    }
    if let Some(n) = find(&nodes, &sel) {
        let label = n.range.start + 2..n.range.end - 1;
        let out = format!("{}{}{}", &text[..n.range.start], &text[label.clone()], &text[n.range.end..]);
        return (out, n.range.start..n.range.start + label.len());
    }
    // Spaces at either end of the selection stay outside, as for links.
    let picked = &text[sel.clone()];
    let start = sel.start + (picked.len() - picked.trim_start().len());
    let end = (sel.end - (picked.len() - picked.trim_end().len())).max(start);
    let out = format!("{}[^{}]{}", &text[..start], &text[start..end], &text[end..]);
    (out, start + 2..end + 2)
}

fn toggle_wrap(text: &str, sel: Range<usize>, refs: &Refs, clip: &str, image: bool) -> (String, Range<usize>) {
    let sel = floor_char(text, sel.start)..floor_char(text, sel.end);
    let nodes = inline::parse(text, refs, &Options::default());
    fn find<'n>(nodes: &'n [Node], sel: &Range<usize>, image: bool) -> Option<&'n Node> {
        nodes.iter().find(|n| n.range.start <= sel.start && sel.end <= n.range.end).and_then(|n| {
            let hit = if image { matches!(n.kind, Kind::Image { .. }) } else { matches!(n.kind, Kind::Link { .. }) };
            find(&n.children, sel, image).or(hit.then_some(n))
        })
    }
    if let Some(n) = find(&nodes, &sel, image) {
        let out = format!("{}{}{}", &text[..n.range.start], &text[n.content.clone()], &text[n.range.end..]);
        let shift = n.content.start - n.range.start;
        if image {
            return (out, n.content.start - shift..n.content.end - shift);
        }
        let clamp = |p: usize| p.clamp(n.content.start, n.content.end) - shift;
        return (out, clamp(sel.start)..clamp(sel.end));
    }
    // Typora leaves spaces at either end of the selection outside.
    let picked = &text[sel.clone()];
    let start = sel.start + (picked.len() - picked.trim_start().len());
    let end = (sel.end - (picked.len() - picked.trim_end().len())).max(start);
    let picked = &text[start..end];
    // It ignores a clipboard of more than 5000 characters.
    let clip = if clip.chars().count() > 5000 { "" } else { clip.trim() };
    let fits = |s: &str| if image { looks_like_image(s) } else { looks_like_url(s) || start == end && looks_like_path(s) };
    let dest = if start < end && fits(picked) {
        picked
    } else if fits(clip) {
        clip
    } else {
        ""
    };
    let open = if image { "![" } else { "[" };
    let out = format!("{}{open}{picked}]({dest}){}", &text[..start], &text[end..]);
    if start == end {
        let caret = start + open.len();
        return (out, caret..caret);
    }
    let caret = end + open.len() + 2 + dest.len();
    (out, caret..caret)
}

/// Typora's test for an address (`isURL`): a scheme, `www.` or a path's
/// first slash at the start, or a `mailto:` anywhere.
pub fn looks_like_url(s: &str) -> bool {
    static URL: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^\s*(([0-9a-z_-]+://)|(www\.)|[0-9a-z_]*/)").unwrap());
    URL.is_match(s) || s.contains("mailto:")
}

/// Typora's test for an image's address (`isImg`): a URL or path with an
/// image's extension in it.
pub fn looks_like_image(s: &str) -> bool {
    static IMAGE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)^\s*(([0-9a-z_-]+://)|(www\.)|[0-9a-z_]*/)?[0-9a-z_./?%&=:@-]+(\.jpe?g|png|gif|webp|svg|tiff)")
            .unwrap()
    });
    IMAGE.is_match(s)
}

/// What else Typora takes from the clipboard for a new, empty link: a file
/// path or a name with an extension.
fn looks_like_path(s: &str) -> bool {
    static OTHER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)[^.a-z0-9\\/% _#@]").unwrap());
    static SPACED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\\/.]\s").unwrap());
    static EXTENSION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\.[a-z]+$").unwrap());
    !s.is_empty()
        && !OTHER.is_match(s)
        && s.encode_utf16().count() < 256
        && (s.contains(['/', '\\']) && !SPACED.is_match(s) || EXTENSION.is_match(s))
}

/// Format ▸ Clear Format: drop the markers of every span inside the
/// selection, keeping its text.
pub fn clear_format(text: &str, sel: Range<usize>, refs: &Refs, opts: &Options) -> (String, Range<usize>) {
    let sel = floor_char(text, sel.start)..floor_char(text, sel.end);
    let nodes = inline::parse(text, refs, opts);
    // Collect the marker ranges of styled spans that overlap the selection.
    fn markers(src: &str, nodes: &[Node], sel: &Range<usize>, out: &mut Vec<Range<usize>>) {
        for n in nodes {
            if n.range.end <= sel.start || n.range.start >= sel.end {
                continue;
            }
            let styled = matches!(
                n.kind,
                Kind::Emph | Kind::Strong | Kind::Strike | Kind::Highlight | Kind::Sup | Kind::Sub | Kind::Code | Kind::Tag { .. }
            );
            if styled {
                out.push(n.range.start..n.content.start);
                out.push(n.content.end..n.range.end);
            }
            markers(src, &n.children, sel, out);
        }
    }
    let mut cut = Vec::new();
    markers(text, &nodes, &sel, &mut cut);
    cut.sort_by_key(|r| r.start);
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    let shift = |p: usize, cut: &[Range<usize>]| -> usize {
        p - cut.iter().map(|r| r.end.min(p).saturating_sub(r.start)).sum::<usize>()
    };
    for r in &cut {
        out.push_str(&text[last..r.start]);
        last = r.end;
    }
    out.push_str(&text[last..]);
    let s = shift(sel.start, &cut);
    let e = shift(sel.end, &cut);
    (out, s..e)
}
