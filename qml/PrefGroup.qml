import QtQuick
import Typo

// A group of preferences, as Typora's: its title in bold with the controls
// beside it, or above them when the panel is narrow, and a hint under the
// title in small faint text.
Item {
    id: group

    property string title
    property string hint
    // What "Global Image Settings…" and the like point at.
    property string groupId
    default property alias content: controls.data
    readonly property bool wide: Ui.prefsWide
    readonly property bool searchHit: Ui.prefsHit(title)

    width: parent ? parent.width : 0
    implicitHeight: wide ? Math.max(header.height, controls.height) + 32
                         : controls.y + controls.height + 12

    Rectangle {
        visible: group.groupId !== "" && Ui.prefsHighlight === group.groupId
        x: -8
        y: -4
        width: group.width + 16
        height: group.implicitHeight - (group.wide ? 24 : 4)
        radius: 4
        color: Qt.alpha(Theme.mark, 0.35)
    }

    Column {
        id: header
        y: group.wide ? 0 : 24
        width: group.wide ? 200 : group.width
        spacing: 2
        Text {
            text: group.title
            width: parent.width
            wrapMode: Text.Wrap
            color: Theme.foreground
            font.family: Ui.uiFont
            font.pixelSize: 16
            font.bold: true
            lineHeight: group.wide ? 26 : 20
            lineHeightMode: Text.FixedHeight
            Rectangle {
                visible: group.searchHit
                z: -1
                width: parent.contentWidth
                height: parent.height
                color: Qt.alpha(Theme.mark, 0.48)
            }
        }
        Text {
            text: group.hint
            visible: text !== ""
            width: parent.width
            wrapMode: Text.Wrap
            color: Theme.muted
            font.family: Ui.uiFont
            font.pixelSize: 12
            lineHeight: 1.4
        }
    }

    Column {
        id: controls
        x: group.wide ? 226 : 16
        y: group.wide ? 0 : header.y + header.height + 4
        width: group.width - x
        spacing: 2
    }
}
