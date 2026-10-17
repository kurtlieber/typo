import QtQuick
import QtQuick.Controls
import Typo

// Source Code Mode (Ctrl+/): the whole document as markdown in one editor,
// laid out like Typora's CodeMirror view: at most 800 wide and centred,
// the current line tinted and its number in the margin. Edits reach the
// document shortly after typing stops, and always before anything reads
// it (`sync`). A whole visit is one undo step there. Inside the view, undo
// is the text's own, and starts empty on each visit, as in Typora.
Flickable {
    id: source

    required property var app
    readonly property var editor: app.editor
    readonly property alias edit: edit
    readonly property alias menu: menu
    readonly property alias highlighter: hl
    // Find's matches, as `[[start, end], …]` JSON.
    property alias hits: hl.hits

    // The markdown last given to or taken from the document.
    property string synced: ""
    property bool loading: false
    property bool syncing: false

    readonly property real zoom: Ui.zoom
    readonly property real pageWidth: Math.min(width, 800 * zoom)
    readonly property real pageX: (width - pageWidth) / 2
    // The current line's box in the text.
    readonly property rect line: {
        edit.length
        edit.contentHeight
        return hl.lineRect(edit.cursorPosition)
    }

    // Typewriter mode: room for the first and last lines to sit in the
    // middle, as Typora pads its CodeMirror.
    readonly property real typewriterRoom: Ui.typewriter ? Math.max(0, (height - Ui.fontSize * 1.6) / 2) : 0

    contentWidth: width
    contentHeight: edit.y + edit.height + (Ui.typewriter ? typewriterRoom : 60 * zoom)
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    flickableDirection: Flickable.VerticalFlick
    ScrollBar.vertical: ScrollBar {
        policy: ScrollBar.AsNeeded
    }

    // Show the document with the caret at the start of block `row`, its
    // line `y` from the top of the view.
    function load(row, y) {
        loading = true
        synced = editor.sourceText()
        edit.text = synced
        typedLength = edit.length
        loading = false
        hl.fixLineHeight()
        editor.sealHistory()
        const pos = hl.linePosition(editor.rowLine(row))
        edit.cursorPosition = pos
        edit.forceActiveFocus()
        contentY = Math.max(0, Math.min(contentHeight - height, edit.y + hl.lineRect(pos).y - y))
    }

    // Hand the text to the document if it changed.
    function sync() {
        pending.stop()
        if (edit.text === synced)
            return
        synced = edit.text
        syncing = true
        editor.setSource(synced, hl.lineNumberAt(edit.cursorPosition))
        syncing = false
    }

    // Leaving: the block the caret is in, and how far down the view its
    // line is.
    function leave() {
        sync()
        editor.sealHistory()
        return { row: editor.rowOfLine(hl.lineNumberAt(edit.cursorPosition)), y: edit.y + line.y - contentY }
    }

    function place(pos) {
        edit.cursorPosition = pos
        edit.forceActiveFocus()
        reveal()
    }

    // Scroll the caret into view: to the middle in typewriter mode, unless
    // the pointer put it there.
    function reveal() {
        if (Ui.typewriter && !margin.pressed && !pointing.active) {
            centre()
            return
        }
        show()
    }
    function show() {
        const r = edit.cursorRectangle
        const top = edit.y + r.y
        const bottom = top + r.height
        const margin = 40 * zoom
        if (top - margin < contentY)
            contentY = Math.max(0, top - margin)
        else if (bottom + margin > contentY + height)
            contentY = Math.min(Math.max(0, contentHeight - height), bottom + margin - height)
    }

    function centre() {
        const r = edit.cursorRectangle
        contentY = Math.max(0, Math.min(Math.max(0, contentHeight - height), edit.y + r.y + r.height / 2 - height / 2))
    }
    // The caret moved: typing keeps its line in the middle in typewriter
    // mode, and with "Always keep caret in middle of screen", every move
    // does; else it is only kept in sight.
    property int typedLength: -1
    function caretShifted() {
        const typed = edit.length !== typedLength
        typedLength = edit.length
        if (typed || Prefs.caretAlwaysCentred)
            reveal()
        else
            show()
    }
    Connections {
        target: Ui
        function onTypewriterChanged() {
            if (source.visible)
                Qt.callLater(source.reveal)
        }
    }

    // Commands from the menus and shortcuts that act on the text here;
    // the ones for the rendered document (formatting, tables, images) do
    // nothing in this mode.
    function command(cmd, arg) {
        switch (cmd) {
        case "undo": edit.undo(); break
        case "redo": edit.redo(); break
        case "cut":
            if (edit.selectedText === "" && Prefs.lineWiseCopy)
                copyLine(true)
            else
                edit.cut()
            break
        case "copy":
        case "copyPlain":
        case "copyMarkdown":
            if (edit.selectedText === "" && Prefs.lineWiseCopy)
                copyLine(false)
            else
                edit.copy()
            break
        case "paste":
        case "pastePlain":
            edit.paste()
            break
        case "selectAll": edit.selectAll(); break
        case "selectWord": edit.selectWord(); break
        case "selectLine":
        case "selectBlock": {
            const start = hl.lineStartAt(edit.cursorPosition)
            const next = hl.linePosition(hl.lineNumberAt(edit.cursorPosition) + 1)
            edit.select(start, next > start ? next - 1 : edit.length)
            break
        }
        case "jumpTop": place(0); break
        case "jumpBottom": place(edit.length); break
        // An outline heading: its line at the top.
        case "jumpToRow": {
            const pos = hl.linePosition(editor.rowLine(arg))
            edit.cursorPosition = pos
            edit.forceActiveFocus()
            contentY = Math.max(0, Math.min(contentHeight - height, edit.y + hl.lineRect(pos).y - 20 * zoom))
            break
        }
        }
    }

    // An underline typed under a line makes it a header, which its own
    // colouring can't foresee. Colouring it again changes the text as far
    // as the TextEdit can tell, so that change is let pass.
    property bool restyling: false
    function restyleAbove() {
        restyling = true
        hl.restyleAbove(edit.cursorPosition)
        restyling = false
    }

    // An edit from the highlighter: `[from, to, text, anchor, caret]`.
    function applyEdit(e) {
        hl.replaceText(e[0], e[1], e[2], false)
        if (e[3] === e[4])
            edit.cursorPosition = e[4]
        else
            edit.select(e[3], e[4])
    }

    // Typing a bracket or quote, as CodeMirror's closebrackets does it:
    // the pair around the selection, or both with the caret between, or
    // over the closer already there. False to type it as it is.
    function typePair(ch) {
        const at = edit.cursorPosition
        const s = edit.selectionStart, e = edit.selectionEnd
        const [what, left, right] = JSON.parse(hl.pairFor(ch, edit.getText(at - 1, at), edit.getText(at, at + 1), s !== e))
        if (what === "skip")
            edit.cursorPosition = at + 1
        else if (what === "both")
            applyEdit([s, e, left + right, s + 1, s + 1])
        else if (what === "wrap")
            applyEdit([s, e, left + edit.getText(s, e) + right, at === s ? e + 1 : s + 1, at === s ? s + 1 : e + 1])
        return what !== "type"
    }

    // Preferences ▸ Editor: with nothing selected, copy or cut the caret's
    // line, with its line break (the one before it for the last line).
    function copyLine(cutting) {
        const text = edit.text
        const at = edit.cursorPosition
        const a = text.lastIndexOf("\n", at - 1) + 1
        const end = text.indexOf("\n", at)
        const z = end < 0 ? text.length : end
        Desktop.setClipboard(text.substring(a, z) + "\n", "", "")
        if (cutting)
            edit.remove(end < 0 ? Math.max(0, a - 1) : a, end < 0 ? z : z + 1)
    }

    // Find: the match selected, while the focus stays in the bar.
    function showFound(hit) {
        if (!hit)
            return
        edit.select(hit[3], hit[4])
        reveal()
    }
    // Replace the match from `start` to `end`: where the replacement ends,
    // or -1 when the text there no longer matches.
    function replaceMatch(start, end, query, replacement, caseSensitive, wholeWord, regex) {
        const r = JSON.parse(editor.replaceInText(edit.text, start, end, query, replacement, caseSensitive, wholeWord, regex))
        if (r === null)
            return -1
        hl.replaceText(start, end, r, false)
        return start + r.length
    }
    // Replace every match as one undo step; how many there were.
    function replaceAll(query, replacement, caseSensitive, wholeWord, regex) {
        const all = JSON.parse(editor.replaceAllInText(edit.text, query, replacement, caseSensitive, wholeWord, regex))
        for (let i = all.length - 1; i >= 0; i--)
            hl.replaceText(all[i][0], all[i][1], all[i][2], i < all.length - 1)
        return all.length
    }

    Timer {
        id: pending
        interval: 300
        onTriggered: source.sync()
    }

    // The document changed elsewhere (reloaded from disk): show it, the
    // caret on the same line.
    Connections {
        target: source.editor
        enabled: source.visible
        function onContentChanged() {
            if (source.syncing || source.loading)
                return
            const text = source.editor.sourceText()
            if (text === source.synced)
                return
            const at = hl.lineNumberAt(edit.cursorPosition)
            source.loading = true
            source.synced = text
            edit.text = text
            source.typedLength = edit.length
            source.loading = false
            hl.fixLineHeight()
            edit.cursorPosition = hl.linePosition(at)
        }
    }

    // Clicks beside the text place the caret on the nearest line. A right
    // click, on the text too (which leaves it to this), opens the menu
    // and leaves the caret and selection be, as Typora's does.
    MouseArea {
        id: margin
        width: source.contentWidth
        height: Math.max(source.contentHeight, source.height)
        cursorShape: Qt.IBeamCursor
        acceptedButtons: Qt.LeftButton | Qt.RightButton
        onPressed: mouse => {
            edit.forceActiveFocus()
            if (mouse.button === Qt.RightButton)
                menu.popup()
            else
                edit.cursorPosition = edit.positionAt(mouse.x - edit.x, mouse.y - edit.y)
        }
    }

    // Typora's menu here has only the clipboard.
    TMenu {
        id: menu
        TMenuItem { text: qsTr("Cut"); hint: "Ctrl+X"; onTriggered: source.command("cut") }
        TMenuItem { text: qsTr("Copy"); hint: "Ctrl+C"; onTriggered: source.command("copy") }
        TMenuItem { text: qsTr("Paste"); hint: "Ctrl+V"; onTriggered: source.command("paste") }
    }

    // The current line, across the page.
    Rectangle {
        x: source.pageX
        y: edit.y + source.line.y
        width: source.pageWidth
        height: source.line.height
        color: Qt.alpha(Theme.foreground, 0.06)
    }
    Text {
        id: number
        text: hl.lineNumberAt(edit.cursorPosition) + 1
        x: edit.x - 2 * digit.zero - width
        y: edit.y + source.line.y + 0.1 * Ui.fontSize
        height: font.pixelSize * 1.6
        verticalAlignment: Text.AlignVCenter
        font.family: Ui.monoFont
        font.pixelSize: 0.8 * Ui.fontSize
        color: Theme.muted
        opacity: 0.6
        FontMetrics {
            id: digit
            font: number.font
            readonly property real zero: advanceWidth("0")
        }
    }
    // Typora numbers the first and last lines and every tenth too: the
    // ones in sight, as `{ n, y }`.
    property var numbered: []
    function numberLines() {
        const last = hl.lineNumberAt(edit.length)
        const top = hl.lineNumberAt(edit.positionAt(0, contentY - edit.y))
        const bottom = hl.lineNumberAt(edit.positionAt(0, contentY + height - edit.y))
        const out = []
        const add = n => {
            if (n >= top && n <= bottom)
                out.push({ n: n, y: hl.lineRect(hl.linePosition(n)).y })
        }
        add(0)
        for (let n = top + (19 - top % 10) % 10; n <= bottom && n < last; n += 10)
            add(n)
        if (last > 0)
            add(last)
        numbered = out
    }
    readonly property var numbering: [edit.text, edit.contentHeight, edit.width, contentY, height]
    onNumberingChanged: Qt.callLater(numberLines)
    Repeater {
        model: source.numbered
        Text {
            required property var modelData
            // The caret's line has its own.
            visible: modelData.n !== hl.keep
            text: modelData.n + 1
            x: edit.x - 2 * digit.zero - width
            y: edit.y + modelData.y + 0.1 * Ui.fontSize
            height: number.height
            verticalAlignment: Text.AlignVCenter
            font: number.font
            color: number.color
            opacity: number.opacity
        }
    }

    TextEdit {
        id: edit
        readOnly: source.editor.readonly
        x: source.pageX + 60 * source.zoom
        y: Ui.typewriter ? source.typewriterRoom : 30 * source.zoom
        width: source.pageWidth - 90 * source.zoom
        textFormat: TextEdit.PlainText
        wrapMode: TextEdit.Wrap
        font.family: Ui.textFont
        font.pixelSize: Ui.fontSize
        color: Theme.foreground
        selectionColor: Theme.selection
        selectedTextColor: Theme.selectionText
        selectByMouse: true
        persistentSelection: true
        cursorDelegate: Caret {
            width: 3
        }
        onTextChanged: {
            if (source.loading || source.restyling)
                return
            pending.restart()
            Qt.callLater(source.restyleAbove)
        }
        // Keys as Typora's CodeMirror takes them: Enter goes on with a list
        // or quote, Tab indents a selection, Shift+Tab gives lines the
        // indent above, and brackets and quotes pair (Preferences ▸
        // Markdown). With nothing selected the field would copy nothing:
        // the line, as Preferences ▸ Editor asks. Caret keys are
        // Ui.caretKeys; a line is a block.
        Keys.onPressed: event => {
            const plain = !(event.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier))
            const shift = event.modifiers & Qt.ShiftModifier
            const s = selectionStart, e = selectionEnd
            const move = Ui.caretMove(event)
            if (move === "centre" || move === "pageUp" || move === "pageDown" || (move === "home" || move === "end") && !shift) {
                move === "centre" ? source.centre() : Ui.scrollMove(source, move)
                event.accepted = true
            } else if (move === "caretPageUp" || move === "caretPageDown") {
                const r = cursorRectangle
                const p = positionAt(r.x, r.y + r.height / 2 + (move === "caretPageUp" ? -0.9 : 0.9) * source.height)
                shift ? moveCursorSelection(p, TextEdit.SelectCharacters) : cursorPosition = p
                event.accepted = true
            } else if (move !== "") {
                Ui.moveCaret(edit, move === "home" ? "top" : move === "end" ? "bottom" : move, shift, true)
                event.accepted = true
            } else if (plain && (event.key === Qt.Key_Return || event.key === Qt.Key_Enter)) {
                if (shift)
                    source.applyEdit([s, e, "\n", s + 1, s + 1])
                else
                    source.applyEdit(JSON.parse(hl.enterAt(s, e, Prefs.orderedSame, Prefs.autoPairBrackets, Prefs.codeIndentSize)))
                event.accepted = true
            } else if (plain && (event.key === Qt.Key_Backtab || event.key === Qt.Key_Tab && (shift || s !== e))) {
                const anchor = cursorPosition === s ? e : s
                source.applyEdit(JSON.parse(hl.indentAt(anchor, cursorPosition, event.key === Qt.Key_Tab && !shift, Prefs.codeIndentSize)))
                event.accepted = true
            } else if (plain && Prefs.autoPairBrackets && event.text.length === 1 && "()[]{}'\"`".indexOf(event.text) >= 0) {
                event.accepted = source.typePair(event.text)
            } else if (plain && !shift && event.key === Qt.Key_Backspace && Prefs.autoPairBrackets && s === e
                       && hl.pairAround(getText(s - 1, s), getText(s, s + 1))) {
                source.applyEdit([s - 1, s + 1, "", s - 1, s - 1])
                event.accepted = true
            } else if (selectedText === "" && Prefs.lineWiseCopy && (event.matches(StandardKey.Copy) || event.matches(StandardKey.Cut))) {
                source.copyLine(event.matches(StandardKey.Cut))
                event.accepted = true
            }
        }
        onCursorRectangleChanged: if (activeFocus) source.caretShifted()
        // Whether the pointer is placing the caret; a click doesn't
        // scroll the line under it away.
        PointHandler {
            id: pointing
            acceptedButtons: Qt.LeftButton
            onActiveChanged: if (!active && Ui.typewriter && Prefs.caretAlwaysCentred) source.centre()
        }
    }

    SourceHighlighter {
        id: hl
        fontSize: Ui.fontSize
        doc: editor.docId
        Component.onCompleted: {
            dim(Ui.focusMode, keep)
            attach(edit.textDocument)
        }
        // Focus mode greys every line but the caret's.
        readonly property int keep: lineNumberAt(edit.cursorPosition)
        readonly property var dimming: [Ui.focusMode, keep]
        onDimmingChanged: dim(Ui.focusMode, keep)
        onFontSizeChanged: {
            refresh()
            fixLineHeight()
        }
    }
    Connections {
        target: Theme
        function onRevisionChanged() { hl.refresh() }
    }
    // Reference definitions or inline syntax preferences changed.
    Connections {
        target: source.editor
        enabled: source.visible
        function onInlineRevisionChanged() { hl.refresh() }
    }
    onHitsChanged: hl.refresh()
}
