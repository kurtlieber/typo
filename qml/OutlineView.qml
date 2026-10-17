import QtQuick
import QtQuick.Controls
import Typo

// The Outline tab: the document's headings, indented by level, the one
// the caret is under in bold. Flat, or (Collapsible Outline) with carets
// that fold a heading's subheadings away; Filter narrows it to headings
// containing some text. Its menu is Typora's outline menu.
Item {
    id: outline

    required property var app
    required property var side
    readonly property var headings: JSON.parse(app.editor.outline)
    property bool filtering: false
    property alias filter: filterInput.text
    // Folded headings, by "level title", so they stay folded as rows move.
    property var folded: ({})
    // What it needs to show every row (the outline popover's height).
    readonly property real fullHeight: filterPanel.height + list.topMargin + list.contentHeight + 10

    function key(h) {
        return h.level + " " + h.title
    }
    readonly property var shown: {
        const out = []
        const needle = filter.trim().toLowerCase()
        if (filtering && needle !== "") {
            for (const h of headings)
                if (h.title.toLowerCase().indexOf(needle) >= 0)
                    out.push({ row: h.row, level: h.level, title: h.title, parent: false, depth: 0 })
            return out
        }
        // Levels nest under the nearest smaller one, so a document that
        // starts at ## isn't indented for nothing.
        const stack = []
        let hiddenBelow = 7
        for (let i = 0; i < headings.length; i++) {
            const h = headings[i]
            while (stack.length && stack[stack.length - 1] >= h.level)
                stack.pop()
            const depth = stack.length
            stack.push(h.level)
            if (h.level <= hiddenBelow)
                hiddenBelow = 7
            else
                continue
            const parent = i + 1 < headings.length && headings[i + 1].level > h.level
            if (Ui.outlineCollapsible && parent && folded[key(h)])
                hiddenBelow = h.level
            out.push({ row: h.row, level: h.level, title: h.title, parent: parent, depth: depth })
        }
        return out
    }
    function setAll(open) {
        const f = {}
        if (!open)
            for (let i = 0; i + 1 < headings.length; i++)
                if (headings[i + 1].level > headings[i].level)
                    f[key(headings[i])] = true
        folded = f
    }
    function startFilter() {
        filtering = true
        filterInput.forceActiveFocus()
        filterInput.selectAll()
    }
    function stopFilter() {
        filtering = false
        filter = ""
    }

    Item {
        id: filterPanel
        visible: outline.filtering
        width: parent.width
        height: visible ? 44 : 0
        TextField {
            id: filterInput
            x: 16
            y: 6
            width: parent.width - 32
            height: 28
            placeholderText: qsTr("Filter")
            placeholderTextColor: Theme.faint
            color: Theme.foreground
            font.pixelSize: 13
            leftPadding: 2
            rightPadding: 22
            background: Rectangle {
                color: "transparent"
                Rectangle {
                    y: parent.height - 1
                    width: parent.width
                    height: 1
                    color: filterInput.activeFocus ? Theme.accent : Theme.border
                }
            }
            Keys.onEscapePressed: outline.stopFilter()
        }
        AbstractButton {
            anchors.right: filterInput.right
            anchors.verticalCenter: filterInput.verticalCenter
            width: 18
            height: 18
            focusPolicy: Qt.NoFocus
            onClicked: outline.stopFilter()
            contentItem: Glyph {
                name: "close"
                size: 10
                color: Theme.muted
            }
            background: null
        }
    }

    ListView {
        id: list
        anchors.top: filterPanel.bottom
        anchors.bottom: parent.bottom
        width: parent.width
        topMargin: 10
        clip: true
        model: outline.shown
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {}
        delegate: AbstractButton {
            id: entry
            required property var modelData
            required property int index
            readonly property bool current: Ui.outlineHighlight && outline.app.currentHeading === modelData.row
            readonly property real indent: 18 + modelData.depth * 14 + (Ui.outlineCollapsible ? 12 : 0)
            width: list.width
            implicitHeight: 26
            hoverEnabled: true
            onClicked: outline.app.run("jumpToRow", modelData.row)
            TapHandler {
                acceptedButtons: Qt.RightButton
                onTapped: outlineMenu.popup()
            }
            background: Rectangle {
                color: entry.hovered ? outline.side.hoverColor : "transparent"
            }
            contentItem: Item {
                AbstractButton {
                    visible: Ui.outlineCollapsible && entry.modelData.parent
                    x: entry.indent - 15
                    anchors.verticalCenter: parent.verticalCenter
                    width: 14
                    height: 18
                    focusPolicy: Qt.NoFocus
                    onClicked: {
                        const f = Object.assign({}, outline.folded)
                        const k = outline.key(entry.modelData)
                        f[k] = !f[k]
                        outline.folded = f
                    }
                    contentItem: Glyph {
                        name: outline.folded[outline.key(entry.modelData)] ? "caretRight" : "caretDown"
                        size: 11
                        color: Theme.muted
                    }
                    background: null
                }
                Text {
                    x: entry.indent
                    width: parent.width - x - 12
                    anchors.verticalCenter: parent.verticalCenter
                    text: entry.modelData.title
                    color: Theme.foreground
                    font.pixelSize: 14
                    font.bold: entry.current
                    elide: Text.ElideRight
                }
            }
        }
        TapHandler {
            acceptedButtons: Qt.RightButton
            onTapped: point => {
                if (list.itemAt(point.position.x, point.position.y + list.contentY) === null)
                    outlineMenu.popup()
            }
        }
    }

    Text {
        anchors.centerIn: parent
        visible: outline.headings.length === 0
        text: qsTr("Outline is Empty.")
        color: Theme.faint
        font.pixelSize: 13
    }

    TMenu {
        id: outlineMenu
        TMenuItem { text: qsTr("Highlight Current Header"); checkable: true; checked: Ui.outlineHighlight; onTriggered: Ui.outlineHighlight = !Ui.outlineHighlight }
        TMenuItem { text: qsTr("Filter"); checkable: true; checked: outline.filtering; onTriggered: outline.filtering ? outline.stopFilter() : outline.startFilter() }
        TSep { visible: Ui.outlineCollapsible }
        TMenuItem { visible: Ui.outlineCollapsible; text: qsTr("Expand All"); onTriggered: outline.setAll(true) }
        TMenuItem { visible: Ui.outlineCollapsible; text: qsTr("Collapse All"); onTriggered: outline.setAll(false) }
        TSep {}
        TMenuItem { text: qsTr("Collapsible Outline"); checkable: true; checked: Ui.outlineCollapsible; onTriggered: Ui.outlineCollapsible = true }
        TMenuItem { text: qsTr("Flat Outline"); checkable: true; checked: !Ui.outlineCollapsible; onTriggered: Ui.outlineCollapsible = false }
    }
}
