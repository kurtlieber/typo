//! Undo/redo for the whole document. Each step stores only the blocks that
//! changed (the slice between the unchanged prefix and suffix), so typing in
//! a large document costs one block per step. Typing in one block within a
//! short pause coalesces into a single step, as in Typora.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::edit::Pos;
use super::model::{Block, ContainerKind, Document, Id, ListItem};

/// A selection: where it started and where the caret is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sel {
    pub anchor: Pos,
    pub caret: Pos,
}

impl Sel {
    pub fn caret(p: Pos) -> Self {
        Sel { anchor: p, caret: p }
    }
}

/// Rows of the block list replaced by an edit, for updating views.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Splice {
    pub at: usize,
    pub removed: usize,
    pub inserted: usize,
}

/// The blocks that differ between two versions of the block list.
pub fn diff(old: &[Block], new: &[Block]) -> Splice {
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let room = old.len().min(new.len()) - prefix;
    let suffix = old.iter().rev().zip(new.iter().rev()).take(room).take_while(|(a, b)| a == b).count();
    Splice { at: prefix, removed: old.len() - prefix - suffix, inserted: new.len() - prefix - suffix }
}

/// How a step may merge with the one before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Merge {
    /// Typing in the block with this id.
    Typing(Id),
    /// Typing in one cell (row, column) of the table with this id.
    Cell(Id, usize, usize),
    /// Editing in Source Code Mode, where each edit replaces the whole
    /// document. A visit is one step, sealed when it ends.
    Source,
    Never,
}

/// Give a freshly parsed block list the ids of the blocks it shares with
/// `old` at either end, so that `diff` finds only the blocks that changed.
/// Source Code Mode reparses the whole document on every edit. A container
/// takes its old id only where that keeps distinct containers distinct.
pub fn adopt_ids(old: &[Block], new: &mut [Block]) {
    let prefix = old.iter().zip(new.iter()).take_while(|(a, b)| alike(a, b)).count();
    let room = old.len().min(new.len()) - prefix;
    let suffix = old.iter().rev().zip(new.iter().rev()).take(room).take_while(|(a, b)| alike(a, b)).count();
    let pairs: Vec<(usize, usize)> =
        (0..prefix).map(|i| (i, i)).chain((1..=suffix).map(|k| (old.len() - k, new.len() - k))).collect();
    // Containers and lists: new id → old.
    let (mut containers, mut lists) = (Ids::default(), Ids::default());
    for &(o, n) in &pairs {
        for (x, y) in old[o].containers.iter().zip(&new[n].containers) {
            containers.adopt(y.id, x.id);
            if let (Some(a), Some(b)) = (x.list_item(), y.list_item()) {
                lists.adopt(b.list, a.list);
            }
        }
    }
    for &(o, n) in &pairs {
        new[n].id = old[o].id;
    }
    for b in new.iter_mut() {
        for c in &mut b.containers {
            c.id = containers.get(c.id);
            if let ContainerKind::ListItem(item) = &mut c.kind {
                item.list = lists.get(item.list);
            }
        }
    }
}

/// Blocks the same but for their ids.
fn alike(a: &Block, b: &Block) -> bool {
    a.kind == b.kind
        && a.text == b.text
        && a.blank_before == b.blank_before
        && a.raw == b.raw
        && a.lazy == b.lazy
        && a.containers.len() == b.containers.len()
        && a.containers.iter().zip(&b.containers).all(|(x, y)| same_kind(&x.kind, &y.kind))
}

/// Container kinds alike but for the list they belong to.
fn same_kind(a: &ContainerKind, b: &ContainerKind) -> bool {
    match (a, b) {
        (ContainerKind::ListItem(x), ContainerKind::ListItem(y)) => ListItem { list: x.list, ..y.clone() } == *x,
        _ => a == b,
    }
}

/// New ids given old ones, one to one.
#[derive(Default)]
struct Ids {
    map: HashMap<Id, Id>,
    taken: HashSet<Id>,
}

impl Ids {
    fn adopt(&mut self, new: Id, old: Id) {
        if !self.map.contains_key(&new) && self.taken.insert(old) {
            self.map.insert(new, old);
        }
    }

    fn get(&self, new: Id) -> Id {
        self.map.get(&new).copied().unwrap_or(new)
    }
}

struct Step {
    at: usize,
    removed: Vec<Block>,
    inserted: Vec<Block>,
    before: Sel,
    after: Sel,
    merge: Merge,
    time: Instant,
    version: u64,
    /// Files the step deleted, as (backup, path), for the editor to put
    /// back when it's undone and delete again when it's redone.
    files: Vec<(PathBuf, PathBuf)>,
}

pub struct History {
    undo: Vec<Step>,
    redo: Vec<Step>,
    next_version: u64,
    saved: u64,
    pub pause: Duration,
    pub limit: usize,
}

impl Default for History {
    fn default() -> Self {
        History { undo: Vec::new(), redo: Vec::new(), next_version: 1, saved: 0, pause: Duration::from_millis(1000), limit: 1000 }
    }
}

impl History {
    /// Record the change from `old` to the document's current blocks.
    /// Returns the rows that changed (none when nothing did).
    pub fn record(&mut self, old: &[Block], doc: &Document, before: Sel, after: Sel, merge: Merge) -> Option<Splice> {
        let s = diff(old, &doc.blocks);
        if s.removed == 0 && s.inserted == 0 {
            return None;
        }
        self.redo.clear();
        let now = Instant::now();
        let version = self.next_version;
        self.next_version += 1;
        // A visit to Source Code Mode becomes one step from the document
        // before it to the latest; one that comes back to where it started
        // leaves none.
        if merge == Merge::Source && self.undo.last().is_some_and(|t| t.merge == Merge::Source) {
            let top = self.undo.last_mut().unwrap();
            let mut base = old.to_vec();
            base.splice(top.at..top.at + top.inserted.len(), top.removed.iter().cloned());
            let d = diff(&base, &doc.blocks);
            let back = base.len() == doc.blocks.len() && base.iter().zip(&doc.blocks).all(|(a, b)| alike(a, b));
            if back {
                self.undo.pop();
            } else {
                top.at = d.at;
                top.removed = base[d.at..d.at + d.removed].to_vec();
                top.inserted = doc.blocks[d.at..d.at + d.inserted].to_vec();
                top.after = after;
                top.time = now;
                top.version = version;
            }
            return Some(s);
        }
        if let Some(top) = self.undo.last_mut() {
            let same_block = top.at == s.at && top.inserted.len() == 1 && s.removed == 1 && s.inserted == 1;
            if merge != Merge::Never && top.merge == merge && same_block && now - top.time < self.pause {
                top.inserted = doc.blocks[s.at..s.at + 1].to_vec();
                top.after = after;
                top.time = now;
                top.version = version;
                return Some(s);
            }
        }
        self.undo.push(Step {
            at: s.at,
            removed: old[s.at..s.at + s.removed].to_vec(),
            inserted: doc.blocks[s.at..s.at + s.inserted].to_vec(),
            before,
            after,
            merge,
            time: now,
            version,
            files: Vec::new(),
        });
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        Some(s)
    }

    /// Stop the next edit from merging into the last step.
    pub fn seal(&mut self) {
        if let Some(top) = self.undo.last_mut() {
            top.merge = Merge::Never;
        }
    }

    /// Tie a file the last step deleted to it, backed up at `backup`.
    pub fn keep_file(&mut self, backup: PathBuf, path: PathBuf) {
        if let Some(top) = self.undo.last_mut() {
            top.files.push((backup, path));
            top.merge = Merge::Never;
        }
    }

    /// The files tied to the step just undone (or redone).
    pub fn files_of_last(&self, redone: bool) -> Vec<(PathBuf, PathBuf)> {
        let step = if redone { self.undo.last() } else { self.redo.last() };
        step.map_or(Vec::new(), |s| s.files.clone())
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self, doc: &mut Document) -> Option<(Splice, Sel)> {
        let mut step = self.undo.pop()?;
        let splice = Splice { at: step.at, removed: step.inserted.len(), inserted: step.removed.len() };
        doc.blocks.splice(step.at..step.at + step.inserted.len(), step.removed.iter().cloned());
        step.merge = Merge::Never;
        let sel = step.before;
        self.redo.push(step);
        Some((splice, sel))
    }

    pub fn redo(&mut self, doc: &mut Document) -> Option<(Splice, Sel)> {
        let step = self.redo.pop()?;
        let splice = Splice { at: step.at, removed: step.removed.len(), inserted: step.inserted.len() };
        doc.blocks.splice(step.at..step.at + step.removed.len(), step.inserted.iter().cloned());
        let sel = step.after;
        self.undo.push(step);
        Some((splice, sel))
    }

    fn version(&self) -> u64 {
        self.undo.last().map_or(0, |s| s.version)
    }

    pub fn mark_saved(&mut self) {
        self.saved = self.version();
    }

    /// Whether the document differs from what was last saved (or loaded).
    pub fn modified(&self) -> bool {
        self.version() != self.saved
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.saved = 0;
    }
}
