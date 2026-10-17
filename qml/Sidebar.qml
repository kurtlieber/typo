import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtCore
import Typo

// The left panel, as Typora's: Files (the open folder as an articles list
// or a file tree) and Outline tabs; Search in place of Outline while the
// folder is searched. The footer under Files makes new files, opens the
// folder menu and switches between list and tree. File operations (new,
// rename, duplicate, delete) live here, for every view to share.
Rectangle {
    id: sidebar
    required property var app
    color: Theme.panel

    readonly property bool outlineView: Ui.sidebarView === 0 && !searching
    property bool searching: false
    // The file or folder being renamed in place.
    property string renaming: ""
    // The file or folder just made (new, or a duplicate), named in place.
    property string madeNow: ""
    // Typora's one file operation to take back from the file menu: the
    // last rename, as {from, to, label}. Naming what was just made doesn't
    // count, and a duplicate or a delete forgets it. Its menu entry shows
    // while both ends are in the open folder.
    property var undoOp: null
    readonly property string undoLabel: undoOp && app.folder !== "" && undoOp.from.startsWith(app.folder + "/")
                                        && undoOp.to.startsWith(app.folder + "/") ? undoOp.label : ""

    // The views, for the tests.
    readonly property alias fileList: list
    readonly property alias fileTree: tree
    readonly property alias folderSearch: search
    readonly property alias footerMenu: footerMenu
    readonly property alias fileMenu: fileMenu

    readonly property color activeColor: Qt.alpha(Theme.foreground, Theme.dark ? 0.12 : 0.07)
    readonly property color hoverColor: Qt.alpha(Theme.foreground, Theme.dark ? 0.07 : 0.04)

    // Bring a ListModel in line with `items`, in place where it can, so
    // views keep their place when the folder reloads.
    function sync(model, items) {
        for (let i = 0; i < items.length; i++) {
            if (i < model.count)
                model.set(i, items[i])
            else
                model.append(items[i])
        }
        if (model.count > items.length)
            model.remove(items.length, model.count - items.length)
    }
    function html(s) {
        return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    }
    readonly property string home: StandardPaths.writableLocation(StandardPaths.HomeLocation).toString().replace(/^file:\/\//, "")
    function tilde(p) {
        return home !== "" && p.startsWith(home + "/") ? "~" + p.substring(home.length) : p
    }
    function parentOf(p) {
        return p.substring(0, p.lastIndexOf("/"))
    }
    function nameOf(p) {
        return p.substring(p.lastIndexOf("/") + 1)
    }

    // Opening a file from the sidebar replaces the document in this
    // window, after asking about unsaved changes.
    function open(path, then) {
        app.openInPlace(path, then)
    }
    function openAt(path, line) {
        open(path, () => app.run("jumpToRow", app.editor.rowOfLine(line)))
    }
    function openSearch() {
        Ui.sidebar = true
        if (Ui.sidebarView === 0)
            Ui.sidebarView = Ui.fileTree ? 2 : 1
        searching = true
        Qt.callLater(() => search.focusInput())
    }
    function closeSearch() {
        searching = false
    }
    // Picking a view (View menu, the tabs) ends a search.
    Connections {
        target: Ui
        function onSidebarViewChanged() { sidebar.searching = false }
    }
    function refresh() {
        list.reload()
        tree.reload()
        if (searching)
            search.run()
    }

    // The folder new files go in: the folder clicked, the folder of the
    // file clicked, or the open folder.
    function folderFor(path, kind) {
        if (kind === "folder" || kind === "root")
            return path
        if (kind === "file" || kind === "search")
            return parentOf(path)
        return app.folder
    }
    function newFile(dir) {
        if (dir === "")
            dir = app.folder
        if (dir === "") {
            app.run("new")
            return
        }
        // Typora opens the new file at once, so unsaved work is settled
        // before anything is made.
        app.openInPlace("", () => {
            const path = dir + "/" + Folder.untitledName(dir, false, Prefs.defaultExtension)
            const error = Folder.create(path, false)
            if (error !== "") {
                app.notify(error)
                return
            }
            app.editor.open(path)
            reveal(path)
            renaming = madeNow = path
        })
    }
    function newFolder(dir) {
        const path = dir + "/" + Folder.untitledName(dir, true, "")
        const error = Folder.create(path, true)
        if (error !== "") {
            app.notify(error)
            return
        }
        tree.expanded[dir] = true
        tree.reload()
        renaming = madeNow = path
    }
    function reveal(path) {
        list.reload()
        tree.reveal(path)
    }
    // Done renaming `path` to `name` ("" leaves it).
    function finishRename(path, name) {
        if (renaming !== path)
            return
        renaming = ""
        const made = madeNow === path
        madeNow = ""
        if (name === "" || name === nameOf(path))
            return
        const result = Folder.rename(path, name)
        if (result.startsWith("!")) {
            app.notify(result.substring(1))
            return
        }
        if (!made)
            undoOp = { from: path, to: result, label: qsTr("Rename %1").arg(nameOf(path)) }
        moved(path, result)
    }
    // After `from` became `to`: the document follows its file, or the
    // folder it is in, and so do the open folders.
    function moved(from, to) {
        const doc = app.editor.path
        if (doc === from)
            app.editor.path = to
        else if (doc.startsWith(from + "/"))
            app.editor.path = to + doc.substring(from.length)
        if (tree.expanded[from]) {
            tree.expanded[to] = true
            delete tree.expanded[from]
        }
        if (app.folder === from)
            app.folder = to
        refresh()
    }
    // Take the last file operation back, and show where it went.
    function undo() {
        const op = undoOp
        undoOp = null
        if (!op)
            return
        const result = Folder.moveFile(op.to, op.from, false)
        if (result.startsWith("!")) {
            app.notify(result.substring(1))
            return
        }
        moved(op.to, op.from)
        tree.reveal(op.from)
    }
    function duplicate(path) {
        const result = Folder.duplicate(path)
        if (result.startsWith("!")) {
            app.notify(result.substring(1))
            return
        }
        undoOp = null
        reveal(result)
        renaming = madeNow = result
    }
    function remove(path) {
        if (Prefs.noWarningForDeleteFile) {
            trash(path)
            return
        }
        app.ask("", qsTr("Are you sure you want to delete %1?").arg(nameOf(path)) + "<br>" + qsTr("You can restore from the Trash."),
                [qsTr("OK"), qsTr("Cancel")], (i, never) => {
            if (i !== 0)
                return
            if (never)
                Prefs.noWarningForDeleteFile = true
            trash(path)
        }, { check: qsTr("Do not show this again") })
    }
    function trash(path) {
        const doc = app.editor.path
        const current = doc === path || doc.startsWith(path + "/")
        // The next file in the folder takes the deleted one's place.
        let next = ""
        if (current && !Desktop.isFolder(path)) {
            const files = JSON.parse(Folder.files(app.folder))
            const at = files.indexOf(path)
            next = at >= 0 && at + 1 < files.length ? files[at + 1] : at > 0 ? files[at - 1] : ""
        }
        if (!Desktop.moveToTrash(path)) {
            app.notify(qsTr("Failed to delete %1.").arg(nameOf(path)))
            return
        }
        undoOp = null
        if (current) {
            if (next !== "")
                app.editor.open(next)
            else
                app.editor.newDocument()
        }
        refresh()
    }

    // Typora's moving files into the folder `dir`, dragged there in the
    // tree or dropped on the sidebar from another program, one after
    // another. Each asks first unless told not to; one already there, or a
    // folder dropped into itself, stays. One file moved while the window
    // holds an empty Untitled opens.
    function moveInto(paths, dir) {
        const opens = paths.length === 1 && app.discardable
        const never = Ui.fileTree ? "noWarningForMoveFile" : "noWarningForMoveFileToList"
        const next = i => {
            if (i >= paths.length)
                return
            const from = paths[i]
            if (from === dir || dir.startsWith(from + "/") || parentOf(from) === dir) {
                tree.reveal(from)
                next(i + 1)
            } else if (Prefs[never]) {
                moveTo(from, dir, opens, () => next(i + 1))
            } else {
                app.ask("", qsTr("Are you sure you want to move %1?").arg(nameOf(from)), [qsTr("OK"), qsTr("Cancel")], (k, ticked) => {
                    if (k !== 0)
                        return next(i + 1)
                    if (ticked)
                        Prefs[never] = true
                    moveTo(from, dir, opens, () => next(i + 1))
                }, { check: qsTr("Do not show this again") })
            }
        }
        next(0)
    }
    // Move `from` into `dir`; a name already there asks whether to keep
    // both (" (copy)" before the first dot, as often as it takes) or
    // replace it. `done` goes on to the next.
    function moveTo(from, dir, opens, done) {
        let name = nameOf(from)
        const to = dir + "/" + name
        if (!Desktop.exists(to)) {
            place(from, to, false, opens)
            return done()
        }
        app.ask("", qsTr("File/directory already exists at target path."), [qsTr("Keep Both"), qsTr("Replace"), qsTr("Cancel")], k => {
            if (k === 0) {
                let path = to
                while (Desktop.exists(path)) {
                    const dot = name.indexOf(".")
                    name = dot < 0 ? name + " (copy)" : name.substring(0, dot) + " (copy)" + name.substring(dot)
                    path = dir + "/" + name
                }
                place(from, path, false, opens)
            } else if (k === 1) {
                place(from, to, true, opens)
            }
            done()
        }, { defaultButton: 1 })
    }
    // The move itself: undoable, unless it replaced something; onto
    // another disk a copy, which forgets the undo.
    function place(from, to, replace, opens) {
        const result = Folder.moveFile(from, to, replace)
        if (result.startsWith("!")) {
            undoOp = null
            app.notify(qsTr("Operation Failed") + "  " + result.substring(1))
            return
        }
        if (result === "copied") {
            undoOp = null
            to = from
        } else {
            if (!replace)
                undoOp = { from: from, to: to, label: qsTr("Move %1").arg(nameOf(from)) }
            moved(from, to)
        }
        if (opens && !Desktop.isFolder(to) && Folder.isListed(to))
            open(to)
        else
            reveal(to)
    }

    // The context menu of the sidebar's files, as Typora shows it for a
    // file, a folder, the folder itself (tree root), empty space, or a
    // search result.
    property string menuPath: ""
    property string menuKind: ""
    function showMenu(path, kind) {
        menuPath = path
        menuKind = kind
        fileMenu.popup()
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        // Tabs: Files and Outline, or Files and Search.
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: false
            Layout.preferredHeight: 46
            Layout.maximumHeight: 46
            spacing: 0
            Repeater {
                model: [qsTr("Files"), sidebar.searching ? qsTr("Search") : qsTr("Outline")]
                delegate: AbstractButton {
                    id: tab
                    required property int index
                    required property string modelData
                    readonly property bool current: index === 1 ? sidebar.searching || sidebar.outlineView : !sidebar.searching && !sidebar.outlineView
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    onClicked: {
                        if (index === 0) {
                            sidebar.searching = false
                            if (Ui.sidebarView === 0)
                                Ui.sidebarView = Ui.fileTree ? 2 : 1
                        } else if (!sidebar.searching) {
                            Ui.sidebarView = 0
                        }
                    }
                    contentItem: Item {
                        Text {
                            id: tabText
                            anchors.centerIn: parent
                            anchors.verticalCenterOffset: 2
                            text: tab.modelData
                            font.pixelSize: 14
                            font.bold: tab.current
                            color: tab.current ? Theme.foreground : Theme.muted
                        }
                        Rectangle {
                            anchors.horizontalCenter: parent.horizontalCenter
                            anchors.bottom: parent.bottom
                            width: tabText.implicitWidth + 40
                            height: 3
                            color: tab.current ? Theme.accent : "transparent"
                        }
                    }
                    background: null
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            OutlineView {
                anchors.fill: parent
                visible: sidebar.outlineView
                app: sidebar.app
                side: sidebar
            }
            FolderSearch {
                id: search
                anchors.fill: parent
                visible: sidebar.searching
                app: sidebar.app
                side: sidebar
            }
            FileList {
                id: list
                anchors.fill: parent
                anchors.topMargin: 4
                visible: !sidebar.outlineView && !sidebar.searching && !Ui.fileTree && sidebar.app.folder !== ""
                app: sidebar.app
                side: sidebar
            }
            FileTree {
                id: tree
                anchors.fill: parent
                visible: !sidebar.outlineView && !sidebar.searching && Ui.fileTree && sidebar.app.folder !== ""
                app: sidebar.app
                side: sidebar
            }
            Column {
                anchors.centerIn: parent
                spacing: 12
                visible: !sidebar.outlineView && !sidebar.searching && sidebar.app.folder === ""
                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: Ui.fileTree ? qsTr("No Folder is Opened.") : qsTr("No Files Available")
                    color: Theme.faint
                    font.pixelSize: 13
                }
            }
        }

        // Footer: New File, the folder menu, and list or tree.
        Item {
            Layout.fillWidth: true
            implicitHeight: 30
            visible: !sidebar.outlineView
            Rectangle {
                width: parent.width
                height: 1
                color: Theme.border
                opacity: 0.6
            }
            AbstractButton {
                id: newButton
                x: 4
                width: 28
                height: parent.height
                hoverEnabled: true
                onClicked: sidebar.newFile(sidebar.app.folder)
                ToolTip.visible: hovered
                ToolTip.delay: 600
                ToolTip.text: qsTr("New File")
                contentItem: Glyph {
                    name: "plus"
                    size: 14
                    color: newButton.hovered ? Theme.foreground : Theme.muted
                }
                background: null
            }
            AbstractButton {
                id: folderButton
                anchors.left: newButton.right
                anchors.right: switchButton.left
                height: parent.height
                hoverEnabled: true
                onClicked: footerMenu.popup(folderButton, 0, -footerMenu.height)
                contentItem: Text {
                    text: sidebar.app.folderName !== "" ? sidebar.app.folderName : qsTr("Open Folder…")
                    color: folderButton.hovered ? Theme.foreground : Theme.muted
                    font.pixelSize: 12
                    elide: Text.ElideMiddle
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                }
                background: null
            }
            AbstractButton {
                id: switchButton
                anchors.right: parent.right
                anchors.rightMargin: 4
                width: 28
                height: parent.height
                hoverEnabled: true
                onClicked: Ui.sidebarView = Ui.fileTree ? 1 : 2
                ToolTip.visible: hovered
                ToolTip.delay: 600
                ToolTip.text: qsTr("Switch File List/Tree View")
                contentItem: Glyph {
                    name: "cells"
                    size: 14
                    color: switchButton.hovered ? Theme.foreground : Theme.muted
                }
                background: null
            }
        }
    }

    // The folder menu in the footer.
    TMenu {
        id: footerMenu
        // Other windows may have opened folders since.
        onAboutToShow: Prefs.reload()
        TMenuItem { text: qsTr("New File"); enabled: sidebar.app.folder !== ""; onTriggered: sidebar.newFile(sidebar.app.folder) }
        TMenuItem { text: qsTr("Search"); enabled: sidebar.app.folder !== ""; onTriggered: sidebar.openSearch() }
        TMenuItem { text: Ui.mac ? qsTr("Reveal in Finder") : qsTr("Open File Location"); enabled: sidebar.app.folder !== ""; onTriggered: Qt.openUrlExternally("file://" + sidebar.app.folder) }
        TMenuItem { text: qsTr("Open Folder…"); onTriggered: sidebar.app.run("openFolder") }
        TMenuItem { text: qsTr("Refresh Folder"); enabled: sidebar.app.folder !== ""; onTriggered: sidebar.refresh() }
        TSep {}
        TMenuItem { text: qsTr("Group By Folder"); checkable: true; checked: sidebar.sortGrouped; onTriggered: sidebar.setSort(sidebar.sortKey, sidebar.sortDescending, !sidebar.sortGrouped) }
        SortItem { key: "natural"; label: qsTr("Sort Naturally") }
        SortItem { key: "name"; label: qsTr("Sort by Name") }
        SortItem { key: "modified"; label: qsTr("Sort by Modification Date") }
        SortItem { key: "created"; label: qsTr("Sort by Creation Date") }
        TSep { visible: sidebar.otherFolders.length > 0 }
        TMenuItem { visible: sidebar.otherFolders.length > 0; text: qsTr("Recent Locations"); enabled: false }
        RecentItem { index: 0 }
        RecentItem { index: 1 }
        RecentItem { index: 2 }
        RecentItem { index: 3 }
        RecentItem { index: 4 }
    }
    readonly property var otherFolders: Prefs.recentFolders.filter(p => p !== app.folder)
    // The order of the view the Files tab shows, the tree's or the list's.
    readonly property string sortKey: Ui.fileTree ? Ui.treeSort : Ui.fileSort
    readonly property bool sortDescending: Ui.fileTree ? Ui.treeSortDescending : Ui.fileSortDescending
    readonly property bool sortGrouped: Ui.fileTree ? Ui.treeGroupByFolder : Ui.groupByFolder
    function setSort(key, descending, grouped) {
        if (Ui.fileTree) {
            Ui.treeSort = key
            Ui.treeSortDescending = descending
            Ui.treeGroupByFolder = grouped
        } else {
            Ui.fileSort = key
            Ui.fileSortDescending = descending
            Ui.groupByFolder = grouped
        }
        sortTurns = Object.assign({}, sortTurns, { [key]: descending })
    }
    // Each order keeps the way it last went while the window is open, as
    // Typora's menu does, and goes that way again when picked.
    property var sortTurns: ({})
    component SortItem: TMenuItem {
        required property string key
        required property string label
        readonly property bool on: sidebar.sortKey === key
        readonly property bool descending: on ? sidebar.sortDescending : sidebar.sortTurns[key] === true
        text: label + " " + (descending ? qsTr("(Descending)") : qsTr("(Ascending)"))
        checkable: true
        checked: on
        // Picking the order in use turns it around.
        onTriggered: sidebar.setSort(key, on ? !descending : descending, sidebar.sortGrouped)
    }
    component RecentItem: TMenuItem {
        required property int index
        readonly property string path: index < sidebar.otherFolders.length ? sidebar.otherFolders[index] : ""
        visible: path !== ""
        text: sidebar.tilde(path)
        onTriggered: sidebar.app.openFolder(path)
    }

    TMenu {
        id: fileMenu
        readonly property string kind: sidebar.menuKind
        readonly property string path: sidebar.menuPath
        readonly property bool tree: Ui.fileTree
        readonly property bool file: kind === "file"
        readonly property bool folder: kind === "folder"
        readonly property bool search: kind === "search"
        readonly property bool blank: kind === "blank"
        readonly property bool root: kind === "root"
        readonly property bool full: !blank || !tree

        TMenuItem { visible: fileMenu.file || fileMenu.search; text: qsTr("Open"); onTriggered: sidebar.open(fileMenu.path) }
        TMenuItem { visible: fileMenu.file || fileMenu.search || (fileMenu.tree && (fileMenu.folder || fileMenu.root)); text: qsTr("Open in New Window"); onTriggered: Desktop.spawn(fileMenu.path) }
        TSep { visible: fileMenu.file || (fileMenu.tree && fileMenu.folder) }
        TMenuItem { visible: sidebar.undoLabel !== ""; text: qsTr("Undo %1").arg(sidebar.undoLabel); onTriggered: sidebar.undo() }
        TSep { visible: sidebar.undoLabel !== "" }
        TMenuItem { visible: !fileMenu.search; text: qsTr("New File"); onTriggered: sidebar.newFile(sidebar.folderFor(fileMenu.path, fileMenu.kind)) }
        TMenuItem { visible: fileMenu.tree && !fileMenu.search; text: qsTr("New Folder"); onTriggered: sidebar.newFolder(sidebar.folderFor(fileMenu.path, fileMenu.kind) || sidebar.app.folder) }
        TSep { visible: fileMenu.full }
        TMenuItem { visible: fileMenu.full; text: qsTr("Search"); checkable: true; checked: sidebar.searching; onTriggered: sidebar.searching ? sidebar.closeSearch() : sidebar.openSearch() }
        TSep { visible: fileMenu.full && !fileMenu.search }
        TMenuItem { visible: fileMenu.full && !fileMenu.search; text: qsTr("Articles (List View)"); checkable: true; checked: !Ui.fileTree; onTriggered: Ui.sidebarView = 1 }
        TMenuItem { visible: fileMenu.full && !fileMenu.search; text: qsTr("File Tree (Tree View)"); checkable: true; checked: Ui.fileTree; onTriggered: Ui.sidebarView = 2 }
        TSep { visible: fileMenu.file || (fileMenu.tree && fileMenu.folder) }
        TMenuItem { visible: fileMenu.file || (fileMenu.tree && fileMenu.folder); text: qsTr("Rename"); onTriggered: sidebar.renaming = fileMenu.path }
        TMenuItem { visible: fileMenu.file; text: qsTr("Duplicate"); onTriggered: sidebar.duplicate(fileMenu.path) }
        TSep { visible: fileMenu.file || (fileMenu.tree && fileMenu.folder) }
        TMenuItem { visible: fileMenu.file || (fileMenu.tree && fileMenu.folder); text: qsTr("Delete"); onTriggered: sidebar.remove(fileMenu.path) }
        TSep { visible: fileMenu.full && !fileMenu.blank }
        TMenuItem { visible: fileMenu.full && !fileMenu.blank; text: qsTr("Copy File Path"); onTriggered: Desktop.setClipboard(fileMenu.path, "", "") }
        TMenuItem { visible: fileMenu.full && !fileMenu.blank; text: Ui.mac ? qsTr("Reveal in Finder") : qsTr("Open File Location"); onTriggered: Desktop.showInFolder(fileMenu.path) }
        TSep { visible: fileMenu.search }
        TMenuItem { visible: fileMenu.search; text: qsTr("Expand All"); onTriggered: search.setAll(true) }
        TMenuItem { visible: fileMenu.search; text: qsTr("Collapse All"); onTriggered: search.setAll(false) }
    }
}
