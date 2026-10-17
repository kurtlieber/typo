import QtQuick
import Typo

// What Typora offers once an image has loaded with the caret in its
// source: copying or moving its file, its HiDPI size, and zooming, in a
// list under the caret that doesn't take the focus. It closes when the
// caret leaves the image, or a row is clicked. Up and Down pick a row
// (past either end closes it), Enter runs it and Escape closes it.
Item {
    id: popup

    // The EditorView it belongs to, in whose content it is placed.
    required property Item view
    // The image (Main.imageHere()), and the URL last offered for, so a
    // reload or the same picture again offers nothing.
    property var image: null
    property string lastUrl: ""
    // The picked row, by key or pointer; -1 for none.
    property int current: -1

    signal chosen(string cmd, string arg)

    readonly property bool shown: image !== null
    readonly property bool loadedFile: shown && image.path !== "" && Desktop.exists(image.path)
    readonly property bool copyable: loadedFile || (shown && image.path === "" && /^(https?|ftp):/i.test(image.url))
    readonly property string zoomTo: shown && image.ratio !== 1 ? Math.round(100 / image.ratio) + "%" : ""
    // [label, command, argument]: Typora's order.
    readonly property var rows: {
        if (!shown)
            return []
        const r = []
        const folder = copyable ? image.folder : ""
        if (folder !== "")
            r.push([qsTr("Copy to %1").arg(folder), "copyImageToFolder", ""])
        if (copyable)
            r.push([qsTr("Copy Image to…"), "copyImageTo", ""])
        if (loadedFile && folder !== "")
            r.push([qsTr("Move to %1").arg(folder), "moveImageToFolder", ""])
        if (loadedFile)
            r.push([qsTr("Rename or Move Image to…"), "moveImageTo", ""])
        if (loadedFile && Prefs.imageUploader !== "")
            r.push([qsTr("Upload Image"), "uploadImage", ""])
        if (zoomTo !== "")
            r.push([qsTr("Zoom Image to %1").arg(zoomTo), "zoomImage", zoomTo])
        r.push([qsTr("Zoom Image"), "", ""])
        return r
    }
    readonly property real em: Ui.fontSize * 0.9
    readonly property real rowHeight: Math.round(em * 2)
    readonly property real pad: 4

    visible: shown
    z: 10
    width: Math.max(180 * Ui.zoom, labels.width + 40 * Ui.zoom)
    height: rows.length * rowHeight + 2 * pad
    // The labels' widths, for the list's.
    Column {
        id: labels
        visible: false
        Repeater {
            model: popup.rows
            Text {
                required property var modelData
                text: modelData[0]
                font.family: Ui.textFont
                font.pixelSize: popup.em * 0.9
            }
        }
    }

    // A picture in `edit` loaded: offer for it when the caret is in its
    // source and it is a new one.
    function offer(edit, img) {
        if (!img || !edit.activeFocus || img.url === lastUrl)
            return
        lastUrl = img.url
        image = img
        current = -1
        const r = edit.positionToRectangle(edit.cursorPosition)
        const p = edit.mapToItem(popup.parent, r.x, r.y)
        x = Math.max(view.contentX, Math.min(view.contentX + view.width - width - 4, p.x))
        const below = p.y + r.height
        y = below + height > view.contentY + view.height && p.y - height > view.contentY ? p.y - height : below
    }
    function close() {
        image = null
        current = -1
        zoomMenu.close()
    }
    // From the editor's keys while shown: true when taken.
    function keyPressed(event) {
        if (event.modifiers & (Qt.ShiftModifier | Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier))
            return false
        const last = rows.length - 1
        switch (event.key) {
        case Qt.Key_Escape:
            close()
            return true
        case Qt.Key_Up:
            if (current === 0)
                close()
            else
                current = current < 0 ? last : current - 1
            return true
        case Qt.Key_Down:
            if (current === last)
                close()
            else
                current++
            return true
        case Qt.Key_Right:
            if (current < 0 || rows[current][1] !== "")
                return false
            run(current, true)
            return true
        case Qt.Key_Return:
        case Qt.Key_Enter:
            if (current < 0)
                return false
            run(current, true)
            return true
        }
        return false
    }
    // Zoom Image opens its menu (at its first entry from the keys); the
    // others close the list and run.
    function run(i, key) {
        const r = rows[i]
        if (r[1] === "") {
            zoomMenu.popup(row(i), row(i).width, 0)
            if (key)
                zoomMenu.currentIndex = 0
            return
        }
        close()
        chosen(r[1], r[2])
    }

    Rectangle {
        anchors.fill: parent
        radius: 4
        color: Theme.panel
        border.color: Theme.border
    }
    Column {
        id: list
        x: 1
        y: popup.pad
        width: parent.width - 2
        Repeater {
            id: rowItems
            model: popup.rows
            Rectangle {
                id: row
                required property var modelData
                required property int index
                width: list.width
                height: popup.rowHeight
                color: index === popup.current ? Theme.selection : "transparent"
                readonly property bool sub: modelData[1] === ""
                Text {
                    x: 14 * Ui.zoom
                    anchors.verticalCenter: parent.verticalCenter
                    text: row.modelData[0]
                    color: Theme.foreground
                    font.family: Ui.textFont
                    font.pixelSize: popup.em * 0.9
                }
                Text {
                    visible: row.sub
                    anchors.right: parent.right
                    anchors.rightMargin: 10 * Ui.zoom
                    anchors.verticalCenter: parent.verticalCenter
                    text: "›"
                    color: Theme.muted
                    font.family: Ui.textFont
                    font.pixelSize: popup.em * 0.9
                }
                MouseArea {
                    id: pointer
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onEntered: popup.current = row.index
                    onExited: if (popup.current === row.index && !row.sub) popup.current = -1
                    onClicked: popup.run(row.index)
                }
            }
        }
    }
    TMenu {
        id: zoomMenu
        TMenuItem { text: "25%"; onTriggered: popup.zoom("25%") }
        TMenuItem { text: "33%"; onTriggered: popup.zoom("33%") }
        TMenuItem { text: "50%"; onTriggered: popup.zoom("50%") }
        TMenuItem { text: "67%"; onTriggered: popup.zoom("67%") }
        TMenuItem { text: "80%"; onTriggered: popup.zoom("80%") }
        TMenuItem { text: "100%"; onTriggered: popup.zoom("100%") }
        TMenuItem { text: "150%"; onTriggered: popup.zoom("150%") }
        TMenuItem { text: "200%"; onTriggered: popup.zoom("200%") }
    }
    function zoom(to) {
        close()
        chosen("zoomImage", to)
    }
    // For scripts.
    readonly property alias zoomMenu: zoomMenu
    function row(i) { return rowItems.itemAt(i) }
}
