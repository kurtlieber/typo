import QtQuick
import QtQuick.Controls
import QtQuick.Window
import Typo

// An inline image, formula or emoji, drawn over the room the highlighter
// leaves for it in the text (src/qt/highlighter.rs). Once it has loaded it reports
// its size, the highlighter hides its source behind a carrier character
// that size, and `objectAnchor` says where that character sits. Until it
// loads, its source shows instead. With the caret in it, as in Typora, an
// image stays after its source, and a formula shows as source with a
// preview of it under the line. An emoji is drawn as text, in place of its
// `:shortcode:` (Typora's `.md-emoji`), and a footnote reference as its
// label in a box (Typora's `sup.md-footnote`).
Item {
    id: obj

    required property int index
    // `{kind, src}`: "image" and its destination, "math" and its TeX, or
    // "emoji" and the emoji.
    // Held by place, so editing one object doesn't remake the others.
    property var info
    // The block's highlighter, text and editor.
    property var highlighter
    property Item textEdit
    property var editor
    // The text's em, its x-height and colour, and the zoom.
    property real em
    property real xHeight
    property color color
    property real scale: 1
    // Bumped once the text has been laid out again.
    property int revision

    readonly property string kind: info ? info.kind : ""
    readonly property string src: info ? info.src : ""
    readonly property bool math: kind === "math"
    readonly property bool emoji: kind === "emoji"
    readonly property bool footnote: kind === "footnote"
    readonly property string tex: math ? Ui.mathTex(src) : ""
    readonly property real dpr: Screen.devicePixelRatio

    readonly property string url: {
        if (emoji || footnote)
            return ""
        if (math) {
            const mathEm = em * 1.18
            return "image://math/i/" + (mathEm * dpr).toFixed(2) + "/" + color.toString().slice(1) + "/" + encodeURIComponent(tex)
        }
        editor.inlineRevision
        editor.path
        const u = editor.resolveImage(src)
        // A local file that isn't there fails without asking Qt.
        return u.startsWith("file://") && !Desktop.exists(decodeURIComponent(u.slice(7))) ? "" : u
    }
    // What the picture loads: `url`, but an image being typed only once
    // typing pauses, so a half-typed path isn't tried at every key.
    property string loadUrl
    readonly property bool typing: !math && textEdit !== null && textEdit.activeFocus
    onUrlChanged: typing ? settle.restart() : loadUrl = url
    Component.onCompleted: typing ? settle.restart() : loadUrl = url
    Timer {
        id: settle
        interval: 400
        onTriggered: obj.loadUrl = obj.url
    }
    readonly property bool loaded: picture.status === Image.Ready
    // Something to preview: the last picture stays up while the next loads.
    readonly property bool shown: picture.status === Image.Ready || picture.status === Image.Loading && picture.implicitWidth > 0
    readonly property string error: math && loaded ? Desktop.mathError(tex) : ""
    // A formula that doesn't typeset shows as source.
    readonly property bool ready: emoji || footnote || loaded && error === ""

    // The picture's size on the page. An image keeps its own size at 100%
    // zoom, or the size its `<img>` tag gives, but fits the line; a formula
    // is typeset at the text's size.
    readonly property real lineWidth: textEdit ? textEdit.width : Infinity
    readonly property size fitted: math ? Ui.imageSize(picture.implicitWidth / dpr, picture.implicitHeight / dpr, null, lineWidth, 1)
                                        : Ui.imageSize(picture.implicitWidth, picture.implicitHeight,
                                                       info ? [info.width, info.height, info.zoom] : null, lineWidth, scale)
    readonly property real shownWidth: fitted.width
    readonly property real shownHeight: fitted.height
    // How far the picture reaches above the baseline. Typora centres an
    // image on the middle of the lowercase letters (vertical-align: middle)
    // and sets a formula on the baseline.
    readonly property real above: {
        if (!math)
            return (shownHeight + xHeight) / 2
        const baseline = ready ? Desktop.mathBaseline(tex) : -1
        return shownHeight * (baseline < 0 ? 1 : baseline)
    }
    // Images have transparent borders, 2px before and 4px after.
    readonly property real before: math ? 0 : 2 * scale
    readonly property real room: shownWidth + (math ? 0 : 6 * scale)

    // A footnote reference as a browser sets Typora's `sup`: a size
    // smaller, its baseline a third of the text's size and a pixel up, in a
    // box padded 2px, and 4px at the sides.
    readonly property real supRaise: em / 3 + 1
    readonly property real supWidth: supFm.advanceWidth(src) + 8
    readonly property real supAbove: supRaise + supFm.ascent + 2
    readonly property var size: emoji ? [fm.advanceWidth(src), fm.ascent, fm.descent]
                                : footnote ? [supWidth, supAbove, Math.max(0, supFm.descent + 2 - supRaise)]
                                : ready ? [room, Math.max(0, above), Math.max(0, shownHeight - above)] : null
    onSizeChanged: {
        if (size)
            Qt.callLater(() => { if (obj.size) obj.highlighter.setObjectSize(obj.info.key, obj.size[0], obj.size[1], obj.size[2]) })
    }

    // Where the carrier character is, and while the image is edited, the
    // source's last character, drawn again before it.
    readonly property point anchor: {
        revision
        return highlighter ? highlighter.objectAnchor(index) : Qt.point(NaN, NaN)
    }
    readonly property string tail: {
        revision
        return highlighter ? highlighter.objectTail(index) : ""
    }
    readonly property bool placed: ready && !isNaN(anchor.x)
    // The text's origin in the block.
    readonly property real originX: textEdit ? textEdit.x + textEdit.leftPadding : 0
    readonly property real originY: textEdit ? textEdit.y + textEdit.topPadding : 0

    FontMetrics {
        id: fm
        font: obj.textEdit ? obj.textEdit.font : Qt.font({})
    }
    FontMetrics {
        id: supFm
        font.family: fm.font.family
        font.pixelSize: Math.max(1, Math.round(obj.em / 1.2))
    }
    Rectangle {
        visible: obj.footnote && obj.placed
        x: obj.originX + obj.anchor.x
        y: obj.originY + obj.anchor.y - obj.supAbove
        width: obj.supWidth
        height: supFm.ascent + supFm.descent + 4
        radius: 4
        color: Qt.tint(Theme.background, Qt.alpha(Theme.foreground, 0.1))
        Text {
            x: 4
            y: 2
            text: obj.src
            font: supFm.font
            color: obj.color
        }
    }

    Text {
        visible: obj.emoji && obj.placed
        text: obj.src
        font.family: fm.font.family
        font.pixelSize: fm.font.pixelSize
        color: obj.color
        x: obj.originX + obj.anchor.x
        y: obj.originY + obj.anchor.y - baselineOffset
    }
    Text {
        visible: obj.placed && obj.tail !== ""
        text: obj.tail
        font: fm.font
        color: Theme.muted
        x: obj.originX + obj.anchor.x
        y: obj.originY + obj.anchor.y - baselineOffset
    }

    // GIFs and WebPs play, as in a browser; other pictures stand still.
    readonly property bool animated: !math && !emoji && /\.(gif|webp)([?#].*)?$/i.test(loadUrl)
    readonly property Image picture: animated ? moving : still
    readonly property real pictureX: originX + anchor.x + (tail !== "" ? fm.advanceWidth(tail) : 0) + before
    Image {
        id: still
        source: obj.animated ? "" : Ui.reloaded(obj.loadUrl)
        asynchronous: true
        retainWhileLoading: true
        visible: obj.placed && !obj.animated && !obj.emoji
        x: obj.pictureX
        y: obj.originY + obj.anchor.y - obj.above
        width: obj.shownWidth
        height: obj.shownHeight
        smooth: true
        mipmap: !obj.math
    }
    AnimatedImage {
        id: moving
        source: obj.animated ? Ui.reloaded(obj.loadUrl) : ""
        visible: obj.placed && obj.animated
        x: obj.pictureX
        y: obj.originY + obj.anchor.y - obj.above
        width: obj.shownWidth
        height: obj.shownHeight
        smooth: true
    }

    // A formula being edited: its preview, or what is wrong with it, in a
    // popover under the line its source starts on.
    readonly property point sourceAnchor: {
        revision
        return highlighter && editing ? highlighter.sourceAnchor(index) : Qt.point(NaN, NaN)
    }
    readonly property bool editing: math && shown && textEdit !== null && textEdit.activeFocus && isNaN(anchor.x)
    Popup {
        id: preview
        visible: obj.editing && !isNaN(obj.sourceAnchor.x)
        closePolicy: Popup.NoAutoClose
        focus: false
        padding: obj.em * 0.5
        x: Math.max(0, Math.min(obj.originX + obj.sourceAnchor.x, (obj.textEdit ? obj.textEdit.width : 0) - width))
        y: obj.originY + obj.sourceAnchor.y + fm.descent + 4 * obj.scale
        background: Rectangle {
            color: Theme.panel
            border.color: Theme.border
            radius: 2
        }
        contentItem: Item {
            implicitWidth: obj.error !== "" ? problem.implicitWidth : obj.shownWidth
            implicitHeight: obj.error !== "" ? problem.implicitHeight : obj.shownHeight
            Image {
                visible: obj.error === ""
                source: obj.loadUrl
                retainWhileLoading: true
                width: obj.shownWidth
                height: obj.shownHeight
                smooth: true
            }
            Text {
                id: problem
                visible: obj.error !== ""
                text: obj.error
                color: Theme.error
                font.family: Ui.textFont
                font.pixelSize: obj.em * 0.9
            }
        }
    }
}
