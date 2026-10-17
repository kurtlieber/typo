//! `Editor`: the open document as a list model, one row per leaf block, for
//! the QML editor's delegates. It owns the `md::Document`, its undo history
//! and file, and turns every edit into the smallest row changes.
//! Offsets cross the bridge as UTF-16 positions (what QML's TextEdit uses)
//! and are converted to byte offsets here.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!(<QtCore/QAbstractListModel>);
        type QAbstractListModel;
        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qhash.h");
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
        include!("cxx-qt-lib/qlist.h");
        type QList_i32 = cxx_qt_lib::QList<i32>;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        /// Absolute path of the file, empty when untitled.
        #[qproperty(QString, path)]
        /// File name shown in the title bar.
        #[qproperty(QString, title)]
        #[qproperty(bool, modified)]
        /// Whether the file uses Windows line endings.
        #[qproperty(bool, crlf)]
        /// The file's encoding, as Reopen with Encoding names it.
        #[qproperty(QString, encoding)]
        #[qproperty(bool, can_undo, cxx_name = "canUndo")]
        #[qproperty(bool, can_redo, cxx_name = "canRedo")]
        #[qproperty(i32, words)]
        #[qproperty(i32, characters)]
        #[qproperty(i32, lines)]
        /// The headings as JSON: `[{"row":3,"level":2,"title":"…"}]`.
        #[qproperty(QString, outline)]
        /// The footnotes as JSON, by label in lower case: `{"n": {"label":
        /// "N", "refs": [[row, start, end]], "def": [first, last] | null}}`,
        /// positions in UTF-16.
        #[qproperty(QString, footnotes)]
        /// Bumped when link references or inline syntax options change, so
        /// every block repaints.
        #[qproperty(i32, inline_revision, cxx_name = "inlineRevision")]
        /// Which document the highlighters read (`qt::shared`); the Mac's
        /// windows share a process.
        #[qproperty(i32, doc_id, cxx_name = "docId")]
        type Editor = super::EditorRust;
    }

    unsafe extern "RustQt" {
        #[inherit]
        #[cxx_name = "beginInsertRows"]
        unsafe fn begin_insert_rows(self: Pin<&mut Editor>, parent: &QModelIndex, first: i32, last: i32);
        #[inherit]
        #[cxx_name = "endInsertRows"]
        unsafe fn end_insert_rows(self: Pin<&mut Editor>);
        #[inherit]
        #[cxx_name = "beginRemoveRows"]
        unsafe fn begin_remove_rows(self: Pin<&mut Editor>, parent: &QModelIndex, first: i32, last: i32);
        #[inherit]
        #[cxx_name = "endRemoveRows"]
        unsafe fn end_remove_rows(self: Pin<&mut Editor>);
        #[inherit]
        #[cxx_name = "beginResetModel"]
        unsafe fn begin_reset_model(self: Pin<&mut Editor>);
        #[inherit]
        #[cxx_name = "endResetModel"]
        unsafe fn end_reset_model(self: Pin<&mut Editor>);
        #[inherit]
        fn index(self: &Editor, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;

        #[qsignal]
        #[inherit]
        #[cxx_name = "dataChanged"]
        fn data_changed(self: Pin<&mut Editor>, top_left: &QModelIndex, bottom_right: &QModelIndex, roles: &QList_i32);
    }

    unsafe extern "RustQt" {
        #[qinvokable]
        #[cxx_override]
        fn data(self: &Editor, index: &QModelIndex, role: i32) -> QVariant;
        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &Editor) -> QHash_i32_QByteArray;
        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &Editor, parent: &QModelIndex) -> i32;
    }

    unsafe extern "RustQt" {
        /// Something went wrong that the user should hear about.
        #[qsignal]
        fn failed(self: Pin<&mut Editor>, message: QString);
        /// Put the caret at `pos` (UTF-16) in `row`, selecting from `anchor`.
        #[qsignal]
        #[cxx_name = "caretRequested"]
        fn caret_requested(self: Pin<&mut Editor>, row: i32, anchor: i32, pos: i32);
        /// Select from (`a_row`, `a_pos`) to the caret at (`z_row`, `z_pos`)
        /// across blocks.
        #[qsignal]
        #[cxx_name = "selectionRequested"]
        fn selection_requested(self: Pin<&mut Editor>, a_row: i32, a_pos: i32, z_row: i32, z_pos: i32);
        /// The text or structure changed (debounced stats refresh, etc.).
        #[qsignal]
        #[cxx_name = "contentChanged"]
        fn content_changed(self: Pin<&mut Editor>);

        /// Open a markdown file; false (and `failed`) if it can't be read.
        #[qinvokable]
        fn open(self: Pin<&mut Editor>, path: &QString) -> bool;
        /// The row of the block at `line` of the file (from 0), for a
        /// folder search hit.
        #[qinvokable]
        #[cxx_name = "rowOfLine"]
        fn row_of_line(self: &Editor, line: i32) -> i32;
        /// Start an empty untitled document.
        #[qinvokable]
        #[cxx_name = "newDocument"]
        fn new_document(self: Pin<&mut Editor>);
        #[qinvokable]
        fn save(self: Pin<&mut Editor>) -> bool;
        #[qinvokable]
        #[cxx_name = "saveAs"]
        fn save_as(self: Pin<&mut Editor>, path: &QString) -> bool;
        /// The file is at `path` now, moved or renamed by the sidebar or
        /// Move To…: saves go there, and the title follows.
        #[qinvokable]
        fn relocate(self: Pin<&mut Editor>, path: &QString);
        /// The whole document as markdown.
        #[qinvokable]
        fn markdown(self: &Editor) -> QString;
        /// The markdown as Source Code Mode shows it, its lines ending in
        /// `\n` whatever the file's line endings.
        #[qinvokable]
        #[cxx_name = "sourceText"]
        fn source_text(self: &Editor) -> QString;
        /// The line of the markdown (from 0) that block `row` starts on.
        #[qinvokable]
        #[cxx_name = "rowLine"]
        fn row_line(self: &Editor, row: i32) -> i32;
        /// Replace the document with Source Code Mode's `text`, its caret on
        /// `line`. A visit to the mode is one undo step (see `sealHistory`).
        #[qinvokable]
        #[cxx_name = "setSource"]
        fn set_source(self: Pin<&mut Editor>, text: &QString, line: i32);
        /// Stop the next edit from joining the last undo step.
        #[qinvokable]
        #[cxx_name = "sealHistory"]
        fn seal_history(self: Pin<&mut Editor>);
        /// Read the file again, as `encoding` when not empty. The reload is
        /// one undo step, as in Typora.
        #[qinvokable]
        fn reload(self: Pin<&mut Editor>, encoding: &QString) -> bool;
        /// Whether something else changed the file: "" when not (or when
        /// it was only touched), "changed", or "missing" (said once).
        #[qinvokable]
        #[cxx_name = "checkDisk"]
        fn check_disk(self: Pin<&mut Editor>) -> QString;
        /// After "changed": load what is on disk now, or keep the document
        /// and stop asking about that version.
        #[qinvokable]
        #[cxx_name = "acceptDisk"]
        fn accept_disk(self: Pin<&mut Editor>, load: bool);
        /// Whether the file still exists.
        #[qinvokable]
        #[cxx_name = "fileExists"]
        fn file_exists(self: &Editor) -> bool;
        /// Keep a copy of the unsaved document in the drafts folder, named
        /// after it and `when`; one file per document, rewritten.
        #[qinvokable]
        #[cxx_name = "backupDraft"]
        fn backup_draft(self: Pin<&mut Editor>, when: &QString) -> bool;
        /// Remove this document's draft (its content is safe on disk).
        #[qinvokable]
        #[cxx_name = "discardDraft"]
        fn discard_draft(self: Pin<&mut Editor>);
        #[qinvokable]
        #[cxx_name = "draftsFolder"]
        fn drafts_folder(self: &Editor) -> QString;

        /// The user typed in `row`; `text` is its new content and `caret`
        /// the caret afterwards (UTF-16).
        #[qinvokable]
        fn edited(self: Pin<&mut Editor>, row: i32, text: &QString, caret: i32);

        /// Check or uncheck the task list item `row` belongs to.
        #[qinvokable]
        #[cxx_name = "toggleTask"]
        fn toggle_task(self: Pin<&mut Editor>, row: i32);

        /// Switch the document between CRLF and LF line endings.
        #[qinvokable]
        #[cxx_name = "setLineEnding"]
        fn set_line_ending(self: Pin<&mut Editor>, crlf: bool);
        /// Preferences ▸ Editor ▸ Default Line Ending: what new documents
        /// get, the empty untitled one open now included.
        #[qinvokable]
        #[cxx_name = "setNewLineEnding"]
        fn set_new_line_ending(self: Pin<&mut Editor>, crlf: bool);

        /// Footnote `label`'s definition as HTML, for its tooltip; empty
        /// when it isn't defined.
        #[qinvokable]
        #[cxx_name = "footnoteHtml"]
        fn footnote_html(self: &Editor, label: &QString) -> QString;

        /// Emoji for the shortcode being typed, as a JSON array of
        /// `[shortcode, emoji]` (see `query::emoji_suggestions`).
        #[qinvokable]
        #[cxx_name = "emojiSuggestions"]
        fn emoji_suggestions(self: &Editor, token: &QString) -> QString;

        /// Recompute word counts, the outline and the footnotes.
        #[qinvokable]
        #[cxx_name = "refreshStats"]
        fn refresh_stats(self: Pin<&mut Editor>);

        /// Run editing command `name` on the selection from (`a_row`,
        /// `a_pos`) to the caret at (`z_row`, `z_pos`), UTF-16 positions.
        /// `arg` and `text` carry a heading level, table size or pasted
        /// markdown. Returns false when the command doesn't apply there, so
        /// the view can do its default (Backspace inside a line, say).
        #[qinvokable]
        fn command(
            self: Pin<&mut Editor>,
            name: &QString,
            a_row: i32,
            a_pos: i32,
            z_row: i32,
            z_pos: i32,
            arg: i32,
            text: &QString,
        ) -> bool;

        /// Set the info string of the code block in `row` (the code
        /// language box): one undo step, leaving the caret alone.
        #[qinvokable]
        #[cxx_name = "setInfo"]
        fn set_info(self: Pin<&mut Editor>, row: i32, info: &QString) -> bool;

        /// Set the `{tag}`s of the math block in `row`, likewise.
        #[qinvokable]
        #[cxx_name = "setMathTags"]
        fn set_math_tags(self: Pin<&mut Editor>, row: i32, before: &QString, after: &QString) -> bool;

        /// Every match of `query`, as JSON `{"error": message, "hits":
        /// [[row, cellRow, cellColumn, start, end], …]}`: UTF-16 positions
        /// in the block's text or the table cell's (cell -1, -1 elsewhere).
        /// At most 5001, so QML can say "5000+".
        #[qinvokable]
        #[cxx_name = "findAll"]
        fn find_all(self: &Editor, query: &QString, case_sensitive: bool, whole_word: bool, regex: bool) -> QString;

        /// Replace the match of `query` from `start` to `end` in `row` (in
        /// cell `r`, `c` of a table, else -1, -1) with `with`, where `$1`
        /// names a group in regex mode: one undo step. Returns where the
        /// replacement ends (UTF-16), or -1 when the text there no longer
        /// matches.
        #[qinvokable]
        #[cxx_name = "replaceMatch"]
        fn replace_match(
            self: Pin<&mut Editor>,
            row: i32,
            r: i32,
            c: i32,
            start: i32,
            end: i32,
            query: &QString,
            with: &QString,
            case_sensitive: bool,
            whole_word: bool,
            regex: bool,
        ) -> i32;

        /// Replace every match of `query` with `with`: one undo step.
        /// Returns how many there were.
        #[qinvokable]
        #[cxx_name = "replaceAll"]
        fn replace_all(
            self: Pin<&mut Editor>,
            query: &QString,
            with: &QString,
            case_sensitive: bool,
            whole_word: bool,
            regex: bool,
        ) -> i32;

        /// Find in Source Code Mode's `text`: as `findAll`, each match as
        /// `[0, -1, -1, start, end]`, the whole text being one row.
        #[qinvokable]
        #[cxx_name = "findInText"]
        fn find_in_text(self: &Editor, text: &QString, query: &QString, case_sensitive: bool, whole_word: bool, regex: bool) -> QString;

        /// What replaces the match from `start` to `end` of `text`, as a
        /// JSON string, or `null` when it no longer matches there.
        #[qinvokable]
        #[cxx_name = "replaceInText"]
        fn replace_in_text(
            self: &Editor,
            text: &QString,
            start: i32,
            end: i32,
            query: &QString,
            with: &QString,
            case_sensitive: bool,
            whole_word: bool,
            regex: bool,
        ) -> QString;

        /// Every match in `text` and what replaces it: `[[start, end,
        /// "replacement"], …]`.
        #[qinvokable]
        #[cxx_name = "replaceAllInText"]
        fn replace_all_in_text(
            self: &Editor,
            text: &QString,
            query: &QString,
            with: &QString,
            case_sensitive: bool,
            whole_word: bool,
            regex: bool,
        ) -> QString;

        /// The user typed in cell (`r`, `c`) of the table in `row`; `text`
        /// is the cell's new content.
        #[qinvokable]
        #[cxx_name = "cellEdited"]
        fn cell_edited(self: Pin<&mut Editor>, row: i32, r: i32, c: i32, text: &QString);

        /// Command `name` (`link`, `image`, `zoomImage`, `imageSyntax`; see
        /// `edit::text_command`) on `text`, a table cell's, selected from
        /// `a` to `z`: the new text and selection as JSON `[a, z, text]`,
        /// or empty when it changes nothing.
        #[qinvokable]
        #[cxx_name = "editText"]
        fn edit_text(self: &Editor, text: &QString, a: i32, z: i32, name: &QString, arg: &QString) -> QString;

        /// Run table command `name` with the caret in cell (`r`, `c`) of the
        /// table in `row`: `rowAbove`, `rowBelow`, `columnBefore`,
        /// `columnAfter`, `deleteRow`, `deleteColumn`, `rowUp`, `rowDown`,
        /// `columnLeft`, `columnRight`, `dragRow`/`dragColumn` (row or column
        /// `a` before `b`), `alignLeft`/`Center`/`Right` (columns `a` to
        /// `b`), `resize` (`a` columns by `b` rows), `prettify` or
        /// `deleteTable`.
        /// Returns the cell the caret goes to as "r,c", "out" when the
        /// table is gone (the caret is requested elsewhere), or "" when the
        /// command doesn't apply.
        #[qinvokable]
        #[cxx_name = "tableCommand"]
        fn table_command(self: Pin<&mut Editor>, row: i32, name: &QString, r: i32, c: i32, a: i32, b: i32) -> QString;

        /// A selection over several cells of the table in `row` (see
        /// TableBlock's `cellSpan`), from cell (`r1`, `c1`) at UTF-16
        /// position `p1` to (`r2`, `c2`) at `p2`, either way round: typed
        /// over with `text` ("" deletes it), as one undo step. Returns
        /// where the caret goes as "r,c,pos", or "".
        #[qinvokable]
        #[cxx_name = "replaceCellSpan"]
        fn replace_cell_span(self: Pin<&mut Editor>, row: i32, r1: i32, c1: i32, p1: i32, r2: i32, c2: i32, p2: i32, text: &QString) -> QString;

        /// That selection to copy, as JSON `[markdown, html]`: a table of
        /// what it takes, as Typora copies it.
        #[qinvokable]
        #[cxx_name = "cellSpanCopy"]
        fn cell_span_copy(self: &Editor, row: i32, r1: i32, c1: i32, p1: i32, r2: i32, c2: i32, p2: i32) -> QString;

        /// The words and characters a selection in a table takes, in one
        /// cell or several, as "words,characters" (see `selectionCounts`).
        #[qinvokable]
        #[cxx_name = "cellSpanCounts"]
        fn cell_span_counts(self: &Editor, row: i32, r1: i32, c1: i32, p1: i32, r2: i32, c2: i32, p2: i32) -> QString;

        /// The selection from (`a_row`, `a_pos`) to (`z_row`, `z_pos`) as
        /// markdown, for the clipboard.
        #[qinvokable]
        #[cxx_name = "selectionMarkdown"]
        fn selection_markdown(self: &Editor, a_row: i32, a_pos: i32, z_row: i32, z_pos: i32) -> QString;

        /// The selection's word and character counts, "words,characters",
        /// as Typora's status bar counts them.
        #[qinvokable]
        #[cxx_name = "selectionCounts"]
        fn selection_counts(self: &Editor, a_row: i32, a_pos: i32, z_row: i32, z_pos: i32) -> QString;

        /// The same selection rendered as HTML.
        #[qinvokable]
        #[cxx_name = "selectionHtml"]
        fn selection_html(self: &Editor, a_row: i32, a_pos: i32, z_row: i32, z_pos: i32) -> QString;

        /// Edit ▸ Selection ▸ Select Styled Scope: the visible text of the
        /// innermost styled span or link around `pos` in `row`, as
        /// "start,end" in UTF-16, or empty when there is none.
        #[qinvokable]
        #[cxx_name = "scopeAt"]
        fn scope_at(self: &Editor, row: i32, pos: i32) -> QString;

        /// The destination of the link around `pos` in `row` (references
        /// resolved), or empty.
        #[qinvokable]
        #[cxx_name = "linkAt"]
        fn link_at(self: &Editor, row: i32, pos: i32) -> QString;

        /// The URL to load for the image a paragraph's `text` consists of,
        /// or empty when it is anything else; local paths resolve as
        /// Typora resolves them.
        #[qinvokable]
        #[cxx_name = "imageUrl"]
        fn image_url(self: &Editor, text: &QString) -> QString;

        /// The size an `<img>` tag a paragraph's `text` consists of gives
        /// its image, as JSON `[width, height, zoom]` in CSS values; empty
        /// strings for a markdown image.
        #[qinvokable]
        #[cxx_name = "imageSize"]
        fn image_size(self: &Editor, text: &QString) -> QString;

        /// How HTML block `text` shows away from the caret: "rendered",
        /// "source" (tags Typora won't draw) or "comment".
        #[qinvokable]
        #[cxx_name = "htmlLook"]
        fn html_look(self: &Editor, text: &QString) -> QString;

        /// HTML block `text` as Qt rich text, `room` pixels wide; `toggled`
        /// lists the `<details>` the reader opened or closed (by order,
        /// comma separated); `colors` are the theme's border, code
        /// background, mark, faint, link and text colours, comma separated.
        #[qinvokable]
        #[cxx_name = "htmlView"]
        fn html_view(self: &Editor, text: &QString, toggled: &QString, room: f64, colors: &QString) -> QString;

        /// The URL to load for an image whose source is `src`.
        #[qinvokable]
        #[cxx_name = "resolveImage"]
        fn resolve_image(self: &Editor, src: &QString) -> QString;

        /// The row of the heading whose anchor (GitHub style, as Typora
        /// makes them) is `anchor`, or -1.
        #[qinvokable]
        #[cxx_name = "headingRow"]
        fn heading_row(self: &Editor, anchor: &QString) -> i32;

        /// What typing `ch` at `pos` in `row` does with auto-pairing on:
        /// empty to insert it, "skip" to step over the same character, or
        /// the closer to insert after it. `markdown` pairs `*`, `` ` `` and
        /// `_` too.
        #[qinvokable]
        #[cxx_name = "pairTyped"]
        fn pair_typed(self: &Editor, row: i32, pos: i32, ch: &QString, markdown: bool) -> QString;

        /// Whether Backspace at `pos` in `row` deletes an empty pair.
        #[qinvokable]
        #[cxx_name = "pairedAt"]
        fn paired_at(self: &Editor, row: i32, pos: i32, markdown: bool) -> bool;

        /// Smart punctuation converted on input: what typing `ch` at `pos`
        /// in `row` does, as JSON `{"back": n, "text": "…", "caret": k}`
        /// (the `n` characters before the caret and `ch` become `text`,
        /// the caret `k` characters into it), "skip" to step over the
        /// closing quote already there, or "" to type `ch` as it is.
        /// `pair` when auto-pairing is on, which pairs opening quotes.
        #[qinvokable]
        #[cxx_name = "smartTyped"]
        fn smart_typed(self: &Editor, row: i32, pos: i32, ch: &QString, pair: bool, markdown: bool) -> QString;
        /// `smartTyped` for a table cell's `text`.
        #[qinvokable]
        #[cxx_name = "cellSmartTyped"]
        fn cell_smart_typed(self: &Editor, text: &QString, pos: i32, ch: &QString, pair: bool, markdown: bool) -> QString;
        /// Preferences ▸ Markdown ▸ Smart Punctuation: smart quotes (in
        /// `pairs`, "“”‘’"), smart dashes, `--` as an em dash, and whether
        /// they convert on input or on rendering.
        #[qinvokable]
        #[cxx_name = "setSmart"]
        fn set_smart(self: Pin<&mut Editor>, quotes: bool, dashes: bool, to_em: bool, pairs: &QString, on_rendering: bool);

        /// Preferences ▸ Markdown: `<br>` tags hidden away from the caret
        /// (Visible <br/> off), math copied as HTML as rendered SVG rather
        /// than TeX, and the language new code fences get: `code_lang`
        /// ("__LAST" for the last one used), when they are added as
        /// Markdown (`when` & 1) or from the menu (`when` & 2).
        #[qinvokable]
        #[cxx_name = "setWriting"]
        fn set_writing(self: Pin<&mut Editor>, hide_br: bool, math_svg: bool, code_lang: &QString, when: i32);

        /// The language a code fence added as Markdown, or from the menu,
        /// gets.
        #[qinvokable]
        #[cxx_name = "codeLangSuggestion"]
        fn code_lang_suggestion(self: &Editor, menu: bool) -> QString;

        /// `pairTyped` and `pairedAt` for a table cell's `text`.
        #[qinvokable]
        #[cxx_name = "cellPairTyped"]
        fn cell_pair_typed(self: &Editor, text: &QString, pos: i32, ch: &QString, markdown: bool) -> QString;
        #[qinvokable]
        #[cxx_name = "cellPaired"]
        fn cell_paired(self: &Editor, text: &QString, pos: i32, markdown: bool) -> bool;

        /// What surrounds `pos` in `row`, for the context menu: any of
        /// `strong`, `em`, `code` and `link` for the inline styles there,
        /// and `list` or `quote` for the containers, comma-separated.
        #[qinvokable]
        #[cxx_name = "contextAt"]
        fn context_at(self: &Editor, row: i32, pos: i32) -> QString;

        /// The formula the caret at (`row`, `pos`) is in, as JSON: its TeX,
        /// whether it is display math, and `fence` for a ```math fence;
        /// "" outside math.
        #[qinvokable]
        #[cxx_name = "mathAt"]
        fn math_at(self: &Editor, row: i32, pos: i32) -> QString;

        /// Markdown for HTML from the clipboard.
        #[qinvokable]
        #[cxx_name = "htmlToMarkdown"]
        fn html_to_markdown(self: &Editor, html: &QString) -> QString;

        /// Preferences ▸ Image: what inserting an image file does (see
        /// image::InsertOptions).
        #[qinvokable]
        #[cxx_name = "setImageOptions"]
        fn set_image_options(
            self: Pin<&mut Editor>,
            action: i32,
            folder: &QString,
            copy_local: bool,
            relative: bool,
            with_dot: bool,
            escape: bool,
        );

        /// The folder inserting the files in `paths` (one a line) would
        /// copy images into, when it doesn't exist yet; otherwise "". The
        /// view asks before it is made.
        #[qinvokable]
        #[cxx_name = "missingImageFolder"]
        fn missing_image_folder(self: &Editor, paths: &QString) -> QString;

        /// Where a picture pasted from the clipboard is saved: the folder
        /// images are copied to, or `fallback` when there is none.
        #[qinvokable]
        #[cxx_name = "pastedImageFolder"]
        fn pasted_image_folder(self: &Editor, fallback: &QString) -> QString;

        /// Insert the files in `paths` (one a line) over the selection, as
        /// Typora inserts, drops and pastes them: images as images, copied
        /// first when the preferences say so (making the folder when
        /// `create`), video and audio as HTML, anything else as a link. Each
        /// file after the first goes in a new paragraph. The error, when a
        /// copy failed, or "".
        #[qinvokable]
        #[cxx_name = "insertFiles"]
        fn insert_files(
            self: Pin<&mut Editor>,
            a_row: i32,
            a_pos: i32,
            z_row: i32,
            z_pos: i32,
            paths: &QString,
            create: bool,
        ) -> QString;

        /// The markdown `insertFiles` would insert for `paths`, side by
        /// side, for a table cell; `after_text` when it follows text there.
        /// The copies are made as for `insertFiles`.
        #[qinvokable]
        #[cxx_name = "filesMarkup"]
        fn files_markup(self: &Editor, paths: &QString, after_text: bool, create: bool) -> QString;

        /// The image the caret at (`row`, `pos`) is in, as JSON `{src, url,
        /// path, syntax, folder, ratio}`: its source as written, the URL it
        /// loads, its file when it is a local one, how it is written
        /// ("markdown", "html" or "ref"), the name of the folder images are
        /// copied to when it isn't already there, and its file's pixels per
        /// point (1 when it says none); empty when the caret is in no image.
        #[qinvokable]
        #[cxx_name = "imageAt"]
        fn image_at(self: &Editor, row: i32, pos: i32) -> QString;

        /// imageAt for the image at `pos` (UTF-16) in a table cell's `text`.
        #[qinvokable]
        #[cxx_name = "cellImageAt"]
        fn cell_image_at(self: &Editor, text: &QString, pos: i32) -> QString;

        /// relocateImage's copy, for an image in a table cell, whose text
        /// the cell changes itself: JSON `{src, error}`, the source that
        /// points at the copy ("" when nothing was copied) and the error.
        #[qinvokable]
        #[cxx_name = "copyImageFile"]
        fn copy_image_file(self: &Editor, src: &QString, target: &QString, move_file: bool) -> QString;

        /// Copy the file of the image at (`row`, `pos`) to `target`, or into
        /// the folder images are copied to when `target` is empty, and point
        /// the image at the copy; with `move_file` the original goes. The
        /// error, or "".
        #[qinvokable]
        #[cxx_name = "relocateImage"]
        fn relocate_image(self: Pin<&mut Editor>, row: i32, pos: i32, target: &QString, move_file: bool) -> QString;

        /// Delete the image at (`row`, `pos`) from the document.
        #[qinvokable]
        #[cxx_name = "deleteImage"]
        fn delete_image(self: Pin<&mut Editor>, row: i32, pos: i32) -> bool;

        /// Delete Image File, after the image is taken out: the file to the
        /// trash, with a copy kept so that undoing that step puts it back
        /// and redoing it deletes it again, as Typora's.
        #[qinvokable]
        #[cxx_name = "trashImageFile"]
        fn trash_image_file(self: Pin<&mut Editor>, path: &QString) -> bool;

        /// Copy All Images to… / Move All Images to…: copy the file of every
        /// local image not already in `folder` there, and point the images
        /// at the copies; with `move_file` the originals go. The caret at
        /// (`row`, `pos`) stays where it is. JSON `{count, error}`: how many
        /// files were copied and the last error.
        #[qinvokable]
        #[cxx_name = "relocateAllImages"]
        fn relocate_all_images(self: Pin<&mut Editor>, folder: &QString, move_file: bool, row: i32, pos: i32) -> QString;

        /// Format ▸ Image ▸ When Insert Local Image: the folder the front
        /// matter's `typora-copy-images-to` ("copy") or `typora-root-url`
        /// ("root") names, as written; empty when it names none.
        #[qinvokable]
        #[cxx_name = "imageSetting"]
        fn image_setting(self: &Editor, name: &QString) -> QString;
        /// Point it at `folder`, written relative to the document's folder,
        /// or take it out when `folder` is empty; the caret at (`row`,
        /// `pos`) stays where it is. False when nothing changed.
        #[qinvokable]
        #[cxx_name = "setImageSetting"]
        fn set_image_setting(self: Pin<&mut Editor>, name: &QString, folder: &QString, row: i32, pos: i32) -> bool;

        /// The equation numbers display math shows under `numbering`
        /// (Typora's "", "ams" or "all"), as JSON: `rows` gives, by block,
        /// what each row of a formula shows at its right, for formulas
        /// that show any; `labels` what each `\label` refers to. `fences`
        /// counts ```math code blocks as display math.
        #[qinvokable]
        #[cxx_name = "mathTags"]
        fn math_tags(self: &Editor, numbering: &QString, fences: bool) -> QString;

        /// Preferences ▸ Editor and ▸ Markdown: how new lists, quotes and
        /// headings are written. `bullet` is "-", "+" or "*"; `indent` 2
        /// to 5; `heading` Typora's headingStyle (0 atx, 1 setext, 2 atx
        /// closed, 3 setext as wide as the title).
        #[qinvokable]
        #[cxx_name = "setSyntax"]
        fn set_syntax(self: &Editor, bullet: &QString, ordered_same: bool, indent: i32, pretty: bool, heading: i32);

        /// Preferences ▸ Markdown: the optional inline syntax.
        #[qinvokable]
        #[cxx_name = "setInlineOptions"]
        fn set_inline_options(
            self: Pin<&mut Editor>,
            math: bool,
            highlight: bool,
            superscript: bool,
            subscript: bool,
            latex: bool,
            auto_links: bool,
            alerts: bool,
            strict: bool,
            remap: bool,
        );

        /// File → Export: write the document as `format` ("pdf", "html",
        /// "html-plain", "image" or one pandoc writes) to `path`, given the
        /// format's extension when it has none, off the GUI thread, then
        /// run its After Export command. `settings` is JSON:
        /// `{"numbering": "", "mathFences": true, "diagrams": true,
        /// "monoFont": "…", "scale": 1, "options": {the format's},
        /// "opened": "folder", "collapsibleOutline": false,
        /// "themeFont": 16, "documents": "folder"}`. `exported` answers.
        #[qinvokable]
        #[cxx_name = "exportTo"]
        fn export_to(self: Pin<&mut Editor>, format: &QString, path: &QString, settings: &QString);
        /// An export finished, written to `path` unless there is an
        /// `error`. `command` is JSON: what the After Export command did,
        /// `{"ran": true, "ok": true, "output": "…", "error": "…"}`, or
        /// `{"ran": false}`.
        #[qsignal]
        fn exported(self: Pin<&mut Editor>, format: QString, path: QString, error: QString, command: QString);
        /// File → Print…: the desktop's print dialog, then the document
        /// typeset for the page it set up and sent to the printer, off the
        /// GUI thread. `settings` as for `exportTo`; `printed` answers.
        #[qinvokable]
        fn print(self: Pin<&mut Editor>, settings: &QString);
        /// File → Import…: `path` converted to markdown by pandoc, off the
        /// GUI thread, then shown as a new untitled document; `imported`
        /// answers.
        #[qinvokable]
        #[cxx_name = "importFile"]
        fn import_file(self: Pin<&mut Editor>, path: &QString);
        #[qsignal]
        fn imported(self: Pin<&mut Editor>, error: QString);
        /// Printing ended: `error` says why it failed, empty when it didn't
        /// (or the user cancelled).
        #[qsignal]
        fn printed(self: Pin<&mut Editor>, error: QString);
    }

    impl cxx_qt::Threading for Editor {}
}

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::{eqnum, export, file, htmlview, image, platform};
use std::pin::Pin;
use std::sync::atomic::{AtomicI32, Ordering};

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QByteArray, QHash, QHashPair_i32_QByteArray, QList, QModelIndex, QString, QVariant};

use crate::md::edit::{self, ListKind, Pos, Style};
use crate::md::table;
use crate::md::history::{History, Merge, Sel, Splice};
use crate::md::inline::{self, Options, Refs};
use crate::md::{self, query, BlockKind, ContainerKind, Document};

/// Byte offset of UTF-16 position `u` in `s` (clamped to the end).
pub fn byte_of(s: &str, u: i32) -> usize {
    let mut n = 0i32;
    for (i, c) in s.char_indices() {
        if n >= u {
            return i;
        }
        n += c.len_utf16() as i32;
    }
    s.len()
}

/// `Editor::pairTyped`'s answer for what typing does.
fn typed_reply(t: md::pair::Typed) -> QString {
    match t {
        md::pair::Typed::Insert => QString::default(),
        md::pair::Typed::Skip => QString::from("skip"),
        md::pair::Typed::Pair(close) => QString::from(&close.to_string()),
    }
}

/// UTF-16 position of byte offset `b` in `s`.
pub fn utf16_of(s: &str, b: usize) -> i32 {
    s[..b.min(s.len())].chars().map(|c| c.len_utf16() as i32).sum()
}

#[repr(i32)]
#[derive(Clone, Copy)]
enum Role {
    Kind = 0x0101,
    Text,
    Level,
    Info,
    Decor,
    Rows,
    BlockId,
    Gap,
}

const ROLES: [(Role, &str); 8] = [
    (Role::Kind, "kind"),
    (Role::Text, "text"),
    (Role::Level, "level"),
    (Role::Info, "info"),
    (Role::Decor, "decor"),
    (Role::Rows, "rows"),
    (Role::BlockId, "blockId"),
    (Role::Gap, "gap"),
];

pub struct EditorRust {
    path: QString,
    title: QString,
    modified: bool,
    crlf: bool,
    can_undo: bool,
    can_redo: bool,
    words: i32,
    characters: i32,
    lines: i32,
    outline: QString,
    footnotes: QString,
    inline_revision: i32,
    doc_id: i32,
    doc: Document,
    history: History,
    /// Layout facts per row that depend on its neighbours.
    derived: Vec<Derived>,
    refs: Refs,
    opts: Options,
    /// The front matter's `typora-root-url`, as written.
    image_root: Option<String>,
    /// Preferences ▸ Image.
    image_opts: image::InsertOptions,
    file: Option<PathBuf>,
    encoding: QString,
    /// How the file is stored, kept when saving.
    file_encoding: &'static encoding_rs::Encoding,
    bom: bool,
    /// The file as last read or written, to notice outside changes.
    stamp: Option<file::Stamp>,
    /// What changed on disk, while the user decides.
    disk_pending: Option<(file::Stamp, Vec<u8>)>,
    /// The file went missing and the user was told.
    missing: bool,
    /// This document's draft in the drafts folder.
    draft: Option<PathBuf>,
    /// Unsaved changes the history doesn't track (line endings).
    dirty: bool,
    /// New documents get CRLF line endings.
    new_crlf: bool,
    /// Smart punctuation converted on input (`opts.smart` has it when it
    /// converts on rendering).
    smart: md::smart::Smart,
    /// `<br>` tags hide away from the caret.
    hide_br: bool,
    /// Math copied as HTML is rendered SVG.
    math_svg: bool,
    /// The default code language and when it applies (`setWriting`), and
    /// the language last given a code fence.
    code_lang: String,
    code_lang_when: i32,
    last_lang: String,
    /// Set while views are told about a change. Their text items echo the
    /// new (or a stale) text back through `edited`, which is not an edit.
    syncing: bool,
}

static DOCS: AtomicI32 = AtomicI32::new(0);

impl Drop for EditorRust {
    fn drop(&mut self) {
        super::shared::forget(self.doc_id);
    }
}

impl Default for EditorRust {
    fn default() -> Self {
        let doc = Document::default();
        let derived = derive(&doc);
        EditorRust {
            path: QString::default(),
            title: QString::from("Untitled"),
            modified: false,
            crlf: false,
            can_undo: false,
            can_redo: false,
            words: 0,
            characters: 0,
            lines: 0,
            outline: QString::from("[]"),
            footnotes: QString::from("{}"),
            inline_revision: 0,
            doc_id: DOCS.fetch_add(1, Ordering::Relaxed),
            doc,
            history: History::default(),
            derived,
            refs: Refs::new(),
            opts: Options::default(),
            image_root: None,
            image_opts: image::InsertOptions::default(),
            syncing: false,
            file: None,
            encoding: QString::from("UTF-8"),
            file_encoding: encoding_rs::UTF_8,
            bom: false,
            stamp: None,
            disk_pending: None,
            missing: false,
            draft: None,
            dirty: false,
            new_crlf: false,
            smart: md::smart::Smart::default(),
            hide_br: false,
            math_svg: false,
            code_lang: String::new(),
            code_lang_when: 1,
            last_lang: String::new(),
        }
    }
}

pub fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Per-row layout that depends on the rows around it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Derived {
    /// What the delegate draws for its containers, outermost first: quote
    /// bars, list markers (only on an item's first block) and footnote
    /// labels (and on its last block, the links back), as JSON. Markers are computed here so renumbering follows
    /// every edit.
    ///
    /// `[{"t":"quote","first":true}, {"t":"item","first":true,"marker":"2.","ordered":true,"task":null,"loose":false,"level":0}]`
    pub decor: String,
    /// Space above the block in pixels at the default font size: the
    /// larger of the two blocks' margins, as CSS collapses them.
    pub gap: f64,
}

/// Margins (top, bottom) in pixels at 16px, after Typora's default theme.
/// Typora's editor puts a list item's text in paragraphs, tight list or
/// loose, with `li p { margin: .5rem 0 }`, and takes the top margin of an
/// item's first block away.
fn margins(doc: &Document, i: usize) -> (f64, f64) {
    let b = &doc.blocks[i];
    let Some(last) = b.containers.last() else { return block_margins(b, false) };
    let in_item = last.list_item().is_some();
    let first = i == 0 || doc.blocks[i - 1].containers.get(b.containers.len() - 1).is_none_or(|p| p.id != last.id);
    let (top, bottom) = block_margins(b, in_item);
    (if in_item && first { 0.0 } else { top }, bottom)
}

fn block_margins(b: &md::Block, in_item: bool) -> (f64, f64) {
    match &b.kind {
        BlockKind::Paragraph | BlockKind::Math { .. } if in_item => (8.0, 8.0),
        BlockKind::Heading { .. } => (16.0, 16.0),
        BlockKind::CodeFence { .. } | BlockKind::IndentedCode => (15.0, 15.0),
        BlockKind::ThematicBreak { .. } => (16.0, 16.0),
        // Typora's `.md-toc-content` margins.
        BlockKind::Toc => (20.0, 20.0),
        // Tables sit in a figure with 1.2em margins, room for the toolbar.
        BlockKind::Table { .. } => (19.2, 19.2),
        BlockKind::FrontMatter => (0.0, 12.8),
        _ => (12.8, 12.8),
    }
}

pub fn derive(doc: &Document) -> Vec<Derived> {
    let decor = decorations(doc);
    decor
        .into_iter()
        .enumerate()
        .map(|(i, decor)| {
            let gap = if i == 0 {
                // The page's first block keeps its top margin; a list
                // there has 30px (`#write > ul:first-child`).
                let list = doc.blocks[0].containers.first().is_some_and(|c| c.list_item().is_some());
                if list { 30.0 } else { margins(doc, 0).0 }
            } else {
                let (top, _) = margins(doc, i);
                let (_, bottom) = margins(doc, i - 1);
                let (a, b) = (&doc.blocks[i - 1], &doc.blocks[i]);
                // A list that starts or ends between them adds its own
                // .8em margin, unless it sits directly in an item.
                let shared = md::common_depth(a, b);
                let edge = |x: &md::Block, y: &md::Block| {
                    x.containers.iter().enumerate().skip(shared).any(|(k, c)| {
                        c.list_item().is_some_and(|it| {
                            let nested = k > 0 && x.containers[k - 1].list_item().is_some();
                            let same = y.containers.get(k).and_then(|p| p.list_item()).is_some_and(|p| p.list == it.list);
                            !nested && !same
                        })
                    })
                };
                let list = if edge(a, b) || edge(b, a) { 12.8 } else { 0.0 };
                top.max(bottom).max(list)
            };
            Derived { decor, gap }
        })
        .collect()
}

fn decorations(doc: &Document) -> Vec<String> {
    // Ordinal of each list item within its list, by container id.
    let mut ordinal: std::collections::HashMap<md::Id, usize> = Default::default();
    let mut counts: std::collections::HashMap<md::Id, usize> = Default::default();
    let mut alerts: std::collections::HashMap<md::Id, &str> = Default::default();
    let mut out = Vec::with_capacity(doc.blocks.len());
    for (i, b) in doc.blocks.iter().enumerate() {
        let mut parts = Vec::new();
        let mut list_level = 0;
        for (d, c) in b.containers.iter().enumerate() {
            let first = i == 0 || doc.blocks[i - 1].containers.get(d).is_none_or(|p| p.id != c.id);
            match &c.kind {
                ContainerKind::Quote { .. } => {
                    // A quote whose first paragraph opens with `[!NOTE]`
                    // is an alert.
                    let alert = *alerts.entry(c.id).or_insert_with(|| {
                        let direct = b.containers.len() == d + 1 && b.kind == BlockKind::Paragraph;
                        if first && direct { query::alert_of(&b.text).map_or("", |(k, _)| k) } else { "" }
                    });
                    parts.push(format!(r#"{{"t":"quote","first":{first},"alert":"{alert}"}}"#))
                }
                ContainerKind::ListItem(item) => {
                    let n = *ordinal.entry(c.id).or_insert_with(|| {
                        let n = counts.entry(item.list).or_insert(0);
                        *n += 1;
                        *n - 1
                    });
                    let marker = match &item.marker {
                        md::ListMarker::Bullet(_) => ["•", "◦", "▪"][list_level % 3].to_string(),
                        m => m.text(n),
                    };
                    let task = match item.task {
                        None => "null",
                        Some(true) => "true",
                        Some(false) => "false",
                    };
                    parts.push(format!(
                        r#"{{"t":"item","first":{first},"marker":{},"ordered":{},"task":{task},"loose":{},"level":{list_level}}}"#,
                        json_str(&marker),
                        item.marker.is_ordered(),
                        item.loose
                    ));
                    list_level += 1;
                }
                ContainerKind::Footnote { label } => {
                    // The last block takes the links back to the references.
                    let last = doc.blocks.get(i + 1).and_then(|n| n.containers.get(d)).is_none_or(|n| n.id != c.id);
                    parts.push(format!(r#"{{"t":"footnote","first":{first},"last":{last},"label":{}}}"#, json_str(label)))
                }
            }
        }
        out.push(format!("[{}]", parts.join(",")));
    }
    out
}

fn rows_json(b: &md::Block) -> String {
    match &b.kind {
        BlockKind::Table { align, rows } => {
            let al: Vec<&str> = align
                .iter()
                .map(|a| match a {
                    md::Align::None => "\"\"",
                    md::Align::Left => "\"left\"",
                    md::Align::Center => "\"center\"",
                    md::Align::Right => "\"right\"",
                })
                .collect();
            let rows: Vec<String> =
                rows.iter().map(|r| format!("[{}]", r.iter().map(|c| json_str(c)).collect::<Vec<_>>().join(","))).collect();
            format!(r#"{{"align":[{}],"rows":[{}]}}"#, al.join(","), rows.join(","))
        }
        // A math block's tags.
        BlockKind::Math { before, after, .. } => format!("[{},{}]", json_str(before), json_str(after)),
        _ => String::new(),
    }
}

fn file_title(path: Option<&Path>) -> String {
    path.and_then(|p| p.file_name()).map_or_else(|| "Untitled".to_string(), |n| n.to_string_lossy().into_owned())
}

impl qobject::Editor {
    fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let Some(b) = usize::try_from(index.row()).ok().and_then(|r| self.doc.blocks.get(r)) else {
            return QVariant::default();
        };
        let row = index.row() as usize;
        match role {
            r if r == Role::Kind as i32 => QVariant::from(&QString::from(b.kind.name())),
            r if r == Role::Text as i32 => QVariant::from(&QString::from(&b.text)),
            r if r == Role::Level as i32 => QVariant::from(&(md::edit::heading_level(b) as i32)),
            r if r == Role::Info as i32 => {
                let info = match &b.kind {
                    BlockKind::CodeFence { info, .. } => inline::unescape(info.trim()),
                    BlockKind::ThematicBreak { marker } => marker.clone(),
                    // The opening delimiter.
                    BlockKind::Math { brackets, .. } => (if *brackets { "\\[" } else { "$$" }).into(),
                    _ => String::new(),
                };
                QVariant::from(&QString::from(&info))
            }
            r if r == Role::Decor as i32 => QVariant::from(&QString::from(self.derived.get(row).map_or("[]", |d| &d.decor))),
            r if r == Role::Gap as i32 => QVariant::from(&self.derived.get(row).map_or(0.0, |d| d.gap)),
            r if r == Role::Rows as i32 => QVariant::from(&QString::from(&rows_json(b))),
            r if r == Role::BlockId as i32 => QVariant::from(&(b.id as i64)),
            _ => QVariant::default(),
        }
    }

    fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut h = QHash::<QHashPair_i32_QByteArray>::default();
        for (role, name) in ROLES {
            h.insert(role as i32, QByteArray::from(name));
        }
        h
    }

    fn row_count(&self, _parent: &QModelIndex) -> i32 {
        self.doc.blocks.len() as i32
    }

    fn row_of_line(&self, line: i32) -> i32 {
        md::serialize::block_at_line(&self.doc, line.max(0) as usize) as i32
    }

    /// Replace the whole document (open, new, reload).
    fn reset(mut self: Pin<&mut Self>, doc: Document, file: Option<PathBuf>) {
        self.as_mut().rust_mut().syncing = true;
        unsafe { self.as_mut().begin_reset_model() };
        {
            let mut r = self.as_mut().rust_mut();
            r.derived = derive(&doc);
            r.doc = doc;
            r.history.clear();
            r.dirty = false;
            r.file = file.clone();
            r.draft = None;
        }
        unsafe { self.as_mut().end_reset_model() };
        self.as_mut().update_refs();
        self.as_mut().rust_mut().syncing = false;
        let crlf = self.doc.line_ending == md::LineEnding::CrLf;
        self.as_mut().set_crlf(crlf);
        let path = file.as_ref().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        self.as_mut().set_path(QString::from(&path));
        self.as_mut().set_title(QString::from(&file_title(file.as_deref())));
        self.as_mut().sync_history();
        self.as_mut().refresh_stats();
        self.content_changed();
    }

    fn open(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let path = PathBuf::from(path.to_string());
        let path = fs::canonicalize(&path).unwrap_or(path);
        match fs::read(&path) {
            Ok(bytes) => {
                let d = file::decode(&bytes, None);
                let doc = md::parse_with(&d.text, self.opts.latex_delimiters, self.opts.remap);
                self.as_mut().reset(doc, Some(path.clone()));
                self.as_mut().stored(&path, &bytes, d.encoding, d.bom);
                true
            }
            Err(e) => {
                self.failed(QString::from(&format!("Could not open {}: {e}", path.display())));
                false
            }
        }
    }

    /// Remember how the file is stored and what it looked like.
    fn stored(mut self: Pin<&mut Self>, path: &Path, bytes: &[u8], encoding: &'static encoding_rs::Encoding, bom: bool) {
        {
            let mut r = self.as_mut().rust_mut();
            r.file_encoding = encoding;
            r.bom = bom;
            r.stamp = Some(file::Stamp::of(path, bytes));
            r.disk_pending = None;
            r.missing = false;
        }
        self.as_mut().set_encoding(QString::from(encoding.name()));
    }

    fn new_document(mut self: Pin<&mut Self>) {
        let crlf = self.new_crlf;
        let doc = Document { line_ending: if crlf { md::LineEnding::CrLf } else { md::LineEnding::Lf }, ..Document::default() };
        self.as_mut().reset(doc, None);
        let mut r = self.as_mut().rust_mut();
        r.file_encoding = encoding_rs::UTF_8;
        r.bom = false;
        r.stamp = None;
        r.disk_pending = None;
        r.draft = None;
    }

    fn markdown(&self) -> QString {
        QString::from(&md::serialize(&self.doc))
    }

    fn source_text(&self) -> QString {
        let mut doc = self.doc.clone();
        doc.line_ending = md::LineEnding::Lf;
        QString::from(&md::serialize(&doc))
    }

    fn row_line(&self, row: i32) -> i32 {
        let starts = md::serialize::serialize_with_starts(&self.doc).1;
        usize::try_from(row).ok().and_then(|r| starts.get(r)).copied().unwrap_or(0) as i32
    }

    fn set_source(mut self: Pin<&mut Self>, text: &QString, line: i32) {
        let mut new = md::parse_with(&text.to_string(), self.opts.latex_delimiters, self.opts.remap);
        md::history::adopt_ids(&self.doc.blocks, &mut new.blocks);
        let row = md::serialize::block_at_line(&new, line.max(0) as usize);
        let old = self.doc.blocks.clone();
        let before = Sel::caret(Pos::new(row.min(old.len().saturating_sub(1)), 0));
        let after = Sel::caret(Pos::new(row, 0));
        // The end of the file isn't in the block list, so the history
        // can't see a change there.
        let tail = new.final_newline != self.doc.final_newline || new.blank_after != self.doc.blank_after;
        {
            let mut r = self.as_mut().rust_mut();
            r.doc.blocks = new.blocks;
            r.doc.final_newline = new.final_newline;
            r.doc.blank_after = new.blank_after;
            if tail {
                r.dirty = true;
            }
        }
        self.as_mut().commit(old, before, after, Merge::Source);
        if tail {
            self.as_mut().sync_history();
            self.content_changed();
        }
    }

    fn seal_history(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().history.seal();
    }

    fn reload(mut self: Pin<&mut Self>, encoding: &QString) -> bool {
        let Some(path) = self.file.clone() else { return false };
        let forced = match encoding.to_string().as_str() {
            "" => Some(self.file_encoding),
            "auto" => None,
            label => file::encoding_for(label),
        };
        match fs::read(&path) {
            Ok(bytes) => {
                self.as_mut().load_bytes(&path, &bytes, forced);
                true
            }
            Err(e) => {
                self.failed(QString::from(&format!("Could not open {}: {e}", path.display())));
                false
            }
        }
    }

    /// Replace the document with the file's `bytes` as one undo step, and
    /// count it as saved.
    fn load_bytes(mut self: Pin<&mut Self>, path: &Path, bytes: &[u8], forced: Option<&'static encoding_rs::Encoding>) {
        let d = file::decode(bytes, forced);
        let new = md::parse_with(&d.text, self.opts.latex_delimiters, self.opts.remap);
        let old = self.doc.blocks.clone();
        let sel = Sel::caret(Pos::new(0, 0));
        let crlf = new.line_ending == md::LineEnding::CrLf;
        self.as_mut().rust_mut().doc = new;
        self.as_mut().commit(old, sel, sel, Merge::Never);
        {
            let mut r = self.as_mut().rust_mut();
            r.history.mark_saved();
            r.dirty = false;
        }
        self.as_mut().set_crlf(crlf);
        self.as_mut().stored(path, bytes, d.encoding, d.bom);
        self.as_mut().sync_history();
        self.request_sel(sel);
    }

    fn check_disk(mut self: Pin<&mut Self>) -> QString {
        let (Some(path), Some(stamp)) = (self.file.clone(), self.stamp.clone()) else { return QString::default() };
        if self.disk_pending.is_some() {
            return QString::default();
        }
        match file::check(&path, &stamp) {
            file::Disk::Same => QString::default(),
            file::Disk::Touched(now) => {
                self.as_mut().rust_mut().stamp = Some(now);
                QString::default()
            }
            file::Disk::Changed(now, bytes) => {
                let mut r = self.as_mut().rust_mut();
                r.disk_pending = Some((now, bytes));
                r.missing = false;
                QString::from("changed")
            }
            file::Disk::Missing if !self.missing => {
                self.as_mut().rust_mut().missing = true;
                QString::from("missing")
            }
            file::Disk::Missing => QString::default(),
        }
    }

    fn accept_disk(mut self: Pin<&mut Self>, load: bool) {
        let Some((stamp, bytes)) = self.as_mut().rust_mut().disk_pending.take() else { return };
        match (load, self.file.clone()) {
            (true, Some(path)) => {
                let encoding = self.file_encoding;
                self.load_bytes(&path, &bytes, Some(encoding));
            }
            _ => self.as_mut().rust_mut().stamp = Some(stamp),
        }
    }

    fn file_exists(&self) -> bool {
        self.file.as_ref().is_some_and(|p| p.exists())
    }

    fn backup_draft(mut self: Pin<&mut Self>, when: &QString) -> bool {
        let text = md::serialize(&self.doc);
        if text.trim().is_empty() {
            return false;
        }
        let dir = file::drafts_dir();
        if fs::create_dir_all(&dir).is_err() {
            return false;
        }
        let path = match self.draft.clone() {
            Some(p) => p,
            None => {
                file::prune_drafts(&dir);
                let name = self.file.as_deref().and_then(|p| p.file_stem()).map_or_else(|| "Untitled".to_string(), |s| s.to_string_lossy().into_owned());
                let p = dir.join(format!("{} {}.md", file::safe_name(&name), file::safe_name(&when.to_string())));
                self.as_mut().rust_mut().draft = Some(p.clone());
                p
            }
        };
        file::write_atomic(&path, text.as_bytes()).is_ok()
    }

    fn discard_draft(mut self: Pin<&mut Self>) {
        if let Some(p) = self.as_mut().rust_mut().draft.take() {
            let _ = fs::remove_file(p);
        }
    }

    fn drafts_folder(&self) -> QString {
        QString::from(&file::drafts_dir().to_string_lossy().into_owned())
    }

    fn save(self: Pin<&mut Self>) -> bool {
        match self.file.clone() {
            Some(p) => self.write_to(&p),
            None => {
                self.failed(QString::from("This document has no file yet; use Save As."));
                false
            }
        }
    }

    fn save_as(mut self: Pin<&mut Self>, path: &QString) -> bool {
        let mut p = PathBuf::from(path.to_string());
        // A name typed without an extension is a markdown file.
        if p.extension().is_none() {
            p.set_extension("md");
        }
        if !self.as_mut().write_to(&p) {
            return false;
        }
        let p = fs::canonicalize(&p).unwrap_or(p);
        self.as_mut().set_path(QString::from(&p.to_string_lossy().into_owned()));
        self.as_mut().set_title(QString::from(&file_title(Some(&p))));
        self.rust_mut().file = Some(p);
        true
    }

    fn relocate(mut self: Pin<&mut Self>, path: &QString) {
        let p = PathBuf::from(path.to_string());
        self.as_mut().set_path(path.clone());
        self.as_mut().set_title(QString::from(&file_title(Some(&p))));
        self.rust_mut().file = Some(p);
    }

    /// Write the document in the file's encoding, atomically.
    fn write_to(mut self: Pin<&mut Self>, path: &Path) -> bool {
        let text = md::serialize(&self.doc);
        let bytes = file::encode(&text, self.file_encoding, self.bom);
        match file::write_atomic(path, &bytes) {
            Ok(()) => {
                {
                    let mut r = self.as_mut().rust_mut();
                    r.history.mark_saved();
                    r.dirty = false;
                    r.stamp = Some(file::Stamp::of(path, &bytes));
                    r.disk_pending = None;
                    r.missing = false;
                }
                self.as_mut().discard_draft();
                self.as_mut().sync_history();
                true
            }
            Err(e) => {
                self.failed(QString::from(&format!("Could not save {}: {e}", path.display())));
                false
            }
        }
    }

    fn sync_history(mut self: Pin<&mut Self>) {
        let (m, u, r) = (self.history.modified() || self.dirty, self.history.can_undo(), self.history.can_redo());
        self.as_mut().set_modified(m);
        self.as_mut().set_can_undo(u);
        self.as_mut().set_can_redo(r);
    }

    /// Tell views which rows an edit replaced, and refresh decorations of
    /// rows whose containers may have changed around it.
    fn apply_splice(mut self: Pin<&mut Self>, s: Splice) {
        self.as_mut().rust_mut().syncing = true;
        let parent = QModelIndex::default();
        let old_len = self.derived.len();
        let common = s.removed.min(s.inserted);
        // Views read new rows as soon as they are inserted, so give them
        // their real layout rather than placeholders.
        let derived = derive(&self.doc);
        if s.removed > common {
            let first = (s.at + common) as i32;
            let last = (s.at + s.removed - 1) as i32;
            // The rows were already removed from the document; views only
            // read data after `end_remove_rows`.
            unsafe { self.as_mut().begin_remove_rows(&parent, first, last) };
            {
                let mut r = self.as_mut().rust_mut();
                r.derived.drain(s.at + common..s.at + s.removed);
            }
            unsafe { self.as_mut().end_remove_rows() };
        }
        if s.inserted > common {
            let first = (s.at + common) as i32;
            let last = (s.at + s.inserted - 1) as i32;
            unsafe { self.as_mut().begin_insert_rows(&parent, first, last) };
            {
                let mut r = self.as_mut().rust_mut();
                let at = s.at + common;
                let n = s.inserted - common;
                r.derived.splice(at..at, derived[at..at + n].iter().cloned());
            }
            unsafe { self.as_mut().end_insert_rows() };
        }
        debug_assert_eq!(self.derived.len(), old_len - s.removed + s.inserted);
        let mut changed: Vec<usize> = (s.at..s.at + common).collect();
        for (i, d) in derived.iter().enumerate() {
            if self.derived[i] != *d && !(s.at..s.at + common).contains(&i) {
                changed.push(i);
            }
        }
        self.as_mut().rust_mut().derived = derived;
        self.as_mut().update_refs();
        changed.sort_unstable();
        for i in changed {
            let idx = self.index(i as i32, 0, &parent);
            self.as_mut().data_changed(&idx, &idx, &QList::default());
        }
        self.as_mut().rust_mut().syncing = false;
        self.as_mut().sync_history();
        self.content_changed();
    }

    fn edited(mut self: Pin<&mut Self>, row: i32, text: &QString, caret: i32) {
        if self.syncing {
            return;
        }
        let Some(i) = usize::try_from(row).ok().filter(|&i| i < self.doc.blocks.len()) else { return };
        let text = text.to_string();
        if self.doc.blocks[i].text == text {
            return;
        }
        let old = self.doc.blocks.clone();
        let before = Sel::caret(Pos::new(i, byte_of(&self.doc.blocks[i].text, caret)));
        let pos = Pos::new(i, byte_of(&text, caret));
        let id = self.doc.blocks[i].id;
        let mut caret_after = pos;
        let mut ruled = false;
        {
            let mut r = self.as_mut().rust_mut();
            r.doc.blocks[i].set_text(text);
            let b = &mut r.doc.blocks[i];
            if b.kind == BlockKind::LinkRefDef && inline::parse_link_ref_def(&b.text).is_none() {
                b.set_kind(BlockKind::Paragraph);
            }
            if let Some(p) = md::edit::input_rule(&mut r.doc, pos) {
                caret_after = p;
                ruled = true;
            }
        }
        let merge = if ruled { Merge::Never } else { Merge::Typing(id) };
        self.as_mut().commit(old, before, Sel::caret(caret_after), merge);
        if ruled {
            let b = &self.doc.blocks[caret_after.block];
            let u = utf16_of(&b.text, caret_after.offset);
            self.caret_requested(caret_after.block as i32, u, u);
        }
    }

    /// Record the change from `old` to the current document as one undo
    /// step and update the views.
    fn commit(mut self: Pin<&mut Self>, old: Vec<md::Block>, before: Sel, after: Sel, merge: Merge) {
        let splice = {
            let r = self.as_mut().rust_mut();
            let r = r.get_mut();
            r.history.record(&old, &r.doc, before, after, merge)
        };
        if merge == Merge::Never {
            self.as_mut().rust_mut().history.seal();
        }
        if let Some(s) = splice {
            self.apply_splice(s);
        }
    }

    fn command(
        mut self: Pin<&mut Self>,
        name: &QString,
        a_row: i32,
        a_pos: i32,
        z_row: i32,
        z_pos: i32,
        arg: i32,
        text: &QString,
    ) -> bool {
        let name = name.to_string();
        match name.as_str() {
            "undo" => return self.undo_redo(false),
            "redo" => return self.undo_redo(true),
            _ => {}
        }
        let (Some(anchor), Some(caret)) = (pos_of(&self.doc, a_row, a_pos), pos_of(&self.doc, z_row, z_pos)) else {
            return false;
        };
        let before = Sel { anchor, caret };
        let old = self.doc.blocks.clone();
        let refs = self.refs.clone();
        let opts = self.opts.clone();
        let text = text.to_string();
        let lang = if name == "codeFence" { self.lang_suggestion(true) } else { String::new() };
        let after = {
            let mut r = self.as_mut().rust_mut();
            apply_command(&mut r.doc, &name, anchor, caret, arg, &text, &refs, &opts)
        };
        let Some(after) = after else { return false };
        // A code fence from the menu gets the default language; one typed
        // with a language sets the last used.
        let i = after.caret.block;
        if let Some(BlockKind::CodeFence { info, .. }) = self.doc.blocks.get(i).map(|b| &b.kind) {
            let info = info.clone();
            let mut r = self.as_mut().rust_mut();
            if name == "codeFence" && info.is_empty() && !lang.is_empty() {
                edit::set_info(&mut r.doc, i, &lang);
            } else if name == "enter" && !info.is_empty() && old.get(i).is_some_and(|b| b.kind == BlockKind::Paragraph) {
                r.last_lang = info.trim().to_string();
            }
        }
        self.as_mut().commit(old, before, after, Merge::Never);
        self.request_sel(after);
        true
    }

    fn set_info(mut self: Pin<&mut Self>, row: i32, info: &QString) -> bool {
        let Some(at) = pos_of(&self.doc, row, 0) else { return false };
        let old = self.doc.blocks.clone();
        if !edit::set_info(&mut self.as_mut().rust_mut().doc, at.block, &info.to_string()) {
            return false;
        }
        self.as_mut().rust_mut().last_lang = info.to_string().trim().to_string();
        self.as_mut().commit(old, Sel::caret(at), Sel::caret(at), Merge::Never);
        true
    }

    fn set_math_tags(mut self: Pin<&mut Self>, row: i32, before: &QString, after: &QString) -> bool {
        let Some(at) = pos_of(&self.doc, row, 0) else { return false };
        let old = self.doc.blocks.clone();
        if !edit::set_math_tags(&mut self.as_mut().rust_mut().doc, at.block, &before.to_string(), &after.to_string()) {
            return false;
        }
        self.as_mut().commit(old, Sel::caret(at), Sel::caret(at), Merge::Never);
        true
    }

    fn edit_text(&self, text: &QString, a: i32, z: i32, name: &QString, arg: &QString) -> QString {
        let text = text.to_string();
        let (a, z) = (byte_of(&text, a.min(z)), byte_of(&text, a.max(z)));
        let Some((out, sel)) = edit::text_command(&text, a..z, &name.to_string(), &arg.to_string(), &self.refs) else {
            return QString::default();
        };
        QString::from(&format!("[{},{},{}]", utf16_of(&out, sel.start), utf16_of(&out, sel.end), json_str(&out)))
    }

    fn find_all(&self, query: &QString, case_sensitive: bool, whole_word: bool, regex: bool) -> QString {
        let opts = query::FindOptions { case_sensitive, whole_word, regex };
        let found = match query::find(&self.doc, &query.to_string(), &opts) {
            Ok(found) => found,
            Err(e) => return QString::from(&format!("{{\"error\":{},\"hits\":[]}}", json_str(&e.to_string()))),
        };
        let hits: Vec<String> = found
            .iter()
            .take(5001)
            .map(|m| {
                let (text, (r, c)) = match_text(&self.doc, m.block, m.cell).unwrap_or(("", (-1, -1)));
                let (s, e) = (utf16_of(text, m.range.start), utf16_of(text, m.range.end));
                format!("[{},{r},{c},{s},{e}]", m.block)
            })
            .collect();
        QString::from(&format!("{{\"error\":\"\",\"hits\":[{}]}}", hits.join(",")))
    }

    fn find_in_text(&self, text: &QString, query: &QString, case_sensitive: bool, whole_word: bool, regex: bool) -> QString {
        let opts = query::FindOptions { case_sensitive, whole_word, regex };
        let text = text.to_string();
        let found = match query::find_in(&text, &query.to_string(), &opts) {
            Ok(found) => found,
            Err(e) => return QString::from(&format!("{{\"error\":{},\"hits\":[]}}", json_str(&e.to_string()))),
        };
        let hits: Vec<String> = found
            .iter()
            .take(5001)
            .map(|r| format!("[0,-1,-1,{},{}]", utf16_of(&text, r.start), utf16_of(&text, r.end)))
            .collect();
        QString::from(&format!("{{\"error\":\"\",\"hits\":[{}]}}", hits.join(",")))
    }

    #[allow(clippy::too_many_arguments)]
    fn replace_in_text(
        &self,
        text: &QString,
        start: i32,
        end: i32,
        query: &QString,
        with: &QString,
        case_sensitive: bool,
        whole_word: bool,
        regex: bool,
    ) -> QString {
        let opts = query::FindOptions { case_sensitive, whole_word, regex };
        let text = text.to_string();
        let range = byte_of(&text, start)..byte_of(&text, end);
        if range.is_empty() {
            return QString::from("null");
        }
        match query::replacement_in(&text, range, &query.to_string(), &with.to_string(), &opts) {
            Some(new) => QString::from(&json_str(&new)),
            None => QString::from("null"),
        }
    }

    fn replace_all_in_text(
        &self,
        text: &QString,
        query: &QString,
        with: &QString,
        case_sensitive: bool,
        whole_word: bool,
        regex: bool,
    ) -> QString {
        let opts = query::FindOptions { case_sensitive, whole_word, regex };
        let (text, query, with) = (text.to_string(), query.to_string(), with.to_string());
        let found = query::find_in(&text, &query, &opts).unwrap_or_default();
        let all: Vec<String> = found
            .into_iter()
            .filter_map(|r| {
                let new = query::replacement_in(&text, r.clone(), &query, &with, &opts)?;
                Some(format!("[{},{},{}]", utf16_of(&text, r.start), utf16_of(&text, r.end), json_str(&new)))
            })
            .collect();
        QString::from(&format!("[{}]", all.join(",")))
    }

    #[allow(clippy::too_many_arguments)]
    fn replace_match(
        mut self: Pin<&mut Self>,
        row: i32,
        r: i32,
        c: i32,
        start: i32,
        end: i32,
        query: &QString,
        with: &QString,
        case_sensitive: bool,
        whole_word: bool,
        regex: bool,
    ) -> i32 {
        let opts = query::FindOptions { case_sensitive, whole_word, regex };
        let Some(at) = pos_of(&self.doc, row, 0) else { return -1 };
        let cell = usize::try_from(r).ok().zip(usize::try_from(c).ok());
        let Some((text, _)) = match_text(&self.doc, at.block, cell) else { return -1 };
        let range = byte_of(text, start)..byte_of(text, end);
        let (query, with) = (query.to_string(), with.to_string());
        // Only what is still a match there.
        let Ok(re) = query::matcher(&query, &opts) else { return -1 };
        if range.is_empty() || re.find_at(text, range.start).is_none_or(|m| m.range() != range) {
            return -1;
        }
        let m = query::Match { block: at.block, cell, range: range.clone() };
        let Ok(new) = query::replacement(&self.doc, &m, &query, &with, &opts) else { return -1 };
        let end = start + utf16_of(&new, usize::MAX);
        let old = self.doc.blocks.clone();
        if query::replace(&mut self.as_mut().rust_mut().doc, &m, &query, &with, &opts).is_err() {
            return -1;
        }
        let (before, after) = match cell {
            Some(cell) => (Sel::caret(table::caret(at.block, cell)), Sel::caret(table::caret(at.block, cell))),
            None => (
                Sel { anchor: Pos::new(at.block, range.start), caret: Pos::new(at.block, range.end) },
                Sel::caret(Pos::new(at.block, range.start + new.len())),
            ),
        };
        self.as_mut().commit(old, before, after, Merge::Never);
        end
    }

    fn replace_all(
        mut self: Pin<&mut Self>,
        query: &QString,
        with: &QString,
        case_sensitive: bool,
        whole_word: bool,
        regex: bool,
    ) -> i32 {
        let opts = query::FindOptions { case_sensitive, whole_word, regex };
        let (query, with) = (query.to_string(), with.to_string());
        let Some(first) = query::find(&self.doc, &query, &opts).ok().and_then(|f| f.first().map(|m| m.block)) else {
            return 0;
        };
        let old = self.doc.blocks.clone();
        let Ok(n) = query::replace_all(&mut self.as_mut().rust_mut().doc, &query, &with, &opts) else { return 0 };
        let here = Sel::caret(Pos::new(first, 0));
        self.as_mut().commit(old, here, here, Merge::Never);
        n as i32
    }

    fn cell_edited(mut self: Pin<&mut Self>, row: i32, r: i32, c: i32, text: &QString) {
        if self.syncing {
            return;
        }
        let (Some(at), Ok(r), Ok(c)) = (pos_of(&self.doc, row, 0), usize::try_from(r), usize::try_from(c)) else { return };
        let i = at.block;
        let old = self.doc.blocks.clone();
        if !table::set_cell(&mut self.as_mut().rust_mut().doc, i, (r, c), &text.to_string()) {
            return;
        }
        let id = self.doc.blocks[i].id;
        let here = Sel::caret(table::caret(i, (r, c)));
        self.as_mut().commit(old, here, here, Merge::Cell(id, r, c));
    }

    fn table_command(mut self: Pin<&mut Self>, row: i32, name: &QString, r: i32, c: i32, a: i32, b: i32) -> QString {
        let (Some(at), Ok(r), Ok(c)) = (pos_of(&self.doc, row, 0), usize::try_from(r), usize::try_from(c)) else {
            return QString::default();
        };
        let i = at.block;
        if table::size(&self.doc, i).is_none() {
            return QString::default();
        }
        let (a, b) = (a.max(0) as usize, b.max(0) as usize);
        let old = self.doc.blocks.clone();
        let cell = (r, c);
        let name = name.to_string();
        let to = {
            let mut this = self.as_mut().rust_mut();
            let doc = &mut this.doc;
            let aligned = |doc: &mut Document, how| table::align(doc, i, a.min(b)..=a.max(b), how).then_some(cell);
            match name.as_str() {
                "rowAbove" | "rowBelow" => table::add_row(doc, i, r, name == "rowAbove"),
                "columnBefore" | "columnAfter" => table::add_column(doc, i, cell, name == "columnBefore"),
                "deleteRow" => table::delete_row(doc, i, cell),
                "deleteColumn" => table::delete_column(doc, i, cell),
                "rowUp" | "rowDown" => table::move_row(doc, i, cell, name == "rowUp"),
                "columnLeft" | "columnRight" => table::move_column(doc, i, cell, name == "columnLeft"),
                "dragRow" => table::drag_row(doc, i, a, b, cell),
                "dragColumn" => table::drag_column(doc, i, a, b, cell),
                "alignLeft" => aligned(doc, md::Align::Left),
                "alignCenter" => aligned(doc, md::Align::Center),
                "alignRight" => aligned(doc, md::Align::Right),
                "resize" => table::resize(doc, i, a, b, cell),
                "prettify" => table::prettify(doc, i).then_some(cell),
                "deleteTable" => {
                    let p = edit::delete_block(doc, i);
                    drop(this);
                    let after = Sel::caret(p);
                    self.as_mut().commit(old, Sel::caret(table::caret(i, cell)), after, Merge::Never);
                    self.request_sel(after);
                    return QString::from("out");
                }
                _ => None,
            }
        };
        let Some(to) = to else { return QString::default() };
        self.as_mut().commit(old, Sel::caret(table::caret(i, cell)), Sel::caret(table::caret(i, to)), Merge::Never);
        QString::from(&format!("{},{}", to.0, to.1))
    }

    /// A cell span's table block and ends, in bytes.
    #[allow(clippy::too_many_arguments)]
    fn cell_span(&self, row: i32, r1: i32, c1: i32, p1: i32, r2: i32, c2: i32, p2: i32) -> Option<(usize, table::At, table::At)> {
        let i = pos_of(&self.doc, row, 0)?.block;
        let BlockKind::Table { rows, .. } = &self.doc.blocks[i].kind else { return None };
        let at = |r: i32, c: i32, p: i32| -> Option<table::At> {
            let (r, c) = (usize::try_from(r).ok()?, usize::try_from(c).ok()?);
            Some(((r, c), byte_of(rows.get(r)?.get(c)?, p)))
        };
        Some((i, at(r1, c1, p1)?, at(r2, c2, p2)?))
    }

    #[allow(clippy::too_many_arguments)]
    fn replace_cell_span(mut self: Pin<&mut Self>, row: i32, r1: i32, c1: i32, p1: i32, r2: i32, c2: i32, p2: i32, text: &QString) -> QString {
        let Some((i, a, z)) = self.cell_span(row, r1, c1, p1, r2, c2, p2) else { return QString::default() };
        let old = self.doc.blocks.clone();
        let Some(((r, c), at)) = table::replace_span(&mut self.as_mut().rust_mut().doc, i, a, z, &text.to_string()) else {
            return QString::default();
        };
        self.as_mut().commit(old, Sel::caret(table::caret(i, a.0)), Sel::caret(table::caret(i, (r, c))), Merge::Never);
        let BlockKind::Table { rows, .. } = &self.doc.blocks[i].kind else { return QString::default() };
        QString::from(&format!("{r},{c},{}", utf16_of(&rows[r][c], at)))
    }

    #[allow(clippy::too_many_arguments)]
    fn cell_span_copy(&self, row: i32, r1: i32, c1: i32, p1: i32, r2: i32, c2: i32, p2: i32) -> QString {
        let Some(md) = self.cell_span(row, r1, c1, p1, r2, c2, p2).and_then(|(i, a, z)| table::span_markdown(&self.doc, i, a, z)) else {
            return QString::default();
        };
        let html = self.html_of(md::parse_with(&md, self.opts.latex_delimiters, self.opts.remap));
        QString::from(&format!("[{},{}]", json_str(&md), json_str(&html)))
    }

    #[allow(clippy::too_many_arguments)]
    fn cell_span_counts(&self, row: i32, r1: i32, c1: i32, p1: i32, r2: i32, c2: i32, p2: i32) -> QString {
        let Some(cells) = self.cell_span(row, r1, c1, p1, r2, c2, p2).and_then(|(i, a, z)| table::span_cells(&self.doc, i, a, z)) else {
            return QString::from("0,0");
        };
        let text = query::cells_text(&cells, &self.refs, &self.opts);
        let text = text.trim_end_matches('\n');
        QString::from(&format!("{},{}", query::words(text), text.encode_utf16().count()))
    }

    fn selection_markdown(&self, a_row: i32, a_pos: i32, z_row: i32, z_pos: i32) -> QString {
        let (Some(a), Some(z)) = (pos_of(&self.doc, a_row, a_pos), pos_of(&self.doc, z_row, z_pos)) else {
            return QString::default();
        };
        QString::from(&md::serialize(&edit::slice(&self.doc, a, z, &self.refs, &self.opts)))
    }

    fn selection_counts(&self, a_row: i32, a_pos: i32, z_row: i32, z_pos: i32) -> QString {
        let (Some(a), Some(z)) = (pos_of(&self.doc, a_row, a_pos), pos_of(&self.doc, z_row, z_pos)) else {
            return QString::from("0,0");
        };
        if a == z {
            return QString::from("0,0");
        }
        let part = edit::slice(&self.doc, a, z, &self.refs, &self.opts);
        let text = query::selected_text(&part, &self.refs, &self.opts);
        QString::from(&format!("{},{}", query::words(&text), text.encode_utf16().count()))
    }

    fn selection_html(&self, a_row: i32, a_pos: i32, z_row: i32, z_pos: i32) -> QString {
        let (Some(a), Some(z)) = (pos_of(&self.doc, a_row, a_pos), pos_of(&self.doc, z_row, z_pos)) else {
            return QString::default();
        };
        QString::from(&self.html_of(edit::slice(&self.doc, a, z, &self.refs, &self.opts)))
    }

    /// Part of the document as HTML for the clipboard.
    fn html_of(&self, mut part: Document) -> String {
        // Link definitions elsewhere in the document still apply.
        part.blocks.extend(self.doc.blocks.iter().filter(|b| matches!(b.kind, BlockKind::LinkRefDef)).cloned());
        // Math as rendered SVG, in the colour of the text it lands in.
        let svg = |tex: &str, display: bool| crate::math::svg(tex, display, "010203").ok().map(|s| s.replace("#010203", "currentColor"));
        md::html::render_math(&part, &self.opts, self.math_svg.then_some(&svg as &dyn Fn(&str, bool) -> Option<String>))
    }

    fn scope_at(&self, row: i32, pos: i32) -> QString {
        let Some(p) = pos_of(&self.doc, row, pos) else { return QString::default() };
        let b = &self.doc.blocks[p.block];
        if !b.kind.is_inline_text() {
            return QString::default();
        }
        let nodes = inline::parse(&b.text, &self.refs, &self.opts);
        let mut best: Option<std::ops::Range<usize>> = None;
        let mut level: &[inline::Node] = &nodes;
        'down: loop {
            for n in level {
                if n.range.start <= p.offset && p.offset <= n.range.end && !n.content.is_empty() {
                    let styled = !matches!(
                        n.kind,
                        inline::Kind::Text
                            | inline::Kind::Escape
                            | inline::Kind::Entity(_)
                            | inline::Kind::Smart(_)
                            | inline::Kind::SoftBreak
                            | inline::Kind::HardBreak
                    );
                    if styled {
                        best = Some(n.content.clone());
                    }
                    level = &n.children;
                    continue 'down;
                }
            }
            break;
        }
        match best {
            Some(r) => QString::from(&format!("{},{}", utf16_of(&b.text, r.start), utf16_of(&b.text, r.end))),
            None => QString::default(),
        }
    }

    fn pair_typed(&self, row: i32, pos: i32, ch: &QString, markdown: bool) -> QString {
        let ch = ch.to_string();
        let (Some(p), Some(c)) = (pos_of(&self.doc, row, pos), ch.chars().next()) else { return QString::default() };
        let b = &self.doc.blocks[p.block];
        if edit::is_atomic(&b.kind) || ch.chars().count() != 1 {
            return QString::default();
        }
        typed_reply(md::pair::typed(&b.kind, &b.text, p.offset, c, markdown, &self.refs, &self.opts))
    }

    fn cell_pair_typed(&self, text: &QString, pos: i32, ch: &QString, markdown: bool) -> QString {
        let (text, ch) = (text.to_string(), ch.to_string());
        let mut chars = ch.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else { return QString::default() };
        let at = byte_of(&text, pos);
        typed_reply(md::pair::typed(&BlockKind::Paragraph, &text, at, c, markdown, &self.refs, &self.opts))
    }

    fn smart_typed(&self, row: i32, pos: i32, ch: &QString, pair: bool, markdown: bool) -> QString {
        let Some(p) = pos_of(&self.doc, row, pos) else { return QString::default() };
        let b = &self.doc.blocks[p.block];
        if !matches!(b.kind, BlockKind::Paragraph | BlockKind::Heading { .. }) {
            return QString::default();
        }
        self.smart_reply(&b.kind, &b.text, p.offset, &ch.to_string(), pair, markdown)
    }

    fn cell_smart_typed(&self, text: &QString, pos: i32, ch: &QString, pair: bool, markdown: bool) -> QString {
        let text = text.to_string();
        self.smart_reply(&BlockKind::Paragraph, &text, byte_of(&text, pos), &ch.to_string(), pair, markdown)
    }

    fn smart_reply(&self, kind: &BlockKind, text: &str, at: usize, ch: &str, pair: bool, markdown: bool) -> QString {
        let mut chars = ch.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else { return QString::default() };
        if self.opts.smart.is_some() || !self.smart.any() {
            return QString::default();
        }
        let nodes = inline::parse(text, &self.refs, &self.opts);
        let Some((back, put)) = md::smart::typed(text, &nodes, at, c, &self.smart) else { return QString::default() };
        let q = self.smart.pairs;
        let mut out = put.clone();
        if pair && (put == q[1].to_string() || put == q[3].to_string()) && text[at..].starts_with(&put) {
            return QString::from("skip");
        }
        // An opening quote gets its closer where the ASCII one would.
        let closer = if put == q[0].to_string() { Some(q[1]) } else if put == q[2].to_string() { Some(q[3]) } else { None };
        if let Some(close) = closer.filter(|_| pair) {
            if let md::pair::Typed::Pair(_) = md::pair::typed(kind, text, at, c, markdown, &self.refs, &self.opts) {
                out.push(close);
            }
        }
        let caret = put.chars().count();
        QString::from(&serde_json::json!({ "back": back, "text": out, "caret": caret }).to_string())
    }

    fn set_smart(mut self: Pin<&mut Self>, quotes: bool, dashes: bool, to_em: bool, pairs: &QString, on_rendering: bool) {
        let smart = md::smart::Smart::new(quotes, dashes, to_em, &pairs.to_string());
        let shown = (on_rendering && smart.any()).then(|| smart.clone());
        self.as_mut().rust_mut().smart = smart;
        let opts = Options { smart: shown, ..self.opts.clone() };
        self.apply_options(opts);
    }

    fn set_writing(mut self: Pin<&mut Self>, hide_br: bool, math_svg: bool, code_lang: &QString, when: i32) {
        let changed = self.hide_br != hide_br;
        {
            let mut r = self.as_mut().rust_mut();
            r.hide_br = hide_br;
            r.math_svg = math_svg;
            r.code_lang = code_lang.to_string();
            r.code_lang_when = when;
        }
        if changed {
            super::shared::set_hide_br(*self.doc_id(), hide_br);
            let rev = *self.inline_revision() + 1;
            self.as_mut().set_inline_revision(rev);
        }
    }

    fn code_lang_suggestion(&self, menu: bool) -> QString {
        QString::from(&self.lang_suggestion(menu))
    }

    /// As Typora suggests a new code fence's language: the front matter's
    /// `defaultCodeLang`, else the preference's, "__LAST" being the last
    /// language a fence was given; nothing when the preference doesn't
    /// apply to fences added this way.
    fn lang_suggestion(&self, menu: bool) -> String {
        if self.code_lang_when & if menu { 2 } else { 1 } == 0 {
            return String::new();
        }
        let lang = image::front_matter(&self.doc, "defaultCodeLang").unwrap_or_else(|| self.code_lang.clone());
        if lang == "__LAST" { self.last_lang.clone() } else { lang }
    }

    fn cell_paired(&self, text: &QString, pos: i32, markdown: bool) -> bool {
        let text = text.to_string();
        md::pair::paired(&BlockKind::Paragraph, &text, byte_of(&text, pos), markdown, &self.opts)
    }

    fn context_at(&self, row: i32, pos: i32) -> QString {
        let Some(p) = pos_of(&self.doc, row, pos) else { return QString::default() };
        let b = &self.doc.blocks[p.block];
        let mut out: Vec<&str> = Vec::new();
        if b.kind.is_inline_text() {
            let nodes = inline::parse(&b.text, &self.refs, &self.opts);
            let mut level: &[inline::Node] = &nodes;
            'down: loop {
                for n in level {
                    if n.content.start <= p.offset && p.offset <= n.content.end && n.range.start < n.range.end {
                        let name = match n.kind {
                            inline::Kind::Strong => "strong",
                            inline::Kind::Emph => "em",
                            inline::Kind::Code => "code",
                            inline::Kind::Link { .. } | inline::Kind::Autolink { .. } => "link",
                            _ => "",
                        };
                        if !name.is_empty() && !out.contains(&name) {
                            out.push(name);
                        }
                        level = &n.children;
                        continue 'down;
                    }
                }
                break;
            }
        }
        for c in &b.containers {
            let name = match c.kind {
                ContainerKind::Quote { .. } => "quote",
                ContainerKind::ListItem(_) => "list",
                ContainerKind::Footnote { .. } => continue,
            };
            if !out.contains(&name) {
                out.push(name);
            }
        }
        QString::from(&out.join(","))
    }

    fn math_at(&self, row: i32, pos: i32) -> QString {
        let Some(p) = pos_of(&self.doc, row, pos) else { return QString::default() };
        let b = &self.doc.blocks[p.block];
        let json = |tex: &str, display: bool, fence: bool| {
            QString::from(&format!("{{\"tex\":{},\"display\":{display},\"fence\":{fence}}}", json_str(tex)))
        };
        match &b.kind {
            BlockKind::Math { .. } => return json(&b.text, true, false),
            BlockKind::CodeFence { info, .. } if info.trim().eq_ignore_ascii_case("math") => return json(&b.text, true, true),
            kind if !kind.is_inline_text() => return QString::default(),
            _ => {}
        }
        let nodes = inline::parse(&b.text, &self.refs, &self.opts);
        let mut level: &[inline::Node] = &nodes;
        while let Some(n) = level.iter().find(|n| n.content.start <= p.offset && p.offset <= n.content.end) {
            if n.kind == inline::Kind::Math {
                return json(&b.text[n.content.clone()], false, false);
            }
            level = &n.children;
        }
        QString::default()
    }

    fn html_to_markdown(&self, html: &QString) -> QString {
        QString::from(&md::from_html::to_markdown(&html.to_string(), &self.opts))
    }

    fn paired_at(&self, row: i32, pos: i32, markdown: bool) -> bool {
        let Some(p) = pos_of(&self.doc, row, pos) else { return false };
        let b = &self.doc.blocks[p.block];
        !edit::is_atomic(&b.kind) && md::pair::paired(&b.kind, &b.text, p.offset, markdown, &self.opts)
    }

    fn image_url(&self, text: &QString) -> QString {
        let text = text.to_string();
        let nodes = inline::parse(&text, &self.refs, &self.opts);
        let Some(src) = inline::lone_image(&text, &nodes).and_then(|n| n.kind.image_src()) else {
            return QString::default();
        };
        self.resolve_image(&QString::from(src))
    }

    fn image_size(&self, text: &QString) -> QString {
        let text = text.to_string();
        let nodes = inline::parse(&text, &self.refs, &self.opts);
        let size = match inline::lone_image(&text, &nodes).map(|n| &n.kind) {
            Some(inline::Kind::ImgTag(t)) => [t.width.as_str(), t.height.as_str(), t.zoom.as_str()],
            _ => ["", "", ""],
        };
        QString::from(&format!("[{}]", size.map(json_str).join(",")))
    }

    fn html_look(&self, text: &QString) -> QString {
        QString::from(match htmlview::look(&text.to_string()) {
            htmlview::Look::Rendered => "rendered",
            htmlview::Look::Source => "source",
            htmlview::Look::Comment => "comment",
        })
    }

    fn html_view(&self, text: &QString, toggled: &QString, room: f64, colors: &QString) -> QString {
        let toggled = toggled.to_string().split(',').filter_map(|s| s.trim().parse().ok()).collect();
        let colors = colors.to_string();
        let mut c = colors.split(',').map(str::trim);
        let mut next = || c.next().unwrap_or("#808080").to_string();
        let (border, code_background, mark, faint, link, foreground) = (next(), next(), next(), next(), next(), next());
        let root = self.image_root.as_deref().map(|r| image::root(r, self.file.as_deref()));
        let resolve = |src: &str| {
            let url = image::url(src, self.file.as_deref(), root.as_deref());
            let file = image::local_path(src, self.file.as_deref(), root.as_deref());
            (url, file)
        };
        let page = htmlview::Page {
            border: &border,
            code_background: &code_background,
            mark: &mark,
            faint: &faint,
            link: &link,
            text: &foreground,
            room,
            toggled: &toggled,
            resolve: &resolve,
        };
        QString::from(&htmlview::rich_text(&text.to_string(), &page))
    }

    fn resolve_image(&self, src: &QString) -> QString {
        let root = self.image_root.as_deref().map(|r| image::root(r, self.file.as_deref()));
        QString::from(&image::url(&src.to_string(), self.file.as_deref(), root.as_deref()))
    }

    fn link_at(&self, row: i32, pos: i32) -> QString {
        let Some(p) = pos_of(&self.doc, row, pos) else { return QString::default() };
        let b = &self.doc.blocks[p.block];
        if !b.kind.is_inline_text() {
            return QString::default();
        }
        fn find(nodes: &[inline::Node], at: usize) -> Option<String> {
            for n in nodes {
                if n.range.start <= at && at <= n.range.end {
                    if let Some(d) = find(&n.children, at) {
                        return Some(d);
                    }
                    if let inline::Kind::Link { dest, .. } | inline::Kind::Autolink { dest } = &n.kind {
                        return Some(dest.clone());
                    }
                }
            }
            None
        }
        let nodes = inline::parse(&b.text, &self.refs, &self.opts);
        QString::from(&find(&nodes, p.offset).unwrap_or_default())
    }

    fn heading_row(&self, anchor: &QString) -> i32 {
        query::heading_with_anchor(&self.doc, &self.refs, &self.opts, &anchor.to_string()).map_or(-1, |i| i as i32)
    }

    /// Put the views' caret and selection at `sel`.
    fn request_sel(self: Pin<&mut Self>, sel: Sel) {
        let u = |p: Pos| self.doc.blocks.get(p.block).map(|b| utf16_of_pos(b, p.offset));
        let (Some(anchor), Some(pos)) = (u(sel.anchor), u(sel.caret)) else { return };
        if sel.anchor.block == sel.caret.block {
            self.caret_requested(sel.caret.block as i32, anchor, pos);
        } else {
            self.selection_requested(sel.anchor.block as i32, anchor, sel.caret.block as i32, pos);
        }
    }

    fn undo_redo(mut self: Pin<&mut Self>, redo: bool) -> bool {
        let step = {
            let r = self.as_mut().rust_mut();
            let r = r.get_mut();
            if redo { r.history.redo(&mut r.doc) } else { r.history.undo(&mut r.doc) }
        };
        let Some((splice, sel)) = step else { return false };
        for (backup, path) in self.history.files_of_last(redo) {
            if redo {
                let _ = fs::remove_file(&path);
            } else if !path.exists() {
                let _ = path.parent().map(fs::create_dir_all);
                let _ = fs::copy(&backup, &path);
            }
        }
        self.as_mut().apply_splice(splice);
        self.request_sel(sel);
        true
    }

    fn toggle_task(mut self: Pin<&mut Self>, row: i32) {
        let Some(i) = usize::try_from(row).ok().filter(|&i| i < self.doc.blocks.len()) else { return };
        let old = self.doc.blocks.clone();
        md::edit::toggle_task(&mut self.as_mut().rust_mut().doc, i);
        let sel = Sel::caret(Pos::new(i, 0));
        self.commit(old, sel, sel, Merge::Never);
    }

    fn set_new_line_ending(mut self: Pin<&mut Self>, crlf: bool) {
        self.as_mut().rust_mut().new_crlf = crlf;
        // The untitled document a window starts with is new too.
        if self.file.is_none() && !self.modified && !self.dirty {
            self.as_mut().rust_mut().doc.line_ending = if crlf { md::LineEnding::CrLf } else { md::LineEnding::Lf };
            self.as_mut().set_crlf(crlf);
        }
    }

    fn set_line_ending(mut self: Pin<&mut Self>, crlf: bool) {
        let ending = if crlf { md::LineEnding::CrLf } else { md::LineEnding::Lf };
        if self.doc.line_ending != ending {
            self.as_mut().rust_mut().doc.line_ending = ending;
            self.as_mut().set_crlf(crlf);
            // Line endings aren't in the block list, so the history can't
            // see this change.
            self.as_mut().rust_mut().dirty = true;
            self.as_mut().sync_history();
        }
    }

    /// Recompute the link reference definitions and the image root; when
    /// they changed, links and images in every block may resolve
    /// differently.
    fn update_refs(mut self: Pin<&mut Self>) {
        let refs = inline::refs_of(&self.doc);
        let root = image::root_setting(&self.doc);
        if refs != self.refs || root != self.image_root {
            super::shared::set_refs(*self.doc_id(), refs.clone());
            self.as_mut().rust_mut().refs = refs;
            self.as_mut().rust_mut().image_root = root;
            let rev = *self.inline_revision() + 1;
            self.as_mut().set_inline_revision(rev);
        }
    }

    fn set_image_options(
        mut self: Pin<&mut Self>,
        action: i32,
        folder: &QString,
        copy_local: bool,
        relative: bool,
        with_dot: bool,
        escape: bool,
    ) {
        let folder = folder.to_string();
        self.as_mut().rust_mut().image_opts = image::InsertOptions { action, folder, copy_local, relative, with_dot, escape };
    }

    /// The folder inserted images are copied to, and whether the front
    /// matter named it.
    fn image_storage(&self) -> Option<(PathBuf, bool)> {
        image::storage(image::copy_setting(&self.doc).as_deref(), &self.image_opts, self.file.as_deref())
    }

    fn missing_image_folder(&self, paths: &QString) -> QString {
        let Some((folder, _)) = self.image_storage().filter(|(f, _)| !f.is_dir()) else { return QString::default() };
        let copies = paths.to_string().lines().any(|p| {
            image::media(p) == image::Media::Image && Path::new(p).is_absolute() && image::needs_copy(Path::new(p), &folder, &self.image_opts)
        });
        if copies { QString::from(&*folder.to_string_lossy()) } else { QString::default() }
    }

    fn pasted_image_folder(&self, fallback: &QString) -> QString {
        match self.image_storage() {
            Some((folder, _)) => QString::from(&*folder.to_string_lossy()),
            None => fallback.clone(),
        }
    }

    fn insert_files(
        mut self: Pin<&mut Self>,
        a_row: i32,
        a_pos: i32,
        z_row: i32,
        z_pos: i32,
        paths: &QString,
        create: bool,
    ) -> QString {
        let (Some(anchor), Some(caret)) = (pos_of(&self.doc, a_row, a_pos), pos_of(&self.doc, z_row, z_pos)) else {
            return QString::default();
        };
        let paths: Vec<String> = paths.to_string().lines().map(str::trim).filter(|p| !p.is_empty()).map(String::from).collect();
        if paths.is_empty() {
            return QString::default();
        }
        let old = self.doc.blocks.clone();
        let mut error = String::new();
        let (start, end) = if anchor <= caret { (anchor, caret) } else { (caret, anchor) };
        let mut at = edit::delete_range(&mut self.as_mut().rust_mut().doc, start, end);
        for (k, path) in paths.iter().enumerate() {
            let text = self.file_markup(path, at.offset > 0, create, &mut error);
            let doc = &mut self.as_mut().rust_mut().doc;
            // A file after the first, or one dropped on a rule or the
            // like, goes in a paragraph of its own after the block.
            if k > 0 || edit::is_atomic(&doc.blocks[at.block].kind) {
                at = edit::insert_paragraph(doc, at.block, true);
            }
            // At the start of a block a link needs no space before it.
            let text = if at.offset == 0 { text.trim_start() } else { &text };
            at = edit::replace_range(doc, at, at, text);
        }
        let after = Sel::caret(at);
        self.as_mut().commit(old, Sel { anchor, caret }, after, Merge::Never);
        self.request_sel(after);
        QString::from(&error)
    }

    /// The text block and offset at (`row`, `pos`), and the image there.
    fn image_here(&self, row: i32, pos: i32) -> Option<(Pos, std::ops::Range<usize>, String, &'static str)> {
        let p = pos_of(&self.doc, row, pos)?;
        let b = &self.doc.blocks[p.block];
        if !b.kind.is_inline_text() {
            return None;
        }
        let (range, src, syntax) = edit::image_at(&b.text, p.offset..p.offset, &self.refs)?;
        Some((p, range, src, syntax))
    }

    fn image_root_path(&self) -> Option<PathBuf> {
        self.image_root.as_deref().map(|r| image::root(r, self.file.as_deref()))
    }

    fn image_at(&self, row: i32, pos: i32) -> QString {
        let Some((_, _, src, syntax)) = self.image_here(row, pos) else { return QString::default() };
        self.image_info(&src, syntax)
    }

    fn cell_image_at(&self, text: &QString, pos: i32) -> QString {
        let text = text.to_string();
        let at = byte_of(&text, pos);
        let Some((_, src, syntax)) = edit::image_at(&text, at..at, &self.refs) else { return QString::default() };
        self.image_info(&src, syntax)
    }

    fn image_info(&self, src: &str, syntax: &str) -> QString {
        let root = self.image_root_path();
        let url = image::url(src, self.file.as_deref(), root.as_deref());
        let path = image::local_path(src, self.file.as_deref(), root.as_deref());
        // "Copy to assets" shows when the image isn't directly in the folder.
        let folder = self
            .image_storage()
            .map(|(f, _)| f)
            .filter(|f| path.as_deref().and_then(Path::parent) != Some(f.as_path()))
            .and_then(|f| f.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_default();
        let ratio = path.as_deref().map_or(1.0, image::pixel_ratio);
        let path = path.map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        QString::from(&format!(
            "{{\"src\":{},\"url\":{},\"path\":{},\"syntax\":\"{syntax}\",\"folder\":{},\"ratio\":{ratio}}}",
            json_str(src),
            json_str(&url),
            json_str(&path),
            json_str(&folder)
        ))
    }

    /// Copy the file of the image written `src` to `target`, or into the
    /// folder images are copied to when `target` is empty; with `move_file`
    /// the original goes. The source that points at the copy, or None when
    /// there is nothing to copy.
    fn copy_image(&self, src: &str, target: &str, move_file: bool) -> Result<Option<String>, String> {
        let root = self.image_root_path();
        let Some(from) = image::local_path(src, self.file.as_deref(), root.as_deref()).filter(|f| f.is_file()) else {
            return Err("Image src is not a file".into());
        };
        let copied = if target.is_empty() {
            let Some((folder, _)) = self.image_storage() else { return Ok(None) };
            fs::create_dir_all(&folder).and_then(|_| image::copy_into(&from, &folder))
        } else {
            let to = PathBuf::from(target);
            if to == from {
                return Ok(None);
            }
            fs::copy(&from, &to).map(|_| to)
        };
        let to = copied.map_err(|e| e.to_string())?;
        if move_file {
            let _ = fs::remove_file(&from);
        }
        Ok(Some(image::insert_src(&to.to_string_lossy(), self.file.as_deref(), root.as_deref(), &self.image_opts)))
    }

    fn copy_image_file(&self, src: &QString, target: &QString, move_file: bool) -> QString {
        let (src, error) = match self.copy_image(&src.to_string(), &target.to_string(), move_file) {
            Ok(src) => (src.unwrap_or_default(), String::new()),
            Err(e) => (String::new(), e),
        };
        QString::from(&format!("{{\"src\":{},\"error\":{}}}", json_str(&src), json_str(&error)))
    }

    fn relocate_image(mut self: Pin<&mut Self>, row: i32, pos: i32, target: &QString, move_file: bool) -> QString {
        let Some((p, range, src, _)) = self.image_here(row, pos) else { return QString::default() };
        let new_src = match self.copy_image(&src, &target.to_string(), move_file) {
            Ok(Some(new_src)) => new_src,
            Ok(None) => return QString::default(),
            Err(e) => return QString::from(&e),
        };
        let text = self.doc.blocks[p.block].text.clone();
        let Some((out, sel)) = edit::set_image_src(&text, range, &self.refs, &new_src) else { return QString::default() };
        let old = self.doc.blocks.clone();
        self.as_mut().rust_mut().doc.blocks[p.block].set_text(out);
        let after = Sel { anchor: Pos::new(p.block, sel.start), caret: Pos::new(p.block, sel.end) };
        self.as_mut().commit(old, Sel::caret(p), after, Merge::Never);
        self.request_sel(after);
        QString::default()
    }

    fn trash_image_file(mut self: Pin<&mut Self>, path: &QString) -> bool {
        static BACKUPS: AtomicI32 = AtomicI32::new(0);
        let file = PathBuf::from(path.to_string());
        let Some(name) = file.file_name() else { return false };
        // ponytail: backups stay in the temp folder until the system clears it.
        let dir = std::env::temp_dir()
            .join(format!("typo-{}-images", std::process::id()))
            .join(BACKUPS.fetch_add(1, Ordering::Relaxed).to_string());
        let backup = dir.join(name);
        let kept = fs::create_dir_all(&dir).and_then(|_| fs::copy(&file, &backup)).is_ok();
        if !super::desktop::qobject::move_file_to_trash(path) {
            return false;
        }
        if kept {
            self.as_mut().rust_mut().history.keep_file(backup, file);
        }
        true
    }

    fn delete_image(mut self: Pin<&mut Self>, row: i32, pos: i32) -> bool {
        let Some((p, range, ..)) = self.image_here(row, pos) else { return false };
        let old = self.doc.blocks.clone();
        let mut text = self.doc.blocks[p.block].text.clone();
        text.replace_range(range.clone(), "");
        self.as_mut().rust_mut().doc.blocks[p.block].set_text(text);
        let after = Sel::caret(Pos::new(p.block, range.start));
        self.as_mut().commit(old, Sel::caret(p), after, Merge::Never);
        self.request_sel(after);
        true
    }

    fn relocate_all_images(mut self: Pin<&mut Self>, folder: &QString, move_file: bool, row: i32, pos: i32) -> QString {
        let folder = PathBuf::from(folder.to_string());
        let root = self.image_root_path();
        let file = self.file.clone();
        // A file shown by several images is copied once.
        let mut copies: HashMap<PathBuf, PathBuf> = HashMap::new();
        let mut error = String::new();
        let (refs, opts) = (&self.refs, &self.image_opts);
        let mut repoint = |old: &str| {
            let mut text = old.to_string();
            // Last to first, so the ranges before stay put.
            for (range, src) in edit::images(old, refs).into_iter().rev() {
                let Some(from) = image::local_path(&src, file.as_deref(), root.as_deref()).filter(|f| f.is_file()) else {
                    continue;
                };
                if from.parent() == Some(folder.as_path()) {
                    continue;
                }
                let to = match copies.get(&from) {
                    Some(to) => to.clone(),
                    None => match image::copy_into(&from, &folder) {
                        Ok(to) => copies.entry(from).or_insert(to).clone(),
                        Err(e) => {
                            error = e.to_string();
                            continue;
                        }
                    },
                };
                let new_src = image::insert_src(&to.to_string_lossy(), file.as_deref(), root.as_deref(), opts);
                if let Some((out, _)) = edit::set_image_src(&text, range, refs, &new_src) {
                    text = out;
                }
            }
            text
        };
        let mut changed = Vec::new();
        for (i, b) in self.doc.blocks.iter().enumerate() {
            let mut new = b.clone();
            match &b.kind {
                k if k.is_inline_text() => new.set_text(repoint(&b.text)),
                BlockKind::Table { align, rows } => new.set_kind(BlockKind::Table {
                    align: align.clone(),
                    rows: rows.iter().map(|row| row.iter().map(|cell| repoint(cell)).collect()).collect(),
                }),
                _ => continue,
            }
            if new != *b {
                changed.push((i, new));
            }
        }
        if move_file {
            for from in copies.keys() {
                let _ = fs::remove_file(from);
            }
        }
        if !changed.is_empty() {
            let old = self.doc.blocks.clone();
            let caret = pos_of(&self.doc, row, pos).unwrap_or(Pos::new(0, 0));
            for (i, block) in changed {
                self.as_mut().rust_mut().doc.blocks[i] = block;
            }
            let text = &self.doc.blocks[caret.block].text;
            let mut offset = caret.offset.min(text.len());
            while !text.is_char_boundary(offset) {
                offset -= 1;
            }
            let after = Sel::caret(Pos::new(caret.block, offset));
            self.as_mut().commit(old, Sel::caret(caret), after, Merge::Never);
        }
        QString::from(&format!("{{\"count\":{},\"error\":{}}}", copies.len(), json_str(&error)))
    }

    fn math_tags(&self, numbering: &QString, fences: bool) -> QString {
        let formulas: Vec<(usize, &str)> = (self.doc.blocks.iter().enumerate())
            .filter(|(_, b)| match &b.kind {
                BlockKind::Math { .. } => true,
                BlockKind::CodeFence { info, .. } => fences && info.trim().eq_ignore_ascii_case("math"),
                _ => false,
            })
            .map(|(i, b)| (i, b.text.as_str()))
            .collect();
        let texts: Vec<&str> = formulas.iter().map(|(_, t)| *t).collect();
        let tags = eqnum::number(&texts, eqnum::Numbering::from_setting(&numbering.to_string()));
        let rows: Vec<String> = (formulas.iter().zip(&tags.rows))
            .filter(|(_, rows)| rows.iter().any(|r| !r.is_empty()))
            .map(|((i, _), rows)| {
                let rows: Vec<String> = rows.iter().map(|r| json_str(r)).collect();
                format!("\"{i}\":[{}]", rows.join(","))
            })
            .collect();
        let mut labels: Vec<String> = tags.labels.iter().map(|(k, v)| format!("{}:{}", json_str(k), json_str(v))).collect();
        labels.sort();
        QString::from(&format!("{{\"rows\":{{{}}},\"labels\":{{{}}}}}", rows.join(","), labels.join(",")))
    }

    fn image_setting(&self, name: &QString) -> QString {
        let value = match name.to_string().as_str() {
            "copy" => image::copy_setting(&self.doc),
            _ => image::root_setting(&self.doc),
        };
        QString::from(&value.unwrap_or_default())
    }

    fn set_image_setting(mut self: Pin<&mut Self>, name: &QString, folder: &QString, row: i32, pos: i32) -> bool {
        let key = if name.to_string() == "copy" { "typora-copy-images-to" } else { "typora-root-url" };
        let folder = folder.to_string();
        let value = match self.file.as_deref().and_then(Path::parent) {
            _ if folder.is_empty() => None,
            Some(base) => Some(image::relative(base, Path::new(&folder), true)),
            None => Some(folder),
        };
        let caret = pos_of(&self.doc, row, pos).unwrap_or(Pos::new(0, 0));
        let old = self.doc.blocks.clone();
        let had = old.first().is_some_and(|b| b.kind == BlockKind::FrontMatter);
        if !edit::set_property(&mut self.as_mut().rust_mut().doc, key, value.as_deref()) {
            return false;
        }
        // New front matter pushes every block down one.
        let block = if had { caret.block } else { caret.block + 1 };
        let text = &self.doc.blocks[block].text;
        let mut offset = caret.offset.min(text.len());
        while !text.is_char_boundary(offset) {
            offset -= 1;
        }
        let after = Sel::caret(Pos::new(block, offset));
        self.as_mut().commit(old, Sel::caret(caret), after, Merge::Never);
        self.request_sel(after);
        true
    }

    fn files_markup(&self, paths: &QString, after_text: bool, create: bool) -> QString {
        let mut error = String::new();
        let parts: Vec<String> = paths
            .to_string()
            .lines()
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .enumerate()
            .map(|(k, p)| self.file_markup(p, after_text || k > 0, create, &mut error))
            .collect();
        QString::from(&parts.concat())
    }

    /// The markdown for inserting the file at `path`, after the copy the
    /// preferences ask for (making the folder when `create`): an image, a
    /// video or audio tag, or a link. `after_text` puts a space before an
    /// image. A failed copy leaves the original path and says why in
    /// `error`.
    fn file_markup(&self, path: &str, after_text: bool, create: bool, error: &mut String) -> String {
        let root = self.image_root.as_deref().map(|r| image::root(r, self.file.as_deref()));
        let file = self.file.as_deref();
        let opts = &self.image_opts;
        let mut path = path.to_string();
        let kind = image::media(&path);
        if kind == image::Media::Image
            && let Some((folder, _)) = self.image_storage()
            && Path::new(&path).is_absolute()
            && image::needs_copy(Path::new(&path), &folder, opts)
        {
            if !folder.is_dir() && create {
                if let Err(e) = fs::create_dir_all(&folder) {
                    *error = e.to_string();
                }
            }
            if folder.is_dir() {
                match image::copy_into(Path::new(&path), &folder) {
                    Ok(copy) => path = copy.to_string_lossy().into_owned(),
                    Err(e) => *error = e.to_string(),
                }
            }
        }
        let src = || image::insert_src(&path, file, root.as_deref(), opts);
        match kind {
            // A space keeps the image off the text before it.
            image::Media::Image => format!("{}![{}]({})", if after_text { " " } else { "" }, image::alt_of(&path), src()),
            image::Media::Video => format!("<video src=\"{}\" controls></video>", src()),
            image::Media::Audio => format!("<audio src=\"{}\"></audio>", src()),
            image::Media::Other if path.contains("://") => format!(" {path} "),
            image::Media::Other => {
                let name = Path::new(&path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let folder = file.and_then(Path::parent);
                let to = folder.map(|f| image::relative(f, Path::new(&path), false)).unwrap_or_else(|| path.clone());
                format!(" [{name}]({to}) ")
            }
        }
    }

    fn set_syntax(&self, bullet: &QString, ordered_same: bool, indent: i32, pretty: bool, heading: i32) {
        edit::set_syntax(edit::Syntax {
            bullet: bullet.to_string().chars().next().filter(|c| "-+*".contains(*c)).unwrap_or(edit::BULLET),
            ordered_same,
            indent: indent.clamp(2, 5) as usize,
            pretty,
            heading: match heading {
                1 => edit::HeadingStyle::Setext,
                2 => edit::HeadingStyle::AtxClosed,
                3 => edit::HeadingStyle::SetextWide,
                _ => edit::HeadingStyle::Atx,
            },
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn set_inline_options(
        self: Pin<&mut Self>,
        math: bool,
        highlight: bool,
        superscript: bool,
        subscript: bool,
        latex: bool,
        auto_links: bool,
        alerts: bool,
        strict: bool,
        remap: bool,
    ) {
        md::edit::set_remap(remap);
        let opts = Options {
            math,
            highlight,
            superscript,
            subscript,
            latex_delimiters: latex,
            bare_urls: auto_links,
            alerts,
            loose_links: !strict,
            smart: self.opts.smart.clone(),
            remap,
            ..Options::default()
        };
        self.apply_options(opts);
    }

    /// New syntax options: everything shown again by them.
    fn apply_options(mut self: Pin<&mut Self>, opts: Options) {
        if opts == self.opts {
            return;
        }
        super::shared::set_options(*self.doc_id(), opts.clone());
        let reparse = opts.latex_delimiters != self.opts.latex_delimiters || opts.remap != self.opts.remap;
        self.as_mut().rust_mut().opts = opts;
        if reparse {
            self.as_mut().reparse();
        }
        let rev = *self.inline_revision() + 1;
        self.as_mut().set_inline_revision(rev);
        self.refresh_stats();
    }

    /// An export of the document as `format` to `path`, with `settings`
    /// as `exportTo` takes them.
    fn export_job(&self, format: &str, path: PathBuf, settings: &QString) -> export::Job {
        let settings: serde_json::Value = serde_json::from_str(&settings.to_string()).unwrap_or_default();
        let flag = |key: &str| settings[key].as_bool().unwrap_or(true);
        export::Job {
            doc: self.doc.clone(),
            refs: self.refs.clone(),
            opts: self.opts.clone(),
            file: self.file.clone(),
            image_root: self.image_root_path(),
            look: export::Look {
                scheme: super::shared::scheme(),
                numbering: eqnum::Numbering::from_setting(settings["numbering"].as_str().unwrap_or("")),
                math_fences: flag("mathFences"),
                diagrams: flag("diagrams"),
                line_breaks: settings["lineBreaks"].as_bool().unwrap_or(false),
                math_tex: settings["mathTex"].as_bool().unwrap_or(false),
                indent_first_line: settings["indentFirstLine"].as_bool().unwrap_or(false),
            },
            mono_font: settings["monoFont"].as_str().unwrap_or("monospace").to_string(),
            format: format.to_string(),
            path,
            options: settings["options"].clone(),
            opened: settings["opened"].as_str().unwrap_or("").to_string(),
            collapsible_outline: settings["collapsibleOutline"].as_bool().unwrap_or(false),
            scale: settings["scale"].as_f64().filter(|s| *s > 0.0).unwrap_or(1.0),
            theme_font: settings["themeFont"].as_f64().filter(|s| *s > 0.0).unwrap_or(16.0),
            page: None,
        }
    }

    fn export_to(self: Pin<&mut Self>, format: &QString, path: &QString, settings: &QString) {
        let format = format.to_string();
        let path = export::with_extension(Path::new(&path.to_string()), &format);
        let job = self.export_job(&format, path.clone(), settings);
        let settings: serde_json::Value = serde_json::from_str(&settings.to_string()).unwrap_or_default();
        // Export and Overwrite skips the check that names pandoc's path.
        if export::needs_pandoc(&format) {
            export::pandoc::set_program(settings["pandocPath"].as_str().unwrap_or(""));
        }
        // After Export ▸ Run command runs in the document's folder, or
        // the documents folder.
        let command = job.command();
        let dir = job.file.as_deref().and_then(Path::parent).map(Path::to_path_buf).unwrap_or_else(|| {
            PathBuf::from(settings["documents"].as_str().filter(|d| !d.is_empty()).unwrap_or("/"))
        });
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let written = job.run().and_then(|out| {
                let bytes = match out {
                    export::Output::Written => return Ok(()),
                    export::Output::Bytes(b) => b,
                    export::Output::Pixels { width, height, rgba } => {
                        let jpeg = path.extension().is_some_and(|e| matches!(e.to_string_lossy().to_lowercase().as_str(), "jpg" | "jpeg"));
                        let b = super::glue::ffi::encode_image(width, height, &rgba, &QString::from(if jpeg { "jpg" } else { "png" }));
                        if b.is_empty() {
                            return Err("The image could not be encoded".into());
                        }
                        b
                    }
                };
                file::write_atomic(&path, &bytes).map_err(|e| format!("Could not write {}: {e}", path.display()))
            });
            let ran = match (&written, command) {
                (Ok(()), Some(command)) => {
                    let ran = export::run_command(&command, &dir);
                    serde_json::json!({"ran": true, "ok": ran.ok, "output": ran.output, "error": ran.error})
                }
                _ => serde_json::json!({"ran": false}),
            };
            let error = written.err().unwrap_or_default();
            let path = path.to_string_lossy().into_owned();
            let ran = ran.to_string();
            let _ = thread.queue(move |obj| {
                obj.exported(QString::from(&format), QString::from(&path), QString::from(&error), QString::from(&ran))
            });
        });
    }

    fn import_file(self: Pin<&mut Self>, path: &QString) {
        let path = PathBuf::from(path.to_string());
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let converted = export::pandoc::import(&path);
            let _ = thread.queue(move |mut obj| match converted {
                Ok(text) => {
                    obj.as_mut().new_document();
                    obj.as_mut().reset(md::parse(&text), None);
                    // It is a new document, not yet saved anywhere.
                    obj.as_mut().rust_mut().dirty = true;
                    obj.as_mut().sync_history();
                    obj.imported(QString::default());
                }
                Err(e) => obj.imported(QString::from(&e)),
            });
        });
    }

    fn print(self: Pin<&mut Self>, settings: &QString) {
        let dir = std::env::var_os("XDG_RUNTIME_DIR").map_or_else(std::env::temp_dir, PathBuf::from);
        let path = dir.join(format!("typo-print-{}.pdf", std::process::id()));
        let mut job = self.export_job("pdf", path.clone(), settings);
        let thread = self.qt_thread();
        std::thread::spawn(move || {
            let title = job.title();
            // The PDF options' page, A4 with 16mm margins unless they say
            // otherwise, then the dialog's.
            let pdf = job.pdf_settings();
            let printed = platform::print(&title, &path, (pdf.page, pdf.margins), |page| {
                job.page = Some(page);
                let export::Output::Bytes(bytes) = job.run()? else { return Err("Nothing to print".to_string()) };
                Ok(bytes)
            });
            let error = printed.err().unwrap_or_default();
            let _ = thread.queue(move |obj| obj.printed(QString::from(&error)));
        });
    }

    /// Read the document again for a change in what its block syntax is,
    /// as one undo step that leaves it as saved as it was, when that changes
    /// its blocks: `\[` lines turning to math or back.
    fn reparse(mut self: Pin<&mut Self>) {
        let new = md::parse_with(&md::serialize(&self.doc), self.opts.latex_delimiters, self.opts.remap);
        let same = new.blocks.len() == self.doc.blocks.len()
            && new.blocks.iter().zip(&self.doc.blocks).all(|(a, b)| a.kind == b.kind && a.text == b.text && a.containers.len() == b.containers.len());
        if same {
            return;
        }
        let clean = !self.dirty;
        let old = self.doc.blocks.clone();
        let sel = Sel::caret(Pos::new(0, 0));
        self.as_mut().rust_mut().doc.blocks = new.blocks;
        self.as_mut().commit(old, sel, sel, Merge::Never);
        if clean {
            let mut r = self.as_mut().rust_mut();
            r.history.mark_saved();
            r.dirty = false;
        }
        self.as_mut().sync_history();
        self.request_sel(sel);
    }

    fn refresh_stats(mut self: Pin<&mut Self>) {
        let source = md::serialize(&self.doc);
        let stats = query::stats(&self.doc, &source, &self.refs, &self.opts);
        let outline = query::outline(&self.doc, &self.refs, &self.opts);
        let json = format!(
            "[{}]",
            outline
                .iter()
                .map(|h| {
                    let html = md::html::inline_html(&self.doc.blocks[h.block].text, &self.refs, &self.opts);
                    format!(r#"{{"row":{},"level":{},"title":{},"html":{}}}"#, h.block, h.level, json_str(&h.title), json_str(&html))
                })
                .collect::<Vec<_>>()
                .join(",")
        );
        self.as_mut().set_words(stats.words as i32);
        self.as_mut().set_characters(stats.characters as i32);
        self.as_mut().set_lines(stats.lines as i32);
        if self.outline.to_string() != json {
            self.as_mut().set_outline(QString::from(&json));
        }
        let notes = query::footnotes(&self.doc, &self.refs, &self.opts);
        let json = format!(
            "{{{}}}",
            notes
                .iter()
                .map(|f| {
                    let refs: Vec<String> = f
                        .refs
                        .iter()
                        .map(|n| {
                            let t = n.text(&self.doc);
                            let cell = n.cell.map_or(String::new(), |(r, c)| format!(",{r},{c}"));
                            format!("[{},{},{}{cell}]", n.block, utf16_of(t, n.range.start), utf16_of(t, n.range.end))
                        })
                        .collect();
                    let def = f.def.map_or("null".to_string(), |(a, z)| format!("[{a},{z}]"));
                    format!(
                        r#"{}:{{"label":{},"refs":[{}],"def":{def}}}"#,
                        json_str(&query::footnote_key(&f.label)),
                        json_str(&f.label),
                        refs.join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join(",")
        );
        if self.footnotes.to_string() != json {
            self.as_mut().set_footnotes(QString::from(&json));
        }
    }

    fn emoji_suggestions(&self, token: &QString) -> QString {
        let list: Vec<String> = query::emoji_suggestions(&token.to_string())
            .into_iter()
            .map(|(name, emoji)| format!("[{},{}]", json_str(name), json_str(emoji)))
            .collect();
        QString::from(&format!("[{}]", list.join(",")))
    }

    fn footnote_html(&self, label: &QString) -> QString {
        let Some(sub) = query::footnote_doc(&self.doc, &self.refs, &self.opts, &label.to_string()) else {
            return QString::default();
        };
        QString::from(&md::html::render(&sub, &self.opts))
    }
}

/// The edit behind `Editor::command`; the selection afterwards, or `None`
/// when the command doesn't apply at this selection.
#[allow(clippy::too_many_arguments)]
/// The inverse of `pos_of`.
fn utf16_of_pos(b: &md::Block, offset: usize) -> i32 {
    match b.kind {
        BlockKind::Table { .. } => offset as i32,
        _ if edit::is_atomic(&b.kind) => offset.min(1) as i32,
        _ => utf16_of(&b.text, offset),
    }
}

/// The model position of UTF-16 position `u` in `row`. A block with no text
/// (a table, a rule) has two positions: 0 before it and 1 after.
/// The text a find match is in: block `block`'s, or its table cell's,
/// with the cell (-1, -1 outside tables).
fn match_text(doc: &Document, block: usize, cell: Option<(usize, usize)>) -> Option<(&str, (i32, i32))> {
    let b = doc.blocks.get(block)?;
    match (&b.kind, cell) {
        (BlockKind::Table { rows, .. }, Some((r, c))) => Some((rows.get(r)?.get(c)?.as_str(), (r as i32, c as i32))),
        (BlockKind::Table { .. }, None) | (_, Some(_)) => None,
        _ => Some((b.text.as_str(), (-1, -1))),
    }
}

fn pos_of(doc: &Document, row: i32, u: i32) -> Option<Pos> {
    let i = usize::try_from(row).ok().filter(|&i| i < doc.blocks.len())?;
    let b = &doc.blocks[i];
    Some(if edit::is_atomic(&b.kind) { Pos::new(i, u.clamp(0, 1) as usize) } else { Pos::new(i, byte_of(&b.text, u)) })
}

fn apply_command(
    doc: &mut Document,
    name: &str,
    anchor: Pos,
    caret: Pos,
    arg: i32,
    text: &str,
    refs: &Refs,
    opts: &Options,
) -> Option<Sel> {
    let (start, end) = if anchor <= caret { (anchor, caret) } else { (caret, anchor) };
    let i = caret.block;
    let collapsed = start == end;
    let one = |p: Pos| Some(Sel::caret(p));
    let style = match name {
        "strong" => Some(Style::Strong),
        "emphasis" => Some(Style::Emphasis),
        "underline" => Some(Style::Underline),
        "code" => Some(Style::Code),
        "strike" => Some(Style::Strike),
        "highlight" => Some(Style::Highlight),
        "superscript" => Some(Style::Superscript),
        "subscript" => Some(Style::Subscript),
        "inlineMath" => Some(Style::Math),
        "comment" => Some(Style::Comment),
        _ => None,
    };
    let text_command = matches!(name, "link" | "image" | "zoomImage" | "imageSyntax" | "footnote");
    if style.is_some() || text_command || name == "clearFormat" {
        // Across blocks, each block's part is formatted; links and images
        // stay within one block.
        if start.block != end.block {
            let (a, z) = match style {
                Some(s) => edit::style_range(doc, start, end, s, refs),
                None if name == "clearFormat" => edit::clear_range(doc, start, end, refs, opts),
                None => return None,
            };
            let (anchor, caret) = if anchor <= caret { (a, z) } else { (z, a) };
            return Some(Sel { anchor, caret });
        }
        if !doc.blocks[i].kind.is_inline_text() {
            return None;
        }
        let src = doc.blocks[i].text.clone();
        let sel = start.offset..end.offset;
        let (new, sel) = match style {
            Some(s) => edit::toggle_style(&src, sel, s, refs),
            // For a link or image `text` is the clipboard, for the address.
            None if text_command => edit::text_command(&src, sel, name, text, refs)?,
            None => edit::clear_format(&src, sel, refs, opts),
        };
        doc.blocks[i].set_text(new);
        return Some(Sel { anchor: Pos::new(i, sel.start), caret: Pos::new(i, sel.end) });
    }
    match name {
        "enter" => {
            let at = if collapsed { caret } else { edit::delete_range(doc, start, end) };
            let b = &doc.blocks[at.block];
            // A link definition is one line: Enter starts a paragraph.
            if b.kind == BlockKind::LinkRefDef {
                return one(edit::insert_paragraph(doc, at.block, true));
            }
            if !b.kind.is_inline_text() {
                return (!collapsed).then(|| Sel::caret(at));
            }
            if at.offset == b.text.len() {
                if let Some(p) = edit::enter_rule(doc, at.block, opts.latex_delimiters) {
                    return one(p);
                }
            }
            one(edit::split(doc, at))
        }
        // Backspace and Delete inside a line are the text field's own.
        "backspace" | "delete" if !collapsed => {
            if start.block == end.block {
                return None;
            }
            one(edit::delete_range(doc, start, end))
        }
        "backspace" if caret.offset == 0 => one(edit::backspace(doc, i)),
        "delete" if caret.offset == doc.blocks[i].text.len() => one(edit::delete_forward(doc, i)),
        "deleteSelection" => one(edit::delete_range(doc, start, end)),
        "deleteBlock" => one(edit::delete_block(doc, i)),
        // Typing over a selection that spans blocks.
        "typeText" if start.block != end.block => one(edit::replace_range(doc, start, end, text)),
        "paste" => {
            let at = if collapsed { caret } else { edit::delete_range(doc, start, end) };
            one(edit::insert_markdown(doc, at, text, opts.latex_delimiters))
        }
        "indent" if start.block != end.block => edit::indent_range(doc, start.block, end.block).then_some(Sel { anchor, caret }),
        "outdent" if start.block != end.block => edit::outdent_range(doc, start.block, end.block).then_some(Sel { anchor, caret }),
        "indent" => {
            let item = doc.blocks[i].containers.iter().any(|c| c.list_item().is_some());
            (item && edit::indent(doc, i)).then_some(Sel { anchor, caret })
        }
        "outdent" => edit::outdent(doc, i).map(|_| Sel { anchor, caret }),
        "heading" | "paragraph" | "headingUp" | "headingDown" => {
            let current = edit::heading_level(&doc.blocks[i]);
            let level = match name {
                "paragraph" => 0,
                // Choosing the block's own level again makes it a paragraph.
                "heading" if current == arg as u8 => 0,
                "heading" => arg.clamp(1, 6) as u8,
                // Paragraph → H6 → … → H1.
                "headingUp" => match current {
                    0 => 6,
                    1 => return None,
                    l => l - 1,
                },
                _ => match current {
                    0 => return None,
                    6 => 0,
                    l => l + 1,
                },
            };
            for j in start.block..=end.block {
                if doc.blocks[j].kind.is_inline_text() {
                    edit::set_heading(doc, j, level);
                }
            }
            Some(Sel { anchor, caret })
        }
        "quote" => {
            edit::toggle_quote(doc, start.block, end.block);
            Some(Sel { anchor, caret })
        }
        "orderedList" | "bulletList" | "taskList" => {
            let kind = match name {
                "orderedList" => ListKind::Ordered,
                "bulletList" => ListKind::Bullet,
                _ => ListKind::Task,
            };
            edit::toggle_list(doc, start.block, end.block, kind);
            Some(Sel { anchor, caret })
        }
        "taskToggle" | "taskDone" | "taskTodo" => {
            let done = match name {
                "taskDone" => Some(true),
                "taskTodo" => Some(false),
                _ => None,
            };
            edit::set_task(doc, i, done);
            Some(Sel { anchor, caret })
        }
        "codeFence" | "mathBlock" | "hr" | "toc" | "frontMatter" | "table" => {
            let kind = match name {
                "codeFence" => BlockKind::CodeFence { info: String::new(), fence: "```".into(), closed: true },
                "mathBlock" => BlockKind::Math { brackets: false, before: String::new(), after: String::new() },
                "hr" => BlockKind::ThematicBreak { marker: "---".into() },
                "toc" => BlockKind::Toc,
                "frontMatter" => BlockKind::FrontMatter,
                // arg = columns * 100 + rows, header included.
                _ => {
                    let (c, r) = if arg > 0 { (arg / 100, arg % 100) } else { (3, 3) };
                    edit::new_table(c.max(1) as usize, r.max(2) as usize)
                }
            };
            if name == "frontMatter" {
                // Front matter only lives at the top of the document.
                if matches!(doc.blocks[0].kind, BlockKind::FrontMatter) {
                    return one(Pos::new(0, 0));
                }
                doc.blocks.insert(0, md::Block::new(BlockKind::FrontMatter, "", Vec::new()));
                return one(Pos::new(0, 0));
            }
            one(edit::insert_block(doc, i, kind))
        }
        "addFootnote" => one(edit::add_footnote_def(doc, text)),
        "paragraphBefore" => one(edit::insert_paragraph(doc, i, false)),
        "paragraphAfter" => one(edit::insert_paragraph(doc, i, true)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaces_lists_as_typora() {
        // Measured in Typora: tight and loose lists alike, 8px between
        // items and paragraphs in an item, .8em around a list.
        let doc = md::parse("Para\n\n- a\n- b\n\n  more\n- c\n\nPara\n\n1. x\n   - y\n   - z\n2. w\n\n- other\n");
        let gaps: Vec<f64> = derive(&doc).iter().map(|d| d.gap).collect();
        assert_eq!(gaps, [12.8, 12.8, 8.0, 8.0, 8.0, 12.8, 12.8, 8.0, 8.0, 8.0, 12.8]);
        let first = |src: &str| derive(&md::parse(src))[0].gap;
        assert_eq!((first("# T\n"), first("- a\n"), first("---\nt: 1\n---\n")), (16.0, 30.0, 0.0));
    }
}
