import QtQuick
import QtQuick.Controls
import Typo

// A preference picked from a list, as Typora's selects. `options` holds
// [value, label] pairs; `label`, when given, sits on the left in a column
// `labelWidth` wide, as in Typora's label-and-input tables.
Row {
    id: choice

    property var target: Prefs.object
    property string key
    property var options: []
    property string label
    property real labelWidth: label === "" ? 0 : labelText.implicitWidth + 12
    property real boxWidth: 240
    property string hint
    // The value shown; `key` keeps it unless the use says otherwise.
    property var value: key !== "" ? target[key] : undefined
    readonly property bool searchHit: Ui.prefsHit(label) || options.some(o => Ui.prefsHit(o[1]))
    signal picked(var value)

    spacing: 0
    height: Math.max(30, labelText.height)

    Text {
        id: labelText
        visible: choice.label !== ""
        width: choice.labelWidth
        anchors.verticalCenter: parent.verticalCenter
        text: choice.label
        wrapMode: Text.Wrap
        color: choice.enabled ? Theme.foreground : Theme.faint
        font.family: Ui.uiFont
        font.pixelSize: 16
        Rectangle {
            visible: Ui.prefsHit(choice.label)
            z: -1
            width: parent.contentWidth
            height: parent.height
            color: Qt.alpha(Theme.mark, 0.48)
        }
    }
    ComboBox {
        id: box
        width: choice.boxWidth
        height: 28
        anchors.verticalCenter: parent.verticalCenter
        model: choice.options.map(o => o[1])
        currentIndex: choice.options.findIndex(o => String(o[0]) === String(choice.value))
        font.family: Ui.uiFont
        font.pixelSize: 14
        onActivated: index => {
            const v = choice.options[index][0]
            if (choice.key !== "")
                choice.target[choice.key] = v
            choice.picked(v)
        }
    }
    PrefHint {
        text: choice.hint
        anchors.verticalCenter: parent.verticalCenter
        x: box.x + box.width + 8
    }
}
