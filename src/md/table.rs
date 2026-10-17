//! Table edits: typing in a cell, the Table menu and the table toolbar,
//! as Typora makes them. Row 0 is the header. Each edit returns the cell
//! the caret goes to, or None when it doesn't apply there.

use std::ops::RangeInclusive;

use super::edit::Pos;
use super::model::{Align, BlockKind, Document};

/// A cell: its row (0 for the header) and column.
pub type Cell = (usize, usize);

/// The caret in cell `cell` of the table at `i`, as the editor and its undo
/// history pass it around: a table's positions are 0 before it and 1 after
/// it, as for any block without text, and from 2 its cells, 4096 to a row.
/// The view reads them back the same way (TableBlock.place).
pub fn caret(i: usize, (r, c): Cell) -> Pos {
    Pos::new(i, 2 + r * 4096 + c.min(4095))
}

/// The table block `i`'s alignments and rows.
fn parts(doc: &Document, i: usize) -> Option<(Vec<Align>, Vec<Vec<String>>)> {
    match &doc.blocks.get(i)?.kind {
        BlockKind::Table { align, rows } if !align.is_empty() && !rows.is_empty() => Some((align.clone(), rows.clone())),
        _ => None,
    }
}

fn store(doc: &mut Document, i: usize, align: Vec<Align>, rows: Vec<Vec<String>>) {
    doc.blocks[i].set_kind(BlockKind::Table { align, rows });
}

/// The table's size: columns and rows, the header included.
pub fn size(doc: &Document, i: usize) -> Option<(usize, usize)> {
    parts(doc, i).map(|(align, rows)| (align.len(), rows.len()))
}

/// Cell (`r`, `c`)'s text.
pub fn cell(doc: &Document, i: usize, (r, c): Cell) -> Option<String> {
    parts(doc, i)?.1.get(r)?.get(c).cloned()
}

/// Typing in a cell. A cell is one line: line breaks in pasted text become
/// spaces. False when nothing changed.
pub fn set_cell(doc: &mut Document, i: usize, (r, c): Cell, text: &str) -> bool {
    let Some((align, mut rows)) = parts(doc, i) else { return false };
    let text = text.replace("\r\n", " ").replace(['\n', '\r'], " ");
    match rows.get_mut(r).and_then(|row| row.get_mut(c)) {
        Some(cell) if *cell != text => *cell = text,
        _ => return false,
    }
    store(doc, i, align, rows);
    true
}

/// Add Row Above / Below (Ctrl+Enter): an empty row; the caret goes to its
/// first cell. A row above the header becomes the new header.
pub fn add_row(doc: &mut Document, i: usize, r: usize, above: bool) -> Option<Cell> {
    let (align, mut rows) = parts(doc, i)?;
    let at = if above { r } else { r + 1 }.min(rows.len());
    rows.insert(at, vec![String::new(); align.len()]);
    store(doc, i, align, rows);
    Some((at, 0))
}

/// Add Column Before / After: an empty column, unaligned; the caret goes
/// to its cell in the same row.
pub fn add_column(doc: &mut Document, i: usize, (r, c): Cell, before: bool) -> Option<Cell> {
    let (mut align, mut rows) = parts(doc, i)?;
    let at = if before { c } else { c + 1 }.min(align.len());
    align.insert(at, Align::None);
    for row in &mut rows {
        row.insert(at.min(row.len()), String::new());
    }
    store(doc, i, align, rows);
    Some((r, at))
}

/// Delete Row, when there is another: the row after it moves up into its
/// place (the header's too) and the caret goes there, or to the row before
/// when it was the last.
pub fn delete_row(doc: &mut Document, i: usize, (r, c): Cell) -> Option<Cell> {
    let (align, mut rows) = parts(doc, i)?;
    if rows.len() < 2 || r >= rows.len() {
        return None;
    }
    rows.remove(r);
    let to = r.min(rows.len() - 1);
    store(doc, i, align, rows);
    Some((to, c))
}

/// Delete Column, when there is another; the caret stays in its row, in
/// the column that takes its place or the one before.
pub fn delete_column(doc: &mut Document, i: usize, (r, c): Cell) -> Option<Cell> {
    let (mut align, mut rows) = parts(doc, i)?;
    if align.len() < 2 || c >= align.len() {
        return None;
    }
    align.remove(c);
    for row in &mut rows {
        if c < row.len() {
            row.remove(c);
        }
    }
    let to = c.min(align.len() - 1);
    store(doc, i, align, rows);
    Some((r, to))
}

/// Move Row Up / Down: swap with its neighbour, header included; the caret
/// moves with it.
pub fn move_row(doc: &mut Document, i: usize, (r, c): Cell, up: bool) -> Option<Cell> {
    let (align, mut rows) = parts(doc, i)?;
    let to = if up { r.checked_sub(1)? } else { r + 1 };
    if to >= rows.len() || r >= rows.len() {
        return None;
    }
    rows.swap(r, to);
    store(doc, i, align, rows);
    Some((to, c))
}

/// Move Column Left / Right: swap with its neighbour, alignment and all;
/// the caret moves with it.
pub fn move_column(doc: &mut Document, i: usize, (r, c): Cell, left: bool) -> Option<Cell> {
    let (mut align, mut rows) = parts(doc, i)?;
    let to = if left { c.checked_sub(1)? } else { c + 1 };
    if to >= align.len() || c >= align.len() {
        return None;
    }
    align.swap(c, to);
    for row in &mut rows {
        if to < row.len() && c < row.len() {
            row.swap(c, to);
        }
    }
    store(doc, i, align, rows);
    Some((r, to))
}

/// Where row or column `k` ends up when `from` moves to index `to`.
fn moved(k: usize, from: usize, to: usize) -> usize {
    if k == from {
        to
    } else if from < k && k <= to {
        k - 1
    } else if to <= k && k < from {
        k + 1
    } else {
        k
    }
}

/// Dragging a row by its handle: row `from` goes before row `before` (the
/// number of rows for after the last), the header's place included, as
/// Typora drops it. None when it would stay where it is. The caret's cell
/// moves with its row.
pub fn drag_row(doc: &mut Document, i: usize, from: usize, before: usize, (r, c): Cell) -> Option<Cell> {
    let (align, mut rows) = parts(doc, i)?;
    if from >= rows.len() || before > rows.len() || before == from || before == from + 1 {
        return None;
    }
    let to = if from < before { before - 1 } else { before };
    let row = rows.remove(from);
    rows.insert(to, row);
    store(doc, i, align, rows);
    Some((moved(r, from, to), c))
}

/// Dragging a column by its handle, as `drag_row`; its alignment goes
/// with it, and short rows are filled out first.
pub fn drag_column(doc: &mut Document, i: usize, from: usize, before: usize, (r, c): Cell) -> Option<Cell> {
    let (mut align, mut rows) = parts(doc, i)?;
    let n = align.len();
    if from >= n || before > n || before == from || before == from + 1 {
        return None;
    }
    let to = if from < before { before - 1 } else { before };
    let a = align.remove(from);
    align.insert(to, a);
    for row in &mut rows {
        row.resize(row.len().max(n), String::new());
        let cell = row.remove(from);
        row.insert(to, cell);
    }
    store(doc, i, align, rows);
    Some((r, moved(c, from, to)))
}

/// The toolbar's alignment buttons: each column in `columns` takes `a`, or
/// loses its alignment when it already has `a`.
pub fn align(doc: &mut Document, i: usize, columns: RangeInclusive<usize>, a: Align) -> bool {
    let Some((mut align, rows)) = parts(doc, i) else { return false };
    let mut changed = false;
    for c in columns {
        if let Some(col) = align.get_mut(c) {
            *col = if *col == a { Align::None } else { a };
            changed = true;
        }
    }
    if changed {
        store(doc, i, align, rows);
    }
    changed
}

/// Resize Table: `columns` by `rows` (the header included), keeping what
/// fits, filling the rest with empty cells. The caret stays in its cell or
/// the nearest one left.
pub fn resize(doc: &mut Document, i: usize, columns: usize, rows: usize, (r, c): Cell) -> Option<Cell> {
    let (mut align, mut cells) = parts(doc, i)?;
    let (columns, rows) = (columns.max(1), rows.max(1));
    if (align.len(), cells.len()) == (columns, rows) {
        return None;
    }
    align.resize(columns, Align::None);
    cells.resize(rows, Vec::new());
    for row in &mut cells {
        row.resize(columns, String::new());
    }
    store(doc, i, align, cells);
    Some((r.min(rows - 1), c.min(columns - 1)))
}

/// Prettify Source Code: write the table again with its columns padded
/// to line up, instead of as it was typed. False when it already was.
pub fn prettify(doc: &mut Document, i: usize) -> bool {
    if parts(doc, i).is_none() || doc.blocks[i].raw.is_none() {
        return false;
    }
    doc.blocks[i].clear_raw();
    true
}

/// A position in a table: a cell and a byte offset in its text.
pub type At = (Cell, usize);

/// What a selection over several cells takes, as a drag across them makes
/// the browser's in Typora: from `a` to `z`, either way round, in reading
/// order: the rest of the first cell, every cell after it, the start of
/// the last. Each cell with the byte range it takes.
fn span(rows: &[Vec<String>], a: At, z: At) -> Vec<(Cell, std::ops::Range<usize>)> {
    let (from, to) = if a <= z { (a, z) } else { (z, a) };
    let mut out = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        for (c, text) in row.iter().enumerate() {
            if (r, c) < from.0 || (r, c) > to.0 {
                continue;
            }
            let start = if (r, c) == from.0 { from.1.min(text.len()) } else { 0 };
            let end = if (r, c) == to.0 { to.1.min(text.len()) } else { text.len() };
            out.push(((r, c), start..end.max(start)));
        }
    }
    out
}

/// The text such a selection takes from each cell, in reading order.
pub fn span_cells(doc: &Document, i: usize, a: At, z: At) -> Option<Vec<String>> {
    let (_, rows) = parts(doc, i)?;
    Some(span(&rows, a, z).into_iter().map(|((r, c), range)| rows[r][c][range].to_string()).collect())
}

/// Typing `text` over such a selection, as Typora does: the selected text
/// goes from every cell, each row and column stays, and `text` goes in
/// where the selection started; where it ends is where the caret goes.
/// Backspace and Delete type "".
pub fn replace_span(doc: &mut Document, i: usize, a: At, z: At, text: &str) -> Option<At> {
    let (align, mut rows) = parts(doc, i)?;
    let taken = span(&rows, a, z);
    let (first, at) = taken.first().map(|(cell, range)| (*cell, range.start))?;
    for (cell, range) in taken.iter().rev() {
        rows[cell.0][cell.1].replace_range(range.clone(), "");
    }
    let text = text.replace("\r\n", " ").replace(['\n', '\r'], " ");
    rows[first.0][first.1].insert_str(at, &text);
    store(doc, i, align, rows);
    Some((first, at + text.len()))
}

/// Such a selection as Typora copies it: a markdown table of the rows it
/// takes, each with the text it takes from its cells, the first row as
/// the header (so rows can differ in length).
pub fn span_markdown(doc: &Document, i: usize, a: At, z: At) -> Option<String> {
    let (_, rows) = parts(doc, i)?;
    let mut lines: Vec<(usize, Vec<String>)> = Vec::new();
    for ((r, c), range) in span(&rows, a, z) {
        if lines.last().is_none_or(|(row, _)| *row != r) {
            lines.push((r, Vec::new()));
        }
        lines.last_mut()?.1.push(super::parse::escape_cell(&rows[r][c][range]));
    }
    let line = |cells: &[String]| format!("| {} |\n", cells.join(" | "));
    let head = &lines.first()?.1;
    let mut out = line(head) + &line(&vec!["---".to_string(); head.len()]);
    for (_, cells) in &lines[1..] {
        out += &line(cells);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::md::{parse, serialize};

    fn doc() -> Document {
        parse("| a | b |\n| :-- | --: |\n| 1 | 2 |\n| 3 | 4 |\n")
    }

    fn cells(doc: &Document) -> Vec<Vec<String>> {
        parts(doc, 0).unwrap().1
    }

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|c| c.to_string()).collect()
    }

    #[test]
    fn spans_cells_as_typora() {
        // What Typora did, a drag from b1 after "b" to b3 after "b".
        let d = parse("| a1 | b1 | c1 |\n| --- | --- | --- |\n| a2 | b2 | c2 |\n| a3 | b3 | c3 |\n");
        let (a, z) = (((0, 1), 1), ((2, 1), 1));
        assert_eq!(span_markdown(&d, 0, a, z).unwrap(), "| 1 | c1 |\n| --- | --- |\n| a2 | b2 | c2 |\n| a3 | b |\n");
        let mut piped = d.clone();
        set_cell(&mut piped, 0, (2, 1), "b|3");
        assert_eq!(span_markdown(&piped, 0, ((2, 1), 0), ((2, 1), 9)).unwrap(), "| b\\|3 |\n| --- |\n");
        // Typing over it, either way round: each cell keeps its place.
        for (a, z) in [(a, z), (z, a)] {
            let mut d = d.clone();
            assert_eq!(replace_span(&mut d, 0, a, z, "X"), Some(((0, 1), 2)));
            assert_eq!(cells(&d), [row(&["a1", "bX", ""]), row(&["", "", ""]), row(&["", "3", "c3"])]);
        }
    }

    #[test]
    fn types_in_a_cell() {
        let mut d = doc();
        assert!(set_cell(&mut d, 0, (1, 1), "x|y"));
        assert!(!set_cell(&mut d, 0, (1, 1), "x|y"));
        assert!(set_cell(&mut d, 0, (2, 0), "two\nlines"));
        assert_eq!(cells(&d)[1], row(&["1", "x|y"]));
        assert_eq!(cells(&d)[2], row(&["two lines", "4"]));
        assert!(serialize(&d).contains(r"x\|y"));
        assert!(!set_cell(&mut d, 0, (9, 0), "z"));
    }

    #[test]
    fn adds_rows_and_columns() {
        let mut d = doc();
        assert_eq!(add_row(&mut d, 0, 1, false), Some((2, 0)));
        assert_eq!(cells(&d)[2], row(&["", ""]));
        assert_eq!(add_row(&mut d, 0, 0, true), Some((0, 0)));
        assert_eq!(cells(&d)[0], row(&["", ""]));
        assert_eq!(cells(&d)[1], row(&["a", "b"]));
        let mut d = doc();
        assert_eq!(add_column(&mut d, 0, (2, 0), false), Some((2, 1)));
        assert_eq!(cells(&d)[0], row(&["a", "", "b"]));
        assert_eq!(parts(&d, 0).unwrap().0, [Align::Left, Align::None, Align::Right]);
        assert_eq!(add_column(&mut d, 0, (0, 0), true), Some((0, 0)));
        assert_eq!(cells(&d)[1], row(&["", "1", "", "2"]));
    }

    #[test]
    fn deletes_rows_and_columns() {
        let mut d = doc();
        // The header's row goes and the next row becomes the header.
        assert_eq!(delete_row(&mut d, 0, (0, 1)), Some((0, 1)));
        assert_eq!(cells(&d), [row(&["1", "2"]), row(&["3", "4"])]);
        // The last row: the caret goes to the one before.
        assert_eq!(delete_row(&mut d, 0, (1, 0)), Some((0, 0)));
        assert_eq!(delete_row(&mut d, 0, (0, 0)), None);
        let mut d = doc();
        assert_eq!(delete_column(&mut d, 0, (1, 1)), Some((1, 0)));
        assert_eq!(cells(&d), [row(&["a"]), row(&["1"]), row(&["3"])]);
        assert_eq!(parts(&d, 0).unwrap().0, [Align::Left]);
        assert_eq!(delete_column(&mut d, 0, (1, 0)), None);
    }

    #[test]
    fn moves_rows_and_columns() {
        let mut d = doc();
        assert_eq!(move_row(&mut d, 0, (2, 1), true), Some((1, 1)));
        assert_eq!(cells(&d)[1], row(&["3", "4"]));
        assert_eq!(move_row(&mut d, 0, (2, 1), false), None);
        assert_eq!(move_row(&mut d, 0, (0, 0), true), None);
        assert_eq!(move_column(&mut d, 0, (0, 0), false), Some((0, 1)));
        assert_eq!(cells(&d)[0], row(&["b", "a"]));
        assert_eq!(parts(&d, 0).unwrap().0, [Align::Right, Align::Left]);
        assert_eq!(move_column(&mut d, 0, (0, 0), true), None);
    }

    #[test]
    fn drags_rows_and_columns() {
        let mut d = doc();
        // Row 2 before row 0: it becomes the header; the caret's row moves
        // down with the rest.
        assert_eq!(drag_row(&mut d, 0, 2, 0, (1, 1)), Some((2, 1)));
        assert_eq!(cells(&d), [row(&["3", "4"]), row(&["a", "b"]), row(&["1", "2"])]);
        // The header after the last row.
        assert_eq!(drag_row(&mut d, 0, 0, 3, (0, 0)), Some((2, 0)));
        assert_eq!(cells(&d), [row(&["a", "b"]), row(&["1", "2"]), row(&["3", "4"])]);
        // Before itself or the next: nothing to do.
        assert_eq!(drag_row(&mut d, 0, 1, 1, (0, 0)), None);
        assert_eq!(drag_row(&mut d, 0, 1, 2, (0, 0)), None);
        assert_eq!(drag_row(&mut d, 0, 1, 4, (0, 0)), None);
        let mut d = parse("| a | b | c |\n| :-- | --- | --: |\n| 1 | 2 |\n");
        assert_eq!(drag_column(&mut d, 0, 0, 3, (1, 2)), Some((1, 1)));
        assert_eq!(cells(&d), [row(&["b", "c", "a"]), row(&["2", "", "1"])]);
        assert_eq!(parts(&d, 0).unwrap().0, [Align::None, Align::Right, Align::Left]);
        assert_eq!(drag_column(&mut d, 0, 2, 1, (0, 2)), Some((0, 1)));
        assert_eq!(cells(&d)[0], row(&["b", "a", "c"]));
        assert_eq!(drag_column(&mut d, 0, 2, 2, (0, 0)), None);
        assert_eq!(drag_column(&mut d, 0, 2, 3, (0, 0)), None);
    }

    #[test]
    fn aligns_columns_or_clears_them() {
        let mut d = doc();
        assert!(align(&mut d, 0, 0..=1, Align::Left));
        assert_eq!(parts(&d, 0).unwrap().0, [Align::None, Align::Left]);
        assert!(serialize(&d).contains("| --- | :-- |") || serialize(&d).contains("| --- | :--- |"), "{}", serialize(&d));
    }

    #[test]
    fn prettifies_the_source() {
        let mut d = parse("|a|b|\n|-|:-:|\n|1|22|\n");
        assert_eq!(serialize(&d), "|a|b|\n|-|:-:|\n|1|22|\n");
        assert!(prettify(&mut d, 0));
        assert_eq!(serialize(&d), "| a   |  b  |\n| --- | :-: |\n| 1   | 22  |\n");
        assert!(!prettify(&mut d, 0));
    }

    #[test]
    fn resizes_keeping_what_fits() {
        let mut d = doc();
        assert_eq!(resize(&mut d, 0, 3, 2, (2, 1)), Some((1, 1)));
        assert_eq!(cells(&d), [row(&["a", "b", ""]), row(&["1", "2", ""])]);
        assert_eq!(resize(&mut d, 0, 3, 2, (0, 0)), None);
        assert_eq!(resize(&mut d, 0, 1, 1, (1, 2)), Some((0, 0)));
        assert_eq!(size(&d, 0), Some((1, 1)));
    }
}
