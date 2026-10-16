import QtQuick
import QtQuick.Controls
import Typo

MenuSeparator {
    height: visible ? implicitHeight : 0
    topPadding: 3
    bottomPadding: 3
    contentItem: Rectangle {
        implicitHeight: 1
        color: Theme.border
    }
}
