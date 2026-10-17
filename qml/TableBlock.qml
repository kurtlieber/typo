import QtQuick
import QtQuick.Controls
import QtQuick.Shapes
import Typo

// A table, edited cell by cell as in Typora. Each cell is a line of inline
// markdown with live preview; the document keeps the table as one block,
// so the view treats it as a block without text (positions 0 before it
// and 1 after it) except while the caret is in a cell. Column widths
// follow the browser's layout of Typora's `table { width: 100% }`. While a
// cell has the caret, the table's toolbar shows above it.
Item {
    id: table

    // The BlockDelegate this table is the content of. (Not `block`: the
    // delegate's own id is that, and `block: block` would bind to itself.)
    required property Item owner
    readonly property var editor: owner.editor
    readonly property Item host: owner.host
    readonly property real scale: owner.scale
    readonly property real em: owner.em

    readonly property var spec: owner.rows ? JSON.parse(owner.rows) : ({ align: [], rows: [] })
    readonly property int columns: spec.align.length
    readonly property int rowCount: spec.rows.length

    // The cell with the caret, or the one that last had it.
    property int curR: 0
    property int curC: 0
    readonly property Item editItem: {
        cells.count
        const cell = cellItem(curR, curC)
        return cell ? cell.edit : null
    }
    readonly property bool focused: editItem !== null && editItem.activeFocus
    readonly property bool active: focused || sizer.opened || more.opened
    // The current column's alignment: "", "left", "center" or "right".
    readonly property string currentAlign: spec.align[curC] || ""

    // The github theme's cell padding, 6px 13px, and its 1px borders.
    readonly property real padX: 13 * scale
    readonly property real padY: 6 * scale
    readonly property real lineHeight: em * 1.6

    // Where each column and row starts, and how big it is.
    property var colX: []
    property var colW: []
    property var rowY: []
    property var rowH: []

    width: parent ? parent.width : 0
    height: rowY.length > 0 ? rowY[rowY.length - 1] + rowH[rowH.length - 1] + 1 : lineHeight + 2 * padY

    readonly property real firstBaseline: padY + lineHeight * 0.8
    readonly property real firstLineMiddle: padY + lineHeight / 2

    function cellItem(r, c) {
        if (r < 0 || c < 0 || r >= rowCount || c >= columns)
            return null
        return cells.itemAt(r * columns + c)
    }

    // Column widths as a browser lays out an auto table at 100% width:
    // columns get their widest content, the room left over is shared in
    // proportion, and when there isn't room each gets what it needs at
    // least (its longest word) and a share of the rest.
    function relayout() {
        const n = columns
        if (n === 0 || cells.count !== rowCount * n) {
            colX = []
            colW = []
            relayoutRows()
            return
        }
        const room = width - 1
        const least = 32 * scale + 2 * padX
        const mins = new Array(n).fill(least)
        const maxs = new Array(n).fill(least)
        for (let i = 0; i < cells.count; i++) {
            const cell = cells.itemAt(i)
            if (!cell)
                continue
            mins[cell.c] = Math.max(mins[cell.c], cell.minWidth + 2 * padX)
            maxs[cell.c] = Math.max(maxs[cell.c], cell.maxWidth + 2 * padX)
        }
        let sumMin = 0
        let sumMax = 0
        for (let c = 0; c < n; c++) {
            maxs[c] = Math.max(maxs[c], mins[c])
            sumMin += mins[c]
            sumMax += maxs[c]
        }
        let w
        if (sumMax <= room)
            w = maxs.map(m => m + (room - sumMax) * m / sumMax)
        else if (sumMin >= room)
            w = mins
        else
            w = mins.map((m, c) => m + (maxs[c] - m) * (room - sumMin) / (sumMax - sumMin))
        const x = []
        let at = 0
        for (let c = 0; c < n; c++) {
            x.push(Math.round(at))
            at += w[c]
        }
        colW = w.map((v, c) => Math.round(x[c] + v) - x[c])
        colX = x
        rowsLater.restart()
    }

    // Rows are as tall as their tallest cell.
    function relayoutRows() {
        const h = new Array(rowCount).fill(lineHeight + 2 * padY)
        for (let i = 0; i < cells.count; i++) {
            const cell = cells.itemAt(i)
            if (cell && cell.r < rowCount)
                h[cell.r] = Math.max(h[cell.r], cell.edit.height + 2 * padY)
        }
        const y = []
        let at = 0
        for (let r = 0; r < rowCount; r++) {
            y.push(at)
            at += h[r]
        }
        rowH = h
        rowY = y
    }

    onWidthChanged: layoutLater.restart()
    onColumnsChanged: layoutLater.restart()
    onRowCountChanged: layoutLater.restart()
    onScaleChanged: layoutLater.restart()

    // Layout waits for the change in hand to finish. Timers, unlike
    // Qt.callLater, stop when the table goes.
    Timer {
        id: layoutLater
        interval: 0
        onTriggered: table.relayout()
    }
    Timer {
        id: rowsLater
        interval: 0
        onTriggered: table.relayoutRows()
    }

    // The column or row at x or y in the table, the nearest when outside.
    function columnAt(x) {
        for (let c = 0; c < colX.length; c++)
            if (x < colX[c] + colW[c])
                return c
        return Math.max(0, columns - 1)
    }
    function rowAt(y) {
        for (let r = 0; r < rowY.length; r++)
            if (y < rowY[r] + rowH[r])
                return r
        return Math.max(0, rowCount - 1)
    }

    // The cell at (x, y) in the table: { r, c, edit }.
    function cellAt(x, y) {
        const r = rowAt(y)
        const c = columnAt(x)
        const cell = cellItem(r, c)
        return cell ? { r: r, c: c, edit: cell.edit } : null
    }

    // Put the caret in cell (r, c), selecting from `anchor` to `pos`.
    // Cells made again after a change may not be there yet; then once
    // they are.
    function focusCell(r, c, anchor, pos, retried) {
        dropSpan()
        curR = Math.max(0, Math.min(rowCount - 1, r))
        curC = Math.max(0, Math.min(columns - 1, c))
        const cell = cellItem(curR, curC)
        if (!cell) {
            if (!retried)
                Qt.callLater(() => table.focusCell(r, c, anchor, pos, true))
            return false
        }
        if (host)
            host.clearSpan()
        const edit = cell.edit
        edit.forceActiveFocus()
        const len = edit.length
        edit.select(Math.min(anchor, len), Math.min(pos, len))
        if (host)
            host.reveal(owner, edit.cursorRectangle)
        return true
    }

    // A position the editor asked for: 0 is before the table (its first
    // cell), 1 after it (the end of its last cell), and from 2 a cell, 4096
    // to a row, as the undo history records them (src/qt/editor.rs).
    function place(anchor, pos) {
        const end = 1e9
        if (pos >= 2) {
            const k = pos - 2
            return focusCell(Math.floor(k / 4096), k % 4096, end, end)
        }
        if (pos === 1)
            return focusCell(rowCount - 1, columns - 1, end, end)
        return focusCell(0, 0, 0, 0)
    }

    // Several cells selected, as a drag from one into another makes the
    // browser's selection in Typora: from `a` to `z` ({r, c, pos}, UTF-16),
    // either way round, in reading order, the cells between whole; the
    // caret stays at `z`. Typing, Backspace, Delete, Cut and Paste replace
    // what it takes from every cell, keeping each row and column, and Copy
    // takes it as a table. null while the selection is in one cell.
    property var cellSpan: null
    // Whether position `x` ({r, c, pos}) comes before `y` in reading order.
    function before(x, y) {
        return x.r < y.r || x.r === y.r && (x.c < y.c || x.c === y.c && x.pos <= y.pos)
    }
    function spanEnds() {
        return before(cellSpan.a, cellSpan.z) ? [cellSpan.a, cellSpan.z] : [cellSpan.z, cellSpan.a]
    }
    // The selection's ends ({r, c, pos}): the span's, or the caret cell's.
    function selectionEnds() {
        if (cellSpan)
            return cellSpan
        const e = editItem
        if (!e)
            return null
        const anchor = e.cursorPosition === e.selectionStart ? e.selectionEnd : e.selectionStart
        return { a: { r: curR, c: curC, pos: anchor }, z: { r: curR, c: curC, pos: e.cursorPosition } }
    }
    function inSpan(r, c, pos) {
        if (!cellSpan)
            return false
        const [f, t] = spanEnds()
        const p = { r: r, c: c, pos: pos }
        return before(f, p) && before(p, t)
    }
    function selectSpan(a, z) {
        if (a.r === z.r && a.c === z.c)
            return focusCell(z.r, z.c, a.pos, z.pos)
        focusCell(z.r, z.c, z.pos, z.pos)
        cellSpan = { a: a, z: z }
        const [f, t] = spanEnds()
        const first = f.r * columns + f.c
        const last = t.r * columns + t.c
        for (let k = first; k <= last; k++) {
            const cell = cells.itemAt(k)
            if (!cell)
                continue
            const from = k === first ? f.pos : 0
            const to = k === last ? t.pos : cell.edit.length
            if (k === z.r * columns + z.c)
                cell.edit.select(z === t ? from : to, z.pos)
            else
                cell.edit.select(from, to)
        }
    }
    function dropSpan() {
        if (!cellSpan)
            return
        const [f, t] = spanEnds()
        cellSpan = null
        for (let k = f.r * columns + f.c; k <= t.r * columns + t.c; k++) {
            const cell = cells.itemAt(k)
            if (cell)
                cell.edit.deselect()
        }
    }
    // Type `text` over the span ("" deletes it).
    function replaceSpan(text) {
        const a = cellSpan.a
        const z = cellSpan.z
        dropSpan()
        const where = editor.replaceCellSpan(owner.index, a.r, a.c, a.pos, z.r, z.c, z.pos, text)
        if (where === "")
            return
        const [r, c, pos] = where.split(",").map(Number)
        Qt.callLater(() => table.focusCell(r, c, pos, pos))
    }
    // The caret going to another block, or the table changing, ends it.
    onSpecChanged: dropSpan()
    Connections {
        target: table.host
        enabled: table.cellSpan !== null
        function onCurrentRowChanged() {
            if (table.host.currentRow !== table.owner.index)
                table.dropSpan()
        }
    }

    // The caret arriving from the keyboard: at the table's start or end,
    // or on its first or last row at window x.
    function enter(where, x) {
        if (where === "start")
            return focusCell(0, 0, 0, 0)
        if (where === "end")
            return focusCell(rowCount - 1, columns - 1, 1e9, 1e9)
        return enterCell(where === "first" ? 0 : rowCount - 1, columnAt(mapFromItem(null, x, 0).x), where, x)
    }

    // Into cell (r, c) on its first or last line at window x.
    function enterCell(r, c, where, x) {
        const cell = cellItem(r, c)
        if (!cell)
            return false
        const e = cell.edit
        const rect = e.positionToRectangle(where === "first" ? 0 : e.length)
        const p = e.positionAt(e.mapFromItem(null, x, 0).x, rect.y + rect.height / 2)
        return focusCell(r, c, p, p)
    }

    // Typora's row and column trackers: unseen handles, 12 by 8 pixels,
    // across the left border of the row the pointer was last over and the
    // top border of its column. Dragging one moves the row or column: it
    // stays behind as a shaded gap while a copy follows the pointer, and
    // arrows at the table's edges mark where it would go.
    property int hoverR: -1
    property int hoverC: -1
    function hover(x, y) {
        if (x < 0 || y < 0 || x >= width || y >= height || cells.count === 0)
            return
        hoverR = rowAt(y)
        hoverC = columnAt(x)
    }
    // The handle at (x, y): `{ kind: "row" | "column", index }`, or null.
    // The column's is on top where they meet.
    function handleAt(x, y) {
        if (moving || hoverR >= rowCount || hoverC >= columns)
            return null
        if (hoverC >= 0 && y >= -4 * scale && y < 4 * scale && x >= colX[hoverC] && x <= colX[hoverC] + colW[hoverC])
            return { kind: "column", index: hoverC }
        if (hoverR >= 0 && x >= -6 * scale && x < 6 * scale && y >= rowY[hoverR] && y <= rowY[hoverR] + rowH[hoverR])
            return { kind: "row", index: hoverR }
        return null
    }

    // The drag in hand: what moves, from where, the place it would go
    // (before row or column `to`; the count for after the last), and the
    // pointer, held where it took the handle.
    property var moving: null
    readonly property bool movingRow: moving !== null && moving.kind === "row"
    readonly property bool movingColumn: moving !== null && moving.kind === "column"
    function startDrag(kind, index, x, y) {
        if (editItem)
            editItem.deselect()
        const row = kind === "row"
        moving = { kind: kind, from: index, to: index, x: x, y: y,
                   grabX: row ? 0 : x - colX[index], grabY: row ? y - rowY[index] : 0 }
    }
    function dragTo(x, y) {
        if (!moving)
            return
        const row = moving.kind === "row"
        const v = row ? y : x
        // The rows' tops and the last one's bottom (the columns' lefts and
        // the last one's right): the place is before the one the pointer
        // is in, or after the last below or beside them all.
        const starts = row ? rowY : colX
        const sizes = row ? rowH : colW
        const g = starts.concat([starts[starts.length - 1] + sizes[sizes.length - 1]])
        let to = g.findIndex(s => s > v)
        if (to > 0)
            to--
        else if (to < 0)
            to = g.length - 1
        moving = Object.assign({}, moving, { to: to, x: x, y: y })
    }
    // Typora's arrows: not on the row itself, nor, for columns, at either
    // of its own edges inside the table.
    readonly property bool marking: moving !== null
        && (movingRow ? moving.to !== moving.from
                      : moving.to >= columns || (moving.to !== moving.from && moving.to !== moving.from + 1))
    function drop() {
        const m = moving
        moving = null
        if (m && m.to !== m.from && m.to !== m.from + 1)
            run(m.kind === "row" ? "dragRow" : "dragColumn", m.from, m.to)
    }
    function cancelDrag() {
        moving = null
    }
    // The page draws the table, the copy included, above the blocks after
    // it while a drag is in hand.
    Binding {
        target: table.owner
        property: "z"
        value: 1
        when: table.moving !== null
    }

    // Run a table command with the caret in the current cell, and put the
    // caret where it says. False when it doesn't apply.
    function run(name, a, b) {
        const where = editor.tableCommand(owner.index, name, curR, curC, a || 0, b || 0)
        if (where === "" || where === "out")
            return where === "out"
        const [r, c] = where.split(",").map(Number)
        const fresh = name === "rowAbove" || name === "rowBelow" || name === "columnBefore" || name === "columnAfter"
        curR = r
        curC = c
        Qt.callLater(() => table.focusCell(r, c, fresh ? 0 : 1e9, fresh ? 0 : 1e9))
        return true
    }

    // The table menu's commands, by the menu bar's names.
    function menuCommand(cmd, arg) {
        switch (cmd) {
        case "tableRow": return run(arg === "above" ? "rowAbove" : "rowBelow")
        case "tableColumn": return run(arg === "before" ? "columnBefore" : "columnAfter")
        case "tableMove": return run({ up: "rowUp", down: "rowDown", left: "columnLeft", right: "columnRight" }[arg])
        case "tableDelete": return run({ row: "deleteRow", column: "deleteColumn", table: "deleteTable" }[arg])
        case "tableAlign": return run({ left: "alignLeft", center: "alignCenter", right: "alignRight" }[arg], curC, curC)
        case "tableCopy": return copyTable()
        case "tableFormat": return run("prettify")
        }
        return false
    }

    // Copy Table: the whole table, as markdown with HTML beside it.
    function copyTable() {
        const md = editor.selectionMarkdown(owner.index, 0, owner.index, 1)
        const html = editor.selectionHtml(owner.index, 0, owner.index, 1)
        Desktop.setClipboard(Prefs.copyMarkdownAsPlain ? md : Desktop.plainText(html), html, md)
        return true
    }

    // Cell-local editing for the Edit menu.
    function copy() {
        if (cellSpan) {
            const a = cellSpan.a
            const z = cellSpan.z
            const json = editor.cellSpanCopy(owner.index, a.r, a.c, a.pos, z.r, z.c, z.pos)
            if (json === "")
                return false
            const [md, html] = JSON.parse(json)
            Desktop.setClipboard(Prefs.copyMarkdownAsPlain ? md : Desktop.plainText(html), html, md)
            return true
        }
        if (!editItem || editItem.selectedText === "")
            return false
        Desktop.setClipboard(editItem.selectedText, "", editItem.selectedText)
        return true
    }
    function cut() {
        if (!copy())
            return false
        if (cellSpan)
            replaceSpan("")
        else
            editItem.remove(editItem.selectionStart, editItem.selectionEnd)
        return true
    }
    // A cell holds one line: pasted line breaks become spaces.
    function paste(plain) {
        if (!editItem)
            return false
        let text = plain ? "" : Desktop.clipboardMarkdown()
        if (text === "")
            text = Desktop.clipboardText()
        text = text.replace(/\r?\n+$/, "").replace(/\r?\n/g, " ")
        if (text === "")
            return true
        if (cellSpan) {
            replaceSpan(text)
            return true
        }
        const at = editItem.selectionStart
        editItem.remove(editItem.selectionStart, editItem.selectionEnd)
        editItem.insert(at, text)
        editItem.cursorPosition = at + text.length
        return true
    }
    function selectAll() {
        if (!editItem)
            return false
        dropSpan()
        editItem.selectAll()
        return true
    }
    function deleteSelection() {
        if (cellSpan)
            replaceSpan("")
        else if (editItem)
            editItem.remove(editItem.selectionStart, editItem.selectionEnd)
        return true
    }

    // Formatting in a cell wraps the selection in the style's markers, or
    // unwraps it when they are already there. Block formats don't apply.
    readonly property var markers: ({
        strong: ["**", "**"], emphasis: ["*", "*"], underline: ["<u>", "</u>"], code: ["`", "`"],
        inlineMath: ["$", "$"], strike: ["~~", "~~"], highlight: ["==", "=="], superscript: ["^", "^"],
        subscript: ["~", "~"], comment: ["<!-- ", " -->"]
    })
    // True when the command was the cell's; false to run it on the table
    // as a block.
    function format(name, text) {
        if (["paragraphBefore", "paragraphAfter", "deleteBlock", "undo", "redo"].indexOf(name) >= 0)
            return false
        const e = editItem
        if (e && ["link", "image", "zoomImage", "imageSyntax", "footnote", "imageSrc", "deleteImage"].indexOf(name) >= 0) {
            // The new text and selection, as the editor makes them.
            const json = editor.editText(e.text, e.selectionStart, e.selectionEnd, name, text || "")
            if (json !== "") {
                const r = JSON.parse(json)
                // Only what changed is replaced: setting the text would cut
                // it loose from the model's, which undo puts back.
                const t = e.text
                const out = r[2]
                let a = 0
                while (a < t.length && a < out.length && t[a] === out[a])
                    a++
                let z = 0
                while (z < t.length - a && z < out.length - a && t[t.length - 1 - z] === out[out.length - 1 - z])
                    z++
                if (a < t.length - z)
                    e.remove(a, t.length - z)
                if (a < out.length - z)
                    e.insert(a, out.substring(a, out.length - z))
                e.select(r[0], r[1])
            }
            return true
        }
        const m = markers[name]
        if (!m || !e)
            return true
        const a = e.selectionStart
        const z = e.selectionEnd
        const t = e.text
        if (a >= m[0].length && t.substring(a - m[0].length, a) === m[0] && t.substring(z, z + m[1].length) === m[1]) {
            e.remove(z, z + m[1].length)
            e.remove(a - m[0].length, a)
            e.select(a - m[0].length, z - m[0].length)
        } else {
            e.insert(z, m[1])
            e.insert(a, m[0])
            e.select(a + m[0].length, z + m[0].length)
        }
        return true
    }

    // Keys in a cell, as Typora's tables take them; the rest go to the
    // view (undo, page keys, the menu key) or stay with the cell.
    function keyPressed(cell, event) {
        const mods = event.modifiers & (Qt.ShiftModifier | Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)
        const plain = mods === Qt.NoModifier
        const shift = mods === Qt.ShiftModifier
        const ctrl = mods === Qt.ControlModifier
        const alt = mods === Qt.AltModifier
        const typing = !(mods & Ui.notTyping)
        const edit = cell.edit
        const r = cell.r
        const c = cell.c
        const pos = edit.cursorPosition
        const len = edit.length
        const none = edit.selectedText === ""
        const x = edit.mapToItem(null, edit.cursorRectangle.x, 0).x
        const accept = done => { event.accepted = done !== false }
        if (host && (host.emojiSuggest.shown && host.emojiSuggest.keyPressed(event) || host.popup && host.popup.shown && host.popup.keyPressed(event)))
            return accept()
        if (event.matches(StandardKey.Copy))
            return accept(copy() || true)
        if (event.matches(StandardKey.Cut))
            return accept(cut() || true)
        if (event.matches(StandardKey.Paste))
            return accept(paste(false))
        // Over several cells, typing and deleting take what is selected
        // from each; other keys (but modifiers alone) end the span and act
        // at the caret. An arrow goes from the end it points to, Left and
        // Right no further, as the browser's do.
        if (cellSpan) {
            if ([Qt.Key_Shift, Qt.Key_Control, Qt.Key_Alt, Qt.Key_Meta].indexOf(event.key) >= 0)
                return
            if ((event.key === Qt.Key_Backspace || event.key === Qt.Key_Delete) && (plain || shift)) {
                replaceSpan("")
                return accept()
            }
            if (event.text.length > 0 && event.text >= " " && typing) {
                replaceSpan(event.text)
                return accept()
            }
            const arrow = plain ? [Qt.Key_Left, Qt.Key_Up, Qt.Key_Right, Qt.Key_Down].indexOf(event.key) : -1
            const at = arrow < 0 ? cellSpan.z : spanEnds()[arrow < 2 ? 0 : 1]
            focusCell(at.r, at.c, at.pos, at.pos)
            if (arrow === 0 || arrow === 2)
                return accept()
            return keyPressed(cellItem(at.r, at.c), event)
        }
        // Select All takes the cell, then the document.
        if (event.matches(StandardKey.SelectAll)) {
            if (host)
                host.selectAll()
            return accept()
        }
        // Smart punctuation converted on input, as in paragraphs.
        if (event.text.length === 1 && none && typing && !Prefs.smartOnRendering && (Prefs.smartQuotes || Prefs.smartDashes)) {
            const r = editor.cellSmartTyped(edit.text, pos, event.text, Prefs.autoPairBrackets, Prefs.autoPairMarkdown)
            if (r === "skip") {
                edit.cursorPosition = pos + 1
                return accept()
            }
            if (r !== "") {
                const t = JSON.parse(r)
                if (t.back > 0)
                    edit.remove(pos - t.back, pos)
                edit.insert(pos - t.back, t.text)
                edit.cursorPosition = pos - t.back + t.caret
                return accept()
            }
        }
        // Auto-pairing (Preferences ▸ Editor), as in paragraphs.
        if (event.text.length === 1 && none && Prefs.autoPairBrackets && typing) {
            const what = editor.cellPairTyped(edit.text, pos, event.text, Prefs.autoPairMarkdown)
            if (what === "skip") {
                edit.cursorPosition = pos + 1
                return accept()
            }
            if (what !== "") {
                edit.insert(pos, event.text + what)
                edit.cursorPosition = pos + 1
                return accept()
            }
        }
        // Caret keys (Ui.caretKeys) move in the cell, but the document's
        // top and bottom.
        const move = Ui.caretMove(event)
        if (move !== "" && move !== "top" && move !== "bottom") {
            Ui.moveCaret(edit, move, (mods & Qt.ShiftModifier) !== 0, false)
            return accept()
        }
        if (move !== "")
            return owner.keyPressed(edit, event)
        switch (event.key) {
        // Tab goes through the cells, selecting each; past the last one
        // it adds a row.
        case Qt.Key_Tab:
            if (!plain)
                break
            if (c + 1 < columns)
                focusCell(r, c + 1, 0, 1e9)
            else if (r + 1 < rowCount)
                focusCell(r + 1, 0, 0, 1e9)
            else
                run("rowBelow")
            return accept()
        case Qt.Key_Backtab:
            if (c > 0)
                focusCell(r, c - 1, 0, 1e9)
            else if (r > 0)
                focusCell(r - 1, columns - 1, 0, 1e9)
            return accept()
        case Qt.Key_Return:
        case Qt.Key_Enter:
            if (shift) {
                // A line break in a cell is HTML, as Typora writes it.
                edit.remove(edit.selectionStart, edit.selectionEnd)
                edit.insert(edit.cursorPosition, "<br />")
            } else if (ctrl) {
                run("rowBelow")
            } else if (plain) {
                if (r + 1 < rowCount)
                    focusCell(r + 1, c, 0, 1e9)
                else if (host)
                    host.leaveBlock(owner.index)
            }
            return accept()
        case Qt.Key_Backspace:
            if (mods === (Qt.ControlModifier | Qt.ShiftModifier))
                return accept(run("deleteRow") || true)
            if (plain && none && pos > 0 && Prefs.autoPairBrackets && editor.cellPaired(edit.text, pos, Prefs.autoPairMarkdown)) {
                edit.remove(pos - 1, pos + 1)
                return accept()
            }
            return
        case Qt.Key_Up:
        case Qt.Key_Down:
        case Qt.Key_Left:
        case Qt.Key_Right:
            if (alt) {
                run({ [Qt.Key_Up]: "rowUp", [Qt.Key_Down]: "rowDown", [Qt.Key_Left]: "columnLeft", [Qt.Key_Right]: "columnRight" }[event.key])
                return accept()
            }
            if (!plain)
                return
            return accept(arrow(event.key, edit, r, c, pos, len, none, x))
        case Qt.Key_Z:
        case Qt.Key_Y:
            if (!(mods & Qt.ControlModifier))
                return
            break
        case Qt.Key_PageUp:
        case Qt.Key_PageDown:
        case Qt.Key_Menu:
        case Qt.Key_F10:
            break
        default:
            return
        }
        owner.keyPressed(edit, event)
    }

    // Arrows at a cell's edges move to the next cell, and out of the
    // table at its edges. False when the cell keeps the key.
    function arrow(key, edit, r, c, pos, len, none, x) {
        const atTop = edit.positionToRectangle(pos).y <= edit.positionToRectangle(0).y
        const atBottom = edit.positionToRectangle(pos).y >= edit.positionToRectangle(len).y
        const out = (step, where) => host && host.moveTo(host.editableRow(owner.index, step), where, x)
        switch (key) {
        case Qt.Key_Left:
            if (pos > 0 || !none)
                return false
            if (c > 0)
                return focusCell(r, c - 1, 1e9, 1e9)
            if (r > 0)
                return focusCell(r - 1, columns - 1, 1e9, 1e9)
            out(-1, "end")
            return true
        case Qt.Key_Right:
            if (pos < len || !none)
                return false
            if (c + 1 < columns)
                return focusCell(r, c + 1, 0, 0)
            if (r + 1 < rowCount)
                return focusCell(r + 1, 0, 0, 0)
            out(1, "start")
            return true
        case Qt.Key_Up:
            if (!atTop)
                return false
            if (r > 0)
                return enterCell(r - 1, c, "last", x)
            if (!out(-1, "last"))
                edit.cursorPosition = 0
            return true
        case Qt.Key_Down:
            if (!atBottom)
                return false
            if (r + 1 < rowCount)
                return enterCell(r + 1, c, "first", x)
            if (host)
                host.leaveBlock(owner.index)
            return true
        }
        return false
    }

    // The cells, apart, so the copy that follows a drag can show them.
    Item {
        id: grid
        width: table.width
        height: table.height
        // The page behind them while the copy takes them, so the rows the
        // theme leaves unshaded aren't left see-through.
        Rectangle {
            width: parent.width + 1
            height: parent.height
            color: Theme.background
            visible: table.moving !== null
        }
        Repeater {
            id: cells
            model: table.rowCount * table.columns
            onCountChanged: layoutLater.restart()
            delegate: Item {
                id: cell
                required property int index
                readonly property int r: Math.floor(index / table.columns)
                readonly property int c: index % table.columns
                readonly property string value: (table.spec.rows[r] || [])[c] || ""
                // Find's current match is in this cell; and its matches here.
                readonly property bool found: table.owner.found && table.host.found.r === r && table.host.found.c === c
                onFoundChanged: hl.caretMoved(edit.activeFocus || found ? edit.cursorPosition : -1)
                readonly property string hits: table.host !== null && table.host.findHits[table.owner.index + ":" + r + ":" + c] || "[]"
                readonly property Item edit: edit
                // The cell's widest and narrowest layouts: all on one line, and
                // broken at every space.
                readonly property real maxWidth: edit.implicitWidth
                readonly property real minWidth: {
                    let w = 0
                    for (const word of edit.text.split(/\s+/))
                        w = Math.max(w, fm.advanceWidth(word))
                    return w
                }
                onMaxWidthChanged: layoutLater.restart()
                onMinWidthChanged: layoutLater.restart()

                x: table.colX[c] || 0
                y: table.rowY[r] || 0
                width: table.colW[c] || 0
                height: table.rowH[r] || 0

                // The header and every other row are shaded, as in the github
                // theme; borders overlap their neighbours' into single lines.
                Rectangle {
                    width: parent.width + 1
                    height: parent.height + 1
                    color: cell.r === 0 || cell.r % 2 === 0 ? Theme.codeBackground : "transparent"
                    border.color: Theme.border
                    border.width: 1
                }

                FontMetrics {
                    id: fm
                    font: edit.font
                }

                TextEdit {
                    id: edit
                    x: table.padX
                    y: table.padY
                    width: Math.max(0, cell.width - 2 * table.padX)
                    text: cell.value
                    textFormat: TextEdit.PlainText
                    wrapMode: TextEdit.Wrap
                    selectByMouse: true
                    // Kept while a menu has the focus; the next cell clears it.
                    persistentSelection: true
                    color: Theme.foreground
                    selectionColor: Theme.selection
                    selectedTextColor: Theme.foreground
                    font.family: Ui.textFont
                    font.pixelSize: table.em
                    font.bold: cell.r === 0
                    horizontalAlignment: ({ left: TextEdit.AlignLeft, center: TextEdit.AlignHCenter, right: TextEdit.AlignRight })[table.spec.align[cell.c]] || TextEdit.AlignLeft
                    // Typora drops the caret while a row or column is
                    // dragged.
                    cursorDelegate: Caret {
                        hidden: table.moving !== null
                    }
                    Keys.onPressed: event => table.keyPressed(cell, event)
                    // Format changes also emit textChanged; only a change of
                    // text is an edit, and the model's own text isn't one.
                    property string seen
                    onTextChanged: {
                        if (text === seen)
                            return
                        seen = text
                        hl.fixLineHeight()
                        if (text !== cell.value)
                            table.editor.cellEdited(table.owner.index, cell.r, cell.c, text)
                        if (table.host && activeFocus)
                            Qt.callLater(() => table.host.emojiSuggest.typed(edit))
                    }
                    onHeightChanged: rowsLater.restart()
                    onCursorPositionChanged: {
                        hl.caretMoved(activeFocus || cell.found ? cursorPosition : -1)
                        if (table.host) {
                            table.host.emojiSuggest.caretMoved(edit)
                            table.host.caretMoved()
                        }
                    }
                    onActiveFocusChanged: {
                        hl.caretMoved(activeFocus || cell.found ? cursorPosition : -1)
                        if (table.host)
                            table.host.emojiSuggest.caretMoved(edit)
                        if (activeFocus) {
                            const old = table.cellItem(table.curR, table.curC)
                            if (old && old !== cell)
                                old.edit.deselect()
                            table.curR = cell.r
                            table.curC = cell.c
                            table.owner.focused(table.owner.index)
                        }
                    }
                    Component.onCompleted: {
                        hl.dim(hl.dimmed, -1)
                        hl.attach(edit.textDocument)
                        hl.fixLineHeight()
                    }
                }
                InlineHighlighter {
                    id: hl
                    // Focus mode: only the caret's row stays, as Typora's
                    // focused `tr`.
                    readonly property bool dimmed: Ui.focusMode && (table.owner.dimmed || cell.r !== table.curR)
                    onDimmedChanged: dim(dimmed, -1)
                    lineHeight: table.lineHeight
                    fontSize: table.em
                    doc: table.editor.docId
                    onLineHeightChanged: fixLineHeight()
                    onFontSizeChanged: refresh()
                    onPlacingChanged: Qt.callLater(() => hl.fixLineHeight())
                    onFormatted: placeLater.restart()
                    readonly property string inputs: Theme.revision + "/" + table.editor.inlineRevision
                    onInputsChanged: refresh()
                    hits: cell.hits
                    onHitsChanged: refresh()
                }
                // Inline images and formulas, as in paragraphs.
                property int placeRevision: 0
                Timer {
                    id: placeLater
                    interval: 0
                    onTriggered: cell.placeRevision++
                }
                Connections {
                    target: edit
                    function onWidthChanged() { placeLater.restart() }
                    function onContentHeightChanged() { placeLater.restart() }
                }
                readonly property var objects: JSON.parse(hl.objects)
                Repeater {
                    model: cell.objects.length
                    InlineObject {
                        info: cell.objects[index]
                        highlighter: hl
                        textEdit: edit
                        editor: table.editor
                        em: table.em
                        xHeight: fm.xHeight
                        color: edit.color
                        scale: table.scale
                        revision: cell.placeRevision
                        opacity: hl.dimmed ? 0.3 : 1
                        onPictureLoaded: if (edit.activeFocus && table.host) table.host.imageLoaded(edit)
                    }
                }
            }
        }
    }

    // A dragged row or column leaves a gap: its cells shaded, their text
    // hidden, as Typora's `typora-on-moving`.
    Repeater {
        model: table.moving === null ? 0 : table.movingRow ? table.columns : table.rowCount
        Rectangle {
            required property int index
            readonly property int r: table.movingRow ? table.moving.from : index
            readonly property int c: table.movingRow ? index : table.moving.from
            x: table.colX[c] || 0
            y: table.rowY[r] || 0
            width: (table.colW[c] || 0) + 1
            height: (table.rowH[r] || 0) + 1
            color: Qt.tint(Theme.background, Qt.alpha(Theme.muted, 0.5))
            border.color: Qt.tint(Theme.background, Qt.alpha(Theme.border, 0.5))
        }
    }

    // The copy that follows the pointer: a row up and down, a column
    // across.
    Rectangle {
        id: ghost
        visible: table.moving !== null
        readonly property int k: visible ? table.moving.from : 0
        x: table.movingColumn ? table.moving.x - table.moving.grabX : 0
        y: table.movingRow ? table.moving.y - table.moving.grabY : 0
        width: table.movingRow ? table.width : (table.colW[k] || 0) + 1
        height: table.movingRow ? (table.rowH[k] || 0) + 1 : table.height
        color: Theme.background
        ShaderEffectSource {
            anchors.fill: parent
            sourceItem: ghost.visible ? grid : null
            sourceRect: Qt.rect(table.movingRow ? 0 : table.colX[ghost.k] || 0, table.movingRow ? table.rowY[ghost.k] || 0 : 0, ghost.width, ghost.height)
        }
    }

    // Where the drag would put it: arrows at both ends of the line between
    // rows or columns.
    component Arrowhead: Shape {
        id: head
        property string points
        property real size: 1
        width: 10 * size
        height: 10 * size
        ShapePath {
            strokeColor: "transparent"
            fillColor: Theme.foreground
            scale: Qt.size(head.size, head.size)
            PathSvg { path: head.points }
        }
    }
    Item {
        id: marker
        visible: table.marking
        readonly property int to: visible ? table.moving.to : 0
        readonly property real line: !visible ? 0
            : table.movingRow ? (to >= table.rowCount ? table.height - 1 : table.rowY[to])
                              : (to >= table.columns ? table.width - 1 : table.colX[to])
        Arrowhead {
            size: table.scale
            x: table.movingRow ? -10 * table.scale : marker.line - 5 * table.scale
            y: table.movingRow ? marker.line - 5 * table.scale : -10 * table.scale
            points: table.movingRow ? "M1 0 L7 5 L1 10 Z" : "M0 1 L10 1 L5 7 Z"
        }
        Arrowhead {
            size: table.scale
            x: table.movingRow ? table.width : marker.line - 5 * table.scale
            y: table.movingRow ? marker.line - 5 * table.scale : table.height
            points: table.movingRow ? "M9 0 L3 5 L9 10 Z" : "M0 9 L10 9 L5 3 Z"
        }
    }

    // The toolbar above the table while it has the caret: Resize Table and
    // the column's alignment on the left, More Actions and Delete Table on
    // the right. Clicks reach it through the view (`controlAt`).
    function controlAt(x, y) {
        if (!bar.visible)
            return null
        const p = mapToItem(bar, x, y)
        const hit = bar.childAt(p.x, p.y)
        if (hit === left || hit === right) {
            const q = hit.mapFromItem(bar, p.x, p.y)
            const button = hit.childAt(q.x, q.y)
            return button && button.activate ? button : null
        }
        return null
    }


    // A number to type in the Resize Table board.
    component SizeField: Rectangle {
        id: field
        property real scale: 1
        property alias text: input.text
        signal accepted
        width: 30 * scale
        height: 20 * scale
        color: "transparent"
        border.color: input.activeFocus ? Theme.accent : Theme.border
        radius: 2
        TextInput {
            id: input
            anchors.fill: parent
            anchors.margins: 3 * field.scale
            horizontalAlignment: TextInput.AlignHCenter
            verticalAlignment: TextInput.AlignVCenter
            validator: IntValidator { bottom: 1; top: 99 }
            color: Theme.foreground
            font.family: Ui.textFont
            font.pixelSize: 12 * field.scale
            selectByMouse: true
            onAccepted: field.accepted()
        }
    }

    // A toolbar button by its glyph, and a square of the Resize Table
    // board (column, row from 1), for tests.
    function button(name) {
        for (const row of [left, right])
            for (let i = 0; i < row.children.length; i++)
                if (row.children[i].glyph === name)
                    return row.children[i]
        return null
    }
    function square(c, r) {
        return squares.itemAt((r - 1) * 6 + (c - 1))
    }
    // Window points for tests: the middle of cell (r, c), and of row or
    // column k's handle.
    function cellPoint(r, c) {
        return mapToItem(null, colX[c] + colW[c] / 2, rowY[r] + rowH[r] / 2)
    }
    function handlePoint(kind, k) {
        return kind === "row" ? mapToItem(null, -3 * scale, rowY[k] + rowH[k] / 2)
                              : mapToItem(null, colX[k] + colW[k] / 2, -2 * scale)
    }

    component BarButton: Rectangle {
        id: button
        property real scale: 1
        property string glyph
        property bool on: false
        property string tip
        signal clicked
        function activate() { clicked() }
        width: 24 * scale
        height: 20 * scale
        radius: 2 * scale
        color: on ? Qt.alpha(Theme.accent, 0.25) : "transparent"
        border.color: on ? Qt.alpha(Theme.accent, 0.6) : "transparent"
        Glyph {
            anchors.centerIn: parent
            name: button.glyph
            size: 13 * button.scale
            color: button.on ? Theme.accent : Theme.muted
        }
    }

    Item {
        id: bar
        // Typora takes the toolbar away while a column is dragged.
        visible: table.active && !table.movingColumn
        x: -4 * table.scale
        y: -height - 2 * table.scale
        width: table.width + 8 * table.scale
        height: 20 * table.scale
        Row {
            id: left
            spacing: 2 * table.scale
            BarButton {
                id: resizeButton
                scale: table.scale
                glyph: "menu"
                tip: qsTr("Resize Table")
                onClicked: sizer.open()
            }
            Item { width: 4 * table.scale; height: 1 }
            BarButton {
                scale: table.scale
                glyph: "alignLeft"
                tip: qsTr("Align Left")
                on: table.currentAlign === "left"
                onClicked: table.run("alignLeft", table.curC, table.curC)
            }
            BarButton {
                scale: table.scale
                glyph: "alignCenter"
                tip: qsTr("Align Center")
                on: table.currentAlign === "center"
                onClicked: table.run("alignCenter", table.curC, table.curC)
            }
            BarButton {
                scale: table.scale
                glyph: "alignRight"
                tip: qsTr("Align Right")
                on: table.currentAlign === "right"
                onClicked: table.run("alignRight", table.curC, table.curC)
            }
        }
        Row {
            id: right
            anchors.right: parent.right
            spacing: 2 * table.scale
            BarButton {
                id: moreButton
                scale: table.scale
                glyph: "more"
                tip: qsTr("More Actions")
                onClicked: more.popup(moreButton, 0, moreButton.height)
            }
            BarButton {
                scale: table.scale
                glyph: "trash"
                tip: qsTr("Delete Table")
                onClicked: table.run("deleteTable")
            }
        }
    }

    // Resize Table: a board of 6 by 10 squares, the table's size shaded,
    // and the size as numbers to type; hovering picks a size, a click or
    // OK applies it. Typora caps tables at 40 columns and 99 rows.
    Popup {
        id: sizer
        parent: resizeButton
        y: resizeButton.height + 4 * table.scale
        padding: 8 * table.scale
        property int hoverC: 0
        property int hoverR: 0
        onAboutToShow: {
            hoverC = 0
            hoverR = 0
            widthInput.text = String(table.columns)
            heightInput.text = String(table.rowCount)
        }
        onClosed: Qt.callLater(() => table.focusCell(table.curR, table.curC, 1e9, 1e9))
        function apply(columns, rows) {
            close()
            table.run("resize", Math.max(1, Math.min(40, columns)), Math.max(1, Math.min(99, rows)))
        }
        background: Rectangle {
            color: Theme.panel
            border.color: Theme.border
            radius: 4
        }
        contentItem: Column {
            spacing: 8 * table.scale
            Grid {
                id: board
                columns: 6
                spacing: 2 * table.scale
                anchors.horizontalCenter: parent.horizontalCenter
                Repeater {
                    id: squares
                    model: 60
                    Rectangle {
                        required property int index
                        readonly property int c: index % 6 + 1
                        readonly property int r: Math.floor(index / 6) + 1
                        readonly property bool hovered: c <= sizer.hoverC && r <= sizer.hoverR
                        readonly property bool inside: c <= table.columns && r <= table.rowCount
                        width: 13 * table.scale
                        height: 13 * table.scale
                        color: hovered ? Qt.alpha(Theme.accent, r === 1 ? 0.7 : 0.45)
                                       : inside ? (r === 1 ? Theme.muted : Theme.faint) : "transparent"
                        border.color: Theme.border
                        MouseArea {
                            anchors.fill: parent
                            hoverEnabled: true
                            onEntered: {
                                sizer.hoverC = parent.c
                                sizer.hoverR = parent.r
                                widthInput.text = String(parent.c)
                                heightInput.text = String(parent.r)
                            }
                            onClicked: sizer.apply(parent.c, parent.r)
                        }
                    }
                }
            }
            Row {
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: 4 * table.scale
                SizeField { id: widthInput; scale: table.scale; onAccepted: sizer.apply(Number(widthInput.text), Number(heightInput.text)) }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "x"
                    color: Theme.muted
                    font.family: Ui.textFont
                    font.pixelSize: 12 * table.scale
                }
                SizeField { id: heightInput; scale: table.scale; onAccepted: sizer.apply(Number(widthInput.text), Number(heightInput.text)) }
                Rectangle {
                    width: ok.implicitWidth + 12 * table.scale
                    height: 20 * table.scale
                    radius: 2
                    color: Theme.accent
                    Text {
                        id: ok
                        anchors.centerIn: parent
                        text: qsTr("OK")
                        color: Theme.accentText
                        font.family: Ui.textFont
                        font.pixelSize: 12 * table.scale
                    }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: sizer.apply(Number(widthInput.text), Number(heightInput.text))
                    }
                }
            }
        }
    }

    // More Actions: the Table menu.
    TMenu {
        id: more
        onClosed: Qt.callLater(() => { if (!table.focused) table.focusCell(table.curR, table.curC, 1e9, 1e9) })
        TMenuItem { text: qsTr("Add Row Above"); onTriggered: table.run("rowAbove") }
        TMenuItem { text: qsTr("Add Row Below"); hint: "Ctrl+Enter"; onTriggered: table.run("rowBelow") }
        TMenuItem { text: qsTr("Add Column Before"); onTriggered: table.run("columnBefore") }
        TMenuItem { text: qsTr("Add Column After"); onTriggered: table.run("columnAfter") }
        TSep {}
        TMenuItem { text: qsTr("Move Row Up"); hint: "Alt+↑"; onTriggered: table.run("rowUp") }
        TMenuItem { text: qsTr("Move Row Down"); hint: "Alt+↓"; onTriggered: table.run("rowDown") }
        TMenuItem { text: qsTr("Move Column Left"); hint: "Alt+←"; onTriggered: table.run("columnLeft") }
        TMenuItem { text: qsTr("Move Column Right"); hint: "Alt+→"; onTriggered: table.run("columnRight") }
        TSep {}
        TMenuItem { text: qsTr("Delete Row"); hint: "Ctrl+Shift+Backspace"; onTriggered: table.run("deleteRow") }
        TMenuItem { text: qsTr("Delete Column"); onTriggered: table.run("deleteColumn") }
        TSep {}
        TMenuItem { text: qsTr("Copy Table"); onTriggered: table.copyTable() }
        TMenuItem { text: qsTr("Prettify Source Code"); onTriggered: table.run("prettify") }
        TSep {}
        TMenuItem { text: qsTr("Delete Table"); onTriggered: table.run("deleteTable") }
    }
}
