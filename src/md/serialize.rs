//! `Document` → markdown. Untouched blocks reproduce their source lines;
//! edited blocks are generated. Container prefixes (`> `, list markers and
//! their indentation, footnote labels) are always rebuilt from the container
//! path, and blank lines come from each block's recorded spacing, raised
//! where markdown needs a blank line to keep two blocks apart.

use std::collections::HashMap;

use unicode_width::UnicodeWidthStr;

use super::model::*;
use super::parse::{atx_text, escape_cell, html_self_terminated, paragraph_lines};

pub fn serialize(doc: &Document) -> String {
    serialize_with_starts(doc).0
}

/// The block at source line `line` (from 0): the last one starting at or
/// before it.
pub fn block_at_line(doc: &Document, line: usize) -> usize {
    let starts = serialize_with_starts(doc).1;
    starts.iter().rposition(|&s| s <= line).unwrap_or(0)
}

/// The markdown, and the line each block starts on (a block that isn't
/// written gets the line after the one before it).
pub fn serialize_with_starts(doc: &Document) -> (String, Vec<usize>) {
    let lists = ListNumbers::new(&doc.blocks);
    let indices: Vec<usize> = (0..doc.blocks.len()).filter(|&i| written(&doc.blocks, i)).collect();
    let blocks: Vec<&Block> = indices.iter().map(|&i| &doc.blocks[i]).collect();
    let mut starts = vec![0; doc.blocks.len()];
    let mut out: Vec<String> = Vec::new();
    for (i, &block) in blocks.iter().enumerate() {
        let prev = i.checked_sub(1).map(|j| blocks[j]);
        let depth = prev.map_or(0, |p| common_depth(p, block));
        if let Some(p) = prev {
            for m in blank_lines(p, block, depth, &lists) {
                out.push(blank_line(&p.containers[..m], &lists));
            }
        } else {
            let leading = block.blank_before.as_ref().map_or(0, Vec::len);
            out.extend(std::iter::repeat_n(String::new(), leading));
        }
        starts[indices[i]] = out.len();
        for (n, line) in block_lines(block).into_iter().enumerate() {
            let mut prefix = String::new();
            let matched = block.lazy.iter().find(|(l, _)| *l == n).map_or(block.containers.len(), |(_, m)| *m);
            for (d, c) in block.containers[..matched].iter().enumerate() {
                if n == 0 && d >= depth {
                    prefix.push_str(&opening(c, &lists));
                } else {
                    prefix.push_str(&continuation(c, &lists));
                }
            }
            if line.is_empty() {
                prefix.truncate(prefix.trim_end().len());
            }
            prefix.push_str(&line);
            out.push(prefix);
        }
    }
    if let Some(last) = blocks.last() {
        for &m in &doc.blank_after {
            out.push(blank_line(&last.containers[..m.min(last.containers.len())], &lists));
        }
    }
    let eol = match doc.line_ending {
        LineEnding::Lf => "\n",
        LineEnding::CrLf => "\r\n",
    };
    let mut s = out.join(eol);
    if doc.final_newline {
        s.push_str(eol);
    }
    // Blocks that aren't written sit where the next one starts.
    let mut next = out.len();
    for i in (0..starts.len()).rev() {
        if indices.binary_search(&i).is_ok() {
            next = starts[i];
        } else {
            starts[i] = next;
        }
    }
    (s, starts)
}

/// Empty paragraphs made while editing have no markdown form, so they are
/// left out, unless one holds an otherwise empty container open (`-`, `>`).
fn written(blocks: &[Block], i: usize) -> bool {
    let b = &blocks[i];
    if !matches!(b.kind, BlockKind::Paragraph) || !b.text.is_empty() || b.raw.is_some() {
        return true;
    }
    let Some(inner) = b.containers.last() else { return blocks.len() == 1 };
    let shares = |j: usize| blocks.get(j).is_some_and(|o| o.containers.iter().any(|c| c.id == inner.id));
    !(i > 0 && shares(i - 1)) && !shares(i + 1)
}

/// Item numbering: each list's first item fixes its numbering style, and each
/// item's position in its list gives its number.
struct ListNumbers {
    first: HashMap<Id, ListMarker>,
    ordinal: HashMap<Id, usize>,
}

impl ListNumbers {
    fn new(blocks: &[Block]) -> Self {
        let mut first = HashMap::new();
        let mut ordinal = HashMap::new();
        let mut counts: HashMap<Id, usize> = HashMap::new();
        for c in blocks.iter().flat_map(|b| &b.containers) {
            if let Some(item) = c.list_item() {
                if !ordinal.contains_key(&c.id) {
                    first.entry(item.list).or_insert_with(|| item.marker.clone());
                    let n = counts.entry(item.list).or_insert(0);
                    ordinal.insert(c.id, *n);
                    *n += 1;
                }
            }
        }
        ListNumbers { first, ordinal }
    }

    fn marker(&self, c: &Container, item: &ListItem) -> String {
        let marker = match (&item.marker, self.first.get(&item.list)) {
            (ListMarker::Ordered { .. }, Some(first @ ListMarker::Ordered { .. })) => first,
            _ => &item.marker,
        };
        marker.text(self.ordinal.get(&c.id).copied().unwrap_or(0))
    }
}

/// The prefix on a container's first line.
fn opening(c: &Container, lists: &ListNumbers) -> String {
    match &c.kind {
        ContainerKind::Quote { padding } => format!(">{}", " ".repeat((*padding).max(1))),
        ContainerKind::ListItem(item) => {
            let mut s = " ".repeat(item.indent);
            s.push_str(&lists.marker(c, item));
            s.push_str(&" ".repeat(item.padding.max(1)));
            match item.task {
                Some(true) => s.push_str("[x] "),
                Some(false) => s.push_str("[ ] "),
                None => {}
            }
            s
        }
        ContainerKind::Footnote { label } => format!("[^{label}]: "),
    }
}

/// The prefix on a container's later lines.
fn continuation(c: &Container, lists: &ListNumbers) -> String {
    match &c.kind {
        ContainerKind::Quote { padding } => format!(">{}", " ".repeat((*padding).max(1))),
        ContainerKind::ListItem(item) => {
            " ".repeat(item.indent + lists.marker(c, item).len() + item.padding.max(1))
        }
        ContainerKind::Footnote { .. } => "    ".into(),
    }
}

/// A blank line inside the given containers: `>` inside a quote.
fn blank_line(containers: &[Container], lists: &ListNumbers) -> String {
    let prefix: String = containers.iter().map(|c| continuation(c, lists)).collect();
    prefix.trim_end().to_string()
}

/// The blank lines between `prev` and `block`, each as the number of prev's
/// containers it is written inside: as recorded, or the default spacing for
/// a new block, plus a blank line where one is required.
fn blank_lines(prev: &Block, block: &Block, depth: usize, lists: &ListNumbers) -> Vec<usize> {
    let mut lines: Vec<usize> = match &block.blank_before {
        Some(recorded) => recorded.iter().map(|&m| m.clamp(depth, prev.containers.len())).collect(),
        None => vec![depth; default_blanks(prev, block, depth)],
    };
    match required_blank(prev, block, depth, lists) {
        Need::Nothing => {}
        Need::Blank if !lines.is_empty() => {}
        Need::Blank | Need::BlankOutside => {
            if !lines.contains(&depth) {
                match lines.last_mut() {
                    Some(last) => *last = depth,
                    None => lines.push(depth),
                }
            }
        }
    }
    lines
}

/// Blank lines between `prev` and `block` when the block has no recorded
/// spacing: none between items of a tight list or blocks inside a tight
/// item, one otherwise.
fn default_blanks(prev: &Block, block: &Block, depth: usize) -> usize {
    if let (Some(a), Some(b)) = (prev.containers.get(depth), block.containers.get(depth)) {
        if let (Some(x), Some(y)) = (a.list_item(), b.list_item()) {
            if x.list == y.list {
                return usize::from(x.loose);
            }
        }
    }
    match depth.checked_sub(1).and_then(|d| block.containers[d].list_item()) {
        Some(item) if !item.loose => 0,
        _ => 1,
    }
}

enum Need {
    Nothing,
    /// Any blank line.
    Blank,
    /// A blank line outside prev's containers that the block doesn't share.
    BlankOutside,
}

/// Whether `block` would otherwise merge into `prev`: a paragraph absorbing
/// the next line, a table taking it as a row, an HTML block running on, or
/// two quotes joining.
fn required_blank(prev: &Block, block: &Block, depth: usize, lists: &ListNumbers) -> Need {
    if let (Some(a), Some(b)) = (prev.containers.get(depth), block.containers.get(depth)) {
        if matches!(a.kind, ContainerKind::Quote { .. }) && matches!(b.kind, ContainerKind::Quote { .. }) {
            return Need::BlankOutside;
        }
    }
    let opens_container = block.containers.len() > depth;
    let paragraph = matches!(prev.kind, BlockKind::Paragraph) && !prev.text.is_empty();
    let needed = if depth == prev.containers.len() {
        // The block's first line lands inside prev's innermost container.
        match &prev.kind {
            _ if paragraph => !interrupts_paragraph(block, depth, lists),
            BlockKind::Table { .. } => !opens_container && !starts_block(block),
            BlockKind::Html => !html_self_terminated(&prev.text),
            BlockKind::CodeFence { closed: false, .. } => true,
            _ => false,
        }
    } else {
        // prev's inner containers end here, unless a paragraph continues lazily.
        paragraph && !opens_container && !starts_block(block)
    };
    if needed { Need::Blank } else { Need::Nothing }
}

/// Whether the block's first line, with no new container, starts a block of
/// its own after any other block.
fn starts_block(block: &Block) -> bool {
    match &block.kind {
        BlockKind::Heading { setext, .. } => setext.is_none(),
        BlockKind::CodeFence { .. }
        | BlockKind::Math { .. }
        | BlockKind::Html
        | BlockKind::ThematicBreak { .. }
        | BlockKind::FrontMatter => true,
        BlockKind::Paragraph
        | BlockKind::IndentedCode
        | BlockKind::Table { .. }
        | BlockKind::Toc
        | BlockKind::LinkRefDef => false,
    }
}

/// Whether the block can start directly below a paragraph in the same
/// container without a blank line.
fn interrupts_paragraph(block: &Block, depth: usize, lists: &ListNumbers) -> bool {
    if let Some(c) = block.containers.get(depth) {
        return match &c.kind {
            ContainerKind::Quote { .. } => true,
            ContainerKind::ListItem(item) => {
                let empty = block.containers.len() == depth + 1
                    && matches!(block.kind, BlockKind::Paragraph)
                    && block.text.is_empty();
                let first = lists.marker(c, item);
                let from_one = !item.marker.is_ordered() || first == "1." || first == "1)";
                !empty && from_one
            }
            ContainerKind::Footnote { .. } => false,
        };
    }
    match &block.kind {
        BlockKind::ThematicBreak { marker } => !marker.trim_end().chars().all(|c| c == '-'),
        BlockKind::Html => {
            let first = block.text.lines().next().unwrap_or("");
            super::parse::html_interrupts_paragraph(first.trim_start())
        }
        // A table's header row joins the paragraph and is split off again
        // when the delimiter row follows.
        BlockKind::Table { .. } => true,
        _ => starts_block(block),
    }
}

/// The block's lines without container prefixes.
fn block_lines(block: &Block) -> Vec<String> {
    if let Some(raw) = &block.raw {
        return raw.clone();
    }
    let text_lines = || block.text.split('\n').map(str::to_string).collect::<Vec<_>>();
    match &block.kind {
        BlockKind::Paragraph => paragraph_lines(&block.text),
        BlockKind::Html | BlockKind::LinkRefDef => text_lines(),
        BlockKind::Heading { level, setext: Some(underline), .. } if *level <= 2 && !block.text.is_empty() => {
            let mut lines = paragraph_lines(&block.text);
            let underline = if underline.is_empty() {
                let width = lines.iter().map(|l| unicode_width::UnicodeWidthStr::width(l.as_str())).max().unwrap_or(0);
                (if *level == 1 { "=" } else { "-" }).repeat(width.max(1))
            } else {
                underline.clone()
            };
            lines.push(underline);
            lines
        }
        BlockKind::Heading { level, closed, .. } => {
            let hashes = "#".repeat(*level as usize);
            let text = atx_text(block.text.replace('\n', " ").trim());
            vec![match (text.is_empty(), closed) {
                (true, _) => hashes,
                (false, true) => format!("{hashes} {text} {hashes}"),
                (false, false) => format!("{hashes} {text}"),
            }]
        }
        BlockKind::CodeFence { info, fence, .. } => {
            let fence = safe_fence(fence, &block.text);
            let mut lines = vec![format!("{fence}{info}")];
            if !block.text.is_empty() {
                lines.extend(text_lines());
            }
            lines.push(fence);
            lines
        }
        BlockKind::IndentedCode => text_lines()
            .into_iter()
            .map(|l| if l.is_empty() { l } else { format!("    {l}") })
            .collect(),
        BlockKind::Math { brackets } => {
            let (open, close) = if *brackets { ("\\[", "\\]") } else { ("$$", "$$") };
            let mut lines = vec![open.to_string()];
            if !block.text.is_empty() {
                lines.extend(text_lines());
            }
            lines.push(close.into());
            lines
        }
        BlockKind::Table { align, rows } => table_lines(align, rows),
        BlockKind::ThematicBreak { marker } => {
            vec![if marker.is_empty() { "---".into() } else { marker.clone() }]
        }
        BlockKind::FrontMatter => {
            let mut lines = vec!["---".to_string()];
            if !block.text.is_empty() {
                lines.extend(text_lines());
            }
            lines.push("---".into());
            lines
        }
        BlockKind::Toc => vec![if block.text.is_empty() { "[toc]".into() } else { block.text.clone() }],
    }
}

/// A fence longer than any run of its character inside the code.
fn safe_fence(fence: &str, code: &str) -> String {
    let c = fence.chars().next().unwrap_or('`');
    let longest = code
        .lines()
        .map(|l| l.trim_start().chars().take_while(|&ch| ch == c).count())
        .max()
        .unwrap_or(0);
    c.to_string().repeat(fence.chars().count().max(3).max(longest + 1))
}

/// A table with cells padded to aligned columns.
pub fn table_lines(align: &[Align], rows: &[Vec<String>]) -> Vec<String> {
    let cols = align.len();
    let cell = |row: &Vec<String>, c: usize| row.get(c).map(|s| escape_cell(s)).unwrap_or_default();
    let mut widths = vec![3usize; cols];
    for row in rows {
        for (c, width) in widths.iter_mut().enumerate() {
            *width = (*width).max(cell(row, c).width());
        }
    }
    let format_row = |row: &Vec<String>| {
        let cells: Vec<String> = (0..cols).map(|c| pad(&cell(row, c), widths[c], align[c])).collect();
        format!("| {} |", cells.join(" | "))
    };
    let delimiter: Vec<String> = (0..cols)
        .map(|c| {
            let w = widths[c];
            match align[c] {
                Align::None => "-".repeat(w),
                Align::Left => format!(":{}", "-".repeat(w - 1)),
                Align::Right => format!("{}:", "-".repeat(w - 1)),
                Align::Center => format!(":{}:", "-".repeat(w - 2)),
            }
        })
        .collect();
    let mut lines = Vec::with_capacity(rows.len() + 1);
    let mut rows = rows.iter();
    lines.push(format_row(rows.next().unwrap_or(&Vec::new())));
    lines.push(format!("| {} |", delimiter.join(" | ")));
    lines.extend(rows.map(format_row));
    lines
}

fn pad(text: &str, width: usize, align: Align) -> String {
    let gap = width.saturating_sub(text.width());
    match align {
        Align::Right => format!("{}{text}", " ".repeat(gap)),
        Align::Center => format!("{}{text}{}", " ".repeat(gap / 2), " ".repeat(gap - gap / 2)),
        Align::None | Align::Left => format!("{text}{}", " ".repeat(gap)),
    }
}
