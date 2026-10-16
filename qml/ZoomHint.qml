import QtQuick
import QtQuick.Controls
import Typo

// Typora's zoom panel (`#zoom-hint`): after any zoom, the page's size in
// the top right corner for three seconds, with buttons to zoom out, in, and
// back to actual size.
Rectangle {
    id: hint
    required property var app
    // Typora drops the fraction: 0.833… shows as 83%.
    readonly property string percent: Math.floor(Ui.zoom * 100 + 1e-9) + "%"
    readonly property alias zoomOut: outButton
    readonly property alias zoomIn: inButton
    readonly property alias reset: resetButton

    function show() {
        visible = true
        hide.restart()
    }

    visible: false
    width: row.width + 12
    height: row.height
    // On a dark interface, Typora darkens what is under it instead.
    color: Theme.dark ? Qt.rgba(0, 0, 0, 0.5) : Theme.background

    Timer {
        id: hide
        interval: 3000
        onTriggered: hint.visible = false
    }

    // A soft halo, as its `box-shadow: 0 0 .5em`.
    Repeater {
        model: 4
        delegate: Rectangle {
            required property int index
            anchors.fill: parent
            anchors.margins: -1.5 * (index + 1)
            radius: 1.5 * (index + 1)
            z: -1
            color: "transparent"
            border.width: 1.5
            border.color: Qt.rgba(0, 0, 0, 0.07 - 0.015 * index)
        }
    }

    component HintButton: AbstractButton {
        id: button
        hoverEnabled: true
        padding: 4
        topInset: 4
        bottomInset: 4
        topPadding: 8
        bottomPadding: 8
        background: Rectangle {
            color: !button.hovered ? "transparent" : Theme.dark ? Qt.rgba(0, 0, 0, 0.7) : Qt.rgba(134 / 255, 134 / 255, 134 / 255, 0.26)
        }
    }

    Row {
        id: row
        x: 12

        Text {
            width: 48
            height: parent.height
            verticalAlignment: Text.AlignVCenter
            text: hint.percent
            color: Theme.foreground
            font.family: Ui.uiFont
            font.pixelSize: 14
        }
        HintButton {
            id: outButton
            width: 24
            onClicked: hint.app.run("zoom", -1)
            contentItem: Item {
                implicitHeight: 18
                Glyph {
                    anchors.centerIn: parent
                    name: "minus"
                    size: 12
                    color: Theme.foreground
                }
            }
        }
        HintButton {
            id: inButton
            width: 24
            onClicked: hint.app.run("zoom", 1)
            contentItem: Item {
                implicitHeight: 18
                Glyph {
                    anchors.centerIn: parent
                    name: "plus"
                    size: 12
                    color: Theme.foreground
                }
            }
        }
        // "Reset", in the accent, set off by a rule.
        Item {
            width: resetButton.width + 8
            height: parent.height
            HintButton {
                id: resetButton
                x: 4
                leftPadding: 8
                rightPadding: 8
                onClicked: hint.app.run("zoom", 0)
                contentItem: Text {
                    text: qsTr("Reset")
                    height: 18
                    verticalAlignment: Text.AlignVCenter
                    color: Theme.accent
                    font.family: Ui.uiFont
                    font.pixelSize: 12
                }
                Rectangle {
                    y: 4
                    width: 1
                    height: parent.height - 8
                    color: Qt.rgba(134 / 255, 134 / 255, 134 / 255, 0.6)
                }
            }
        }
    }
}
