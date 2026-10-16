import QtQuick
import Typo

// Emoji suggestions while a shortcode is typed, as Typora's: `:` and a
// word just before the caret list the emoji whose shortcodes match, under
// the caret, five rows high and scrolling. Nothing is chosen until Down
// (or Up, Page Up/Down, Home, End) picks a row; Enter or a click writes
// `:shortcode:` over what was typed, Escape closes the list.
Item {
    id: suggest

    // The EditorView it belongs to, in whose content it is placed.
    required property Item view
    // The row being typed in and the UTF-16 range the choice replaces:
    // from the `:` to the end of the word (and colons) after the caret.
    property int row: -1
    property int start: 0
    property int end: 0
    // [[shortcode, emoji]] matching what was typed, and the chosen one.
    property var matches: []
    property int index: -1
    // The text's length when last looked at.
    property int length: 0

    readonly property bool shown: row >= 0 && matches.length > 0
    readonly property real em: Ui.fontSize * 0.9
    readonly property real rowHeight: Math.round(em * 2 + 2)
    readonly property real pad: 6
    readonly property alias list: list

    visible: shown
    z: 10
    width: Math.max(160 * Ui.zoom, iconWidth + widest.advanceWidth + 10 * Ui.zoom)
    height: Math.min(5, matches.length) * rowHeight + 2 * pad
    readonly property real iconWidth: 38 * Ui.zoom

    TextMetrics {
        id: widest
        font.family: Ui.textFont
        font.pixelSize: suggest.em * 0.9
        text: {
            let w = ""
            for (const m of suggest.matches)
                if (m[0].length > w.length)
                    w = m[0]
            return ":" + w + ":"
        }
    }

    // The text of `row` changed: list what the word before the caret
    // matches, or close.
    function typed(row, edit) {
        if (!Prefs.emojiAutocomplete || !edit.activeFocus || edit.selectedText !== "")
            return close()
        const pos = edit.cursorPosition
        const word = /:(\w+)$/.exec(edit.text.substring(0, pos))
        const after = /^(\w*:*)(\s|$)/.exec(edit.text.substring(pos))
        if (!word || !after)
            return close()
        const start = pos - word[1].length - 1
        length = edit.length
        const token = (word[1] + after[1]).replace(/:+$/, "")
        matches = JSON.parse(view.editor.emojiSuggestions(token))
        if (row !== suggest.row || start !== suggest.start || !shown) {
            index = -1
            suggest.row = row
            suggest.start = start
            suggest.end = pos + after[1].length
            place(edit, pos)
        } else {
            suggest.end = pos + after[1].length
            index = matches.length > 0 ? Math.min(index, 0) : -1
        }
        list.positionViewAtBeginning()
    }
    // The caret moved without typing: the list stays while it is in the
    // word, as Typora's does.
    function caretMoved(row, edit) {
        // Typing moves the caret too, before `typed` catches up.
        if (row === suggest.row && edit.length !== length)
            return
        if (shown && (row !== suggest.row || !edit.activeFocus || edit.cursorPosition <= start || edit.cursorPosition > end))
            close()
    }
    function close() {
        row = -1
        matches = []
        index = -1
    }

    // Under the caret, 15px to its left, kept in the view; above it when
    // there is no room below.
    function place(edit, pos) {
        const r = edit.positionToRectangle(pos)
        const p = edit.mapToItem(suggest.parent, r.x, r.y)
        x = Math.max(view.contentX, Math.min(view.contentX + view.width - width - 4, p.x - 15 * Ui.zoom))
        const below = p.y + r.height
        y = below + height > view.contentY + view.height && p.y - height > view.contentY ? p.y - height : below
    }

    // Keys while the list is shown; true when the list took the key.
    function keyPressed(event) {
        const n = matches.length
        const nav = [Qt.Key_Down, Qt.Key_Up, Qt.Key_PageDown, Qt.Key_PageUp, Qt.Key_Home, Qt.Key_End].indexOf(event.key) >= 0
        if (event.modifiers & Qt.ShiftModifier && nav) {
            close()
            return false
        }
        if (event.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier))
            return false
        const pick = i => {
            index = i
            list.positionViewAtIndex(i, ListView.Contain)
            return true
        }
        switch (event.key) {
        case Qt.Key_Down:
            return pick(index < 0 ? 0 : Math.min(index + 1, n - 1))
        case Qt.Key_Up:
            return index >= 0 && pick(Math.max(index - 1, 0))
        case Qt.Key_PageDown:
            return pick(index < 0 ? 0 : Math.min(index + 5, n - 1))
        case Qt.Key_PageUp:
            return index >= 0 && pick(Math.max(index - 5, 0))
        case Qt.Key_End:
            return pick(n - 1)
        case Qt.Key_Home:
            return index >= 0 && pick(0)
        case Qt.Key_Return:
        case Qt.Key_Enter:
            if (index >= 0) {
                apply(index)
                return true
            }
            close()
            return false
        case Qt.Key_Escape:
            close()
            return true
        }
        return false
    }

    // Write match `i` over the shortcode typed so far.
    function apply(i) {
        const edit = view.editAt(row)
        const text = ":" + matches[i][0] + ":"
        const a = start, z = end
        close()
        if (!edit)
            return
        edit.remove(a, z)
        edit.insert(a, text)
        edit.cursorPosition = a + text.length
        edit.forceActiveFocus()
    }

    Rectangle {
        anchors.fill: parent
        radius: 6
        color: Theme.panel
        border.color: Theme.border
    }
    ListView {
        id: list
        x: 1
        y: suggest.pad
        width: parent.width - 2
        height: parent.height - 2 * suggest.pad
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        model: suggest.matches
        currentIndex: suggest.index
        delegate: Rectangle {
            id: choice
            required property var modelData
            required property int index
            width: ListView.view.width
            height: suggest.rowHeight
            color: choice.index === suggest.index || pointer.containsMouse ? Theme.selection : "transparent"
            Text {
                width: suggest.iconWidth
                leftPadding: 6 * Ui.zoom
                anchors.verticalCenter: parent.verticalCenter
                horizontalAlignment: Text.AlignHCenter
                text: choice.modelData[1]
                color: Theme.foreground
                font.family: Ui.textFont
                font.pixelSize: suggest.em
            }
            Text {
                x: suggest.iconWidth
                anchors.verticalCenter: parent.verticalCenter
                text: ":" + choice.modelData[0] + ":"
                color: Theme.foreground
                font: widest.font
            }
            MouseArea {
                id: pointer
                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: suggest.apply(choice.index)
            }
        }
    }
}
