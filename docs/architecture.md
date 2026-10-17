# typo architecture

## Layers

```text
src/md/      pure Rust, no Qt: document model, parsers, serializer, edits,
               undo, HTML export, outline, stats, search. All unit-tested.
src/palette.rs Omarchy theme reading and colour derivation (pure Rust).
src/qt/        cxx-qt bridges: Editor (QAbstractListModel over blocks),
               InlineHighlighter, Theme and Desktop (QML singletons).
cpp/           the only hand-written C++: typo_glue.h, a thin adapter over Qt
               APIs cxx cannot reach (QSyntaxHighlighter internals, synthetic
               key events). No logic lives here.
qml/           layout and interaction: window, menus, editor delegates,
               sidebar, footer, dialogs, preferences. Ui and Prefs are QML
               singletons persisted with QtCore Settings in
               ~/.config/typo/typo.conf.
```

## App shell

- `Main.qml` holds the `Editor` model and a single `run(cmd, arg)` that every
  menu item, shortcut, footer button and sidebar action calls. Unbuilt
  commands show a "Not available yet" toast, so the menus already mirror
  Typora's in full.
- Controls use the Fusion style with the window palette filled from `Theme`.
  Fusion takes every colour from the palette, so stock controls follow the
  Omarchy theme. `TMenu`, `TMenuItem` and `TSep` draw Typora-style menus:
  shortcut on the right, check marks, sized to their widest entry.
- `Editor` roles per row: `kind`, `text`, `level`, `info` (code language),
  `decor` (container decorations as JSON: quote bars, list markers with
  ordinals and bullets, task state, footnote labels), `rows` (table cells as
  JSON), `blockId` and `gap`. Rust computes `gap` as the collapsed CSS margin
  above the block (paragraph 0.8em, headings 1rem, none inside a tight list),
  so QML does no layout logic.
- Text lines use a fixed pixel height, like CSS `line-height` (1.6 for text,
  1.2 or 1.4 for headings), set through `QTextBlockFormat` in the glue.
  Qt's proportional height would scale the font's own height (about 1.36em).
- The writing column follows Typora's default theme: 860px wide (1024px
  from a 1400px window, 1200px from 1800px), less 30px padding on each side.

## Document model

A `Document` is a flat `Vec<Block>` of **leaf blocks**. Container structure
(block quotes, list items, footnote definitions) is carried on each leaf as a
path of `Container`s, outermost first. Each container instance has an id, so
consecutive leaves that share an id belong to the same quote or list item.
For example:

```markdown
- a          leaf "a"       [Item#1]
  more       leaf "more"    [Item#1]          (second paragraph of item 1)
  - nested   leaf "nested"  [Item#1, Item#2]
- c          leaf "c"       [Item#3]
```

This flat shape maps directly onto a QML list of delegates, keeps edits
simple (split / merge / retype a leaf, push / pop a container), and still
serializes back to nested markdown. A delegate draws its own decorations: a
bullet or number for every container level where the leaf is the item's first
leaf, quote bars for every quote level, and indentation per level.

Leaf kinds: Paragraph, Heading(1–6, atx/setext), CodeFence(info, fence),
IndentedCode, Math, Html, Table(alignments, rows of cells), ThematicBreak,
FrontMatter, Toc, LinkRefDef. Footnote definitions are containers, like
quotes and list items. A list item's task state lives on its container.

Text-bearing leaves hold their **inline markdown source** with container
prefixes stripped (so the TextEdit shows exactly the inline source). Soft line
breaks are `\n`.

The block parser is our own line-based CommonMark-style parser (container
matching, then leaf starts), so it can emit stripped leaf text, preserve
spacing and markers for round-trips, and support Typora's extensions
(`$$` math blocks, `[toc]`, front matter, alerts). pulldown-cmark is a
**dev-dependency only**, used as an oracle in tests.

The serializer is deterministic and aims for byte-identical round-trips of
ordinary documents. Each leaf keeps its source lines (container prefixes
stripped, lazy continuation lines marked) until it is edited, and the blank
lines before it with the container depth each was written at (`>` inside a
quote). List items keep their marker, indentation and padding. Container
prefixes are always rebuilt, so renumbering and re-nesting just work. A
generated block escapes any line that would otherwise start a different
block (`\# not a heading`) and adds the blank lines markdown needs to keep
neighbours apart.

Tests: unit tests per construct; byte-exact round trips of a canonical
corpus and of Typora's own help documents; and a pulldown-cmark oracle. Every
document, regenerated from the model alone, must render to the same HTML as
the source. That runs over the 717 CommonMark/GFM examples shipped in the
pulldown-cmark crate (693 also round-trip byte for byte).

`md::inline` parses inline markdown into a node tree. It implements the
CommonMark delimiter and bracket algorithm plus Typora's extensions
(`==mark==`, `^sup^`, `~sub~`, `$math$`, `:emoji:`, `[^n]`, bare URLs, paired
inline HTML tags such as `<u>`). Each node keeps its full source `range` and
its `content` range; the difference is the syntax markers the editor hides.
The optional syntax follows Typora's preferences, which ship with math, sub,
sup and highlight off. `md::html` renders a document in pulldown-cmark's HTML
shape. With extensions off, the corpus, Typora's help documents and 698 of
the 717 spec examples render identically. The remaining examples are
multi-line link reference definitions and points where pulldown-cmark and
cmark-gfm disagree.

## Inline live preview

`InlineHighlighter` (Rust, subclassing `TypoHighlighterBase`, which is a
`QSyntaxHighlighter`) parses a block's inline source with `md::inline` and
applies character formats: weight, italics, monospace, colours, sizes. It
parses the whole `TextEdit` text, not one `QTextBlock`, because a span can
cross a soft line break, and caches the runs until the text changes. Colours,
and each document's link definitions and inline options (by `Editor.docId`,
since the Mac's windows share a process), live in a thread-local
(`qt::shared`) that `Theme` and `Editor` publish to; a change bumps
`Theme.revision` or `Editor.inlineRevision`, and each highlighter refreshes.
Line height is a fixed pixel height per block (`QTextBlockFormat`), since
Qt's proportional height scales the font's height, not the em. Syntax
markers (`**`, `](url)`, …) are formatted 1px, transparent, with negative
letter spacing, so they take no room, unless the caret is inside or at the
edge of the span. In that case they show in a muted colour, as Typora does.
Moving the caret only re-applies formats; it never changes the text, so undo,
IME and selection are unaffected.

Inline images and inline math need visual space that a character format
can't provide. They render as overlay items positioned at their source range
(`positionToRectangle`). The hidden source span reserves the room with
letter spacing on one transparent glyph.

## Code fences

`src/code.rs` (no Qt) highlights fences with syntect and bat's grammars
(the `two-face` crate). TextMate scopes map to a small set of tokens, each
coloured from the Omarchy palette the way Omarchy's Helix template colours
syntax. `Highlight` keeps each line's parser state, so an edit re-parses
from the changed line until the state settles. `CodeHighlighter` paints a
fence's `TextEdit` line by line from it; each line's Qt block state hashes
how the next line looks, so Qt repaints exactly the lines that changed.
The language box under a focused fence edits the info string through
`editor.setInfo`, one undo step that leaves the caret alone; a block's
`controlAt(x, y)` lets the editor-wide pointer hand it clicks. Shift+Tab
outdents by the code indent size, or, with "Use Shift+Tab to auto indent
selected code", gives each line the indent of the line above:
`CodeHighlighter.autoIndent` runs Source Code Mode's `source::reindent`
over the fence's lines (`indent_lines`, shared with `indentAt`). Code
Tools ▸ Auto Indent Selected Code does the same, and Auto Indent Whole
Code does every line, the selection keeping its lines and columns; out of
code both outdent, as Typora's.

## Math

`src/math.rs` (no Qt) typesets LaTeX in process: the mitex crate converts
it to Typst, and typst lays that out and draws it with its bundled math
fonts. mitex's output calls functions from mitex's own Typst package; the
three files that define them are vendored under `third_party/mitex` and
served to typst from memory, with the converted formula as one more file.
mitex builds its command table from those same files when it compiles,
which is why the build needs the `typst` command (the `generate` feature),
and why the vendored files and the git tag in `Cargo.toml` move together.

QML shows a formula as `image://math/<d|i>/<em px>/<rrggbb>/<TeX>`. The
provider is C++ (`cpp/typo_math.cpp`), registered on the engine before
Main.qml loads, and Qt runs it on its image threads; it calls
`qt::math::render_math` across the glue bridge. A formula that fails
returns one clear pixel and leaves its message for `Desktop.mathError`.
`MathView.qml` loads each new image off screen and swaps it in when ready,
so typing doesn't flicker, and keeps the last good formula above the error
in the editing preview, as Typora does. A math block shows only the
formula; focusing it unfolds the source between `$$` lines (highlighted as
LaTeX) with the preview under it, and a field beside each `$$` for the
`{tag}` it can carry (`$$ {#eq:one}`, a pandoc-crossref label). As in
Typora, a `$$` line opens a block only when a closing one follows. A ```` ```math ```` code block shows its
formula the same way, folded when the caret is elsewhere, and `\[ \]`
blocks and `\( \)` / `\[ \]` inline math keep their delimiters
(Preferences: "LaTeX Math Delimiter", on by default, and "Code Block
Math"; turning the first one over parses the document again as one undo
step). mitex knows fewer AMS environments than MathJax: `known_environments`
writes `flalign`, `alignat`, `eqnarray` and the starred `gather` and
`multline` as the nearest `align` or `gather`. A formula wider than the
page scrolls sideways rather than shrinking.

Equation numbers follow MathJax under Typora's "Auto Numbering for Math
Equations": `src/eqnum.rs` reads every display formula in document order
and says what each row shows at its right — `(n)`, a `\tag` — and what each
`\label` names. `Editor.mathTags` hands that to QML as JSON;
`EditorView.updateMath` keeps it current on every change, MathView draws
each row's tag at the page's right edge, and `Ui.mathTex` writes `\eqref`
and `\ref` out as text before a formula is typeset (`???` for an unknown
label). Rows are spaced evenly down the formula, not measured.

Copy as MathML converts with math-core (no typst involved), the document's
numbers written into the TeX as `\tag*` (`eqnum::tagged`) because math-core
numbers each formula from one; Copy as Image typesets the formula in black
at twice the text size onto the clipboard; Copy as Tex is the source.

## Diagrams

`src/diagram.rs` (no Qt) draws ```` ```mermaid ````, ```` ```flow ```` and
```` ```sequence ```` fences with the merman crate, which parses, lays out
and rasterises Mermaid in process. It is built without its ELK layouts
(EPL-2.0): every diagram family, the Cytoscape layouts, SVG and PNG.
Typora's other two kinds are other libraries' languages (flowchart.js and
js-sequence-diagrams); `flow_to_mermaid` and `sequence_to_mermaid`
translate them line by line, giving every symbol and participant an id
because their names may hold spaces or Mermaid keywords. Colours come from
the page through Mermaid's `base` theme — nodes a tint of the accent with
accent borders, as Typora's themes feed `--node-fill` and friends to
Mermaid — and the root's white background is made transparent. The
renderer has no fontconfig, so the font list names Arial's metric twin,
Liberation Sans, before `sans-serif`. Errors read as Typora's:
`ERROR: [Mermaid] …`, `[Flowchart]`, `[Sequence]`.

QML asks for
`image://diagram/<lang>/<pixels per point>/<gantt width>/<colours>/<source>`;
the provider in `cpp/typo_math.cpp` calls `qt::diagram::render_diagram` on
Qt's image threads and decodes the PNG, and a failure is one clear pixel
with its message for `Desktop.diagramError`. `DiagramView.qml` swaps each
drawing in when it is ready, centred and shrunk to the width (Typora's
`max-width: 100%`), with "Painting Diagram..." while there is none and the
error under it. A diagram fence folds to its drawing when the caret is
elsewhere; with the caret in it, the code shows with the drawing in a
bordered panel 10px below, as Typora's focused `md-diagram-panel`
(Preferences: "Diagrams", on by default). A top-level gantt chart's panel
takes the window's width while the sidebar is closed (98% folded, less
.8em a side and at most 1140px open), and Mermaid draws the chart to it
with Typora's paddings (75 left, 20 right).

## HTML

Typora renders HTML blocks in its page; Qt has no browser engine in the
editor, but `Text`'s rich text takes a subset of HTML 4 and CSS.
`src/htmlview.rs` parses the block with `scraper` and writes that subset:
known tags with a whitelist of attributes, block-level unknowns as `div`,
the rest as `span`, scripts and forms dropped. What Qt lacks is rewritten:
`<mark>` and `<kbd>` as spans with the theme's colours, `<details>` as a
summary line linking to `typo-details:N` (the delegate keeps which ones the
reader toggled, and the page is redrawn), `<svg>` as a data URL picture,
`<video>`, `<audio>` and iframes as links to their sources, `<img>` resolved
by Typora's image rules and sized to the page. Qt ignores `linkColor` for
rich text, so links carry the theme's colour in their style.

`htmlview::look` decides how a block shows away from the caret: rendered;
the source, faint (`<script>`, `<style>` and the other tags whose first tag
Typora won't draw); or, for a block that is only comments, the comment's
text in the comment colour. The parser keeps CommonMark's HTML block
boundaries, which the oracle checks, rather than Typora's.

Inline HTML stays source in the text, as in Typora: tags faint and
monospace, `<!-- comments -->` in the theme's yellow, and the text between a
pair of tags in their look (`highlighter::tag_look`): bold, underline,
`<mark>`, `<kbd>` and the like, colours from a `style` or `<font color>`.

## Footnotes

`md::query::footnotes` lists each footnote's label, its references (row and
UTF-16 range, with the table cell for one in a cell) and its definition's rows: the definition is
the first one, its blocks those after it in the same container. The editor
publishes the list as `Editor.footnotes` with the stats, so it trails edits
by a moment; EditorView keeps it by row, or `"row:r:c"` for a
cell (`footnoteRefs`, `noteEdit`), for hover and Ctrl+click. The tooltip renders the definition's blocks, with the link
reference definitions, through `md::query::footnote_doc` and the HTML
writer. A definition's `[^x] :` label is drawn by the delegate in room the
text leaves on its first line (`Highlighter.firstIndent` sets the first
block's text indent), so wrapped lines run under the label as in Typora.

## TOC, alerts, link definitions

A TOC block draws `Editor.outline`, whose entries carry each heading's
inline HTML. A click beside the entries selects the block as a whole (a
one-row span from 0 to 1); `edit::delete_range` removes an atomic block so
selected. An alert is a quote whose first paragraph opens with `[!KIND]`
alone on its line (`md::query::alert_of`): the decorations mark the quote
container with its kind, the delegate colours the bar from
`Theme.alertColor` and draws the title in the first line's indent, as it
does a footnote's label, and the highlighter hides the marker unless the
caret is on it. Link reference definitions are edited in a text field with
their own runs (`highlighter::link_def_looks`) instead of inline markdown;
an edit that breaks one makes it a paragraph.

Emoji suggestions (`qml/EmojiSuggest.qml`) belong to the view, like the
footnote tooltip. A text block's edit, or a table cell's, asks it to look
at the word before the caret (`:` then word characters, with only word
characters or colons up to the next space after it);
`Editor.emojiSuggestions` lists the matches in Typora's order
(`md::query::emoji_suggestions`, over the `emojis` crate's shortcodes).
The view hands it keys first while it is
shown, and choosing writes `:shortcode:` into the text field, so the edit
goes through `edited` and undo like typing.

An emoji shortcode away from the caret is an inline object like an image
or formula (`highlighter::objects`, kind "emoji"): the highlighter leaves
room the width of the emoji in the text font, and `InlineObject` draws the
emoji there as text.

## Images

A paragraph that is one image (`md::inline::lone_image`) shows the image
centred at its own size, never wider than the page, as Typora's
`p>.md-image:only-child img` rule does. `Editor.imageUrl` resolves the
source with Typora's rules (`src/image.rs`): web and data URLs load as
written, `//host/…` becomes `http://host/…`, percent escapes are undone the
way Typora undoes them, and a local path resolves against the folder the
front matter's `typora-root-url` names, or else the document's folder. The
QML `Image` loads asynchronously; once it is ready the highlighter's
`imageShown` hides the whole source while the caret is elsewhere. With the
caret inside, the source shows above the image behind an image icon; an
image that is loading or failed keeps its source, with a broken-image icon
and Typora's "image is loading …" / "image load failed" hint.

Files come in through Insert Local Images, a drop or a paste, and all go to
`Editor.insertFiles` (or `filesMarkup` in a table cell). `image::storage`
picks the folder images are copied to: the front matter's
`typora-copy-images-to`, else the Image preference (Main.qml passes the
prefs with `setImageOptions`). A local image not already in that folder is
copied there under a free name; the source written is relative to the
document (or the root URL) when the preference says so. When the folder
doesn't exist, `missingImageFolder` names it and Main.qml asks before
`insertFiles(…, create)` makes it. A pasted picture is saved by the C++
glue into the storage folder (`pastedImageFolder`), or into
`~/.config/typo/typo-user-images` when there is none, and then inserted
like any file. Drops follow Typora's order: HTML, then files (a folder
opens in the sidebar, a markdown file opens), then text.

An `<img>` tag with a source is an image too (`md::imgtag`, inline
`Kind::ImgTag`): its width and height come from its style, else its
attributes, and its `zoom` from the style; `Ui.imageSize` turns them into
a size on the page, at most the line wide. A line that is one `<img>`
stays a paragraph, so it shows centred like a markdown image. GIFs and
WebPs play through `AnimatedImage`.

The image commands (Format ▸ Image and the context menu's image group)
work on the image at the caret: `Editor.imageAt` describes it (source,
URL, file, syntax, the storage folder's name when the image isn't in it,
and its file's pixels per point, read from a PNG's pHYs or a JPEG's JFIF
density as Typora does, which offers Zoom Image to N% for its own size).
The edits are text edits on its block: Zoom Image rewrites the style
(a markdown image becomes a tag first), Switch Image Syntax rewrites the
node, and the file commands copy or move the file and repoint the image
(`relocateImage`, `relocateAllImages`, which also goes through table
cells), each one undo step. Deleting an
image's file sends it to the trash (`Editor.trashImageFile`), keeping a
copy in the temp folder tied to the step that took the image out
(`History::keep_file`): undoing that step copies the file back, redoing it
deletes it again, as Typora's `restoreMedia` / `deleteMedia`. When Insert
Local Image writes the front matter's `typora-copy-images-to` and
`typora-root-url` (`edit::set_property`). Reload All Images gives every
picture's URL a new fragment (`Ui.reloaded`), which misses Qt's cache.
When a picture loads in the block with the caret (`EditorView.imageLoaded`)
and the caret is in its source, Main offers its commands in
`qml/ImagePopup.qml`, under the caret, unless it offered for that URL
last or the picture is one a copy or move just pointed at; moving the
caret out of the image closes it. While it is shown it takes Up, Down,
Enter and Escape from the editor's keys (`EditorView.popup`, which a
cell's keys check too), as the emoji list does.

## Inline objects

Images inside a line and inline math (`$…$`) are drawn by QML over room
the text leaves for them, since a `TextEdit` can only show its own text
and that text must stay the markdown source.

- **Strut fonts.** `src/strut.rs` builds 21 tiny fonts, "Typo Strut 0" to
  "Typo Strut 20", loaded at startup. Font *n* has an ascent of *n*/20 em,
  a descent of the rest, and maps every character to one empty glyph with
  no advance. Set at the right pixel size, one character in font *n*
  reaches exactly as far above and below the baseline as an object does;
  absolute letter spacing makes it as wide.
- **The highlighter** lists a block's objects (`objects`, JSON of kind and
  source; a paragraph that is one image is not one) and, for each whose
  size QML has reported (`setObjectSize`), hides the source but for its
  first character, which it sets in the strut font (`applyStrut`). Lines
  holding one switch from a fixed to a minimum line height (`placing`), so
  they grow to fit it. `objectAnchor` reads the carrier's x and the
  baseline's y from the text layout (`anchorAt`).
- **`InlineObject.qml`**, one per object, keyed by place so editing one
  doesn't remake the others, loads the picture, reports its size and draws
  it at the anchor after each layout (`formatted` bumps the block's
  `placeRevision`). Images keep their own size at 100% zoom, at most the
  line's width, centred on the middle of the lowercase letters with 2px
  and 4px of border, as Typora's `vertical-align: middle` and img borders
  do. Formulas are typeset with their ink as the line box and a dot no one
  sees after them; the dot's position is the baseline
  (`Desktop.mathBaseline`).
- **With the caret in an object** its source shows, as in Typora. An image
  stays after it: the source's last character carries the room, and QML
  draws that character again before the picture. A formula shows its TeX
  in the theme's magenta with a preview popover under the line, or the
  error when it doesn't typeset.

## Tables

A table is one block in the document (`BlockKind::Table`: alignments and
rows of cell text, row 0 the header) and stays atomic to the rest of the
editor: positions 0 before it and 1 after it, deleted and selected whole
across blocks. Its cells are edited on their own.

- **Model.** `md::table` makes Typora's edits as pure functions on the
  document: typing in a cell (line breaks become spaces), add, delete and
  move rows and columns, alignment toggles, resize, prettify. Each returns
  the cell the caret goes to. `Editor.cellEdited` and `Editor.tableCommand`
  commit them; typing merges per cell (`Merge::Cell`).
- **Caret positions.** The undo history needs to put the caret back in a
  cell, so a table's positions go on past 1: from 2, one per cell, 4096 to
  a row (`table::caret`). The view's `focusRow` hands any position in a
  table to `TableBlock.place`, which reads them back; 0 is the first cell
  and 1 the end of the last, so arrows and Backspace into a table land
  where Typora puts them.
- **`TableBlock.qml`** lays out a `TextEdit` plus `InlineHighlighter` per
  cell, sized as a browser lays out Typora's `table { width: 100% }`:
  columns get their widest content, the rest of the width is shared in
  proportion, and when there isn't room each gets at least its longest
  word and a share of what is left. Rows are as tall as their tallest
  cell. Its `editItem` is the cell with the caret, so the view's caret
  code (reveal, page keys) works in cells, while `EditorView.editAt`
  returns null for a table so selections and commands treat it as a block.
- **Keys and clicks.** Cells take Tab/Shift+Tab (next and previous cell,
  selected; past the last a new row), Enter (the cell below, or out of
  the table), Shift+Enter (`<br />`), Ctrl+Enter (row below), Alt+arrows
  (move row or column), Ctrl+Shift+Backspace (delete row) and arrows at
  their edges. Undo, page keys and the menu key go on to the view. `hit`
  returns the cell under the pointer; a drag that leaves or enters the
  table takes it whole. Copy, paste, select all and inline styles act in
  the cell (`EditorView.tableAt`). So do links, footnotes and the image
  commands: `TableBlock.format` gets the new text from `Editor.editText`
  (`edit::text_command`) and replaces only what changed, so the cell's
  text stays bound to the model's for undo. For an image in a cell, Main
  reads it with `Editor.cellImageAt`, copies its file with
  `Editor.copyImageFile` and repoints or deletes it there; a cell's
  picture loading offers the image popup as a paragraph's does.
- **Several cells.** A drag from one cell into another, or Shift+click
  from the caret's cell, makes `TableBlock.cellSpan` ({a, z}): Typora's
  (the browser's) selection, in reading order, each cell's part selected
  in its own `TextEdit`, the caret in `z`'s. Copy gives
  `table::span_markdown` (a ragged table, as Typora copies it); typing,
  Backspace, Delete, Cut and Paste go through `table::replace_span`, which
  empties what it takes from each cell and keeps every row and column.
  The caret going to another block, a change to the table, or any other
  key ends it; an arrow puts the caret at the end it points to (Up and
  Down then move on from there). The footer counts a table's selection,
  in one cell or several, through `Editor.cellSpanCounts`
  (`table::span_cells`).
- **Toolbar.** While a cell has the caret, Resize Table (a 6×10 board and
  size fields), the column's alignment, More Actions (the Table menu) and
  Delete Table sit above the table. Clicks reach them through
  `controlAt`, which the view also asks of the block after the one under
  the pointer, since the toolbar overlaps it.

## Editing flow

- Each text leaf is a `TextEdit` in a QML delegate. Typing goes straight into
  it; `onTextChanged` reports `(row, text, caret)` to `EditorModel`, which
  updates the Rust document, records undo, and applies input rules (for example
  `## ` turns the leaf into a heading). Structural results come back as model
  changes plus a `caretRequested(row, pos)` signal.
- While the model tells views about a change, its `syncing` flag makes
  `edited` a no-op: delegates' text items echo the new (or, mid-update, a
  stale) text back. Format-only changes (highlighting, line height) also
  emit `textChanged`, so the delegate passes on only a change of text.
- Keys with structural meaning (Enter, Shift+Enter, Backspace at start,
  Delete at end, Tab/Shift+Tab, arrows at block edges, Ctrl+Home/End,
  Ctrl+Z/Y) are intercepted in `EditorView.keyPressed` and become
  `Editor.command(name, anchor row/pos, caret row/pos, arg, text)`. A command
  returns false when the text field's default applies (Backspace inside a
  line, Enter inside code). Menu items and their shortcuts (formatting,
  headings, lists, blocks) run the same commands through `app.run`. Each
  command is one undo step and ends with `caretRequested`.
- An editor-wide `MouseArea` (`pointer`) sits over the blocks and handles
  click, drag (autoscrolling at the edges), double- and triple-click,
  Shift+click, Ctrl+click on links, clicks on task checkboxes and the right
  button, which opens `ContextMenu.qml` (so do the Menu key and Shift+F10,
  at the caret). The menu reads the caret's block kind and the inline spans
  around it (`editor.contextAt`) to decide which rows and entries to show,
  and runs everything through `app.run`. `hit(x, y)`
  finds the row by binary search and aims at the middle of the line, since
  `positionAt` above the glyphs returns the line start.
- A selection inside one block is the `TextEdit`'s own. One that crosses
  blocks is `EditorView.span` (anchor and caret as row/position): each block
  shows its part as a persistent selection, atomic blocks (rule, table, TOC)
  are tinted, and the caret's block has focus. Typing, Enter, Backspace and
  Delete over a span run `typeText`/`deleteSelection`, which call
  `md::edit::replace_range`. Several-block results come back through
  `selectionRequested`.
- Copy writes markdown as `text/plain` and HTML as `text/html`, both from
  `md::edit::slice`. The slice cuts the first and last blocks and keeps the
  markers of any span the selection cuts through, as Typora does (selecting
  `bo` inside `**bold**` copies `**bo**`). Paste takes the plain text as
  markdown; html→markdown conversion is still to come.
- Undo/redo is document-level in Rust: snapshots of the leaf list plus caret,
  with time- and kind-based coalescing.

## Theme

`src/palette.rs` reads a theme's `colors.toml` and resolves names the way
`omarchy-theme-color` does: legacy `color0`–`color15`, short names, and
derived shades mixed from the base colours. It decides light or dark from
`mode`, else from the background. `Scheme` maps the palette to semantic
colours following Omarchy's own Obsidian template: headings
red/green/yellow/blue/magenta, links blue, code cyan on the lighter
background, quote bars in the accent, marks in yellow.

The `Theme` QML singleton exposes them as properties. QML polls it once a
second; a changed `colors.toml` (mtime, size or inode) under
`~/.local/state/omarchy/current/theme` reloads it. A theme switch replaces
that directory, so polling is simpler than re-arming a watcher. The Themes
menu lists `/usr/share/omarchy/themes` and `~/.config/omarchy/themes`; a
choice there applies to typo only. Body text uses Open Sans, as Typora's
default theme does, falling back to Noto Sans. Code uses `monospace`, which
fontconfig resolves to the Omarchy font.

## Files

- `src/file.rs` (no Qt) reads and writes documents. It decodes by byte order
  mark, then as UTF-8 when the bytes are valid, then by chardetng's guess,
  and saves back in the same encoding with the same mark (encoding_rs only
  decodes UTF-16, so that is encoded by hand). Writes go to a temporary file
  beside the target and are renamed over it, keeping its permissions.
- A `Stamp` (mtime, length, hash) records the file as typo last saw it. QML
  calls `editor.checkDisk()` every 1.5s; a changed time or size reads the
  file and compares hashes, so a touch alone is no change. An unedited
  document reloads silently; an edited one asks first, and the reload is
  one undo step, as in Typora.
- Open and save dialogs go through xdg-desktop-portal's FileChooser
  (`src/portal.rs`, zbus on a worker thread). The answer comes back to the
  GUI thread through cxx-qt's `Threading` as `Desktop.fileChosen`. Scripts
  queue answers with `Desktop.fakeChoice`, so tests never open a chooser.
- On Linux every window is its own `typo` process (`Desktop.spawn`), as
  Typora's windows are independent. On the Mac they share one process (one
  Dock icon), each on a `QQmlApplicationEngine` of its own
  (`typoOpenWindow`), so the QML singletons are still per window; the
  engine goes when its window closes. The Mac's typo stays open with no
  window, and a click on the Dock icon opens one, unless "Quit typo when
  last window is closed" is set; the next launch doesn't bring back a
  window closed while typo stayed open. Each window has its own session file
  (`<pid>` or `<pid>.<n>`), and Quit tells every window. New and Open reuse the window only when it holds
  an untitled, unedited document. Preferences live in the shared
  settings file: a window writes only the keys it changed and reads the
  file back every second, so a change reaches every window. Recent and
  closed files are read back before each change too.
- Closing an edited document asks Save / Discard / Cancel (`TDialog.qml`),
  unless auto save is on and the file exists. While a document is edited, a
  draft is written to `$XDG_DATA_HOME/typo/drafts` every 30 seconds and
  before the close prompt; saving or closing cleanly removes it. Drafts are
  kept for 30 days.

## Sidebar

- `src/folder.rs` (no Qt) is the folder as Typora shows it: the files it
  lists (Typora's fifteen markdown and text extensions, nothing hidden,
  symlinked folders not followed), the articles walk (the root's files,
  then each subfolder's depth first, sorted per folder or all together,
  giving up past 200 files as Typora does), the tree's `children` (folders
  first), summaries (the visible text of the first 4KB through `md::parse`
  and `query::visible_text`), Typora's natural order (digit runs by value
  after words, month names by month, case ignored), names for New File /
  New Folder (`Untitled N`, one past the highest) and Duplicate
  (`name (copy).ext`), the folder search (`query::matcher`, so the same
  case / word / regex options as Find; most matches first, then files whose
  name alone matches; lines cut around the first match, ranges in UTF-16)
  and Quick Open's ranking (every word in the path; name starts with, name
  contains, words in the name, path only).
- `qt::folder` is the `Folder` singleton: those as JSON for QML, plus
  create / rename (Typora's character rules and its "File with same name
  already exists" refusal) / duplicate. `watch(root)` sets up a recursive
  `notify` watch on a worker thread (a big tree takes a while) and hands it
  back through `Threading`; a generation number drops one that finishes
  after the folder changed again. Events on hidden paths or unlisted files
  are ignored, and a burst becomes one `changed` signal 250ms after it
  settles.
- `Sidebar.qml` owns the tabs (Files | Outline, or Files | Search while
  searching), the footer (New File, the folder menu with Typora's sort
  orders and recent locations, `Prefs.pinnedFolders` first and the open
  folder last, list/tree switch) and every file operation,
  so the views share them: `open` replaces this window's document after
  `Main.openInPlace` settles unsaved changes (Typora opens sidebar files in
  place), new files open at once and are named in place (`RenameField`),
  renaming the open file or a folder above it moves the document's path,
  and Delete asks, trashes, and opens the next file. One `TMenu` is
  Typora's file menu, its entries shown per kind (file, folder, tree root,
  empty space, search result) as Typora's `for-file` / `for-folder` /
  `for-search` rules have it.
- The views reload from `Folder` on `changed` and keep their place:
  `Sidebar.sync` updates their `ListModel`s in place. `FileList` cards read
  their summaries lazily (only built delegates ask); dates are Typora's
  `date2NaturalLang`. `FileTree` flattens the open folders into rows and
  opens the folders down to the file being edited. `FolderSearch` searches
  300ms after typing stops. `OutlineView` nests headings under the nearest
  smaller level, flat or with fold carets (folds kept by level and title),
  with Typora's outline menu and filter. `QuickOpen` (Ctrl+P) lists recent
  folders and files, then matches among recent files and the folder's.
- With no folder open, opening a file mounts its folder, as Typora does. A
  search hit opens its file at the hit's line: `serialize::block_at_line`
  maps a source line to its block from the serializer's line count.

## Find and replace

`md::query::find` searches each block's markdown text and each table cell
(never a rule or a TOC), so a match can sit in syntax the page hides, as
Typora's can. `Editor.findAll` hands QML every match as
`[row, cellRow, cellColumn, start, end]` in UTF-16; `replaceMatch` (one
match, checked again first) and `replaceAll` are one undo step each.
`FindBar.qml` is Typora's search panel over the top of the page. It gives
each block's highlighter its matches through `EditorView.findHits` (`hits`
on `InlineHighlighter` and `CodeHighlighter`): a faint version of the
theme's yellow laid under the text by `addBackground` (C++ glue), which
keeps each character's own format. Hidden markers and the characters that
make room for objects are left untinted. The current match is
`EditorView.found`: the bar selects it in its block without taking the
focus (`showFound`), and the block shows as if it had the caret, so a match
in a link's address shows the link's source and a folded math or diagram
fence opens. When the bar closes, the match becomes the selection, with
the focus (`takeFound`). Matches are counted from the caret, from the last
current one as the text changes, and again from the caret once the focus
returns to the page.

## Source Code Mode

Ctrl+/ swaps the page for `SourceView.qml`: the whole document as markdown
(`Editor.sourceText`, always LF) in one plain `TextEdit`, at most 800 wide
and centred, the current line tinted and its number in the margin, like
Typora's CodeMirror view. `SourceHighlighter` colours it one line at a
time: `source::line` reads a line given the `State` the line before it
left (inside a fence, math block, HTML block or front matter, and whether
it continues a list), which Qt keeps per block and re-highlights onwards
from when it changes. Container markers, headings, rules, fences and link
definitions are read by the block parser's own helpers, and a paragraph's
text by the inline parser, so both views agree on what is syntax. The
theme's palette gives the inks (`Ink.source`); the glue's line-height
pass gives every line Typora's 1.6, and is kept off the text's undo.

Edits reach the document 300ms after typing stops, and before anything
reads it (`Main.settle`: save, close, reload, autosave, the disk check):
`Editor.setSource` parses the text and `history::adopt_ids` gives the
blocks that kept their markdown, at the start and end, their old ids, so
the splice is only what changed and the page keeps its state. Every sync
of one visit merges into a single `Merge::Source` step, sealed on entry
and exit and dropped if the text comes back to where it started. Inside
the view, undo is the `TextEdit`'s own and starts empty, as in Typora.
Commands outside `Main.everywhere` (file, window, view, find) go to
`SourceView.command`, which handles the editing ones and ignores the
formatting ones. Find runs `query::find_in` on the text itself
(`findInText`), its matches painted by the highlighter's `hits`; Replace
All is one edit of the text. Entering and leaving keep the caret's block
and its height in the view (`Editor.rowLine` / `rowOfLine`).

## Focus and typewriter modes

Focus mode (F8) greys everything but what the caret is in, in the theme's
`blur` colour (the text mixed 65% into the background, as Typora's
`--blur-text-color`). The focused block is `EditorView.focusedRow`: find's
current match while the find bar has one, else the caret's block. Text is
greyed by its highlighter, last of all, so the colours under it are kept
for when focus returns: each highlighter holds a `Dim { blur, keep }` and
calls the glue's `dimLine` for every line but `keep`, which recolours a
line's runs except the transparent ones that hide markers. When only the
kept line moves, `dim` re-highlights just the two lines; anything else
re-highlights the lot. A paragraph or heading is greyed whole (`keep` is
-1); a fence, math or HTML source, and Source Code Mode keep the caret's
line; a table greys every row but the caret's. What isn't text (list
markers, checkboxes, alert labels, footnote labels and back links,
pictures, formulas, diagrams) follows `BlockDelegate.dimmed`.

Typewriter mode (F9) pads the page above and below by half the view, less
a line, so the first and last lines can sit in the middle. Typing keeps
the caret's line there: the active text field's `cursorRectangleChanged`
with a changed length centres it (`centreCaret`), and `reveal` centres
instead of scrolling the least it can, unless the pointer is down, so a
click never moves the line under it. Typora's "Always keep caret in middle
of screen" (`Prefs.caretAlwaysCentred`) centres every move, and on
release. Source Code Mode does the same with its one `TextEdit`.

Turning either mode on shows Typora's notice at the top of the page
(`Notice.qml`, and at startup for a mode left on), which "Do not show this
again" silences through `Prefs`. Zoom steps are Chromium's zoom levels,
each 1.2 times the last (`Ui.zoom`, from 1.2^-7 to 1.2^8), and every zoom
shows Typora's zoom panel (`ZoomHint.qml`) for three seconds.

## Export and print

File → Export never goes through a browser. `src/export/tree.rs` nests the
flat blocks into the quotes, alerts, lists and items they sit in, takes
footnote definitions out of the flow (numbered by first reference, table
cells included, a footnote's own references at its first one; unreferenced
ones left out: Typora's `appendFootnote`) and gives headings Typora's anchors. Every writer walks that
tree, so the formats agree:

- HTML (`html.rs`, `css.rs`): Typora's page skeleton (`#write`) under a
  stylesheet written from the theme's `Scheme`, with the editor's sizes;
  math as inline SVG inked `currentColor`, diagrams as SVG, code
  highlighted into `tk-*` spans. "HTML (without Styles)" drops the
  stylesheet and keeps math as TeX, fences as `language-x`, `[TOC]` as
  text. Relative images are rebased when the page goes to another folder.
- PDF and Image (`print.rs`): Typst markup with every piece of text a
  string literal, typeset by typst with the system's fonts (typst-kit) and
  typst's math fonts; the prelude mirrors the editor's metrics in `px`
  that scale with the font. PDF prints at Typora's 13px on A4 with 16mm
  margins, headers and footers with `${title}`-style placeholders; Image is
  one tall page 640px wide at 24px, refused past 38400px, scaled by the
  screen's pixel ratio and encoded by QImage (PNG or JPEG). A dark theme
  prints in `Scheme::paper`, as Typora's dark themes print light.
- pandoc (`pandoc.rs`): Typora hands pandoc its own AST, so typo writes
  pandoc's JSON AST from the tree (tasks as ☐/☒, alerts as pandoc's alert
  divs, footnotes as notes, `[TOC]` as each format's raw TOC) and runs
  `pandoc -f json -s -o OUT -t FORMAT` from the document's folder, with the
  AST version pandoc itself reports. Import runs Typora's arguments
  (`--wrap=none -t gfm-smart+tex_math_dollars-…`) and opens the markdown
  as a new untitled document.

`export::Job` carries a copy of the document and runs on a worker thread;
`Editor.exported` reports back, and `Main.qml` shows Typora's notification
(`Notice.progress`, then `Notice.tell` with Reveal in File Explorer). The
save dialog is the FileChooser portal with the format's filters. Print asks
the Print portal for a page setup (`PreparePrint`), typesets the PDF for
it and passes the file descriptor to `Print`.

## Testing

- `cargo test`: core model, parsers, serializer round-trips, edit operations.
- `bin/shot OUT.png [file] [--cursor=ROW:POS] [--menu=N] [--theme=ID]
  [--sidebar]`: renders the real app offscreen to PNG for visual checks, with
  throwaway settings. `Desktop.screenshot` grabs the whole window, open menus
  included.
- `bin/e2e [--update] [NAME…]`: runs each `tests/e2e/NAME.script` against
  the real app offscreen (`--script=FILE`) and diffs the saved markdown with
  `NAME.md`; a QML warning fails the test. Script keys go through QtTest's
  window-level key and mouse events, so shortcuts and focus behave as on the
  desktop, one key per timer tick. Steps also click and drag at text
  positions, set and save the clipboard, and evaluate QML expressions (see
  `tests/e2e/README.md`).
- `tools/sandbox`: a private headless sway session for end-to-end runs
  (both typo and the reference Typora) without touching the desktop.
  `tools/sandbox keys` drives typo with wtype, modifier chords included.
