import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Typo

// A modal question like Typora's: a title, a message and a row of buttons
// on the right, the first one the default unless `options.defaultButton`
// says otherwise. Enter picks the default and Escape the last (Cancel).
// `options.check` adds a check box under the message ("Do not show this
// again"). `ask` calls back with the chosen index and whether the box was
// ticked.
Popup {
    id: dialog

    property string title
    property string text
    property var buttons: []
    property var callback: null
    property int chosen: -1
    property int defaultButton: 0
    property string checkLabel: ""
    readonly property alias checkBox: checkBox
    // Room for wide text, such as a command's output; until it closes.
    property bool wide: false

    function ask(title, text, buttons, callback, options) {
        dialog.title = title
        dialog.text = text
        dialog.buttons = buttons
        dialog.callback = callback
        defaultButton = options && options.defaultButton !== undefined ? options.defaultButton : 0
        checkLabel = options && options.check ? options.check : ""
        checkBox.checked = false
        chosen = -1
        open()
    }
    function choose(i) {
        chosen = i
        close()
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(wide ? 640 : 440, parent.width - 32)
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape
    padding: 0

    onOpened: buttonRepeater.itemAt(defaultButton) && buttonRepeater.itemAt(defaultButton).forceActiveFocus()
    onClosed: {
        wide = false
        const cb = callback
        callback = null
        // Escape or a click outside counts as the last button.
        if (cb)
            cb(chosen < 0 ? buttons.length - 1 : chosen, checkBox.checked)
    }

    Overlay.modal: Rectangle {
        color: Qt.rgba(0, 0, 0, 0.35)
    }
    background: Rectangle {
        color: Theme.panel
        border.color: Theme.border
        radius: 6
    }

    contentItem: ColumnLayout {
        spacing: 0
        Keys.onReturnPressed: dialog.choose(dialog.defaultButton)
        Keys.onEnterPressed: dialog.choose(dialog.defaultButton)

        Text {
            text: dialog.title
            visible: text !== ""
            color: Theme.foreground
            font.family: Ui.uiFont
            font.pixelSize: 15
            font.bold: true
            Layout.fillWidth: true
            Layout.margins: 18
            Layout.bottomMargin: 0
        }
        Text {
            text: dialog.text
            textFormat: Text.StyledText
            wrapMode: Text.Wrap
            color: Theme.foreground
            font.family: Ui.uiFont
            font.pixelSize: 13
            lineHeight: 1.3
            Layout.fillWidth: true
            Layout.margins: 18
        }
        PrefCheck {
            id: checkBox
            visible: dialog.checkLabel !== ""
            text: dialog.checkLabel
            font.pixelSize: 13
            Layout.fillWidth: true
            Layout.leftMargin: 18
            Layout.rightMargin: 18
        }
        Row {
            id: buttonRow
            spacing: 8
            Layout.alignment: Qt.AlignRight
            Layout.margins: 14
            Layout.topMargin: 4
            Repeater {
                id: buttonRepeater
                model: dialog.buttons
                delegate: AbstractButton {
                    id: button
                    required property string modelData
                    required property int index
                    readonly property bool primary: index === dialog.defaultButton
                    text: modelData
                    padding: 6
                    leftPadding: 16
                    rightPadding: 16
                    focusPolicy: Qt.StrongFocus
                    Keys.onReturnPressed: dialog.choose(index)
                    Keys.onEnterPressed: dialog.choose(index)
                    Keys.onSpacePressed: dialog.choose(index)
                    Keys.onLeftPressed: nextItemInFocusChain(false).forceActiveFocus()
                    Keys.onRightPressed: nextItemInFocusChain(true).forceActiveFocus()
                    onClicked: dialog.choose(index)
                    contentItem: Text {
                        text: button.text
                        color: button.primary ? Theme.accentText : Theme.foreground
                        font.family: Ui.uiFont
                        font.pixelSize: 13
                        horizontalAlignment: Text.AlignHCenter
                    }
                    background: Rectangle {
                        radius: 4
                        color: button.primary ? Theme.accent : button.hovered ? Theme.selection : "transparent"
                        border.color: button.activeFocus ? Theme.foreground : button.primary ? Theme.accent : Theme.border
                    }
                }
            }
        }
    }
}
