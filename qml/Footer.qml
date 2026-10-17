import QtQuick
import QtQuick.Controls
import Typo

// The status bar under the editor: sidebar and source-mode toggles on the
// left, the word count (which opens the count popover) on the right.
Rectangle {
    id: footer
    required property var app
    implicitHeight: 30
    color: Theme.background

    readonly property var editor: app.editor
    readonly property int minutes: Math.round(editor.words / Math.max(1, Prefs.readingSpeed))

    // The selection's counts, which Typora takes a moment after it changes.
    property int selWords: 0
    property int selChars: 0
    readonly property bool selected: selWords > 0 || selChars > 0
    function updateSelection() {
        const v = app.view
        const s = v.selection()
        const t = v.tableAt()
        const ends = t ? t.selectionEnds() : null
        if (Ui.sourceMode || (t ? !ends : v.collapsed(s))) {
            selWords = 0
            selChars = 0
            return
        }
        const c = (t ? editor.cellSpanCounts(s.zRow, ends.a.r, ends.a.c, ends.a.pos, ends.z.r, ends.z.c, ends.z.pos)
                     : editor.selectionCounts(s.aRow, s.aPos, s.zRow, s.zPos)).split(",")
        selWords = Number(c[0])
        selChars = Number(c[1])
    }
    Timer {
        id: selTimer
        interval: 50
        onTriggered: footer.updateSelection()
    }
    Connections {
        target: footer.app.view
        function onCaretMoved() { selTimer.restart() }
        function onSpanChanged() { selTimer.restart() }
    }

    // The count as Typora words it: while some (not all) of the document
    // is selected, the selection's words or characters out of the whole;
    // reading time rounded, under a minute "≤ 1 Minute".
    function countText(mode) {
        const of = (sel, all, one, many) => {
            const part = Math.min(sel, all)
            return part > 0 && part < all ? part + " / " + all + " " + many : all + " " + (all === 1 ? one : many)
        }
        switch (mode) {
        case 1: return of(selChars, editor.characters, qsTr("Character"), qsTr("Characters"))
        case 2: return editor.lines + " " + (editor.lines === 1 ? qsTr("Line") : qsTr("Lines"))
        case 3: return minutes === 0 ? "≤ 1 " + qsTr("Minute") : minutes + " " + (minutes === 1 ? qsTr("Minute") : qsTr("Minutes"))
        default: return of(selWords, editor.words, qsTr("Word"), qsTr("Words"))
        }
    }

    // Typora's footer items: as tall as the bar, shaded under the pointer.
    readonly property color hoverColor: Qt.alpha(Theme.foreground, Theme.dark ? 0.08 : 0.06)
    component FooterButton: AbstractButton {
        id: button
        property string tip
        property bool shaded: false
        implicitHeight: 30
        implicitWidth: implicitContentWidth + leftPadding + rightPadding
        hoverEnabled: true
        ToolTip.visible: hovered && tip !== ""
        ToolTip.text: tip
        ToolTip.delay: 600
        background: Rectangle {
            color: button.hovered || button.shaded ? footer.hoverColor : "transparent"
        }
    }

    Row {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter

        // The sidebar toggle: a ring while the sidebar is hidden (filled
        // under the pointer), a chevron to close it while it shows. Source
        // Code Mode has none.
        FooterButton {
            id: sidebarButton
            objectName: "sidebarButton"
            tip: qsTr("Toggle Sidebar")
            visible: !Ui.sourceMode
            implicitWidth: 30
            onClicked: footer.app.run("sidebar")
            contentItem: Item {
                Glyph {
                    anchors.centerIn: parent
                    size: Ui.sidebar ? 14 : 13
                    name: Ui.sidebar ? "left" : sidebarButton.hovered ? "ringOn" : "ringOff"
                    color: Theme.foreground
                    opacity: sidebarButton.hovered ? 1 : 0.75
                }
            }
        }
        // Source Code Mode, which says how to leave it while it is on.
        FooterButton {
            id: sourceButton
            objectName: "sourceButton"
            tip: qsTr("Toggle Source Code Mode")
            shaded: Ui.sourceMode
            leftPadding: Ui.sourceMode ? 10 : 8
            rightPadding: 8
            onClicked: footer.app.run("sourceMode")
            contentItem: Row {
                spacing: 5
                opacity: 0.8
                Glyph {
                    anchors.verticalCenter: parent.verticalCenter
                    name: "code"
                    size: 14
                    color: Theme.foreground
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    visible: Ui.sourceMode
                    text: qsTr("Exit Source Code Mode")
                    color: Theme.foreground
                    font.pixelSize: 12
                }
            }
        }
    }

    readonly property alias wordCount: count
    readonly property alias sidebarButton: sidebarButton
    readonly property alias sourceButton: sourceButton

    FooterButton {
        id: count
        // Typora hides the count in Source Code Mode.
        visible: !Ui.sourceMode && (Prefs.alwaysShowWordCount || footer.hovered)
        anchors.right: parent.right
        anchors.rightMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        leftPadding: 12
        rightPadding: 12
        shaded: popover.visible
        onClicked: popover.visible ? popover.close() : popover.open()
        contentItem: Row {
            spacing: 2
            opacity: count.hovered || popover.visible ? 1 : 0.75
            Text {
                id: label
                anchors.verticalCenter: parent.verticalCenter
                text: footer.countText(Prefs.countMode)
                color: Theme.foreground
                font.pixelSize: 12
            }
            // Arrows that say it opens, while the pointer is on the bar.
            Glyph {
                anchors.verticalCenter: parent.verticalCenter
                name: "upDown"
                size: 12
                color: Theme.foreground
                opacity: footer.hovered || popover.visible ? 1 : 0
                Behavior on opacity { NumberAnimation { duration: 300 } }
            }
        }
    }
    readonly property bool hovered: hover.hovered
    HoverHandler { id: hover }

    // Typora's count menu: the totals (reading time, lines, words,
    // characters), each a row that puts it in the status bar, then the
    // selection's words and characters while there is one.
    component CountHeading: Text {
        leftPadding: 20
        rightPadding: 20
        topPadding: 2
        bottomPadding: 2
        color: Theme.foreground
        opacity: 0.7
        font.pixelSize: 14
    }
    component CountDivider: Item {
        width: popover.rowWidth
        height: 9
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width
            height: 1
            color: Theme.border
        }
    }
    component CountRow: AbstractButton {
        id: countRow
        property string number
        property string label
        width: popover.rowWidth
        height: Math.round(14 * 1.8)
        hoverEnabled: enabled
        background: Rectangle {
            color: countRow.hovered ? Theme.accent : "transparent"
        }
        contentItem: Row {
            Text {
                width: popover.numberWidth + 40
                leftPadding: 20
                rightPadding: 20
                horizontalAlignment: Text.AlignRight
                anchors.verticalCenter: parent.verticalCenter
                text: countRow.number
                color: countRow.hovered ? Theme.accentText : Theme.foreground
                font.pixelSize: 14
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: countRow.label
                color: countRow.hovered ? Theme.accentText : Theme.foreground
                font.pixelSize: 14
            }
        }
    }

    Popup {
        id: popover
        objectName: "countMenu"
        x: footer.width - width - 4
        y: -height - 4
        padding: 0
        topPadding: 8
        bottomPadding: 8
        onAboutToShow: footer.updateSelection()
        background: Rectangle {
            color: Theme.panel
            border.color: Theme.border
            radius: 4
        }
        FontMetrics {
            id: countMetrics
            font.pixelSize: 14
        }
        readonly property var totals: [
            { mode: 3, number: footer.minutes === 0 ? "≤ 1" : String(footer.minutes), label: qsTr("Minutes") },
            { mode: 2, number: String(footer.editor.lines), label: qsTr("Lines") },
            { mode: 0, number: String(footer.editor.words), label: qsTr("Words") },
            { mode: 1, number: String(footer.editor.characters), label: qsTr("Characters") }
        ]
        readonly property var inSelection: [
            { number: String(footer.selWords), label: qsTr("Words") },
            { number: String(footer.selChars), label: qsTr("Characters") }
        ]
        // As a table lays them out: the numbers right-aligned in a column
        // as wide as the widest.
        readonly property real numberWidth: Math.ceil(Math.max(...totals.concat(inSelection).map(r => countMetrics.advanceWidth(r.number))))
        readonly property real rowWidth: Math.max(160, numberWidth + 40 + Math.ceil(countMetrics.advanceWidth(qsTr("Characters"))) + 24)
        contentItem: Column {
            CountHeading {
                text: footer.selected ? qsTr("Total") : qsTr("Word Count")
            }
            CountDivider {}
            Item {
                width: 1
                height: 4
            }
            Repeater {
                id: totalRows
                model: popover.totals
                delegate: CountRow {
                    required property var modelData
                    number: modelData.number
                    label: modelData.label
                    onClicked: {
                        Prefs.countMode = modelData.mode
                        popover.close()
                    }
                }
            }
            Column {
                visible: footer.selected
                Item {
                    width: 1
                    height: 16
                }
                CountHeading {
                    text: qsTr("In Selection")
                }
                CountDivider {}
                Repeater {
                    model: popover.inSelection
                    delegate: CountRow {
                        required property var modelData
                        enabled: false
                        number: modelData.number
                        label: modelData.label
                    }
                }
            }
        }
    }
    readonly property alias countMenu: popover
    readonly property alias countRows: totalRows

    function toggleCount() {
        popover.visible ? popover.close() : popover.open()
    }
}
