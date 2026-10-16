import QtQuick
import QtQuick.Controls
import Typo

// A preference typed in: a line of text, written back when it is left or
// Enter is pressed. `label`, when given, sits on the left as in PrefChoice.
// With `numeric`, only whole numbers from `from` to `to` are taken.
Row {
    id: field

    property var target: Prefs.object
    property string key
    property string label
    property real labelWidth: label === "" ? 0 : labelText.implicitWidth + 12
    property real boxWidth: 240
    property string placeholder
    property string suffix
    property bool numeric: false
    property int from: 0
    property int to: 9999
    property alias input: input
    readonly property bool searchHit: Ui.prefsHit(label)
    // A value given when it is written back, for uses without a key.
    signal edited(var value)

    spacing: 0
    height: Math.max(30, labelText.height)

    function commit() {
        let v = input.text
        if (numeric) {
            v = parseInt(v, 10)
            if (isNaN(v))
                v = key !== "" ? target[key] : from
            v = Math.max(from, Math.min(to, v))
            input.text = String(v)
        }
        if (key !== "" && target[key] !== v)
            target[key] = v
        edited(v)
    }

    Text {
        id: labelText
        visible: field.label !== ""
        width: field.labelWidth
        anchors.verticalCenter: parent.verticalCenter
        text: field.label
        wrapMode: Text.Wrap
        color: field.enabled ? Theme.foreground : Theme.faint
        font.family: Ui.uiFont
        font.pixelSize: 16
        Rectangle {
            visible: field.searchHit
            z: -1
            width: parent.contentWidth
            height: parent.height
            color: Qt.alpha(Theme.mark, 0.48)
        }
    }
    TextField {
        id: input
        width: field.boxWidth
        height: 28
        anchors.verticalCenter: parent.verticalCenter
        text: field.key !== "" ? String(field.target[field.key]) : ""
        placeholderText: field.placeholder
        validator: field.numeric ? numbers : null
        selectByMouse: true
        font.family: Ui.uiFont
        font.pixelSize: 14
        onEditingFinished: field.commit()
        IntValidator {
            id: numbers
            bottom: field.from
            top: field.to
        }
    }
    Text {
        visible: field.suffix !== ""
        anchors.verticalCenter: parent.verticalCenter
        leftPadding: 6
        text: field.suffix
        color: Theme.foreground
        font.family: Ui.uiFont
        font.pixelSize: 14
    }
}
