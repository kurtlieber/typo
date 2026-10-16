import QtQuick
import QtQuick.Controls
import Typo

// The faint mark Typora puts after a preference with more to say; hovering
// it shows the text.
Item {
    id: hint

    property string text

    visible: text !== ""
    width: 16
    height: 16

    Glyph {
        anchors.centerIn: parent
        name: "alertNote"
        size: 14
        color: area.containsMouse ? Theme.foreground : Theme.muted
    }
    MouseArea {
        id: area
        anchors.fill: parent
        anchors.margins: -2
        hoverEnabled: true
    }
    ToolTip {
        visible: area.containsMouse
        delay: 300
        width: Math.min(implicitWidth, 420)
        contentItem: Text {
            text: hint.text
            textFormat: Text.StyledText
            wrapMode: Text.Wrap
            color: Theme.foreground
            font.family: Ui.uiFont
            font.pixelSize: 13
            lineHeight: 1.3
        }
        background: Rectangle {
            color: Theme.panel
            border.color: Theme.border
            radius: 4
        }
    }
}
