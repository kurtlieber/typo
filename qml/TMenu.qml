import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Typo

// A menu styled like Typora's: label on the left, shortcut on the right,
// check marks for toggles, themed from Omarchy.
Menu {
    id: menu

    // Menus don't grow to fit their entries, so size to the widest.
    function fit() {
        let w = 220
        for (let i = 0; i < count; i++) {
            const item = itemAt(i)
            if (item)
                w = Math.max(w, (item.naturalWidth || item.implicitWidth) + leftPadding + rightPadding)
        }
        width = Math.ceil(w)
    }
    onAboutToShow: fit()
    onCountChanged: if (visible) fit()
    background: Rectangle {
        implicitWidth: 220
        color: Theme.panel
        border.color: Theme.border
        radius: 4
    }

    delegate: TMenuItem {}
}
