import QtQuick
import QtQuick.Controls
import Typo

// A preference that is on or off. `key` names it in `target` (Prefs unless
// said otherwise); `inverted` shows one of Typora's "no…" settings the
// other way round. A hint shows on hovering the mark after the label.
CheckBox {
    id: check

    property var target: Prefs.object
    property string key
    property bool inverted: false
    property string hint
    readonly property bool searchHit: Ui.prefsHit(text)

    checked: key !== "" && (!!target[key] !== inverted)
    onToggled: if (key !== "") target[key] = checked !== inverted

    width: parent ? parent.width : implicitWidth
    topPadding: 3
    bottomPadding: 3
    leftPadding: 0
    spacing: 8
    font.family: Ui.uiFont
    font.pixelSize: 16

    // A box in the theme's colours, filled with the accent when on, level
    // with the label's first line.
    indicator: Rectangle {
        x: check.leftPadding
        y: check.topPadding + 3
        implicitWidth: 16
        implicitHeight: 16
        radius: 3
        opacity: check.enabled ? 1 : 0.45
        color: check.checked ? Theme.accent : "transparent"
        border.color: check.checked ? Theme.accent : check.hovered || check.visualFocus ? Theme.foreground : Theme.muted
        Glyph {
            anchors.centerIn: parent
            visible: check.checked
            name: "check"
            size: 12
            color: Theme.accentText
        }
    }

    contentItem: Item {
        implicitWidth: label.implicitWidth + (check.hint !== "" ? 24 : 0) + check.indicator.width + check.spacing
        implicitHeight: Math.max(20, label.implicitHeight)
        Text {
            id: label
            x: check.indicator.width + check.spacing
            width: Math.min(implicitWidth, parent.width - x - (check.hint !== "" ? 24 : 0))
            text: check.text
            wrapMode: Text.Wrap
            color: check.enabled ? Theme.foreground : Theme.faint
            font: check.font
            lineHeight: 1.15
            Rectangle {
                visible: check.searchHit
                z: -1
                width: parent.contentWidth
                height: parent.height
                color: Qt.alpha(Theme.mark, 0.48)
            }
        }
        PrefHint {
            text: check.hint
            x: label.x + label.width + 6
            y: (label.lineCount > 1 ? 2 : (label.height - height) / 2)
        }
    }
}
