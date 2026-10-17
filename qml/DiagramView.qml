import QtQuick
import QtQuick.Window
import Typo

// A diagram fence's drawing, from the `image://diagram/` provider
// (src/qt/diagram.rs), laid out as Typora's diagram panel: centred, shrunk
// to the width but never enlarged, "Painting Diagram..." while there is
// nothing to show, and what is wrong in red beneath, left-aligned, with
// `mark` saying where in the code (for the fence's red wave). A new
// drawing loads off screen and replaces the old one when it is ready, so
// typing doesn't flicker; one that fails clears it, as Typora's does.
Item {
    id: view

    // `mermaid`, `flow` or `sequence`, and the fence's code.
    property string lang
    property string code
    // The em of the text around it, in pixels, and the editor's zoom.
    property real em: Ui.fontSize
    property real zoom: Ui.zoom
    // Off, it draws nothing (a fence that isn't a diagram).
    property bool live: true

    property string error: ""
    property string mark: "[]"
    // The drawing shown, already in Qt's cache.
    property string good: ""

    readonly property real dpr: Screen.devicePixelRatio
    readonly property bool drawn: good !== ""
    readonly property real pad: 8 * zoom
    // Typora gives sequence diagrams more room above.
    readonly property real above: (lang === "sequence" ? 16 : 0) * zoom + pad
    readonly property string colors: [Theme.background, Theme.foreground, Theme.accent, Theme.border]
        .map(c => c.toString().slice(1)).join(",")

    // An edit moves the code under the old mark.
    onCodeChanged: mark = "[]"

    implicitHeight: above + (drawn ? shown.height : painting.height + 10 * zoom)
        + (error !== "" ? message.height : 0) + pad

    Image {
        id: loader
        visible: false
        asynchronous: true
        source: !view.live ? "" : "image://diagram/" + view.lang + "/" + (view.zoom * view.dpr).toFixed(2) + "/"
                + view.colors + "/" + encodeURIComponent(view.code)
        onStatusChanged: {
            if (status !== Image.Ready)
                return
            view.error = Desktop.diagramError(view.lang, view.code)
            view.mark = Desktop.diagramErrorMark(view.lang, view.code)
            view.good = view.error === "" ? source.toString() : ""
        }
    }

    Text {
        id: painting
        visible: !view.drawn
        y: view.above
        width: view.width
        horizontalAlignment: Text.AlignHCenter
        text: qsTr("Painting Diagram...")
        color: Theme.faint
        font.family: Ui.uiFont
        font.pixelSize: view.em * 0.95
    }

    Image {
        id: shown
        visible: view.drawn
        y: view.above
        x: (view.width - width) / 2
        source: view.good
        width: Math.min(view.width, implicitWidth / view.dpr)
        height: implicitWidth > 0 ? width * implicitHeight / implicitWidth : 0
        smooth: true
        mipmap: true
    }

    Text {
        id: message
        visible: view.error !== ""
        y: view.above + (view.drawn ? shown.height : painting.height + 10 * view.zoom)
        width: view.width
        leftPadding: 8 * view.zoom
        wrapMode: Text.Wrap
        text: view.error
        color: Theme.error
        font.family: Ui.uiFont
        font.pixelSize: view.em * 0.95 * 0.8
    }
}
