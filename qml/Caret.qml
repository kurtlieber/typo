import QtQuick
import Typo

// A blinking caret in the theme's cursor colour.
Rectangle {
    id: caret
    width: 1.5
    color: Theme.cursor
    property bool blinking: parent && parent.activeFocus && !parent.readOnly
    // Hidden while something else has the pointer's attention, such as a
    // table row being dragged.
    property bool hidden: false
    visible: blinking && !hidden
    opacity: 1

    SequentialAnimation on opacity {
        running: caret.blinking
        loops: Animation.Infinite
        alwaysRunToEnd: false
        PropertyAction { value: 1 }
        PauseAnimation { duration: 530 }
        PropertyAction { value: 0 }
        PauseAnimation { duration: 530 }
    }
}
