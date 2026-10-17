import QtQuick
import QtQuick.Controls
import Typo

// The document: one delegate per block in a centred column, as wide as
// Typora's writing area (860px, 1024px on wide windows, less 30px padding
// each side), with Typora's generous padding above and below.
//
// A selection inside one block belongs to that block's text field. One
// across blocks is `span`: every field in it shows its part (fields keep
// their selection when unfocused), and the field at the caret's end has
// the focus.
Flickable {
    id: view

    required property var editor
    // The row whose block has the caret, or -1.
    property int currentRow: -1
    // A selection across blocks, { aRow, aPos, zRow, zPos } in UTF-16 from
    // the anchor to the caret; a block without text has positions 0
    // (before it) and 1 (after it). Null for a selection inside one block.
    property var span: null
    // The caret moved in a block, so the selection may have changed.
    signal caretMoved()
    // A picture finished loading in `edit`, the block with the caret.
    signal imageLoaded(Item edit)
    // Find's matches for the highlighters: by row, or "row:r:c" for a
    // table cell, each "[[start, end], …]" in UTF-16.
    property var findHits: ({})
    // Find's current match, { row, r, c, start, end } (r and c -1 outside
    // tables): selected in its block, which shows as if it had the caret,
    // while the find bar keeps the focus. Null when there is none.
    property var found: null
    // The block focus mode leaves alone: find's match while there is one,
    // else the caret's, as Typora moves `md-focus`.
    readonly property int focusedRow: found !== null ? found.row : currentRow

    // Right-click, the Menu key or Shift+F10: open the context menu at
    // window point (x, y).
    signal menuRequested(real x, real y)
    // Files to insert at the selection (pasted), or dropped on the page
    // with the caret already where they fell: absolute paths.
    signal filesPasted(var paths)
    signal filesDropped(var paths)

    // Something dragged in from another app: HTML or text is pasted where
    // it falls, and files go to the window (filesDropped), as Typora takes
    // them.
    DropArea {
        x: pointer.x
        y: pointer.y
        width: pointer.width
        height: pointer.height
        onDropped: drop => {
            const files = drop.hasUrls ? drop.urls.map(u => u.toString()).filter(u => u.startsWith("file://"))
                .map(u => decodeURIComponent(u.substring(7))) : []
            if (!drop.hasHtml && !drop.hasText && files.length === 0)
                return
            drop.accept(Qt.CopyAction)
            const p = view.hit(drop.x, drop.y)
            if (p && !p.control && !p.task)
                view.goTo(p)
            if (drop.hasHtml && drop.html !== "")
                view.command("paste", 0, view.editor.htmlToMarkdown(drop.html))
            else if (files.length > 0)
                view.filesDropped(files)
            else if (drop.text !== "")
                view.command("paste", 0, drop.text)
        }
    }

    // Equation numbers, by block (see Editor.mathTags), and what labels
    // refer to (Ui.mathLabels), kept in step with the document.
    property var mathRows: ({})
    property string mathJson: ""
    function updateMath() {
        const json = editor.mathTags(Prefs.mathNumbering, Prefs.mathFences)
        if (json === mathJson)
            return
        mathJson = json
        const m = JSON.parse(json)
        mathRows = m.rows
        Ui.mathLabels = m.labels
    }
    // Refresh All Math Expressions: typesetting follows the source, so only
    // the numbers can be stale.
    function refreshMath() {
        mathJson = ""
        updateMath()
    }
    readonly property string mathNumbering: Prefs.mathNumbering + Prefs.mathFences
    onMathNumberingChanged: updateMath()
    Component.onCompleted: updateMath()

    // Typora's media queries measure the window, sidebar included.
    readonly property real maxWidth: (Window.width >= 1800 ? 1200 : Window.width >= 1400 ? 1024 : 860) * Ui.zoom
    readonly property real padding: 30 * Ui.zoom

    // Typewriter mode (F9): room above the first line and below the last
    // for either to sit in the middle of the view, where typing keeps the
    // caret's line (Typora's `typeWriterScroll`).
    readonly property real typewriterRoom: Ui.typewriter ? Math.max(0, (height - Ui.fontSize * 1.6) / 2) : 0

    contentWidth: width
    contentHeight: column.y + column.height + (Ui.typewriter ? typewriterRoom : 100 * Ui.zoom)
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    flickableDirection: Flickable.VerticalFlick
    ScrollBar.vertical: ScrollBar {
        policy: ScrollBar.AsNeeded
    }

    // Keys while a selection ends on a block without text. With no caret
    // at all, as when the window has just opened, typing goes to the end
    // of the document, as Typora's does.
    Keys.onPressed: event => {
        if (!span && currentRow < 0 && event.text !== "" && typeAtEnd(event))
            return
        view.keyPressed(view.span ? view.span.zRow : view.currentRow, null, event)
    }
    function typeAtEnd(event) {
        const row = blocks.count - 1
        const edit = editAt(row)
        if (!edit || !focusRow(row, edit.length, edit.length))
            return false
        keyPressed(row, edit, event)
        if (!event.accepted && event.text.charCodeAt(0) >= 32 && !(event.modifiers & Ui.notTyping)) {
            edit.insert(edit.cursorPosition, event.text)
            event.accepted = true
        }
        return true
    }

    Column {
        id: column
        width: Math.min(view.width, view.maxWidth) - 2 * view.padding
        x: (view.width - width) / 2
        // Typora's 30px of padding; the first block adds its top margin.
        y: Ui.typewriter ? view.typewriterRoom : 30 * Ui.zoom

        // Typora's table trackers follow the pointer from cell to cell.
        // Here, under the blocks, the blocks' own hovering comes first;
        // the margin takes in the row handles left of a table.
        HoverHandler {
            margin: 20 * Ui.zoom
            onPointChanged: {
                if (hovered && !pointer.pressed) {
                    const p = column.mapToItem(pointer, point.position.x, point.position.y)
                    view.hover(p.x, p.y)
                }
            }
            onHoveredChanged: if (!hovered && !pointer.moving) pointer.handle = ""
        }

        Repeater {
            id: blocks
            model: view.editor
            delegate: BlockDelegate {
                editor: view.editor
                host: view
                onFocused: row => view.rowFocused(row)
            }
        }
    }

    // Clicks and drags anywhere on the page place the caret and select,
    // across blocks too: double-click selects a word, triple-click the
    // block, Shift+click extends, Ctrl+click follows a link.
    MouseArea {
        id: pointer
        width: view.contentWidth
        height: Math.max(view.contentHeight, view.height)
        // Over a table's row or column handle, the pointer says which way
        // it drags, as Typora's trackers do.
        cursorShape: handle === "row" ? Qt.SizeVerCursor : handle === "column" ? Qt.SizeHorCursor : Qt.IBeamCursor
        preventStealing: true
        acceptedButtons: Qt.LeftButton | Qt.RightButton

        property var anchor: null
        // The table whose row or column is being dragged, and the handle
        // under the pointer: "row", "column" or "".
        property Item moving: null
        property string handle: ""
        property int clicks: 0
        property real lastTime: 0
        property point lastPoint
        // Where the pointer is in the viewport, for scrolling while dragging.
        property real viewX
        property real viewY

        onPressed: mouse => view.press(mouse)
        onPositionChanged: mouse => {
            if (!(pressedButtons & Qt.LeftButton))
                return
            if (moving) {
                const t = mapToItem(moving, mouse.x, mouse.y)
                moving.dragTo(t.x, t.y)
                return
            }
            viewX = mouse.x
            viewY = mouse.y - view.contentY
            view.drag()
        }
        onCanceled: {
            if (moving)
                moving.cancelDrag()
            moving = null
        }
        onReleased: {
            if (moving) {
                moving.drop()
                moving = null
                view.hover(mouseX, mouseY)
            }
            scroller.stop()
            if (Ui.typewriter && Prefs.caretAlwaysCentred)
                view.centreCaret()
        }
    }

    FootnoteTip {
        id: footnoteTip
        view: view
    }
    EmojiSuggest {
        id: emojiSuggest
        view: view
    }
    // Main's ImagePopup, which takes some keys while shown.
    property Item popup: null
    // For scripts.
    readonly property alias pointer: pointer
    readonly property alias footnoteTip: footnoteTip
    readonly property alias emojiSuggest: emojiSuggest

    // Scrolls while a drag is above or below the view.
    Timer {
        id: scroller
        interval: 30
        repeat: true
        property real step: 0
        onTriggered: {
            const before = view.contentY
            view.contentY = Math.max(0, Math.min(view.contentHeight - view.height, view.contentY + step))
            if (view.contentY !== before)
                view.drag()
        }
    }

    function itemAt(row) {
        return blocks.itemAt(row)
    }

    // The writing area, in content coordinates.
    function pageRect() {
        return Qt.rect(column.x, column.y, column.width, column.height)
    }

    // The document's footnotes (Editor.footnotes), and the references to
    // them in each row, or in each table cell under "row:r:c":
    // `{where: [{key, start, end}]}`.
    readonly property var footnotes: JSON.parse(editor.footnotes)
    readonly property var footnoteRefs: {
        const rows = {}
        for (const key in footnotes)
            for (const r of footnotes[key].refs) {
                const where = r.length > 3 ? cellKey(r[0], r[3], r[4]) : r[0]
                if (!rows[where])
                    rows[where] = []
                rows[where].push({ key: key, start: r[1], end: r[2] })
            }
        return rows
    }
    function cellKey(row, r, c) {
        return row + ":" + r + ":" + c
    }
    // The text field `where` names: a row's, or a table cell's.
    function noteEdit(where) {
        const p = String(where).split(":").map(Number)
        if (p.length === 1)
            return editAt(p[0])
        const item = blocks.itemAt(p[0])
        const cell = item && item.table ? item.table.cellItem(p[1], p[2]) : null
        return cell ? cell.edit : null
    }
    // The reference at UTF-16 position `pos` in `row`, or null.
    function footnoteRefAt(row, pos) {
        for (const r of footnoteRefs[row] || [])
            if (r.start < pos && pos < r.end)
                return r
        return null
    }
    // Position `pos` in `edit`, block `row`, hit at `x`, moved into the
    // footnote reference there: one not being edited is drawn as a box over
    // its first character, the rest shrunk to nothing, so the pointer
    // anywhere on the box is in it.
    function footnoteSnap(row, edit, x, pos) {
        for (const r of footnoteRefs[row] || []) {
            if (pos < r.start || pos > r.end || r.end - r.start < 2)
                continue
            const a = edit.positionToRectangle(r.start)
            const b = edit.positionToRectangle(r.start + 1)
            const z = edit.positionToRectangle(r.end)
            if (a.y === z.y && z.x - b.x < 1 && a.x <= x && x < z.x)
                return r.start + 1
        }
        return pos
    }
    // The pointer is at `pos` in `row` (or a "row:r:c" cell; -1: it left):
    // on a reference, its footnote shows, unless the caret is in it.
    function footnoteHover(row, pos) {
        const r = pos < 0 ? null : footnoteRefAt(row, pos)
        const edit = noteEdit(row)
        if (!r || edit && edit.activeFocus && edit.cursorPosition >= r.start && edit.cursorPosition <= r.end) {
            footnoteTip.leave()
            return
        }
        footnoteTip.show(r.key, row, r.start, r.end)
    }
    // Ctrl+click on a reference: the caret goes to the end of the
    // footnote's definition; with none, the tooltip says how to write it.
    function jumpToFootnoteDef(r, row) {
        const f = footnotes[r.key]
        if (f && f.def) {
            footnoteTip.close()
            const last = f.def[1]
            const edit = editAt(last)
            if (edit)
                focusRow(last, edit.length, edit.length)
            else
                jumpToRow(f.def[0])
            return
        }
        footnoteTip.show(r.key, row, r.start, r.end)
    }
    // A footnote's ↩ link: the caret goes to reference `i` to it.
    function jumpToFootnoteRef(key, i) {
        const r = footnotes[key] ? footnotes[key].refs[i] : null
        const item = r ? blocks.itemAt(r[0]) : null
        if (r && r.length > 3 && item && item.table)
            item.table.focusCell(r[3], r[4], r[1], r[1])
        else if (r)
            focusRow(r[0], r[1], r[1])
    }

    // The text field of `row`, or null for a block without text. A table
    // has none here: its positions are the block's, 0 and 1.
    function editAt(row) {
        const item = blocks.itemAt(row)
        return item && !item.table ? item.editItem : null
    }

    // The table with the caret, when there is no selection across blocks.
    function tableAt() {
        if (span)
            return null
        const item = blocks.itemAt(currentRow)
        return item ? item.table : null
    }

    // The Table menu's commands. False outside a table.
    function tableCommand(cmd, arg) {
        const t = tableAt()
        return t ? t.menuCommand(cmd, arg) : false
    }

    // Put the caret in `row` at `pos` (UTF-16), selecting from `anchor`.
    // In a table they are the editor's positions (TableBlock.place).
    function focusRow(row, anchor, pos) {
        const item = blocks.itemAt(row)
        if (!item || !item.editItem)
            return false
        if (item.table)
            return item.table.place(anchor, pos)
        clearSpan()
        const edit = item.editItem
        edit.forceActiveFocus()
        // A field that kept the focus while rows went in above it (new
        // front matter) is at a new row now.
        rowFocused(row)
        const len = edit.length
        edit.select(Math.min(anchor, len), Math.min(pos, len))
        reveal(item, edit.cursorRectangle)
        return true
    }

    // A block's field took the focus. The block that had it lets go of its
    // selection, unless both are in the cross-block selection.
    function rowFocused(row) {
        if (row === currentRow)
            return
        if (span && !inSpan(row))
            clearSpan()
        if (!span) {
            const old = blocks.itemAt(currentRow)
            if (old && old.editItem)
                old.editItem.deselect()
        }
        currentRow = row
    }

    function inSpan(row) {
        return span !== null && row >= Math.min(span.aRow, span.zRow) && row <= Math.max(span.aRow, span.zRow)
    }

    // Whether (row, pos) falls inside a selection that isn't empty.
    function inSelection(row, pos) {
        const s = selection()
        if (collapsed(s))
            return false
        const before = (r1, p1, r2, p2) => r1 < r2 || (r1 === r2 && p1 <= p2)
        const forward = before(s.aRow, s.aPos, s.zRow, s.zPos)
        const start = forward ? [s.aRow, s.aPos] : [s.zRow, s.zPos]
        const end = forward ? [s.zRow, s.zPos] : [s.aRow, s.aPos]
        return before(start[0], start[1], row, pos) && before(row, pos, end[0], end[1])
    }

    // The selection: the span, or the current block's field.
    function selection() {
        if (span) {
            const edit = editAt(span.zRow)
            return { aRow: span.aRow, aPos: span.aPos, zRow: span.zRow, zPos: edit && edit.activeFocus ? edit.cursorPosition : span.zPos }
        }
        const row = Math.max(0, currentRow)
        const edit = editAt(row)
        return { aRow: row, aPos: edit ? anchorOf(edit) : 0, zRow: row, zPos: edit ? edit.cursorPosition : 0 }
    }

    function collapsed(s) {
        return s.aRow === s.zRow && s.aPos === s.zPos
    }

    // Select from (aRow, aPos) to the caret at (zRow, zPos).
    function select(aRow, aPos, zRow, zPos) {
        if (aRow === zRow) {
            if (focusRow(zRow, aPos, zPos))
                return
            // A block without text: the caret goes to the nearest text.
            const next = editableRow(zRow, 1)
            const prev = editableRow(zRow, -1)
            if (next >= 0)
                focusRow(next, 0, 0)
            else if (prev >= 0) {
                // The end of its text, or of a table (position 1).
                const e = editAt(prev)
                focusRow(prev, e ? e.length : 1, e ? e.length : 1)
            }
            return
        }
        clearSpan()
        span = { aRow: aRow, aPos: aPos, zRow: zRow, zPos: zPos }
        const forward = aRow < zRow
        const first = Math.min(aRow, zRow)
        const last = Math.max(aRow, zRow)
        const startPos = forward ? aPos : zPos
        const endPos = forward ? zPos : aPos
        let focused = false
        for (let r = first; r <= last; r++) {
            const item = blocks.itemAt(r)
            if (!item)
                continue
            const edit = item.table ? null : item.editItem
            if (!edit) {
                // The selection takes a block without text when it covers
                // it, or when it starts there.
                item.spanned = r !== last || endPos > 0
                continue
            }
            item.spanned = true
            const len = edit.length
            const a = r === first ? Math.min(startPos, len) : 0
            const z = r === last ? Math.min(endPos, len) : len
            if (r === zRow) {
                edit.forceActiveFocus()
                if (forward)
                    edit.select(a, z)
                else
                    edit.select(z, a)
                reveal(item, edit.cursorRectangle)
                focused = true
            } else {
                edit.select(a, z)
            }
        }
        if (!focused)
            view.forceActiveFocus()
    }

    // Select a block without text as a whole, as a click on Typora's TOC
    // does: Backspace or Delete then removes it.
    function selectBlock(row) {
        clearSpan()
        span = { aRow: row, aPos: 0, zRow: row, zPos: 1 }
        blocks.itemAt(row).spanned = true
        view.forceActiveFocus()
    }

    function clearSpan() {
        if (!span)
            return
        const first = Math.min(span.aRow, span.zRow)
        const last = Math.max(span.aRow, span.zRow)
        span = null
        for (let r = first; r <= last && r < blocks.count; r++) {
            const item = blocks.itemAt(r)
            if (!item)
                continue
            item.spanned = false
            if (item.editItem && !item.editItem.activeFocus)
                item.editItem.deselect()
        }
    }

    // The text field find's match `f` is in.
    function foundEdit(f) {
        const item = blocks.itemAt(f.row)
        if (!item)
            return null
        if (f.r < 0)
            return item.table ? null : item.editItem
        const cell = item.table ? item.table.cellItem(f.r, f.c) : null
        return cell ? cell.edit : null
    }

    // Make find's match `hit` ([row, r, c, start, end]) the current one:
    // selected and scrolled into view, the focus left where it is. Null
    // clears it.
    function showFound(hit) {
        const old = found
        found = hit ? { row: hit[0], r: hit[1], c: hit[2], start: hit[3], end: hit[4] } : null
        if (old && (!found || old.row !== found.row || old.r !== found.r || old.c !== found.c)) {
            const e = foundEdit(old)
            if (e && !e.activeFocus)
                e.deselect()
        }
        if (!found)
            return
        clearSpan()
        const edit = foundEdit(found)
        if (!edit)
            return
        edit.select(Math.min(found.start, edit.length), Math.min(found.end, edit.length))
        revealIn(edit, edit.positionToRectangle(found.start))
    }

    // The find bar closed: its current match becomes the selection, with
    // the focus. False when there was none.
    function takeFound() {
        const f = found
        if (!f)
            return false
        const item = blocks.itemAt(f.row)
        const ok = f.r >= 0 ? item !== null && item.table !== null && item.table.focusCell(f.r, f.c, f.start, f.end)
                            : focusRow(f.row, f.start, f.end)
        found = null
        return ok
    }

    // Scroll so `rect` (in `item`'s editItem coordinates) is visible.
    function reveal(item, rect) {
        revealIn(item.editItem, rect)
    }

    // Scroll so `rect` (in `edit`'s coordinates) is visible: in the middle
    // in typewriter mode, unless the pointer put it there.
    function revealIn(edit, rect) {
        if (Ui.typewriter && !pointer.pressed) {
            centreIn(edit, rect)
            return
        }
        column.forceLayout()
        const p = edit.mapToItem(column, rect.x, rect.y)
        const top = column.y + p.y
        const bottom = top + rect.height
        const margin = 40 * Ui.zoom
        if (top - margin < contentY)
            contentY = Math.max(0, top - margin)
        else if (bottom + margin > contentY + height)
            contentY = Math.min(Math.max(0, contentHeight - height), bottom + margin - height)
    }

    function centreIn(edit, rect) {
        column.forceLayout()
        const p = edit.mapToItem(column, rect.x, rect.y)
        contentY = Math.max(0, Math.min(Math.max(0, contentHeight - height), column.y + p.y + rect.height / 2 - height / 2))
    }

    // The text field with the caret, in a block, a cell or a fence.
    readonly property Item caretItem: {
        const f = Window.activeFocusItem
        if (!f || f.cursorRectangle === undefined)
            return null
        let p = f
        while (p && p !== view)
            p = p.parent
        return p ? f : null
    }
    function centreCaret() {
        if (caretItem)
            centreIn(caretItem, caretItem.cursorRectangle)
    }
    // Typing keeps the caret's line in the middle in typewriter mode, and
    // with "Always keep caret in middle of screen", every move does.
    property int typedLength: -1
    onCaretItemChanged: typedLength = caretItem ? caretItem.length : -1
    Connections {
        target: view.caretItem
        enabled: Ui.typewriter
        function onCursorRectangleChanged() {
            const typed = view.caretItem.length !== view.typedLength
            view.typedLength = view.caretItem.length
            if ((typed || Prefs.caretAlwaysCentred) && !pointer.pressed)
                Qt.callLater(view.centreCaret)
        }
    }
    Connections {
        target: Ui
        function onTypewriterChanged() {
            if (Ui.typewriter && view.visible)
                Qt.callLater(view.centreCaret)
        }
    }

    // Of the headings at `rows`, the one Typora's outline marks for what
    // the view shows (`highlightVisibleHeader`): the last once its top is
    // in sight or above, else the first in sight, else the last above the
    // view (the first when none is).
    function headingInView(rows) {
        if (rows.length === 0)
            return -1
        const where = row => {
            const y = rowY(row)
            return y < 0 ? -1 : y > height ? 1 : 0
        }
        if (where(rows[rows.length - 1]) !== 1)
            return rows[rows.length - 1]
        let above = rows[0]
        for (const row of rows) {
            const w = where(row)
            if (w === 0)
                return row
            if (w === 1)
                break
            above = row
        }
        return above
    }

    // How far down the view block `row` starts.
    function rowY(row) {
        const item = blocks.itemAt(row)
        if (!item)
            return 0
        column.forceLayout()
        return column.y + item.y + item.topSpace - contentY
    }
    // Scroll so block `row` starts `y` down the view, and put the caret at
    // its start (coming back from Source Code Mode).
    function showRow(row, y) {
        const item = blocks.itemAt(row)
        if (!item)
            return
        column.forceLayout()
        contentY = Math.max(0, Math.min(contentHeight - height, column.y + item.y + item.topSpace - y))
        select(row, 0, row, 0)
    }

    // Scroll `row` to the top and put the caret at its start, or its end.
    function jumpToRow(row, atEnd) {
        const item = blocks.itemAt(row)
        if (!item)
            return
        column.forceLayout()
        contentY = Math.max(0, Math.min(contentHeight - height, column.y + item.y + item.topSpace - 20 * Ui.zoom))
        if (item.editItem) {
            const at = atEnd ? item.editItem.length : 0
            focusRow(row, at, at)
        }
    }

    // The caret's selection in a text field: the end it started from and
    // where the caret is. TextEdit only tells start and end.
    function anchorOf(edit) {
        return edit.cursorPosition === edit.selectionStart ? edit.selectionEnd : edit.selectionStart
    }

    // Run an editing command on the selection. In a table cell, styles
    // apply to the cell and block formats don't.
    function command(name, arg, text) {
        const t = tableAt()
        if (t && t.format(name, text))
            return true
        const s = selection()
        return editor.command(name, s.aRow, s.aPos, s.zRow, s.zPos, arg || 0, text || "")
    }

    // The nearest row before or after `row` (step -1 or 1) with text to
    // put the caret in, or -1.
    function editableRow(row, step) {
        for (let r = row + step; r >= 0 && r < blocks.count; r += step) {
            const item = blocks.itemAt(r)
            if (item && item.editItem)
                return r
        }
        return -1
    }

    // The position in `edit` at (x, y). With a fixed line height a line's
    // glyphs sit below its top, and Qt counts a hit above them as the start
    // of the line, so aim at the glyphs.
    function textPos(edit, x, y) {
        const r = edit.positionToRectangle(edit.positionAt(x, y))
        return edit.positionAt(x, y < r.y ? r.y + r.height / 2 : y)
    }

    // The row of the block at y on the page.
    function blockAt(y) {
        const cy = y - column.y
        let lo = 0
        let hi = blocks.count - 1
        while (lo < hi) {
            const mid = (lo + hi + 1) >> 1
            if (blocks.itemAt(mid).y <= cy)
                lo = mid
            else
                hi = mid - 1
        }
        return lo
    }

    // A table's row or column handle at (x, y) on the page: the block's
    // own table, or the next block's, whose column handles reach above it.
    function handleAt(x, y) {
        if (blocks.count === 0 || y - column.y > column.height)
            return null
        const lo = blockAt(y)
        for (const row of [lo, lo + 1]) {
            const item = row < blocks.count ? blocks.itemAt(row) : null
            if (!item || !item.table)
                continue
            const t = pointer.mapToItem(item.table, x, y)
            const h = item.table.handleAt(t.x, t.y)
            if (h)
                return { row: row, table: item.table, kind: h.kind, index: h.index }
        }
        return null
    }

    // The pointer moving with no button held: the table under it notes
    // the cell it is over, which places its handles, and a footnote
    // reference in the cell shows its footnote.
    property bool cellNoteHovered: false
    function hover(x, y) {
        let where = ""
        let pos = -1
        if (blocks.count > 0 && y - column.y <= column.height) {
            const row = blockAt(y)
            const item = blocks.itemAt(row)
            if (item.table) {
                const t = pointer.mapToItem(item.table, x, y)
                item.table.hover(t.x, t.y)
                const cell = item.table.cellAt(t.x, t.y)
                where = cell ? cellKey(row, cell.r, cell.c) : ""
                if (footnoteRefs[where] !== undefined) {
                    const p = pointer.mapToItem(cell.edit, x, y)
                    pos = footnoteSnap(where, cell.edit, p.x, textPos(cell.edit, p.x, p.y))
                }
            }
        }
        if (pos >= 0 || cellNoteHovered)
            footnoteHover(where, pos)
        cellNoteHovered = pos >= 0
        const h = handleAt(x, y)
        pointer.handle = h ? h.kind : ""
    }

    // The row and position at (x, y) on the page (the pointer's coordinates).
    function hit(x, y) {
        if (blocks.count === 0)
            return null
        const cy = y - column.y
        if (cy > column.height) {
            const last = blocks.count - 1
            const edit = editAt(last)
            return { row: last, pos: edit ? edit.length : 1 }
        }
        const lo = blockAt(y)
        const item = blocks.itemAt(lo)
        const local = pointer.mapToItem(item, x, y)
        if (item.taskAt(local.x, local.y))
            return { row: lo, pos: 0, task: true }
        // A table's toolbar, or a selected TOC's header, sits above it,
        // over the block before.
        const next = lo + 1 < blocks.count ? blocks.itemAt(lo + 1) : null
        if (next && (next.table || next.kind === "toc")) {
            const n = pointer.mapToItem(next, x, y)
            const above = next.controlAt(n.x, n.y)
            if (above)
                return { row: lo + 1, pos: 0, control: above }
        }
        const control = item.controlAt(local.x, local.y)
        if (control)
            return { row: lo, pos: 0, control: control }
        if (item.table) {
            const t = pointer.mapToItem(item.table, x, y)
            const cell = item.table.cellAt(t.x, t.y)
            if (cell) {
                const p = pointer.mapToItem(cell.edit, x, y)
                const pos = textPos(cell.edit, p.x, Math.max(0, Math.min(cell.edit.height - 1, p.y)))
                return { row: lo, pos: footnoteSnap(cellKey(lo, cell.r, cell.c), cell.edit, p.x, pos), cell: cell }
            }
        }
        const edit = item.editItem
        if (!edit)
            return { row: lo, pos: local.y < item.topSpace + (item.height - item.topSpace) / 2 ? 0 : 1 }
        const p = pointer.mapToItem(edit, x, y)
        return { row: lo, pos: footnoteSnap(lo, edit, p.x, textPos(edit, p.x, Math.max(0, Math.min(edit.height - 1, p.y)))) }
    }

    function press(mouse) {
        // A table's row or column handle: drag it to move it.
        const h = mouse.button === Qt.LeftButton ? handleAt(mouse.x, mouse.y) : null
        if (h) {
            const t = pointer.mapToItem(h.table, mouse.x, mouse.y)
            pointer.anchor = null
            pointer.moving = h.table
            h.table.startDrag(h.kind, h.index, t.x, t.y)
            return
        }
        const p = hit(mouse.x, mouse.y)
        // A right-click outside the selection moves the caret there first.
        if (mouse.button === Qt.RightButton) {
            if (p && p.cell) {
                const e = p.cell.edit
                const inside = e.activeFocus && p.pos >= e.selectionStart && p.pos <= e.selectionEnd && e.selectedText !== ""
                if (!inside && !blocks.itemAt(p.row).table.inSpan(p.cell.r, p.cell.c, p.pos))
                    goTo(p)
            } else if (p && !p.task && !p.control && !inSelection(p.row, p.pos)) {
                select(p.row, p.pos, p.row, p.pos)
            }
            const w = pointer.mapToItem(null, mouse.x, mouse.y)
            menuRequested(w.x, w.y)
            return
        }
        if (!p)
            return
        if (p.task) {
            editor.toggleTask(p.row)
            return
        }
        if (p.control && p.control.activate) {
            p.control.activate()
            pointer.anchor = null
            return
        }
        if (p.control) {
            const at = pointer.mapToItem(p.control, mouse.x, mouse.y)
            p.control.forceActiveFocus()
            p.control.cursorPosition = p.control.positionAt(at.x, at.y)
            pointer.anchor = null
            return
        }
        const now = Date.now()
        const near = Math.abs(mouse.x - pointer.lastPoint.x) < 5 && Math.abs(mouse.y - pointer.lastPoint.y) < 5
        pointer.clicks = now - pointer.lastTime < 400 && near ? pointer.clicks % 3 + 1 : 1
        pointer.lastTime = now
        pointer.lastPoint = Qt.point(mouse.x, mouse.y)
        pointer.viewX = mouse.x
        pointer.viewY = mouse.y - contentY
        if (mouse.modifiers & Qt.ShiftModifier) {
            // In the table with the caret, Shift+click extends there,
            // across cells too.
            const t = p.cell ? blocks.itemAt(p.row).table : null
            if (t && t.focused) {
                const a = t.cellSpan ? t.cellSpan.a : { r: t.curR, c: t.curC, pos: anchorOf(t.editItem) }
                pointer.anchor = { row: p.row, pos: a.pos, cell: t.cellItem(a.r, a.c) }
                t.selectSpan(a, { r: p.cell.r, c: p.cell.c, pos: p.pos })
                return
            }
            const s = selection()
            pointer.anchor = { row: s.aRow, pos: s.aPos }
            select(s.aRow, s.aPos, p.row, p.cell ? (p.row > s.aRow ? 1 : 0) : p.pos)
            return
        }
        pointer.anchor = p
        if ((mouse.modifiers & Qt.ControlModifier) && followLink(p.row, p.pos, p.cell))
            return
        if (!p.cell && blocks.itemAt(p.row).kind === "toc") {
            selectBlock(p.row)
            pointer.anchor = null
            return
        }
        goTo(p)
        const edit = p.cell ? p.cell.edit : editAt(p.row)
        if (!edit)
            return
        if (pointer.clicks === 2)
            edit.selectWord()
        else if (pointer.clicks === 3)
            edit.select(0, edit.length)
    }

    // Extend the selection to the pointer, scrolling when it is outside.
    function drag() {
        if (!pointer.anchor || pointer.clicks > 1)
            return
        const vy = pointer.viewY
        scroller.step = vy < 0 ? vy / 2 : vy > height ? (vy - height) / 2 : 0
        if (scroller.step !== 0)
            scroller.start()
        else
            scroller.stop()
        const p = hit(pointer.viewX, Math.max(contentY, Math.min(contentY + height, contentY + vy)))
        if (!p || p.task || p.control)
            return
        const a = pointer.anchor
        // Inside a table a drag selects within its cell, or from cell to
        // cell as Typora's (the browser's) selection does. Out of it, or
        // into one, the selection takes the table whole.
        if (a.cell && p.row === a.row) {
            if (p.cell)
                blocks.itemAt(a.row).table.selectSpan({ r: a.cell.r, c: a.cell.c, pos: a.pos }, { r: p.cell.r, c: p.cell.c, pos: p.pos })
            return
        }
        const down = p.row > a.row
        const aPos = a.cell ? (down ? 0 : 1) : a.pos
        const zPos = p.cell ? (down ? 1 : 0) : p.pos
        select(a.row, aPos, p.row, zPos)
    }

    // Put the caret where `p` (from `hit`) is: in a cell, or in a block.
    function goTo(p) {
        if (p.cell)
            blocks.itemAt(p.row).table.focusCell(p.cell.r, p.cell.c, p.pos, p.pos)
        else
            select(p.row, p.pos, p.row, p.pos)
    }

    // Ctrl+click: open the link under the pointer. Links to a heading
    // (`#title`) jump there; anything else opens beside the document or in
    // the desktop's handler. A footnote reference, in `cell` too, goes to
    // its definition.
    function followLink(row, pos, cell) {
        const where = cell ? cellKey(row, cell.r, cell.c) : row
        const note = footnoteRefAt(where, pos)
        if (note) {
            jumpToFootnoteDef(note, where)
            return true
        }
        const dest = editor.linkAt(row, pos)
        if (dest === "")
            return false
        openLink(dest)
        return true
    }

    // Open link destination `dest`, as followLink does.
    function openLink(dest) {
        if (dest.startsWith("#")) {
            const target = editor.headingRow(dest.substring(1))
            if (target >= 0)
                jumpToRow(target)
        } else if (/^[a-z][a-z0-9+.-]*:/i.test(dest)) {
            Qt.openUrlExternally(dest)
        } else if (editor.path !== "") {
            Qt.openUrlExternally("file://" + editor.path.substring(0, editor.path.lastIndexOf("/") + 1) + dest)
        }
    }

    // Blocks edited as code.
    readonly property var codeKinds: ["code", "frontmatter", "math", "html"]

    function codeNewline(edit) {
        const at = edit.selectionStart
        edit.remove(edit.selectionStart, edit.selectionEnd)
        const t = edit.text
        const lineStart = t.lastIndexOf("\n", at - 1) + 1
        const indent = /^[ \t]*/.exec(t.substring(lineStart, at))[0]
        edit.insert(at, "\n" + indent)
        edit.cursorPosition = at + 1 + indent.length
    }

    // Indent (or outdent) every line the selection touches by the code
    // indent size (Preferences ▸ Markdown ▸ Code Fences, four spaces
    // unless set), keeping those lines selected.
    function codeIndent(edit, more) {
        const t = edit.text
        const s0 = edit.selectionStart
        const e0 = edit.selectionEnd
        const first = t.lastIndexOf("\n", s0 - 1) + 1
        // A selection that ends at a line start leaves that line alone.
        const last = e0 > s0 && t[e0 - 1] === "\n" ? e0 - 1 : e0
        const starts = [first]
        for (let i = t.indexOf("\n", first); i >= 0 && i < last; i = t.indexOf("\n", i + 1))
            starts.push(i + 1)
        let a = s0
        let z = e0
        // Last line first, so earlier offsets stay valid. Positions after a
        // line's start move with its indentation.
        for (let k = starts.length - 1; k >= 0; k--) {
            const at = starts[k]
            const size = Prefs.codeIndentSize
            let n = size
            if (more) {
                edit.insert(at, " ".repeat(size))
            } else {
                const lead = new RegExp("^( {1," + size + "}|\t)?").exec(edit.text.substring(at, at + size))[0]
                if (lead === "")
                    continue
                edit.remove(at, at + lead.length)
                n = -lead.length
            }
            if (a > at)
                a = Math.max(at, a + n)
            if (z > at)
                z = Math.max(at, z + n)
        }
        if (s0 === e0)
            edit.cursorPosition = a
        else
            edit.select(a, z)
    }

    // Shift+Tab with "Use Shift+Tab to auto indent selected code" on, or
    // Auto Indent Selected Code: the lines touched (every line, for the
    // whole code) take the indent of the line above, as CodeMirror's
    // indentAuto. The whole code's selection keeps its lines and columns,
    // as Typora puts it back.
    function codeAutoIndent(edit, whole) {
        const caret = edit.cursorPosition
        const anchor = caret === edit.selectionStart ? edit.selectionEnd : edit.selectionStart
        // Each end as [line, column].
        const ends = [anchor, caret].map(p => {
            const lines = edit.text.substring(0, p).split("\n")
            return [lines.length - 1, lines[lines.length - 1].length]
        })
        const r = JSON.parse(whole ? edit.highlighter.autoIndent(0, edit.length, Prefs.codeIndentSize)
                                   : edit.highlighter.autoIndent(anchor, caret, Prefs.codeIndentSize))
        if (edit.text.substring(r[0], r[1]) !== r[2]) {
            edit.remove(r[0], r[1])
            edit.insert(r[0], r[2])
        }
        if (whole) {
            const lines = edit.text.split("\n")
            const at = ([n, c]) => lines.slice(0, n).reduce((p, l) => p + l.length + 1, 0) + Math.min(c, lines[n].length)
            r[3] = at(ends[0])
            r[4] = at(ends[1])
        }
        if (r[3] === r[4])
            edit.cursorPosition = r[4]
        else
            edit.select(r[3], r[4])
    }

    // Keys with structural meaning, before the text field sees them.
    // Everything else (typing, moving within a line) is the field's own.
    function keyPressed(row, edit, event) {
        const mods = event.modifiers & (Qt.ShiftModifier | Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)
        const ctrl = mods === Qt.ControlModifier
        const shift = mods === Qt.ShiftModifier
        const ctrlShift = mods === (Qt.ControlModifier | Qt.ShiftModifier)
        const plain = mods === Qt.NoModifier
        const s = selection()
        const multi = span !== null
        const pos = edit ? edit.cursorPosition : s.zPos
        const len = edit ? edit.length : 1
        const run = (name, arg, text) => editor.command(name, s.aRow, s.aPos, s.zRow, s.zPos, arg || 0, text || "")
        if (emojiSuggest.shown && emojiSuggest.keyPressed(event) || popup && popup.shown && popup.keyPressed(event)) {
            event.accepted = true
            return
        }
        // The text field would take these for itself.
        if (event.matches(StandardKey.Copy)) {
            copy("")
            event.accepted = true
            return
        }
        if (event.matches(StandardKey.Cut)) {
            cut()
            event.accepted = true
            return
        }
        if (event.matches(StandardKey.Paste)) {
            paste(false)
            event.accepted = true
            return
        }
        if (event.matches(StandardKey.SelectAll)) {
            selectAll()
            event.accepted = true
            return
        }
        if (event.key === Qt.Key_Menu || (shift && event.key === Qt.Key_F10)) {
            const r = edit ? edit.cursorRectangle : null
            const w = edit ? edit.mapToItem(null, r.x, r.y + r.height) : blocks.itemAt(row).mapToItem(null, 0, 0)
            menuRequested(w.x, w.y)
            event.accepted = true
            return
        }
        // With the caret at the edge of its block, Shift+arrows carry the
        // selection into the next one.
        const atTop = !edit || edit.positionToRectangle(pos).y <= edit.positionToRectangle(0).y
        const atBottom = !edit || edit.positionToRectangle(pos).y >= edit.positionToRectangle(len).y
        const x = edit ? edit.mapToItem(null, edit.cursorRectangle.x, 0).x : 0
        const extend = (r, where) => {
            if (r < 0) {
                select(s.aRow, s.aPos, s.zRow, where === "first" || where === "start" ? len : 0)
                return
            }
            const e = editAt(r)
            let p = 0
            if (!e)
                p = where === "end" || where === "last" ? 1 : 0
            else if (where === "end")
                p = e.length
            else if (where === "first" || where === "last") {
                const rect = e.positionToRectangle(where === "first" ? 0 : e.length)
                p = e.positionAt(e.mapFromItem(null, x, 0).x, rect.y + rect.height / 2)
            }
            select(s.aRow, s.aPos, r, p)
        }
        if (shift) {
            const next = r => r + 1 < blocks.count ? r + 1 : -1
            const prev = r => r - 1
            switch (event.key) {
            case Qt.Key_Left:
                if (pos === 0 || !edit) {
                    extend(prev(s.zRow), "end")
                    event.accepted = true
                }
                return
            case Qt.Key_Right:
                if (pos === len || !edit) {
                    extend(next(s.zRow), "start")
                    event.accepted = true
                }
                return
            case Qt.Key_Up:
                if (atTop) {
                    extend(prev(s.zRow), "last")
                    event.accepted = true
                }
                return
            case Qt.Key_Down:
                if (atBottom) {
                    extend(next(s.zRow), "first")
                    event.accepted = true
                }
                return
            case Qt.Key_PageUp:
            case Qt.Key_PageDown:
                page(event.key === Qt.Key_PageDown ? 1 : -1, true)
                event.accepted = true
                return
            }
        }
        // Caret keys (Ui.caretKeys): the document's top and bottom are
        // the view's, the rest the field's.
        const move = Ui.caretMove(event)
        const extending = (event.modifiers & Qt.ShiftModifier) !== 0
        if (move === "top" || move === "bottom") {
            const last = blocks.count - 1
            if (!extending)
                move === "top" ? jumpTop() : jumpBottom()
            else if (move === "top")
                select(s.aRow, s.aPos, 0, 0)
            else
                select(s.aRow, s.aPos, last, editAt(last) ? editAt(last).length : 1)
            event.accepted = true
            return
        }
        if (multi) {
            const forward = s.aRow < s.zRow
            const start = forward ? [s.aRow, s.aPos] : [s.zRow, s.zPos]
            const end = forward ? [s.zRow, s.zPos] : [s.aRow, s.aPos]
            switch (event.key) {
            case Qt.Key_Return:
            case Qt.Key_Enter:
                event.accepted = run("enter")
                return
            case Qt.Key_Backspace:
            case Qt.Key_Delete:
                event.accepted = run("deleteSelection")
                return
            case Qt.Key_Escape:
                select(s.zRow, s.zPos, s.zRow, s.zPos)
                event.accepted = true
                return
            // Over list items Tab indents them all; elsewhere it types a
            // tab over the selection, as Typora does.
            case Qt.Key_Tab:
                if (plain && !run("indent"))
                    run("typeText", 0, "\t")
                event.accepted = true
                return
            case Qt.Key_Backtab:
                run("outdent")
                event.accepted = true
                return
            case Qt.Key_Left:
            case Qt.Key_Up:
                if (plain) {
                    select(start[0], start[1], start[0], start[1])
                    event.accepted = true
                    return
                }
                break
            case Qt.Key_Right:
            case Qt.Key_Down:
                if (plain) {
                    select(end[0], end[1], end[0], end[1])
                    event.accepted = true
                    return
                }
                break
            }
            // Typing replaces the selection.
            if (event.text !== "" && event.text.charCodeAt(0) >= 32 && !(mods & Ui.notTyping)) {
                event.accepted = run("typeText", 0, event.text)
                return
            }
        }
        if (edit && !multi && event.text.length === 1 && !(mods & Ui.notTyping)
                && (typeFenceLang(row, edit, event.text) || typeSmart(row, edit, event.text) || typePaired(row, edit, event.text))) {
            event.accepted = true
            return
        }
        // Code keeps CodeMirror's keys, as in Typora's fences: Enter keeps
        // the line's indentation, Tab types a tab or indents the selected
        // lines, Shift+Tab outdents (or auto indents, if set).
        if (edit && !multi && codeKinds.indexOf(itemAt(row).kind) >= 0) {
            if ((event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && plain) {
                codeNewline(edit)
                event.accepted = true
                return
            }
            if (event.key === Qt.Key_Tab && plain) {
                if (edit.selectedText === "")
                    edit.insert(edit.cursorPosition, "\t")
                else
                    codeIndent(edit, true)
                event.accepted = true
                return
            }
            if (event.key === Qt.Key_Backtab) {
                if (Prefs.shiftTabAutoIndent && edit.highlighter)
                    codeAutoIndent(edit)
                else
                    codeIndent(edit, false)
                event.accepted = true
                return
            }
        }
        if (edit && !multi && move !== "") {
            Ui.moveCaret(edit, move, extending, codeKinds.indexOf(itemAt(row).kind) >= 0)
            event.accepted = true
            return
        }
        switch (event.key) {
        case Qt.Key_Return:
        case Qt.Key_Enter:
            if (shift && edit) {
                // A soft line break; Qt would insert a line separator.
                edit.remove(edit.selectionStart, edit.selectionEnd)
                edit.insert(edit.cursorPosition, "\n")
                event.accepted = true
            } else if (plain) {
                event.accepted = run("enter")
            }
            return
        case Qt.Key_Backspace:
            if (plain && edit && edit.selectedText === "" && pos > 0 && Prefs.autoPairBrackets
                    && editor.pairedAt(row, pos, Prefs.autoPairMarkdown)) {
                edit.remove(pos - 1, pos + 1)
                event.accepted = true
            } else if (plain || shift) {
                event.accepted = run("backspace")
            }
            return
        case Qt.Key_Delete:
            if (plain)
                event.accepted = run("delete")
            return
        case Qt.Key_Tab:
            if (plain)
                event.accepted = run("indent")
            return
        case Qt.Key_Backtab:
            run("outdent")
            event.accepted = true
            return
        case Qt.Key_Z:
            if (ctrl || ctrlShift) {
                editor.command(ctrl ? "undo" : "redo", 0, 0, 0, 0, 0, "")
                event.accepted = true
            }
            return
        case Qt.Key_Y:
            if (ctrl) {
                editor.command("redo", 0, 0, 0, 0, 0, "")
                event.accepted = true
            }
            return
        case Qt.Key_PageUp:
        case Qt.Key_PageDown:
            if (plain) {
                page(event.key === Qt.Key_PageDown ? 1 : -1, false)
                event.accepted = true
            }
            return
        }
        if (!edit)
            return
        switch (event.key) {
        case Qt.Key_Left:
            if (plain && pos === 0 && edit.selectedText === "")
                event.accepted = moveTo(editableRow(row, -1), "end")
            return
        case Qt.Key_Right:
            if (plain && pos === len && edit.selectedText === "")
                event.accepted = moveTo(editableRow(row, 1), "start")
            return
        // Up on the document's first line goes to its start, and Down on
        // its last line to its end, as in Typora.
        case Qt.Key_Up:
            if (plain && atTop) {
                if (!enterTag(row, 0) && !moveTo(editableRow(row, -1), "last", x))
                    edit.cursorPosition = 0
                event.accepted = true
            }
            return
        // Down out of a code, math or other non-text block at the end of
        // the document opens a paragraph after it, so there is a way out.
        case Qt.Key_Down:
            if (plain && atBottom) {
                if (enterTag(row, 1) || moveTo(editableRow(row, 1), "first", x))
                    ;
                else if (!blocks.itemAt(row).textual)
                    run("paragraphAfter")
                else
                    edit.cursorPosition = len
                event.accepted = true
            }
            return
        }
    }

    // Out past the end of block `row` (a math block's Finish Editing
    // button): into the next block, or a new paragraph when there is none.
    function leaveBlock(row) {
        if (!moveTo(editableRow(row, 1), "first", 0))
            editor.command("paragraphAfter", row, 1, row, 1, 0, "")
    }

    // Auto-pairing (Preferences ▸ Editor): typing an opener adds its closer,
    // typing a closer steps over the one already there. True when it
    // handled the key.
    function typePaired(row, edit, ch) {
        if (!Prefs.autoPairBrackets || edit.selectedText !== "")
            return false
        const pos = edit.cursorPosition
        // `* ` starts a list; drop the closer the `*` was paired with.
        if (ch === " " && Prefs.autoPairMarkdown && edit.text === "**" && pos === 1) {
            edit.remove(1, 2)
            return false
        }
        const what = editor.pairTyped(row, pos, ch, Prefs.autoPairMarkdown)
        if (what === "")
            return false
        if (what === "skip") {
            edit.cursorPosition = pos + 1
            return true
        }
        edit.insert(pos, ch + what)
        edit.cursorPosition = pos + 1
        return true
    }

    // The third backtick (or tilde) of a code fence brings the default
    // code language after it, selected to type over (Preferences ▸
    // Markdown ▸ Code Fences), as Typora suggests it. True when it handled
    // the key.
    function typeFenceLang(row, edit, ch) {
        if ((ch !== "`" && ch !== "~") || edit.selectedText !== "" || edit.text !== ch + ch || edit.cursorPosition !== 2
                || itemAt(row).kind !== "paragraph")
            return false
        const lang = editor.codeLangSuggestion(false)
        if (lang === "")
            return false
        edit.insert(2, ch + lang)
        edit.select(3, 3 + lang.length)
        return true
    }

    // Smart punctuation converted on input (Preferences ▸ Markdown): a
    // curly quote for a straight one, a dash for two hyphens. True when it
    // handled the key.
    function typeSmart(row, edit, ch) {
        if (edit.selectedText !== "" || Prefs.smartOnRendering || !(Prefs.smartQuotes || Prefs.smartDashes))
            return false
        const pos = edit.cursorPosition
        const r = editor.smartTyped(row, pos, ch, Prefs.autoPairBrackets, Prefs.autoPairMarkdown)
        return applySmart(edit, pos, r)
    }
    function applySmart(edit, pos, r) {
        if (r === "")
            return false
        if (r === "skip") {
            edit.cursorPosition = pos + 1
            return true
        }
        const t = JSON.parse(r)
        if (t.back > 0)
            edit.remove(pos - t.back, pos)
        edit.insert(pos - t.back, t.text)
        edit.cursorPosition = pos - t.back + t.caret
        return true
    }

    // Move the caret into `row`: its start or end, or its first or last
    // line at window x. False when there is no such row.
    function moveTo(row, where, x) {
        if (row < 0)
            return false
        const item = blocks.itemAt(row)
        if (item.table)
            return item.table.enter(where, x)
        if (where === "first" && enterTag(row, 0) || where === "last" && enterTag(row, 1))
            return true
        const edit = item.editItem
        let pos = 0
        if (where === "end")
            pos = edit.length
        else if (where === "first" || where === "last") {
            // Aim at the middle of the line: a hit above the first line's
            // text counts as its start.
            const r = edit.positionToRectangle(where === "first" ? 0 : edit.length)
            pos = edit.positionAt(edit.mapFromItem(null, x, 0).x, r.y + r.height / 2)
        }
        return focusRow(row, pos, pos)
    }

    // A math block's {tag} on the $$ line before (0) or after (1) its
    // source, when it has one, is on the way into and out of it by line,
    // selected, as Typora's. True when it took the caret.
    function enterTag(row, i) {
        const item = blocks.itemAt(row)
        const tags = item && item.body ? item.body.tagFields : undefined
        const field = tags && tags.count === 2 ? tags.itemAt(i) : null
        if (!field || field.text === "")
            return false
        field.forceActiveFocus()
        field.selectAll()
        return true
    }

    // Page Up and Page Down: scroll by a page and keep the caret where it
    // was in the view.
    function page(step, extend) {
        const s = selection()
        const item = blocks.itemAt(s.zRow)
        if (!item)
            return
        const edit = item.editItem
        const p = edit ? edit.mapToItem(pointer, edit.cursorRectangle.x, edit.cursorRectangle.y + edit.cursorRectangle.height / 2)
                       : item.mapToItem(pointer, 0, item.height / 2)
        const before = contentY
        contentY = Math.max(0, Math.min(Math.max(0, contentHeight - height), contentY + step * height * 0.9))
        let target
        if (contentY === before) {
            const last = blocks.count - 1
            target = step < 0 ? { row: 0, pos: 0 } : { row: last, pos: editAt(last) ? editAt(last).length : 1 }
        } else {
            target = hit(p.x, p.y + contentY - before)
        }
        if (extend)
            select(s.aRow, s.aPos, target.row, target.cell ? (target.row > s.aRow ? 1 : 0) : target.pos)
        else
            goTo(target)
    }

    function jumpTop() {
        contentY = 0
        select(0, 0, 0, 0)
    }

    function jumpBottom() {
        const last = blocks.count - 1
        contentY = Math.max(0, contentHeight - height)
        const edit = editAt(last)
        select(last, edit ? edit.length : 1, last, edit ? edit.length : 1)
    }

    // Edit ▸ Jump to Selection.
    function jumpSelection() {
        const s = selection()
        const item = blocks.itemAt(s.zRow)
        if (item && item.editItem)
            reveal(item, item.editItem.cursorRectangle)
    }

    // Copy the selection: markdown as text with HTML beside it, or one form
    // alone ("markdown", "html", "plain"). False when nothing is selected.
    function copy(as) {
        const t = tableAt()
        if (t)
            return t.copy()
        const s = selection()
        if (collapsed(s))
            return Prefs.lineWiseCopy && copyLine(as, false)
        // Inside a code, math or HTML block, its text as it is.
        const edit = editAt(s.zRow)
        if (!span && edit && !blocks.itemAt(s.zRow).textual) {
            Desktop.setClipboard(edit.selectedText, "", "")
            return true
        }
        const md = editor.selectionMarkdown(s.aRow, s.aPos, s.zRow, s.zPos)
        const html = as === "markdown" ? "" : editor.selectionHtml(s.aRow, s.aPos, s.zRow, s.zPos)
        if (as === "markdown")
            Desktop.setClipboard(md, "", md)
        else if (as === "html")
            Desktop.setClipboard(html, "", "")
        else if (as === "plain")
            Desktop.setClipboard(Desktop.plainText(html), "", "")
        else
            // Other apps get the rendered text, unless Preferences ▸ Editor
            // says to copy markdown source as plain text.
            Desktop.setClipboard(Prefs.copyMarkdownAsPlain ? md : Desktop.plainText(html), html, md)
        return true
    }

    // Delete Selected: remove the selection, across blocks or within one.
    function deleteSelection() {
        const t = tableAt()
        if (t) {
            t.deleteSelection()
            return
        }
        const s = selection()
        if (collapsed(s))
            return
        if (span) {
            command("deleteSelection")
            return
        }
        const edit = editAt(s.zRow)
        if (edit)
            edit.remove(edit.selectionStart, edit.selectionEnd)
    }

    function cut() {
        const t = tableAt()
        if (t) {
            t.cut()
            return
        }
        if (collapsed(selection())) {
            if (Prefs.lineWiseCopy)
                copyLine("", true)
            return
        }
        if (copy(""))
            command("deleteSelection")
    }

    // Preferences ▸ Editor: with nothing selected, copy or cut the line
    // the caret is on: in a code, math or HTML block that line, elsewhere
    // the whole paragraph or heading.
    function copyLine(as, cutting) {
        const s = selection()
        const item = blocks.itemAt(s.zRow)
        const edit = editAt(s.zRow)
        if (!item || !edit)
            return false
        const text = edit.text
        if (!item.textual) {
            const at = edit.cursorPosition
            const a = text.lastIndexOf("\n", at - 1) + 1
            const end = text.indexOf("\n", at)
            const z = end < 0 ? text.length : end
            Desktop.setClipboard(text.substring(a, z) + "\n", "", "")
            // The line goes with its line break: the one after it, or for
            // the last line the one before.
            if (cutting)
                edit.remove(end < 0 ? Math.max(0, a - 1) : a, end < 0 ? z : z + 1)
            return true
        }
        if (text === "") {
            Desktop.setClipboard("\n", "", "\n")
        } else {
            select(s.zRow, 0, s.zRow, text.length)
            copy(as)
        }
        if (cutting)
            command("deleteBlock")
        else
            focusRow(s.zRow, s.zPos, s.zPos)
        return true
    }

    // Paste: typo's own copies as the markdown they were, rich HTML from
    // other apps converted to markdown, and plain text read as markdown.
    // Paste as Plain Text, and any paste into a code block, takes only the
    // text.
    function paste(plain) {
        if (editor.readonly)
            return
        const t = tableAt()
        if (t) {
            t.paste(plain)
            return
        }
        const s = selection()
        const item = blocks.itemAt(s.zRow)
        const code = !span && item && !item.textual
        let text = plain || code ? "" : Desktop.clipboardMarkdown()
        if (text === "" && !plain && !code) {
            const html = Desktop.clipboardHtml()
            if (html !== "")
                text = editor.htmlToMarkdown(html)
        }
        // Files copied in a file manager go in as files, and a picture
        // (a screenshot, say) is saved as a file and goes in as one.
        if (text === "" && !plain && !code) {
            const files = Desktop.clipboardFiles()
            if (files !== "") {
                filesPasted(files.split("\n"))
                return
            }
            const picture = Desktop.saveClipboardImage(editor.pastedImageFolder(Desktop.pastedImagesFolder()))
            if (picture !== "") {
                filesPasted([picture])
                return
            }
        }
        if (text === "")
            text = Desktop.clipboardText()
        if (text !== "")
            command("paste", 0, text)
    }

    // Insert files (paths one a line) at selection `s`, or in the table
    // cell being edited. `create` makes a missing image folder. The error
    // when a copy failed, or "".
    function insertFiles(list, create, s) {
        const t = tableAt()
        if (t && t.editItem) {
            const e = t.editItem
            const a = Math.min(e.selectionStart, e.selectionEnd)
            const md = editor.filesMarkup(list, a > 0, create)
            e.remove(a, Math.max(e.selectionStart, e.selectionEnd))
            e.insert(a, md)
            return ""
        }
        return editor.insertFiles(s.aRow, s.aPos, s.zRow, s.zPos, list, create)
    }

    // Edit ▸ Select All: the whole document; in a code, math or HTML
    // block, first the block's text.
    function selectAll() {
        const t = tableAt()
        if (t && t.editItem && !(t.editItem.selectionStart === 0 && t.editItem.selectionEnd === t.editItem.length)) {
            t.selectAll()
            return
        }
        const s = selection()
        const item = blocks.itemAt(s.zRow)
        const edit = item ? item.editItem : null
        if (!span && edit && !item.textual && !(edit.selectionStart === 0 && edit.selectionEnd === edit.length)) {
            edit.selectAll()
            return
        }
        const last = blocks.count - 1
        const end = editAt(last) ? editAt(last).length : 1
        select(0, 0, last, end)
    }

    // Edit ▸ Selection: the block, sentence, styled scope or word around the
    // caret. False when that is empty.
    function selectUnit(unit) {
        const s = selection()
        const t = tableAt()
        const edit = t ? t.editItem : editAt(s.zRow)
        if (!edit)
            return false
        const text = edit.text
        const at = edit.cursorPosition
        let a = 0
        let z = text.length
        if (unit === "word") {
            // The word around the caret, or the one it ends.
            // Letters change with case; CJK and other scripts without
            // case count as letters too.
            const word = c => c !== "" && (c.toLowerCase() !== c.toUpperCase() || /[0-9_']/.test(c) || c.charCodeAt(0) >= 0x3040)
            a = at
            z = at
            if (word(text.charAt(at)) || word(text.charAt(at - 1))) {
                while (a > 0 && word(text.charAt(a - 1)))
                    a--
                while (z < text.length && word(text.charAt(z)))
                    z++
            }
        } else if (unit === "line") {
            // A sentence ends at . ! ? (or their full-width forms) and the
            // spaces after them, or at a line break.
            const ends = /[.!?。！？]+\s*|\n/g
            let m
            while ((m = ends.exec(text)) !== null) {
                const stop = m.index + m[0].length
                if (stop <= at) {
                    a = stop
                } else {
                    z = m[0] === "\n" ? m.index : stop
                    break
                }
            }
        } else if (unit === "scope") {
            const scope = editor.scopeAt(s.zRow, at)
            if (scope !== "")
                [a, z] = scope.split(",").map(Number)
        }
        if (t)
            edit.select(a, z)
        else
            focusRow(s.zRow, a, z)
        return a !== z
    }

    // Edit ▸ Delete: the same units.
    function deleteUnit(unit) {
        if (unit === "block") {
            command("deleteBlock")
            return
        }
        if (selectUnit(unit))
            command("deleteSelection")
    }

    Connections {
        target: view.editor
        function onCaretRequested(row, anchor, pos) {
            view.focusRow(row, anchor, pos)
        }
        function onSelectionRequested(aRow, aPos, zRow, zPos) {
            view.select(aRow, aPos, zRow, zPos)
        }
        function onModelReset() {
            view.span = null
            emojiSuggest.close()
            view.updateMath()
        }
        function onContentChanged() {
            view.updateMath()
        }
    }
}
