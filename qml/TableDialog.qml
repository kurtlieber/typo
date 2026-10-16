import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Typo

// Typora's Insert Table dialog: Columns (3) and Rows (4, the header
// included) side by side, Cancel and OK. Fields take digits only; Up and
// Down step them within 1–40 columns and 1–99 rows. Enter in Columns goes
// on to Rows, Enter in Rows is OK, Escape cancels. `ask` calls back with
// the columns and rows, or not at all when cancelled.
Popup {
    id: dialog

    property var callback: null
    readonly property alias columns: columns
    readonly property alias rows: rows

    function ask(callback) {
        dialog.callback = callback
        columns.text = "3"
        rows.text = "4"
        open()
    }
    function accept() {
        columns.fix()
        rows.fix()
        const cb = callback
        callback = null
        close()
        if (cb)
            cb(Number(columns.text), Number(rows.text))
    }

    parent: Overlay.overlay
    x: Math.round((parent.width - width) / 2)
    y: 100
    width: Math.min(600, parent.width - 32)
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape
    padding: 0

    onOpened: {
        columns.forceActiveFocus()
        columns.selectAll()
    }
    onClosed: callback = null

    Overlay.modal: Rectangle {
        color: Qt.rgba(0, 0, 0, 0.35)
    }
    background: Rectangle {
        color: Theme.panel
        border.color: Theme.border
        radius: 6
    }

    component Field: RowLayout {
        id: field
        property alias label: addon.text
        property alias text: input.text
        property int max: 99
        // What an empty field becomes when left, as in Typora.
        property int empty: 1
        property Item next: null
        signal entered()
        function forceActiveFocus() { input.forceActiveFocus() }
        function selectAll() { input.selectAll() }
        function fix() {
            if (input.text.trim() === "")
                input.text = String(empty)
            input.text = String(Math.max(1, Math.min(max, Number(input.text))))
        }
        spacing: 8
        Layout.fillWidth: true
        Layout.preferredWidth: 1

        Label {
            id: addon
            leftPadding: 12
            rightPadding: 12
            topPadding: 6
            bottomPadding: 6
            color: Theme.foreground
            font.family: Ui.uiFont
            font.pixelSize: 13
            background: Rectangle {
                color: Theme.background
                border.color: Theme.border
            }
        }
        TextField {
            id: input
            Layout.fillWidth: true
            color: Theme.foreground
            selectionColor: Theme.selection
            selectedTextColor: Theme.foreground
            font.family: Ui.uiFont
            font.pixelSize: 13
            validator: IntValidator { bottom: 0; top: 999 }
            background: Rectangle {
                color: Theme.background
                radius: 3
                border.color: input.activeFocus ? Theme.accent : Theme.border
            }
            onActiveFocusChanged: if (!activeFocus) field.fix()
            Keys.onUpPressed: text = String(Math.min(field.max, Number(text) + 1))
            Keys.onDownPressed: text = String(Math.max(1, Number(text) - 1))
            Keys.onReturnPressed: field.entered()
            Keys.onEnterPressed: field.entered()
            Keys.onTabPressed: event => {
                if (field.next) {
                    field.next.forceActiveFocus()
                    field.next.selectAll()
                } else {
                    event.accepted = false
                }
            }
        }
    }

    contentItem: ColumnLayout {
        spacing: 0

        Text {
            text: qsTr("Insert Table")
            color: Theme.foreground
            font.family: Ui.uiFont
            font.pixelSize: 16
            font.bold: true
            Layout.fillWidth: true
            Layout.margins: 15
            Layout.bottomMargin: 0
        }
        RowLayout {
            spacing: 20
            Layout.fillWidth: true
            Layout.margins: 15
            Field {
                id: columns
                label: qsTr("Columns")
                max: 40
                empty: 2
                next: rows
                onEntered: {
                    rows.forceActiveFocus()
                    rows.selectAll()
                }
            }
            Field {
                id: rows
                label: qsTr("Rows")
                max: 99
                empty: 1
                onEntered: dialog.accept()
            }
        }
        Row {
            spacing: 8
            Layout.alignment: Qt.AlignRight
            Layout.margins: 15
            Layout.topMargin: 0
            Repeater {
                model: [qsTr("Cancel"), qsTr("OK")]
                delegate: AbstractButton {
                    id: button
                    required property string modelData
                    required property int index
                    readonly property bool primary: index === 1
                    text: modelData
                    padding: 6
                    leftPadding: 16
                    rightPadding: 16
                    focusPolicy: Qt.StrongFocus
                    Keys.onReturnPressed: clicked()
                    Keys.onEnterPressed: clicked()
                    Keys.onSpacePressed: clicked()
                    onClicked: primary ? dialog.accept() : dialog.close()
                    contentItem: Text {
                        text: button.text
                        color: button.primary ? Theme.accentText : Theme.foreground
                        font.family: Ui.uiFont
                        font.pixelSize: 13
                        horizontalAlignment: Text.AlignHCenter
                    }
                    background: Rectangle {
                        implicitWidth: 13 * 5.14
                        radius: 4
                        color: button.primary ? Theme.accent : button.hovered ? Theme.selection : "transparent"
                        border.color: button.activeFocus ? Theme.foreground : button.primary ? Theme.accent : Theme.border
                    }
                }
            }
        }
    }
}
