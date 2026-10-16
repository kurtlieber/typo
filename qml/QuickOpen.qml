import QtQuick
import QtQuick.Controls
import Typo

// Open Quickly (Ctrl+P), as Typora's: a box at the top of the window that
// finds files by name in the open folder and among recent files, with the
// recent folders and files shown before anything is typed. Up and Down
// pick, Enter opens, Escape closes.
Popup {
    id: quick

    required property var app
    required property var side
    // The open folder's files, read as the box opens.
    property var folderFiles: []

    function show() {
        folderFiles = app.folder !== "" ? JSON.parse(Folder.files(app.folder)) : []
        input.text = ""
        update()
        open()
        input.forceActiveFocus()
    }
    function update() {
        const q = input.text.trim()
        const here = app.editor.path
        const recent = Prefs.recentFiles.filter(p => p !== here && Desktop.exists(p))
        const rows = []
        if (q === "") {
            for (const p of Prefs.recentFolders.filter(p => Desktop.isFolder(p)).slice(0, 5))
                rows.push({ path: p, folder: true, section: qsTr("Pinned and Recent Locations") })
            for (const p of recent.slice(0, 10))
                rows.push({ path: p, folder: false, section: qsTr("Recent Files") })
        } else {
            const recents = JSON.parse(Folder.quickOpen(JSON.stringify(recent), q, ""))
            for (const p of recents.slice(0, 10))
                rows.push({ path: p, folder: false, section: qsTr("Recent Files") })
            const others = folderFiles.filter(p => p !== here && recents.indexOf(p) < 0)
            for (const p of JSON.parse(Folder.quickOpen(JSON.stringify(others), q, app.folder)).slice(0, 50))
                rows.push({ path: p, folder: false, section: qsTr("File Results") })
        }
        items.clear()
        for (const r of rows)
            items.append(r)
        list.currentIndex = items.count > 0 ? 0 : -1
    }
    function choose(i) {
        if (i < 0 || i >= items.count)
            return
        const r = items.get(i)
        close()
        if (r.folder)
            app.openFolder(r.path)
        else
            side.open(r.path)
    }

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: 40
    width: Math.min(420, parent.width - 32)
    height: Math.min(implicitHeight, parent.height - 80)
    padding: 0
    modal: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    onClosed: app.focusPage()

    background: Rectangle {
        color: Theme.panel
        border.color: Theme.border
        radius: 4
    }

    contentItem: Column {
        TextField {
            id: input
            width: parent.width
            height: 40
            leftPadding: 12
            rightPadding: 12
            placeholderText: qsTr("Search by file name")
            placeholderTextColor: Theme.faint
            color: Theme.foreground
            selectionColor: Theme.selection
            selectedTextColor: Theme.selectionText
            font.pixelSize: 14
            background: Rectangle {
                color: Theme.background
                radius: 4
                Rectangle {
                    y: parent.height - 1
                    width: parent.width
                    height: 1
                    color: Theme.border
                }
            }
            onTextChanged: quick.update()
            Keys.onDownPressed: list.currentIndex = Math.min(items.count - 1, list.currentIndex + 1)
            Keys.onUpPressed: list.currentIndex = Math.max(0, list.currentIndex - 1)
            Keys.onReturnPressed: quick.choose(list.currentIndex)
            Keys.onEnterPressed: quick.choose(list.currentIndex)
            Keys.onEscapePressed: quick.close()
        }
        ListView {
            id: list
            width: parent.width
            height: Math.min(contentHeight, 440)
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar {}
            model: ListModel { id: items }
            highlightMoveDuration: 0
            section.property: "section"
            section.delegate: Text {
                required property string section
                width: list.width
                leftPadding: 12
                topPadding: 8
                bottomPadding: 4
                text: section
                color: Theme.faint
                font.pixelSize: 11
                font.bold: true
            }
            delegate: AbstractButton {
                id: entry
                required property int index
                required property string path
                required property bool folder
                width: list.width
                height: 40
                hoverEnabled: true
                onClicked: quick.choose(index)
                background: Rectangle {
                    color: entry.ListView.isCurrentItem ? Theme.selection : entry.hovered ? Qt.alpha(Theme.selection, 0.5) : "transparent"
                }
                contentItem: Item {
                    Glyph {
                        x: 12
                        anchors.verticalCenter: parent.verticalCenter
                        name: entry.folder ? "folder" : "file"
                        size: 14
                        color: entry.ListView.isCurrentItem ? Theme.selectionText : Theme.muted
                    }
                    Text {
                        x: 34
                        y: 4
                        width: parent.width - x - 12
                        text: entry.path.substring(entry.path.lastIndexOf("/") + 1)
                        color: entry.ListView.isCurrentItem ? Theme.selectionText : Theme.foreground
                        font.pixelSize: 14
                        elide: Text.ElideRight
                    }
                    Text {
                        x: 34
                        y: 22
                        width: parent.width - x - 12
                        text: quick.side.tilde(entry.path.substring(0, entry.path.lastIndexOf("/")))
                        color: entry.ListView.isCurrentItem ? Theme.selectionText : Theme.faint
                        font.pixelSize: 11
                        elide: Text.ElideMiddle
                    }
                }
            }
        }
        Text {
            visible: items.count === 0 && input.text.trim() !== ""
            width: parent.width
            height: 40
            leftPadding: 12
            verticalAlignment: Text.AlignVCenter
            text: qsTr("No Results")
            color: Theme.faint
            font.pixelSize: 13
        }
    }
}
