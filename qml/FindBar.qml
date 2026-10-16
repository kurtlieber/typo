import QtQuick
import QtQuick.Controls
import Typo

// Find and Replace (Ctrl+F, Ctrl+H), as Typora's search panel: a bar over
// the top of the page with the text to find, its case, whole word and
// regular expression toggles, previous and next, where the current match
// is ("3 / 12"), and for replacing a second row with the replacement and
// the Replace and All buttons. Every match is tinted where it is; the
// current one is selected in its block while the focus stays here, and
// stays selected when the bar closes.
Rectangle {
    id: bar

    required property var app
    required property var view
    // Source Code Mode's view: there the whole markdown is one row.
    property var source: null
    readonly property bool inSource: Ui.sourceMode && source !== null
    readonly property var editor: view.editor

    property bool replacing: false
    property bool caseSensitive: false
    property bool wholeWord: false
    property bool regex: false
    // The last search's text and what it found, [row, r, c, start, end]
    // each in document order, and which is current (-1 before one is).
    property string searched: ""
    property var hits: []
    property int current: -1
    // Where the next match is looked for from: { row, cell, pos }, cell
    // r * 4096 + c in a table, else -1.
    property var origin: null
    // "Replaced 3 occurrence(s)", for a few seconds after All.
    property string message: ""
    // Set while the bar fills the box itself, which isn't typing.
    property bool seeding: false

    readonly property alias text: input.text
    readonly property alias replacement: replaceInput.text
    readonly property int count: hits.length

    visible: false
    height: 5 + 30 + (replacing ? 30 : 0) + (message !== "" ? 32 : 0) + 4
    color: Theme.background

    // The bar's shadow on the page under it.
    Rectangle {
        y: parent.height
        width: parent.width
        height: 6
        gradient: Gradient {
            GradientStop { position: 0; color: Qt.alpha(Theme.foreground, 0.12) }
            GradientStop { position: 1; color: "transparent" }
        }
    }
    Rectangle {
        y: parent.height - 1
        width: parent.width
        height: 1
        color: Theme.border
    }
    // Clicks on the bar stay on it.
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
    }

    function show(replace) {
        if (!visible) {
            origin = here()
            const seed = selectedText()
            seeding = true
            if (seed !== "")
                input.text = seed
            seeding = false
            visible = true
            message = ""
            if (input.text !== "")
                search(false)
        }
        replacing = replace
        input.forceActiveFocus()
        input.selectAll()
    }

    function close() {
        if (!visible)
            return
        visible = false
        message = ""
        searchLater.stop()
        hits = []
        current = -1
        paint()
        if (inSource)
            source.edit.forceActiveFocus()
        else if (!view.takeFound()) {
            const edit = view.editAt(Math.max(0, view.currentRow))
            if (edit)
                edit.forceActiveFocus()
            else
                view.forceActiveFocus()
        }
    }

    // The caret, or the start of the selection.
    function here() {
        if (inSource)
            return { row: 0, cell: -1, pos: source.edit.selectionStart }
        const s = view.selection()
        const first = s.aRow < s.zRow || (s.aRow === s.zRow && s.aPos <= s.zPos)
        return { row: first ? s.aRow : s.zRow, cell: -1, pos: first ? s.aPos : s.zPos }
    }

    // The selected text when it is in one block, its first line.
    function selectedText() {
        if (inSource)
            return source.edit.selectedText.split(/[\n\u2029]/)[0]
        if (view.span)
            return ""
        const item = view.itemAt(Math.max(0, view.currentRow))
        const edit = item ? item.editItem : null
        return edit ? edit.selectedText.split(/[\n\u2029]/)[0] : ""
    }

    // Search for the box's text. `select` makes the first match from the
    // caret current, as typing does; opening the bar only counts them.
    function search(select) {
        if (current >= 0) {
            const h = hits[current]
            origin = { row: h[0], cell: cellOf(h), pos: h[3] }
        }
        searched = input.text
        current = -1
        hits = searched === "" ? [] : findHits()
        paint()
        if (select && hits.length > 0)
            next(false)
        else
            showFound(null)
    }

    // Every match of the searched text: in the document, or in Source Code
    // Mode's text.
    function findHits() {
        const found = inSource ? editor.findInText(source.edit.text, searched, caseSensitive, wholeWord, regex)
                               : editor.findAll(searched, caseSensitive, wholeWord, regex)
        return JSON.parse(found).hits
    }

    function showFound(hit) {
        if (inSource)
            source.showFound(hit)
        else
            view.showFound(hit)
    }

    function cellOf(h) {
        return h[1] < 0 ? -1 : h[1] * 4096 + h[2]
    }

    // The first match at or after `o`; with `strictly`, one that starts
    // at or after it rather than one that ends after it.
    function firstFrom(o, strictly) {
        for (let i = 0; i < hits.length; i++) {
            const h = hits[i], cell = cellOf(h)
            if (h[0] > o.row || (h[0] === o.row && (cell > o.cell || (cell === o.cell && (strictly ? h[3] >= o.pos : h[4] > o.pos)))))
                return i
        }
        return 0
    }

    // The next match, or the previous; the first one from the caret when
    // none is current yet.
    function next(back) {
        if (!visible) {
            show(false)
            if (hits.length === 0)
                return
        }
        message = ""
        if (input.text !== searched)
            search(false)
        if (hits.length === 0)
            return
        if (current < 0)
            current = origin ? firstFrom(origin, false) : 0
        else
            current = (current + (back ? hits.length - 1 : 1)) % hits.length
        showFound(hits[current])
    }

    // Replace the current match and go on to the next.
    function replaceOne() {
        if (!replacing)
            return
        message = ""
        if (input.text !== searched)
            search(false)
        if (hits.length === 0)
            return
        if (current < 0) {
            next(false)
            return
        }
        const h = hits[current]
        const end = inSource ? source.replaceMatch(h[3], h[4], searched, replaceInput.text, caseSensitive, wholeWord, regex)
                             : editor.replaceMatch(h[0], h[1], h[2], h[3], h[4], searched, replaceInput.text, caseSensitive, wholeWord, regex)
        if (end < 0) {
            search(true)
            return
        }
        hits = findHits()
        current = hits.length > 0 ? firstFrom({ row: h[0], cell: cellOf(h), pos: end }, true) : -1
        paint()
        showFound(current >= 0 ? hits[current] : null)
    }

    function replaceAll() {
        if (input.text !== searched)
            search(false)
        if (hits.length === 0)
            return
        const n = inSource ? source.replaceAll(searched, replaceInput.text, caseSensitive, wholeWord, regex)
                           : editor.replaceAll(searched, replaceInput.text, caseSensitive, wholeWord, regex)
        current = -1
        showFound(null)
        search(false)
        message = qsTr("Replaced %1 occurrence(s)").arg(n)
        messageTimer.restart()
    }

    function undoReplaceAll() {
        message = ""
        app.run("undo")
        search(false)
        input.forceActiveFocus()
    }

    // Hand the matches to the blocks' highlighters.
    function paint() {
        if (source)
            source.hits = inSource ? JSON.stringify(hits.map(h => [h[3], h[4]])) : "[]"
        if (inSource) {
            view.findHits = {}
            return
        }
        const by = {}
        for (const h of hits) {
            const key = h[1] < 0 ? h[0] : h[0] + ":" + h[1] + ":" + h[2]
            ;(by[key] = by[key] || []).push([h[3], h[4]])
        }
        const out = {}
        for (const key in by)
            out[key] = JSON.stringify(by[key])
        view.findHits = out
    }

    // The focus went to the page while the bar is open: matches are found
    // from the caret there from now on, as from a new search.
    readonly property Item focusItem: Window.activeFocusItem
    onFocusItemChanged: {
        if (!visible || focusItem === null)
            return
        for (let p = focusItem; p; p = p.parent)
            if (p === bar)
                return
        if (!inSource && view.found !== null)
            view.found = null
        current = -1
        // Once the field the focus went to has its selection.
        Qt.callLater(() => bar.origin = bar.here())
    }
    // Escape closes the bar wherever the focus is, as in Typora.
    Shortcut {
        sequence: "Escape"
        enabled: bar.visible
        onActivated: bar.close()
    }

    onCaseSensitiveChanged: if (visible) search(true)
    onWholeWordChanged: if (visible) search(true)
    onRegexChanged: if (visible) search(true)

    Timer {
        id: searchLater
        interval: 50
        onTriggered: bar.search(true)
    }
    Timer {
        id: messageTimer
        interval: 5000
        onTriggered: bar.message = ""
    }
    // The document changed under the bar: find again, keeping the place.
    Connections {
        target: bar.editor
        enabled: bar.visible
        function onContentChanged() { refreshLater.restart() }
    }
    Timer {
        id: refreshLater
        interval: 200
        onTriggered: {
            if (!bar.visible || bar.searched === "")
                return
            const h = bar.current >= 0 ? bar.hits[bar.current] : null
            bar.hits = bar.findHits()
            bar.current = h ? bar.hits.findIndex(x => x[0] === h[0] && x[1] === h[1] && x[2] === h[2] && x[3] === h[3]) : -1
            bar.paint()
            if (bar.current < 0)
                bar.showFound(null)
        }
    }

    // A button in Typora's input-group style: a grey box with a border.
    component Addon: AbstractButton {
        id: addon
        property bool flat: false
        height: 26
        hoverEnabled: true
        focusPolicy: Qt.NoFocus
        font.pixelSize: 12
        leftPadding: 8
        rightPadding: 8
        background: Rectangle {
            color: addon.flat ? "transparent" : addon.down || addon.hovered ? Qt.tint(Theme.background, Qt.alpha(Theme.foreground, 0.14))
                                                                            : Qt.tint(Theme.background, Qt.alpha(Theme.foreground, 0.07))
            border.color: addon.flat ? "transparent" : Theme.border
        }
    }
    component AddonText: Text {
        color: Theme.muted
        font.pixelSize: 12
        verticalAlignment: Text.AlignVCenter
        horizontalAlignment: Text.AlignHCenter
    }
    // Case, whole word and regular expression, at the input's right edge.
    component Toggle: AbstractButton {
        id: toggle
        required property string label
        required property string hint
        property bool on: false
        width: 22
        height: 16
        hoverEnabled: true
        focusPolicy: Qt.NoFocus
        ToolTip.visible: hovered
        ToolTip.delay: 600
        ToolTip.text: hint
        background: Rectangle {
            radius: 3
            color: toggle.on ? Theme.selection : "transparent"
            border.color: Theme.border
        }
        contentItem: Text {
            text: toggle.label
            font.pixelSize: 11
            font.bold: toggle.label === "Aa"
            color: toggle.on ? Theme.selectionText : Theme.muted
            opacity: toggle.on || toggle.hovered ? 1 : 0.8
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }
    component Field: TextField {
        id: field
        height: 26
        font.pixelSize: 12
        color: Theme.foreground
        placeholderTextColor: Theme.faint
        selectionColor: Theme.selection
        selectedTextColor: Theme.selectionText
        leftPadding: 6
        verticalAlignment: TextInput.AlignVCenter
        background: Rectangle {
            color: "transparent"
            border.color: field.activeFocus ? Theme.accent : Theme.border
        }
        Keys.onEscapePressed: bar.close()
    }

    // Labels on the left: "Search ▾" turns replacing on, "Replace ▴" off.
    Addon {
        id: searchLabel
        x: 18
        y: 7
        width: 76
        visible: !bar.replacing
        onClicked: {
            bar.replacing = true
            replaceInput.forceActiveFocus()
        }
        contentItem: Row {
            spacing: 2
            AddonText { text: qsTr("Search"); height: 24 }
            Glyph { name: "caretDown"; size: 10; color: Theme.muted; anchors.verticalCenter: parent.verticalCenter }
        }
    }
    Addon {
        x: 18
        y: 37
        width: 76
        visible: bar.replacing
        onClicked: {
            bar.replacing = false
            bar.message = ""
            input.forceActiveFocus()
        }
        contentItem: Row {
            spacing: 2
            AddonText { text: qsTr("Replace"); height: 24 }
            Glyph { name: "caretUp"; size: 10; color: Theme.muted; anchors.verticalCenter: parent.verticalCenter }
        }
    }

    Field {
        id: input
        objectName: "findInput"
        x: 96
        y: 7
        width: right.x - x - 2
        placeholderText: qsTr("search text")
        rightPadding: toggles.width + 10
        onTextChanged: if (!bar.seeding && bar.visible) searchLater.restart()
        Keys.onPressed: event => {
            if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                searchLater.stop()
                if (text === bar.searched)
                    bar.next(event.modifiers & Qt.ShiftModifier)
                else
                    bar.search(true)
                event.accepted = true
            } else if (event.key === Qt.Key_Tab) {
                if (bar.replacing)
                    replaceInput.forceActiveFocus()
                event.accepted = true
            }
        }
    }
    Row {
        id: toggles
        anchors.right: input.right
        anchors.rightMargin: 5
        anchors.verticalCenter: input.verticalCenter
        spacing: 5
        Toggle {
            label: "Aa"
            hint: qsTr("Case Sensitive")
            on: bar.caseSensitive
            onClicked: bar.caseSensitive = !bar.caseSensitive
        }
        Toggle {
            label: "ab"
            hint: qsTr("Whole Word")
            on: bar.wholeWord
            onClicked: bar.wholeWord = !bar.wholeWord
            Rectangle {
                x: 5
                y: 12
                width: 12
                height: 1
                color: parent.on ? Theme.selectionText : Theme.muted
            }
        }
        Toggle {
            label: ".*"
            hint: qsTr("Regular Expression")
            on: bar.regex
            onClicked: bar.regex = !bar.regex
        }
    }
    Field {
        id: replaceInput
        objectName: "replaceInput"
        visible: bar.replacing
        x: 96
        y: 37
        width: right.x - x - 2
        placeholderText: qsTr("replace with")
        Keys.onPressed: event => {
            if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                bar.replaceOne()
                event.accepted = true
            } else if (event.key === Qt.Key_Tab && !(event.modifiers & Qt.ShiftModifier)) {
                event.accepted = true
            }
        }
    }

    // Previous, next, where we are, close; under them Replace and All.
    Item {
        id: right
        x: bar.width - width - 8
        y: 7
        width: Math.max(prevNext.width, replaceButtons.width)
        height: 56
        Row {
            id: prevNext
            Addon {
                onClicked: bar.next(true)
                contentItem: Glyph { name: "up"; size: 12; color: Theme.muted }
            }
            Addon {
                onClicked: bar.next(false)
                contentItem: Glyph { name: "down"; size: 12; color: Theme.muted }
            }
            Text {
                id: status
                objectName: "findStatus"
                readonly property bool bad: bar.searched !== "" && (bar.count === 0 || bar.count > 5000)
                width: Math.max(80, implicitWidth + 12)
                height: 26
                rightPadding: 12
                horizontalAlignment: Text.AlignRight
                verticalAlignment: Text.AlignVCenter
                font.pixelSize: 12
                color: bad ? Theme.error : Theme.muted
                text: bar.searched === "" ? "" : bar.count > 5000 ? "5000+" : bar.count === 0 ? qsTr("No Results")
                    : (Math.max(0, bar.current) + 1) + " / " + bar.count
                HoverHandler { id: statusHover }
                ToolTip.visible: statusHover.hovered && bar.count > 5000
                ToolTip.text: qsTr("Too many matches")
            }
            Addon {
                flat: true
                leftPadding: 6
                rightPadding: 2
                onClicked: bar.close()
                contentItem: Glyph { name: "close"; size: 12; color: Theme.muted; opacity: 0.8 }
            }
        }
        Row {
            id: replaceButtons
            y: 30
            visible: bar.replacing
            Addon {
                width: 64
                onClicked: bar.replaceAll()
                contentItem: AddonText { text: qsTr("All"); horizontalAlignment: Text.AlignLeft }
            }
            Addon {
                width: 72
                onClicked: bar.replaceOne()
                contentItem: AddonText { text: qsTr("Replace"); horizontalAlignment: Text.AlignRight }
            }
        }
    }

    // After All: how many, with Dismiss and Undo.
    Rectangle {
        visible: bar.message !== ""
        x: 16
        y: 5 + 30 + (bar.replacing ? 30 : 0) + 2
        width: bar.width - 24
        height: 28
        radius: 4
        color: Qt.tint(Theme.background, Qt.alpha(Theme.foreground, 0.07))
        Row {
            x: 15
            height: parent.height
            spacing: 6
            Text { text: "✓"; color: Theme.muted; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter }
            Text { text: bar.message; color: Theme.foreground; font.pixelSize: 13; anchors.verticalCenter: parent.verticalCenter }
        }
        Row {
            anchors.right: parent.right
            anchors.rightMargin: 8
            height: parent.height
            spacing: 8
            Repeater {
                model: [{ label: qsTr("Dismiss"), undo: false }, { label: qsTr("Undo"), undo: true }]
                delegate: Text {
                    required property var modelData
                    anchors.verticalCenter: parent.verticalCenter
                    text: modelData.label
                    color: Theme.muted
                    font.pixelSize: 12
                    font.underline: true
                    TapHandler {
                        onTapped: {
                            if (parent.modelData.undo)
                                bar.undoReplaceAll()
                            else
                                bar.message = ""
                        }
                    }
                }
            }
        }
    }
}
