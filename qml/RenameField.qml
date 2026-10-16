import QtQuick
import QtQuick.Controls
import Typo

// Renaming a file or folder in the sidebar, in place of its name, as
// Typora does: the name without its extension selected; Enter or leaving
// it renames, Escape doesn't.
TextField {
    id: field

    required property var side
    required property string path
    property string name

    visible: side.renaming === path
    text: name
    padding: 2
    leftPadding: 4
    font.pixelSize: 14
    color: Theme.foreground
    selectionColor: Theme.selection
    selectedTextColor: Theme.selectionText
    background: Rectangle {
        color: Theme.background
        border.color: Theme.accent
        radius: 2
    }

    property bool done: false
    // Enter or Escape: back to the page, as Typora restores the caret, so
    // the key's release can't click the row the field leaves focused.
    function finish(apply) {
        if (done)
            return
        done = true
        side.app.focusPage()
        side.finishRename(path, apply ? text : "")
    }
    function begin() {
        done = false
        text = name
        forceActiveFocus()
        const dot = name.lastIndexOf(".")
        select(0, dot > 0 ? dot : name.length)
    }
    onVisibleChanged: if (visible) begin()
    // A row made for a file already being renamed (a new one) starts at
    // once.
    Component.onCompleted: if (visible) begin()
    onAccepted: finish(true)
    // Leaving it renames, once whatever took the focus is done: a reload
    // may be rebuilding this very row.
    onActiveFocusChanged: {
        if (activeFocus || !visible || done)
            return
        done = true
        const from = path, to = text
        Qt.callLater(() => side.finishRename(from, to))
    }
    Keys.onEscapePressed: finish(false)
}
