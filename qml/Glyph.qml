import QtQuick
import QtQuick.Shapes
import Typo

// A small line icon drawn as a vector path on a 16×16 grid, so it takes
// the theme's colours at any zoom. `name` picks the drawing.
Item {
    id: glyph

    property string name
    property color color: Theme.foreground
    property real size: 16

    implicitWidth: size
    implicitHeight: size

    // Strokes, and the few icons whose dots are filled.
    readonly property var paths: ({
        cut: "M6.5 12.5 A2 2 0 1 1 2.5 12.5 A2 2 0 1 1 6.5 12.5 Z M13.5 12.5 A2 2 0 1 1 9.5 12.5 A2 2 0 1 1 13.5 12.5 Z M5.6 10.7 L11 2 M10.4 10.7 L5 2",
        copy: "M3.5 5.5 H9.5 V13.5 H3.5 Z M6.5 5.5 V2.5 H12.5 V10.5 H9.5",
        paste: "M5.5 3.5 H3.5 V14 H12.5 V3.5 H10.5 M6 2.5 H10 V4.5 H6 Z",
        trash: "M2.5 4.5 H13.5 M6 4.5 V2.5 H10 V4.5 M4 4.5 L4.8 14 H11.2 L12 4.5 M6.8 7 V11.5 M9.2 7 V11.5",
        code: "M5 4 L1.5 8 L5 12 M11 4 L14.5 8 L11 12 M9.2 2.5 L6.8 13.5",
        link: "M7 9 A2.5 2.5 0 0 0 10.5 9 L13 6.5 A2.5 2.5 0 0 0 9.5 3 L8.5 4 M9 7 A2.5 2.5 0 0 0 5.5 7 L3 9.5 A2.5 2.5 0 0 0 6.5 13 L7.5 12",
        quote: "M2.5 7 H6.5 V10 C6.5 12 5.5 13 3.5 13.5 M2.5 7 V4 H6.5 V7 M9.5 7 H13.5 V10 C13.5 12 12.5 13 10.5 13.5 M9.5 7 V4 H13.5 V7",
        ordered: "M2.5 2.8 L3.6 2.2 V6.5 M2.4 9.6 C2.6 8.6 4.6 8.6 4.6 9.8 C4.6 10.8 2.4 11.8 2.4 13.5 H4.8 M7 4.5 H14 M7 11.5 H14",
        bullet: "M6 4 H14 M6 8 H14 M6 12 H14",
        task: "M14 8 A6 6 0 1 1 2 8 A6 6 0 1 1 14 8 Z M5.3 8.2 L7.2 10.1 L10.8 6.2",
        left: "M10 3 L5 8 L10 13",
        // The footer's sidebar toggle, and the arrows on its word count.
        ringOff: "M13.5 8 A5.5 5.5 0 1 1 2.5 8 A5.5 5.5 0 1 1 13.5 8 Z",
        ringOn: "M13.5 8 A5.5 5.5 0 1 1 2.5 8 A5.5 5.5 0 1 1 13.5 8 Z",
        upDown: "M4.5 6 L8 2.5 L11.5 6 M4.5 10 L8 13.5 L11.5 10",
        right: "M6 3 L11 8 L6 13",
        check: "M2.5 8.5 L6 12 L13.5 4",
        menu: "M2.5 4 H13.5 M2.5 8 H13.5 M2.5 12 H13.5",
        alignLeft: "M2.5 3.5 H13.5 M2.5 6.5 H9.5 M2.5 9.5 H13.5 M2.5 12.5 H9.5",
        alignCenter: "M2.5 3.5 H13.5 M4.5 6.5 H11.5 M2.5 9.5 H13.5 M4.5 12.5 H11.5",
        alignRight: "M2.5 3.5 H13.5 M6.5 6.5 H13.5 M2.5 9.5 H13.5 M6.5 12.5 H13.5",
        image: "M2.5 3.5 H13.5 V12.5 H2.5 Z M2.5 10.5 L6 7 L9 10 L10.8 8.2 L13.5 10.9 M11.4 6 A1 1 0 1 1 9.4 6 A1 1 0 1 1 11.4 6 Z",
        // The sidebar.
        file: "M3.5 1.5 H9.5 L12.5 4.5 V14.5 H3.5 Z M9.5 1.5 V4.5 H12.5 M5.5 7.5 H10.5 M5.5 9.5 H10.5 M5.5 11.5 H10.5",
        search: "M10.5 6.5 A4 4 0 1 1 2.5 6.5 A4 4 0 1 1 10.5 6.5 Z M9.5 9.5 L13.5 13.5",
        plus: "M8 2.5 V13.5 M2.5 8 H13.5",
        minus: "M2.5 8 H13.5",
        cells: "M2.5 2.5 H13.5 V13.5 H2.5 Z M2.5 6.2 H13.5 M2.5 9.8 H13.5",
        close: "M4 4 L12 12 M12 4 L4 12",
        // Pinning a recent location.
        pin: "M5.5 2.5 H10.5 M6.5 2.5 V7 L4 9.5 H12 L9.5 7 V2.5 M8 9.5 V14",
        // Find's previous and next.
        up: "M3.5 10.5 L8 6 L12.5 10.5",
        down: "M3.5 6 L8 10.5 L12.5 6",
        // Alert titles.
        alertNote: "M14 8 A6 6 0 1 1 2 8 A6 6 0 1 1 14 8 Z M8 7.3 V11.3 M8 4.6 V5.2",
        alertTip: "M6 13.5 H10 M6.5 11 C6.5 9.5 4 8.5 4 6 A4 4 0 1 1 12 6 C12 8.5 9.5 9.5 9.5 11 Z",
        alertImportant: "M2.5 2.5 H13.5 V10.5 H7.5 L4.5 13.5 V10.5 H2.5 Z M8 4.8 V7.6 M8 9 V9.4",
        alertWarning: "M8 2 L14.5 13.5 H1.5 Z M8 6.2 V9.6 M8 11.2 V11.6",
        alertCaution: "M5.5 1.8 H10.5 L14.2 5.5 V10.5 L10.5 14.2 H5.5 L1.8 10.5 V5.5 Z M8 4.8 V8.8 M8 10.6 V11",
        // The notice's bell, and the footer's focus and typewriter modes.
        bell: "M4 11.5 V7 A4 4 0 0 1 12 7 V11.5 L13.5 13 H2.5 Z M6.5 13.5 A1.5 1.5 0 0 0 9.5 13.5",
        bulb: "M6 13.5 H10 M6.5 11 C6.5 9.5 4 8.5 4 6 A4 4 0 1 1 12 6 C12 8.5 9.5 9.5 9.5 11 Z",
        infinity: "M8 8 C6.5 5.6 2.5 5.6 2.5 8 C2.5 10.4 6.5 10.4 8 8 C9.5 5.6 13.5 5.6 13.5 8 C13.5 10.4 9.5 10.4 8 8 Z",
        brokenImage: "M7.5 3.5 H2.5 V12.5 H7 L8 10 L6.5 8 L7.5 3.5 M9.8 3.5 H13.5 V12.5 H9.2 L10.2 10 L8.8 8 L9.8 3.5 M2.5 10.5 L5 8"
    })
    readonly property var dots: ({
        folder: "M1.5 3 H6.2 L7.7 4.5 H14.5 V13.5 H1.5 Z",
        caretRight: "M6 4.5 L10.5 8 L6 11.5 Z",
        ringOn: "M10.6 8 A2.6 2.6 0 1 1 5.4 8 A2.6 2.6 0 1 1 10.6 8 Z",
        caretDown: "M4.5 6 H11.5 L8 10.5 Z",
        caretUp: "M4.5 10.5 H11.5 L8 6 Z",
        more: "M9.2 3.5 A1.2 1.2 0 1 1 6.8 3.5 A1.2 1.2 0 1 1 9.2 3.5 Z M9.2 8 A1.2 1.2 0 1 1 6.8 8 A1.2 1.2 0 1 1 9.2 8 Z M9.2 12.5 A1.2 1.2 0 1 1 6.8 12.5 A1.2 1.2 0 1 1 9.2 12.5 Z",
        moreH: "M4.7 8 A1.2 1.2 0 1 1 2.3 8 A1.2 1.2 0 1 1 4.7 8 Z M9.2 8 A1.2 1.2 0 1 1 6.8 8 A1.2 1.2 0 1 1 9.2 8 Z M13.7 8 A1.2 1.2 0 1 1 11.3 8 A1.2 1.2 0 1 1 13.7 8 Z",
        bullet: "M3.6 4 A1.1 1.1 0 1 1 1.4 4 A1.1 1.1 0 1 1 3.6 4 Z M3.6 8 A1.1 1.1 0 1 1 1.4 8 A1.1 1.1 0 1 1 3.6 8 Z M3.6 12 A1.1 1.1 0 1 1 1.4 12 A1.1 1.1 0 1 1 3.6 12 Z"
    })

    Shape {
        anchors.centerIn: parent
        width: 16
        height: 16
        scale: glyph.size / 16
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: glyph.color
            strokeWidth: 1.3
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: glyph.paths[glyph.name] || "" }
        }
        ShapePath {
            strokeColor: "transparent"
            fillColor: glyph.color
            PathSvg { path: glyph.dots[glyph.name] || "" }
        }
    }
}
