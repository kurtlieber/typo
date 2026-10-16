import QtQuick
import QtQuick.Controls
import QtQuick.Window
import Typo

// Display math, typeset by the `image://math/` provider (src/qt/math.rs) and
// centred, or what is wrong with it. A new image loads off screen and
// replaces the old one when it is ready, so typing doesn't flicker. With
// `keepLast`, as in the preview under the source, the last formula that
// worked stays up with the error beneath it, as in Typora. A formula wider
// than the page scrolls sideways. Equation numbers (`tags`, one for each
// row of the formula, "" for none) sit at the right, level with their
// rows, which are taken to be of a height.
Item {
    id: view

    property string tex
    // The em of the text around it, in pixels.
    property real em: Ui.fontSize
    property bool keepLast: false
    // Off, it typesets nothing (a math block's preview while not editing).
    property bool live: true
    property var tags: []

    property string error: ""
    // The last image that worked, already in Qt's cache.
    property string good: ""

    readonly property real dpr: Screen.devicePixelRatio
    readonly property bool showFormula: good !== "" && (error === "" || keepLast)
    implicitHeight: (showFormula ? strip.height : 0) + (error !== "" ? message.height + (showFormula ? 8 * Ui.zoom : 0) : 0)

    // Typora's math is a little larger than its text, matching x-heights
    // between the text and math fonts.
    readonly property real mathEm: em * 1.18

    Image {
        id: loader
        visible: false
        asynchronous: true
        source: !view.live ? "" : "image://math/d/" + (view.mathEm * view.dpr).toFixed(2) + "/"
                + Theme.foreground.toString().slice(1) + "/" + encodeURIComponent(view.tex)
        onStatusChanged: {
            if (status !== Image.Ready)
                return
            view.error = Desktop.mathError(view.tex)
            if (view.error === "")
                view.good = source.toString()
        }
    }

    // Room left of the numbers. The formula is centred on the line, but
    // never runs under a number.
    readonly property real room: view.width - (tagColumn.width > 0 ? tagColumn.width + 8 * Ui.zoom : 0)

    Flickable {
        id: strip
        visible: view.showFormula
        width: (view.width + shown.width) / 2 > view.room ? view.room : view.width
        height: shown.height + (interactive ? bar.height : 0)
        contentWidth: Math.max(width, shown.width)
        contentHeight: shown.height
        flickableDirection: Flickable.HorizontalFlick
        boundsBehavior: Flickable.StopAtBounds
        interactive: shown.width > width + 0.5
        clip: true
        ScrollBar.horizontal: ScrollBar {
            id: bar
            policy: strip.interactive ? ScrollBar.AlwaysOn : ScrollBar.AlwaysOff
        }

        Image {
            id: shown
            x: (strip.contentWidth - width) / 2
            source: view.good
            width: implicitWidth / view.dpr
            height: implicitHeight / view.dpr
            smooth: true
            mipmap: true
        }
    }

    Item {
        id: tagColumn
        anchors.right: parent.right
        // The widest number.
        width: {
            let w = 0
            for (let i = 0; i < tagRepeater.count; i++) {
                const tag = tagRepeater.itemAt(i)
                if (tag && tag.visible)
                    w = Math.max(w, tag.width)
            }
            return w
        }
        height: shown.height

        Repeater {
            id: tagRepeater
            model: view.showFormula ? view.tags : []
            delegate: Image {
                required property string modelData
                required property int index
                visible: modelData !== ""
                anchors.right: parent.right
                y: (index + 0.5) * shown.height / view.tags.length - height / 2
                width: implicitWidth / view.dpr
                height: implicitHeight / view.dpr
                source: modelData === "" ? "" : "image://math/d/" + (view.mathEm * view.dpr).toFixed(2) + "/"
                    + Theme.foreground.toString().slice(1) + "/" + encodeURIComponent("\\text{" + modelData + "}")
                smooth: true
            }
        }
    }

    Text {
        id: message
        visible: view.error !== ""
        y: view.showFormula ? strip.height + 8 * Ui.zoom : 0
        width: view.width
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.Wrap
        text: view.error
        color: Theme.error
        font.family: Ui.textFont
        font.pixelSize: view.em * 0.9
    }
}
