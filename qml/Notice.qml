import QtQuick
import QtQuick.Controls
import Typo

// Typora's notification over the top of the page (`#md-notification`): a
// bell, what happened, a fainter line under it, "Do not show this again"
// and Dismiss. Turning focus or typewriter mode on shows one, and so does
// an export: "Exporting in progress..." with a spinner while it runs, then
// how it went, with a button that shows the file. The page moves down
// under it.
Rectangle {
    id: notice

    // The `Prefs` flag "Do not show this again" sets; empty for a notice
    // without the box.
    property string key: ""
    // Styled text: `<b>Focus Mode</b> has been turned on.`
    property string title: ""
    property string body: ""
    // A button before Dismiss, and what it does.
    property string action: ""
    property var callback: null
    // Something is in progress: a spinner, and no buttons.
    property bool busy: false
    readonly property alias neverBox: never
    readonly property alias dismissButton: dismiss
    readonly property alias actionButton: act

    function show(key, title, body) {
        if (Prefs[key])
            return
        tell(title, body, "", null)
        notice.key = key
    }
    // A notice with no "Do not show this again".
    function tell(title, body, action, callback) {
        notice.key = ""
        notice.title = title
        notice.body = body
        notice.action = action
        notice.callback = callback
        notice.busy = false
        never.checked = false
        visible = true
    }
    function progress(title) {
        tell(title, "", "", null)
        busy = true
    }
    function close() {
        visible = false
        busy = false
    }

    visible: false
    height: 64
    color: Theme.background

    // Its shadow on the page under it, as the find bar's.
    Rectangle {
        y: parent.height
        width: parent.width
        height: 6
        gradient: Gradient {
            GradientStop { position: 0; color: Qt.alpha(Theme.foreground, 0.12) }
            GradientStop { position: 1; color: "transparent" }
        }
    }
    Rectangle {
        y: parent.height - 1
        width: parent.width
        height: 1
        color: Theme.border
    }

    Glyph {
        x: 26
        y: 11
        name: "bell"
        size: 16
        color: Theme.foreground
    }
    Text {
        x: 64
        y: 9
        width: parent.width - x - 24
        text: notice.title
        textFormat: Text.StyledText
        elide: Text.ElideRight
        color: Theme.foreground
        font.family: Ui.uiFont
        font.pixelSize: 14
    }
    Text {
        x: 64
        y: actions.y + (actions.height - height) / 2
        width: actions.x - x - 16
        text: notice.body
        elide: Text.ElideRight
        opacity: 0.7
        color: Theme.foreground
        font.family: Ui.uiFont
        font.pixelSize: 13
    }
    // Typora's export spinner: five bars that stretch in turn.
    Row {
        visible: notice.busy
        anchors.right: parent.right
        anchors.rightMargin: 24
        anchors.verticalCenter: parent.verticalCenter
        spacing: 2
        Repeater {
            model: 5
            Rectangle {
                id: bar
                required property int index
                anchors.verticalCenter: parent.verticalCenter
                width: 3
                height: 18
                color: Theme.muted
                transform: Scale { id: stretch; origin.y: 9; yScale: 0.4 }
                SequentialAnimation {
                    running: notice.busy && notice.visible
                    loops: Animation.Infinite
                    PauseAnimation { duration: bar.index * 100 }
                    NumberAnimation { target: stretch; property: "yScale"; to: 1; duration: 240 }
                    NumberAnimation { target: stretch; property: "yScale"; to: 0.4; duration: 240 }
                    PauseAnimation { duration: (4 - bar.index) * 100 + 240 }
                }
            }
        }
    }

    Row {
        id: actions
        visible: !notice.busy
        anchors.right: parent.right
        anchors.rightMargin: 16
        y: 32
        spacing: 16

        // Typora saves the choice as soon as the box changes.
        AbstractButton {
            id: never
            visible: notice.key !== ""
            checkable: true
            anchors.verticalCenter: parent.verticalCenter
            opacity: 0.7
            onToggled: Prefs[notice.key] = checked
            contentItem: Row {
                spacing: 8
                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 13
                    height: 13
                    radius: 2
                    color: never.checked ? Theme.accent : "transparent"
                    border.color: never.checked ? Theme.accent : Theme.muted
                    Glyph {
                        anchors.centerIn: parent
                        visible: never.checked
                        name: "check"
                        size: 11
                        color: Theme.accentText
                    }
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Do not show this again")
                    color: Theme.foreground
                    font.family: Ui.uiFont
                    font.pixelSize: 13
                }
            }
        }
        AbstractButton {
            id: act
            visible: notice.action !== ""
            padding: 3
            leftPadding: 10
            rightPadding: 10
            onClicked: {
                const f = notice.callback
                notice.close()
                if (f)
                    f()
            }
            contentItem: Text {
                text: notice.action
                color: Theme.foreground
                font.family: Ui.uiFont
                font.pixelSize: 13
                horizontalAlignment: Text.AlignHCenter
            }
            background: Rectangle {
                radius: 4
                color: act.hovered ? Theme.selection : "transparent"
                border.color: Theme.border
            }
        }
        AbstractButton {
            id: dismiss
            padding: 3
            leftPadding: 10
            rightPadding: 10
            onClicked: notice.close()
            contentItem: Text {
                text: qsTr("Dismiss")
                color: Theme.foreground
                font.family: Ui.uiFont
                font.pixelSize: 13
                horizontalAlignment: Text.AlignHCenter
            }
            background: Rectangle {
                radius: 4
                color: dismiss.hovered ? Theme.selection : "transparent"
                border.color: Theme.border
            }
        }
    }
}
