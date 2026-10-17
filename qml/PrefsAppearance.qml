import QtQuick
import QtQuick.Controls
import Typo

// Preferences ▸ Appearance. The editor toolbar is a Typora feature typo
// doesn't have, so its group is left out.
Column {
    id: section

    required property var app

    width: parent ? parent.width : 0

    // Typora's zoom steps, each 1.2 times the last.
    readonly property int zoomLevel: Math.round(Math.log(Ui.zoom) / Math.log(1.2))

    PrefGroup {
        title: qsTr("Font Size")
        Row {
            spacing: 24
            RadioButton {
                id: autoSize
                readonly property bool searchHit: Ui.prefsHit(text)
                text: qsTr("Auto ( Recommended )")
                checked: !Prefs.useCustomFontSize
                onClicked: Prefs.useCustomFontSize = false
                font.family: Ui.uiFont
                font.pixelSize: 16
            }
            RadioButton {
                readonly property bool searchHit: Ui.prefsHit(text)
                text: qsTr("Customized")
                checked: Prefs.useCustomFontSize
                onClicked: Prefs.useCustomFontSize = true
                font.family: Ui.uiFont
                font.pixelSize: 16
            }
            PrefField {
                visible: Prefs.useCustomFontSize
                anchors.verticalCenter: autoSize.verticalCenter
                key: "customFontSize"
                numeric: true
                from: 9
                to: 24
                boxWidth: 56
                suffix: "px"
            }
        }
        Text {
            width: parent.width - 16
            wrapMode: Text.Wrap
            text: qsTr("If custom font size is used, the actual font size will depend on the base font size and css style.")
            color: Theme.muted
            font.family: Ui.uiFont
            font.pixelSize: 12
        }
    }

    PrefGroup {
        title: qsTr("Zoom")
        Row {
            spacing: 8
            PrefChoice {
                options: [[-3, "57%"], [-2, "69%"], [-1, "83%"], [0, "100%"], [1, "120%"], [2, "144%"], [3, "172%"],
                    [4, "207%"], [5, "248%"], [6, "298%"]]
                value: section.zoomLevel
                onPicked: v => Ui.zoom = Math.pow(1.2, v)
                boxWidth: 120
            }
            PrefButton {
                anchors.verticalCenter: parent.verticalCenter
                text: qsTr("Reset to Default")
                onClicked: Ui.zoom = 1
            }
        }
        PrefCheck {
            text: qsTr("Zoom when using mouse wheel and holding %1 key").arg("Ctrl")
            key: "zoomByWheel"
        }
    }

    PrefGroup {
        title: qsTr("Status Bar")
        PrefCheck {
            text: qsTr("Show status bar")
            target: Ui.object
            key: "statusBar"
        }
    }

    PrefGroup {
        title: qsTr("Word Count")
        PrefCheck {
            visible: Ui.mac
            text: qsTr("Always Show Word Count")
            key: "alwaysShowWordCount"
        }
        Row {
            spacing: 8
            PrefField {
                label: qsTr("Reading Speed") + ":"
                key: "readingSpeed"
                numeric: true
                from: 1
                to: 999
                boxWidth: 64
                suffix: qsTr("words / min")
            }
            ComboBox {
                id: speeds
                anchors.verticalCenter: parent.verticalCenter
                width: 220
                height: 28
                displayText: qsTr("Reset to Default")
                font.family: Ui.uiFont
                font.pixelSize: 14
                model: [qsTr("120  words/minute (Slow)"), qsTr("240  words/minute (Medium)"), qsTr("382  words/minute (Fast)")]
                onActivated: index => Prefs.readingSpeed = [120, 240, 382][index]
            }
        }
    }

    PrefGroup {
        title: qsTr("Themes")
        PrefChoice {
            label: qsTr("Theme")
            labelWidth: 80
            options: [["", Ui.mac ? qsTr("Follow System") : qsTr("Follow Omarchy")]].concat(Theme.themes.map(t => t.split("|")))
            value: Ui.theme
            onPicked: v => section.app.run("theme", v)
            boxWidth: 260
        }
        Item { width: 1; height: 6 }
        Row {
            spacing: 8
            PrefButton {
                text: qsTr("Open Theme Folder")
                onClicked: section.app.openThemeFolder()
            }
            PrefButton {
                text: qsTr("Get Themes")
                onClicked: Qt.openUrlExternally("https://learn.omacom.io/2/the-omarchy-manual/90/extra-themes")
            }
        }
    }
}
