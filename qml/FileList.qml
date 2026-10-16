import QtQuick
import QtQuick.Controls
import Typo

// The Files tab as Typora's articles list: every file in the folder and
// its subfolders, a card each with its folder, when it last changed, its
// name and the start of its text. Reloads when the folder changes on disk.
ListView {
    id: list

    required property var app
    required property var side
    readonly property string root: app.folder

    property bool more: false
    // Bumped on each reload, so the cards read their summaries again.
    property int version: 0
    // The time the dates are relative to, a minute at a time.
    property double now: Date.now()

    function reload() {
        if (root === "") {
            more = false
            items.clear()
            return
        }
        const data = JSON.parse(Folder.articles(root, Ui.fileSort, Ui.fileSortDescending, Ui.groupByFolder))
        more = data.more
        side.sync(items, more ? [] : data.items)
        version++
        now = Date.now()
    }
    onRootChanged: reload()
    Component.onCompleted: reload()
    Connections {
        target: Folder
        function onChanged() { list.reload() }
    }
    Connections {
        target: Ui
        function onFileSortChanged() { list.reload() }
        function onFileSortDescendingChanged() { list.reload() }
        function onGroupByFolderChanged() { list.reload() }
    }
    Timer {
        interval: 60000
        repeat: true
        running: list.visible
        onTriggered: list.now = Date.now()
    }

    // Typora's words for when: "just now", "5 minutes ago", a time today,
    // "Yesterday", "3 days ago", "Oct 01" this year, 2024-10-01 before.
    function naturalDate(ms) {
        if (!ms)
            return ""
        const e = new Date(ms), r = new Date(now), s = (r - e) / 1000
        const two = n => (n > 9 ? "" : "0") + n
        if (s < 30)
            return qsTr("just now")
        if (s < 60)
            return qsTr("1 minutes ago")
        if (s < 3600)
            return qsTr("%1 minutes ago").arg(Math.floor((30 + s) / 60))
        if (s < 18000 && r.getDay() === e.getDay())
            return two(e.getHours()) + ":" + two(e.getMinutes())
        if (s < 172800 && r.getDay() === (e.getDay() + 1) % 7)
            return qsTr("Yesterday")
        if (s < 518400)
            return qsTr("%1 days ago").arg(Math.floor(s / 86400))
        if (r.getFullYear() === e.getFullYear())
            return ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][e.getMonth()] + " " + two(e.getDate())
        return [e.getFullYear(), two(e.getMonth() + 1), two(e.getDate())].join("-")
    }

    clip: true
    boundsBehavior: Flickable.StopAtBounds
    ScrollBar.vertical: ScrollBar {}
    model: ListModel { id: items }

    delegate: AbstractButton {
        id: card
        required property int index
        required property string path
        required property string folder
        required property string name
        required property string ext
        required property double modified
        required property double created
        readonly property bool current: list.app.editor.path === path
        width: list.width
        height: 100
        hoverEnabled: true
        onClicked: list.side.open(path)
        TapHandler {
            acceptedButtons: Qt.RightButton
            onTapped: list.side.showMenu(card.path, "file")
        }
        background: Rectangle {
            color: card.current ? list.side.activeColor : card.hovered ? list.side.hoverColor : "transparent"
            Rectangle {
                visible: card.current
                width: 4
                height: parent.height
                color: Theme.accent
            }
            Rectangle {
                y: parent.height - 1
                width: parent.width
                height: 1
                color: Theme.border
                opacity: 0.6
            }
        }
        contentItem: Item {
            Text {
                x: 24
                y: 12
                width: parent.width - x - when.width - 28
                text: card.folder.substring(card.folder.lastIndexOf("/") + 1)
                color: Theme.faint
                font.pixelSize: 12
                elide: Text.ElideRight
            }
            Text {
                id: when
                anchors.right: parent.right
                anchors.rightMargin: 16
                y: 12
                text: list.naturalDate(Ui.fileSort === "created" ? card.created : card.modified)
                color: Theme.faint
                font.pixelSize: 12
            }
            Text {
                visible: list.side.renaming !== card.path
                x: 24
                y: 30
                width: parent.width - x - 16
                textFormat: Text.StyledText
                text: "<b>" + list.side.html(card.name) + "</b><font color='" + Theme.faint + "'>" + list.side.html(card.ext) + "</font>"
                color: Theme.foreground
                font.pixelSize: 14
                elide: Text.ElideRight
            }
            RenameField {
                x: 20
                y: 27
                width: parent.width - x - 12
                side: list.side
                path: card.path
                name: card.name + card.ext
            }
            Text {
                x: 24
                y: 52
                width: parent.width - x - 16
                text: { list.version; return Folder.summary(card.path) }
                color: Theme.muted
                font.pixelSize: 13
                lineHeightMode: Text.FixedHeight
                lineHeight: 18
                wrapMode: Text.Wrap
                maximumLineCount: 2
                elide: Text.ElideRight
            }
        }
    }

    // Right-clicking below the cards.
    TapHandler {
        acceptedButtons: Qt.RightButton
        onTapped: point => {
            if (list.itemAt(point.position.x, point.position.y + list.contentY) === null)
                list.side.showMenu("", "blank")
        }
    }

    Column {
        anchors.centerIn: parent
        width: parent.width - 48
        spacing: 12
        visible: list.more
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "⚠"
            color: Theme.faint
            font.pixelSize: 24
        }
        Text {
            width: parent.width
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            textFormat: Text.StyledText
            text: qsTr("Selected folders contains too many files. Please switch to <a href='tree'>File Tree view</a> for better performance.")
            color: Theme.muted
            linkColor: Theme.link
            font.pixelSize: 13
            onLinkActivated: Ui.sidebarView = 2
        }
    }
    Text {
        anchors.centerIn: parent
        visible: !list.more && list.count === 0 && list.root !== ""
        text: qsTr("No Files Available")
        color: Theme.faint
        font.pixelSize: 13
    }

    // Files dropped on the list from another program move into the open
    // folder, as Typora's do; a ring shows while they are over it.
    DropArea {
        id: drops
        parent: list
        anchors.fill: parent
        onEntered: drag => drag.accepted = drag.hasUrls && list.root !== ""
        onDropped: drop => {
            const files = drop.urls.map(u => u.toString()).filter(u => u.startsWith("file://"))
                .map(u => decodeURIComponent(u.substring(7)))
            if (files.length === 0)
                return
            drop.accept(Qt.CopyAction)
            Qt.callLater(() => list.side.moveInto(files, list.root))
        }
    }
    Rectangle {
        parent: list
        anchors.fill: parent
        anchors.margins: 1
        visible: drops.containsDrag
        color: "transparent"
        border.color: Theme.accent
        border.width: 2
    }
}
