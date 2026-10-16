import QtQuick
import Typo

// A footnote's text under its reference while the pointer is on it, as
// Typora's footnote tooltip, or for a footnote with no definition, how to
// write one, with a link that adds it. It stays while the pointer is on
// it and goes half a second after the pointer leaves both.
Item {
    id: tip

    // The EditorView it belongs to, in whose content it is placed.
    required property Item view
    // The footnote shown (its key in `view.footnotes`) and the reference
    // it hangs from: row (or "row:r:c" for a table cell) and UTF-16 range.
    property string key: ""
    property var row: -1
    property int start: 0
    property int end: 0

    readonly property var note: key !== "" ? view.footnotes[key] || null : null
    readonly property string label: note ? note.label : ""
    readonly property real em: Ui.fontSize * 0.8
    readonly property string html: {
        view.editor.footnotes
        return note && note.def ? view.editor.footnoteHtml(label) : ""
    }
    readonly property bool shown: key !== ""
    readonly property bool hovered: hover.hovered
    readonly property alias body: text

    visible: shown
    z: 10
    width: box.width
    height: box.height + arrow.height / 2

    function show(key, row, start, end) {
        hide.stop()
        if (key === tip.key && row === tip.row && start === tip.start)
            return
        tip.key = key
        tip.row = row
        tip.start = start
        tip.end = end
        place()
    }
    function leave() {
        if (shown && !hide.running)
            hide.restart()
    }
    function close() {
        hide.stop()
        key = ""
        row = -1
    }

    // Under the reference, centred on it and kept on the page; above it
    // when there is no room below.
    property bool above: false
    function place() {
        const edit = view.noteEdit(row)
        if (!edit)
            return close()
        const a = edit.positionToRectangle(start)
        const z = edit.positionToRectangle(end)
        const p = edit.mapToItem(tip.parent, (a.x + z.x) / 2, a.y)
        const page = view.pageRect()
        x = Math.max(page.x, Math.min(page.x + page.width - width, p.x - width / 2))
        arrow.x = Math.max(6, Math.min(width - arrow.width - 6, p.x - x - arrow.width / 2))
        above = p.y + a.height + height > view.contentY + view.height && p.y - height > view.contentY
        y = above ? p.y - height : p.y + a.height
    }
    onWidthChanged: if (shown) place()
    onHeightChanged: if (shown) place()

    Timer {
        id: hide
        interval: 500
        onTriggered: {
            if (!tip.hovered)
                tip.close()
        }
    }
    HoverHandler {
        id: hover
        onHoveredChanged: hovered ? hide.stop() : tip.leave()
    }

    Rectangle {
        id: arrow
        width: 10 * Ui.zoom
        height: width
        rotation: 45
        y: tip.above ? box.y + box.height - height / 2 : 0
        color: box.color
        border.color: box.border.color
    }
    Rectangle {
        id: box
        y: tip.above ? 0 : arrow.height / 2
        width: text.width + 2 * pad
        height: text.height + 2 * pad
        readonly property real pad: tip.em * 0.6
        radius: 4
        color: Theme.panel
        border.color: Theme.border

        Text {
            id: text
            x: box.pad
            y: box.pad
            width: Math.min(implicitWidth, tip.view.pageRect().width * 0.8)
            wrapMode: Text.Wrap
            textFormat: Text.RichText
            horizontalAlignment: Text.AlignHCenter
            color: Theme.foreground
            linkColor: Theme.link
            font.family: Ui.textFont
            font.pixelSize: tip.em
            lineHeight: 1.2
            text: tip.html !== ""
                  ? tip.html.replace(/<a /g, "<a style=\"color:" + Theme.link + "\" ")
                  : qsTr("<span style=\"color:%1\">⚠</span> Need to define the reference footnote <b>%2</b> with syntax <a href=\"typo:add\" style=\"color:%3\">[^%2]: www.example.com</a>")
                    .arg(Theme.error).arg(tip.label.replace(/&/g, "&amp;").replace(/</g, "&lt;")).arg(Theme.link)
            onLinkActivated: link => {
                if (link === "typo:add") {
                    const label = tip.label
                    tip.close()
                    tip.view.command("addFootnote", 0, label)
                } else {
                    tip.close()
                    tip.view.openLink(link)
                }
            }
            HoverHandler {
                cursorShape: text.hoveredLink !== "" ? Qt.PointingHandCursor : Qt.ArrowCursor
            }
        }
    }
}
