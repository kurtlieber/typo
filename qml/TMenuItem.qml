import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Typo

// A menu entry: label left, shortcut right, check mark for toggles.
MenuItem {
    id: item
    implicitHeight: 28
    // Hidden entries take no room.
    height: visible ? implicitHeight : 0
    // Arrow keys skip entries that can't take focus, so hidden ones mustn't.
    focusPolicy: visible ? Qt.StrongFocus : Qt.NoFocus
    // A check mark shows what its binding says. A click checks an entry
    // before it runs, and would uncheck a checked one where the state it
    // shows might not change (picking the sort in use turns it around), so
    // the mark would go stale; alone in an exclusive group, a checked entry
    // stays checked until its binding says otherwise.
    ButtonGroup.group: ButtonGroup {}
    // Shortcut text for an entry whose action must not own the shortcut
    // (the menu bar's does).
    property string hint: ""
    readonly property string keys: hint !== "" ? Prefs.keysFor(text, hint) : item.action && item.action.shortcut ? String(item.action.shortcut) : ""
    // The layout's implicit width lags until it is polished, so the menu
    // sizes itself from the texts directly.
    readonly property real naturalWidth: label.implicitWidth + (keys !== "" ? 24 + shortcut.implicitWidth : 0)
                                         + leftPadding + rightPadding + (subMenu ? 16 : 0) + 8
    contentItem: RowLayout {
        spacing: 24
        Text {
            id: label
            text: item.text.replace("&", "")
            color: item.enabled ? (item.highlighted ? Theme.accentText : Theme.foreground) : Theme.faint
            font: item.font
            Layout.fillWidth: true
            leftPadding: 18
            elide: Text.ElideRight
        }
        Text {
            id: shortcut
            text: item.keys
            visible: text !== ""
            color: item.highlighted ? Theme.accentText : Theme.faint
            font: item.font
            rightPadding: item.subMenu ? 0 : 6
        }
    }
    indicator: Text {
        x: 6
        anchors.verticalCenter: parent.verticalCenter
        text: item.checkable && item.checked ? "✓" : ""
        color: item.highlighted ? Theme.accentText : Theme.foreground
        font: item.font
    }
    arrow: Text {
        x: item.width - width - 8
        anchors.verticalCenter: parent.verticalCenter
        visible: item.subMenu
        text: "›"
        color: item.highlighted ? Theme.accentText : Theme.foreground
        font.pixelSize: item.font.pixelSize + 2
    }
    background: Rectangle {
        implicitWidth: 200
        color: item.highlighted ? Theme.accent : "transparent"
        radius: 3
    }
}
