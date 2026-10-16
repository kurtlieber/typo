import QtQuick
import QtQuick.Controls
import Typo

// Typora's Preferences panel. It covers the window under the menu bar: a
// header with the close button, the sections on the left under a Search
// field, and the chosen section's groups on the right. Searching marks
// the labels that match, dims the sections without any, and moves to the
// first section with one.
Rectangle {
    id: panel

    required property var app
    readonly property var names: [qsTr("Files"), qsTr("Editor"), qsTr("Image"), qsTr("Markdown"), qsTr("Export"),
        qsTr("Appearance"), qsTr("General")]
    readonly property var pages: [files, editor, image, markdown, exporting, appearance, general]
    // How many labels match the search in each section.
    property var hits: []
    readonly property bool searching: Ui.prefsQuery.trim() !== ""

    visible: Ui.prefsOpen
    color: Theme.background

    // Typora's layout steps with the window's width (electron.css).
    readonly property real sideWidth: width <= 800 ? 190 : width <= 1270 ? 235 : 200 + Math.max(0, (width - 1200) / 2)
    readonly property real contentLeft: width <= 800 ? 32 : width <= 1270 ? 60 : 120
    readonly property real contentRight: width <= 800 ? 24 : width <= 1270 ? 60 : 0
    Binding {
        target: Ui.object
        property: "prefsWide"
        value: panel.width > 1040
    }

    onVisibleChanged: {
        if (visible) {
            search.forceActiveFocus()
        } else {
            Ui.prefsQuery = ""
            app.focusPage()
        }
    }
    Keys.onEscapePressed: Ui.prefsOpen = false

    // For the tests: the section list, and the control showing a setting.
    readonly property alias nav: navList
    function pref(key, item = panel) {
        if (item.key === key)
            return item
        for (const child of item.children) {
            const hit = pref(key, child)
            if (hit)
                return hit
        }
        return null
    }

    function count(item) {
        let n = item.searchHit === true ? 1 : 0
        for (const child of item.children)
            n += count(child)
        return n
    }
    function recount() {
        hits = pages.map(p => count(p))
        if (!searching) {
            if (Ui.prefsSection < 0)
                Ui.prefsSection = 0
            return
        }
        if (Ui.prefsSection < 0 || !hits[Ui.prefsSection])
            Ui.prefsSection = hits.findIndex(n => n > 0)
    }
    Connections {
        target: Ui.object
        function onPrefsQueryChanged() { Qt.callLater(panel.recount) }
    }

    // The editor underneath takes no clicks or wheel turns.
    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.AllButtons
        onWheel: wheel => wheel.accepted = true
    }

    Rectangle {
        id: header
        z: 2
        width: parent.width
        height: 56
        color: Theme.background
        Text {
            x: 35
            height: parent.height
            verticalAlignment: Text.AlignVCenter
            text: qsTr("Preferences")
            color: Theme.foreground
            font.family: Ui.uiFont
            font.pixelSize: 16
        }
        AbstractButton {
            id: close
            anchors.right: parent.right
            width: 56
            height: parent.height
            focusPolicy: Qt.NoFocus
            onClicked: Ui.prefsOpen = false
            contentItem: Item {
                Rectangle {
                    width: 1
                    height: 24
                    anchors.verticalCenter: parent.verticalCenter
                    color: Qt.alpha(Theme.muted, 0.7)
                }
                Glyph {
                    anchors.centerIn: parent
                    name: "close"
                    size: 18
                    color: close.hovered ? Theme.accent : Theme.foreground
                }
            }
            background: null
        }
        // Typora's soft shadow under the header.
        Column {
            anchors.top: parent.bottom
            width: parent.width
            Repeater {
                model: [0.10, 0.06, 0.03, 0.015]
                Rectangle {
                    required property real modelData
                    width: parent.width
                    height: 1
                    color: Qt.alpha("black", Theme.dark ? modelData * 2 : modelData)
                }
            }
        }
    }

    Item {
        id: side
        anchors.top: header.bottom
        anchors.bottom: parent.bottom
        width: panel.sideWidth

        Column {
            anchors.right: parent.right
            y: panel.height <= 600 ? 16 : 42
            width: Math.min(200, side.width - 16)

            TextField {
                id: search
                width: parent.width
                height: 28
                placeholderText: qsTr("Search")
                font.family: Ui.uiFont
                font.pixelSize: 13
                selectByMouse: true
                onTextChanged: Ui.prefsQuery = text
                Keys.onEscapePressed: event => {
                    if (text !== "")
                        text = ""
                    else
                        Ui.prefsOpen = false
                }
                Connections {
                    target: Ui.object
                    function onPrefsOpenChanged() {
                        if (!Ui.prefsOpen)
                            search.text = ""
                    }
                }
            }
            Item { width: 1; height: 18 }
            Repeater {
                id: navList
                model: panel.names
                delegate: Item {
                    id: nav
                    required property string modelData
                    required property int index
                    readonly property bool active: Ui.prefsSection === index
                    width: parent.width
                    height: label.implicitHeight + 12 + 4
                    opacity: panel.searching && !panel.hits[index] ? 0.5 : 1
                    Rectangle {
                        y: 2
                        width: parent.width
                        height: parent.height - 4
                        radius: 2
                        color: nav.active ? Qt.alpha(Theme.foreground, Theme.dark ? 0.12 : 0.07)
                             : hover.containsMouse ? Qt.alpha(Theme.foreground, Theme.dark ? 0.07 : 0.04) : "transparent"
                    }
                    Text {
                        id: label
                        x: 11
                        anchors.verticalCenter: parent.verticalCenter
                        text: nav.modelData
                        color: Theme.foreground
                        font.family: Ui.uiFont
                        font.pixelSize: 16
                    }
                    MouseArea {
                        id: hover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: Ui.prefsSection = nav.index
                    }
                }
            }
        }
    }

    Flickable {
        id: pane
        anchors.top: header.bottom
        anchors.bottom: parent.bottom
        x: panel.sideWidth
        width: panel.width - x
        clip: true
        contentWidth: width
        contentHeight: body.height + 64
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {}
        // A new section starts at its top.
        Connections {
            target: Ui.object
            function onPrefsSectionChanged() { pane.contentY = 0 }
        }

        Column {
            id: body
            x: panel.contentLeft
            width: Math.min(760, pane.width - panel.contentLeft - panel.contentRight - 16)

            Text {
                visible: Ui.prefsSection >= 0
                topPadding: panel.height <= 600 ? 24 : 56
                bottomPadding: 28
                text: Ui.prefsSection >= 0 ? panel.names[Ui.prefsSection] : ""
                color: Theme.foreground
                font.family: Ui.uiFont
                font.pixelSize: 24
                font.bold: true
            }
            Text {
                visible: Ui.prefsSection < 0
                width: parent.width
                topPadding: pane.height * 0.4
                horizontalAlignment: Text.AlignHCenter
                text: qsTr("No results found for \"%1\"").arg(Ui.prefsQuery)
                color: Theme.foreground
                font.family: Ui.uiFont
                font.pixelSize: 16
            }

            PrefsFiles { id: files; app: panel.app; visible: Ui.prefsSection === 0 }
            PrefsEditor { id: editor; app: panel.app; visible: Ui.prefsSection === 1 }
            PrefsImage { id: image; app: panel.app; visible: Ui.prefsSection === 2 }
            PrefsMarkdown { id: markdown; app: panel.app; visible: Ui.prefsSection === 3 }
            PrefsExport { id: exporting; app: panel.app; visible: Ui.prefsSection === 4 }
            PrefsAppearance { id: appearance; app: panel.app; visible: Ui.prefsSection === 5 }
            PrefsGeneral { id: general; app: panel.app; visible: Ui.prefsSection === 6 }
        }
    }

    // "Global Image Settings…" and the like: the group they point at shows
    // marked for a few seconds.
    function show(section, group) {
        Ui.prefsSection = section
        Ui.prefsHighlight = group || ""
        Ui.prefsOpen = true
        if (group)
            unmark.restart()
    }
    Timer {
        id: unmark
        interval: 4000
        onTriggered: Ui.prefsHighlight = ""
    }
}
