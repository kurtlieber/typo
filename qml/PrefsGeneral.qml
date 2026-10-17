import QtQuick
import QtQuick.Controls
import Typo

// Preferences ▸ General. Language, updates, the debug switch and usage
// info are Typora's own services, so their groups are left out.
Column {
    id: section

    required property var app

    width: parent ? parent.width : 0

    PrefGroup {
        visible: Ui.mac
        title: qsTr("Quit")
        PrefCheck {
            text: qsTr("Quit typo when last window is closed")
            key: "quitAfterWindowClose"
        }
    }

    PrefGroup {
        title: qsTr("Shortcut Keys")
        PrefButton {
            text: qsTr("Custom Shortcut Keys")
            onClicked: section.app.openAdvancedSettings()
        }
        Text {
            width: parent.width - 16
            wrapMode: Text.Wrap
            text: qsTr("Map a menu item's label to keys under \"keyBinding\", e.g. \"Always on Top\": \"Ctrl+Shift+P\".")
            color: Theme.muted
            font.family: Ui.uiFont
            font.pixelSize: 12
            topPadding: 4
        }
    }

    PrefGroup {
        title: qsTr("Dialogs")
        PrefButton {
            text: qsTr("Reset All Dialog Warnings")
            onClicked: {
                Prefs.noFocusModeNotice = false
                Prefs.noTypewriterNotice = false
                Prefs.noExportOverwriteWarning = false
                section.app.notify(qsTr("Dialog warnings have been reset."))
            }
        }
    }

    PrefGroup {
        title: qsTr("Advanced Settings")
        Row {
            spacing: 16
            PrefButton {
                text: qsTr("Open Advanced Settings")
                onClicked: section.app.openAdvancedSettings()
            }
            PrefButton {
                text: qsTr("Reset Advanced Settings")
                onClicked: section.app.ask("", qsTr("Reset Advanced Settings"), [qsTr("Reset"), qsTr("Cancel")],
                                           i => { if (i === 0) section.app.resetAdvancedSettings() })
            }
        }
    }
}
