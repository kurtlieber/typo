import QtQuick
import QtQuick.Controls
import Typo

// A button in the Preferences panel: square, outlined, Typora's
// `btn-default` on Linux.
AbstractButton {
    id: button

    readonly property bool searchHit: Ui.prefsHit(text)

    topPadding: 5
    bottomPadding: 5
    leftPadding: 12
    rightPadding: 12
    font.family: Ui.uiFont
    font.pixelSize: 14
    focusPolicy: Qt.StrongFocus
    Keys.onReturnPressed: clicked()
    Keys.onSpacePressed: clicked()

    contentItem: Text {
        text: button.text
        color: button.enabled ? Theme.foreground : Theme.faint
        font: button.font
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
    background: Rectangle {
        implicitHeight: 30
        color: button.searchHit ? Qt.alpha(Theme.mark, 0.48)
             : button.down ? Qt.alpha(Theme.foreground, 0.14)
             : button.hovered ? Qt.alpha(Theme.foreground, 0.07) : "transparent"
        border.color: button.activeFocus ? Theme.accent : Theme.border
    }
}
