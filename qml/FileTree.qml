import QtQuick
import QtQuick.Controls
import Typo

// The Files tab as Typora's file tree: the folder at the top, then its
// folders (with a caret to open them) and files, a row each. Folders open
// on a click and stay open while the window is; the file being edited is
// marked, and the folders it is in open to show it. Rows drag out as the
// files themselves, and files dragged onto a folder move into it.
ListView {
    id: tree

    required property var app
    required property var side
    readonly property string root: app.folder
    readonly property string current: app.editor.path

    // The open folders, by path.
    property var expanded: ({})

    function reload() {
        if (root === "") {
            rows.clear()
            return
        }
        const out = [{ path: root, name: root.substring(root.lastIndexOf("/") + 1), dir: true, depth: 0, open: true }]
        const walk = (dir, depth) => {
            for (const n of JSON.parse(children(dir))) {
                const open = n.dir && expanded[n.path] === true
                out.push({ path: n.path, name: n.name, dir: n.dir, depth: depth, open: open })
                if (open)
                    walk(n.path, depth + 1)
            }
        }
        walk(root, 1)
        side.sync(rows, out)
    }
    // The middle of the row of `path` in the window, for the tests.
    function pointOf(path) {
        for (let i = 0; i < rows.count; i++)
            if (rows.get(i).path === path) {
                const item = itemAtIndex(i)
                return item.mapToItem(null, item.width / 2, item.height / 2)
            }
        return null
    }
    function uriOf(path) {
        return "file://" + path.split("/").map(encodeURIComponent).join("/")
    }

    // Files dragged over the tree, from its rows or from another program,
    // drop into the folder under the pointer, or the folder of the file
    // under it. As Typora's: a folder's row marks the folder, its rows
    // included, and the mark stays on the rows inside it; a folder with
    // something in it opens after a second over its row (an open one
    // closes after three), then turns every five.
    property string marked: ""
    function dragOver(x, y) {
        const i = indexAt(x + contentX, y + contentY)
        const r = i >= 0 ? rows.get(i) : null
        if (r && r.dir)
            marked = r.path
        else if (!r || (marked !== "" && !r.path.startsWith(marked + "/")))
            marked = ""
        if (r && r.dir && r.depth > 0 && JSON.parse(children(r.path)).length > 0) {
            if (spring.path !== r.path) {
                spring.path = r.path
                spring.interval = r.open ? 3000 : 1000
                spring.restart()
            }
        } else {
            spring.path = ""
            spring.stop()
        }
    }
    function dragEnd() {
        marked = ""
        spring.path = ""
        spring.stop()
    }
    Timer {
        id: spring
        property string path: ""
        onTriggered: {
            tree.toggle(path)
            interval = 5000
            start()
        }
    }
    DropArea {
        parent: tree
        anchors.fill: parent
        onEntered: drag => {
            if (!drag.hasUrls || tree.root === "")
                drag.accepted = false
            else
                tree.dragOver(drag.x, drag.y)
        }
        onPositionChanged: drag => tree.dragOver(drag.x, drag.y)
        onExited: tree.dragEnd()
        onDropped: drop => {
            const i = tree.indexAt(drop.x + tree.contentX, drop.y + tree.contentY)
            tree.dragEnd()
            const files = drop.urls.map(u => u.toString()).filter(u => u.startsWith("file://"))
                .map(u => decodeURIComponent(u.substring(7)))
            if (i < 0 || files.length === 0)
                return
            // A copy as far as the program it came from can tell, which
            // must leave its file alone; the move is done here.
            drop.accept(Qt.CopyAction)
            const r = rows.get(i)
            const dir = r.dir ? r.path : tree.side.parentOf(r.path)
            if (dir !== root && !expanded[dir]) {
                expanded[dir] = true
                reload()
            }
            Qt.callLater(() => tree.side.moveInto(files, dir))
        }
    }

    function children(dir) {
        return Folder.children(root, dir, Ui.treeSort, Ui.treeSortDescending, Ui.treeGroupByFolder)
    }
    function toggle(path) {
        expanded[path] = !expanded[path]
        reload()
    }
    function setAll(open) {
        if (!open) {
            expanded = {}
        } else {
            // Every folder, as far as Expand All goes in Typora: all of them.
            const walk = dir => {
                for (const n of JSON.parse(children(dir)))
                    if (n.dir) {
                        expanded[n.path] = true
                        walk(n.path)
                    }
            }
            walk(root)
        }
        reload()
    }
    // Open the folders down to `path`, so its row shows.
    function reveal(path) {
        if (root === "" || !path.startsWith(root + "/"))
            return
        let dir = path.substring(0, path.lastIndexOf("/"))
        let changed = false
        while (dir.length > root.length) {
            if (!expanded[dir]) {
                expanded[dir] = true
                changed = true
            }
            dir = dir.substring(0, dir.lastIndexOf("/"))
        }
        if (changed)
            reload()
        for (let i = 0; i < rows.count; i++)
            if (rows.get(i).path === path) {
                positionViewAtIndex(i, ListView.Contain)
                break
            }
    }

    onRootChanged: {
        expanded = {}
        reload()
        reveal(current)
    }
    onCurrentChanged: reveal(current)
    Component.onCompleted: {
        reload()
        reveal(current)
    }
    Connections {
        target: Folder
        function onChanged() { tree.reload() }
    }
    Connections {
        target: Ui
        function onTreeSortChanged() { tree.reload() }
        function onTreeSortDescendingChanged() { tree.reload() }
        function onTreeGroupByFolderChanged() { tree.reload() }
    }

    clip: true
    boundsBehavior: Flickable.StopAtBounds
    ScrollBar.vertical: ScrollBar {}
    topMargin: 6
    model: ListModel { id: rows }

    delegate: AbstractButton {
        id: row
        required property int index
        required property string path
        required property string name
        required property bool dir
        required property int depth
        required property bool open
        readonly property bool active: !dir && tree.current === path
        readonly property bool marked: tree.marked !== "" && (path === tree.marked || path.startsWith(tree.marked + "/"))
        readonly property real indent: 16 + 17 * depth
        width: tree.width
        height: 29
        hoverEnabled: true
        // Dragged out as the file itself, for other programs as well as
        // the folders here.
        property bool dragEnded: false
        DragHandler {
            id: dragger
            target: null
            enabled: row.depth > 0 && tree.side.renaming !== row.path && !row.dragEnded
        }
        // The drag swallows the release, so the handler stays active (and
        // keeps the pointer) until it is turned off and on again.
        Drag.onDragFinished: {
            dragEnded = true
            Qt.callLater(() => dragEnded = false)
        }
        Drag.active: dragger.active
        Drag.dragType: Drag.Automatic
        Drag.supportedActions: Qt.CopyAction | Qt.MoveAction
        Drag.mimeData: ({ "text/uri-list": [tree.uriOf(row.path)] })
        onClicked: {
            if (!dir)
                tree.side.open(path)
            else if (depth > 0)
                tree.toggle(path)
        }
        TapHandler {
            acceptedButtons: Qt.RightButton
            onTapped: tree.side.showMenu(row.path, row.depth === 0 ? "root" : row.dir ? "folder" : "file")
        }
        background: Rectangle {
            color: row.active ? tree.side.activeColor : row.hovered || row.marked ? tree.side.hoverColor : "transparent"
            Rectangle {
                visible: row.active
                width: 4
                height: parent.height
                color: Theme.accent
            }
        }
        contentItem: Item {
            Glyph {
                visible: row.dir && row.depth > 0
                x: row.indent - 12
                anchors.verticalCenter: parent.verticalCenter
                name: row.open ? "caretDown" : "caretRight"
                size: 11
                color: Theme.muted
            }
            Glyph {
                id: icon
                x: row.indent
                anchors.verticalCenter: parent.verticalCenter
                name: row.dir ? "folder" : "file"
                size: 14
                color: Theme.muted
            }
            Text {
                visible: tree.side.renaming !== row.path
                x: row.indent + 17
                width: parent.width - x - 8
                anchors.verticalCenter: parent.verticalCenter
                text: row.name
                color: row.active ? Theme.foreground : Theme.muted
                font.pixelSize: 14
                elide: Text.ElideRight
            }
            RenameField {
                x: row.indent + 13
                width: parent.width - x - 8
                anchors.verticalCenter: parent.verticalCenter
                side: tree.side
                path: row.path
                name: row.name
            }
        }
    }

    // Right-clicking below the rows: new files and folders at the top.
    TapHandler {
        acceptedButtons: Qt.RightButton
        onTapped: point => {
            if (tree.itemAt(point.position.x, point.position.y + tree.contentY) === null)
                tree.side.showMenu(tree.root, "blank")
        }
    }

    Text {
        anchors.centerIn: parent
        visible: tree.root === ""
        text: qsTr("No Folder is Opened.")
        color: Theme.faint
        font.pixelSize: 13
    }
}
