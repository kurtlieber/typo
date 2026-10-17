import QtQuick
import QtQuick.Controls
import QtQuick.Shapes
import Typo

// One leaf block of the document: its container decorations (quote bars,
// list markers, footnote labels) in the left gutter, and the block itself.
Item {
    id: block

    required property var editor
    // View ▸ Readonly Mode: nothing takes input or shows its source.
    readonly property bool locked: editor.readonly
    // The EditorView, which handles structural keys.
    property Item host
    required property int index
    required property string kind
    required property string text
    required property int level
    required property string info
    required property string decor
    required property string rows
    required property real gap

    readonly property var containers: JSON.parse(decor)
    // The footnote this block is part of: Typora sets definitions a size
    // smaller, faded, with the label in front of the first line.
    readonly property var footnote: containers.find(c => c.t === "footnote") || null
    readonly property real em: Ui.fontSize * (footnote ? 0.9 : 1)
    readonly property real scale: Ui.zoom
    // Typewriter mode takes the first block's top margin away.
    readonly property real topSpace: (index === 0 && Ui.typewriter ? 0 : gap) * scale
    readonly property bool quoted: containers.some(c => c.t === "quote")
    // The alert (`note`, `tip`, …) of a quote, or "": none when
    // Preferences ▸ Markdown turns GitHub's alerts off.
    function alertOf(c) {
        return Prefs.alerts ? c.alert || "" : ""
    }
    // The alert of the innermost quote.
    readonly property string alert: {
        const q = containers.filter(c => c.t === "quote")
        return q.length ? alertOf(q[q.length - 1]) : ""
    }
    // The paragraph that opens an alert with `[!NOTE]`, under its title.
    readonly property bool alertTitle: {
        const c = containers[containers.length - 1]
        return kind === "paragraph" && c !== undefined && c.t === "quote" && c.first && alertOf(c) !== ""
    }
    // Display math's equation numbers, a tag for each row.
    readonly property var mathTags: host && host.mathRows ? host.mathRows[index] || [] : []

    // Indentation of each container level, after Typora's default theme:
    // quotes have a 4px bar and 15px padding, lists a 30px gutter, and
    // footnotes none: their label sits in the first line.
    function containerWidth(c) {
        if (c.t === "footnote")
            return 0
        // An alert's bar and padding: Typora's .25em border and 1em.
        if (c.t === "quote")
            return (alertOf(c) ? 20 : 19) * scale
        // Indent first line of paragraphs moves list items in by 2em.
        return 30 * scale + (Prefs.indentFirstLine ? 2 * em : 0)
    }
    readonly property real indent: {
        let x = 0
        for (const c of containers)
            x += containerWidth(c)
        return x
    }

    // Paragraphs and headings, as opposed to code, math and the like.
    readonly property bool textual: kind === "paragraph" || kind === "heading"
    // Inside a selection across blocks. Text fields show their own part;
    // blocks without text are tinted whole.
    property bool spanned: false
    // Find's current match is in this block: it shows as with the caret,
    // unfolded, the match selected.
    readonly property bool found: host !== null && host.found !== null && host.found.row === index
    // Find's matches in the block's text, for its highlighter.
    readonly property string hits: host !== null && host.findHits[index] || "[]"
    // Focus mode greys every block but the caret's (Typora's
    // `md-focus`) and fades their pictures.
    readonly property bool dimmed: Ui.focusMode && host !== null && host.focusedRow !== index
    readonly property real pictureOpacity: dimmed ? 0.3 : 1
    // The line (from 0) that UTF-16 position `pos` is on in `text`.
    function lineOf(text, pos) {
        let n = 0
        for (let i = text.indexOf("\n"); i >= 0 && i < pos; i = text.indexOf("\n", i + 1))
            n++
        return n
    }

    // Whether (x, y) is on a task item's checkbox.
    function taskAt(x, y) {
        const deco = childAt(x, y)
        if (!deco || deco === content)
            return false
        const box = deco.childAt(x - deco.x, y - deco.y)
        return box !== null && box.objectName === "task"
    }

    // The editable text item, for the editor's caret handling: in a table,
    // the cell with the caret.
    readonly property Item editItem: content.item ? content.item.editItem || null : null
    // What the block draws (the loaded component), for scripts.
    readonly property Item body: content.item
    // A table, which the view hands clicks, keys and positions to.
    readonly property Item table: kind === "table" && content.item && content.item.columns !== undefined ? content.item : null

    signal focused(int row)

    function keyPressed(edit, event) {
        if (host)
            host.keyPressed(index, edit, event)
    }

    Connections {
        target: block.editItem
        function onActiveFocusChanged() {
            if (block.editItem.activeFocus)
                block.focused(block.index)
        }
    }

    width: parent ? parent.width : 0
    height: topSpace + content.height + (backLinks.visible && backLinks.below ? backLinks.height : 0)

    // Container decorations.
    Repeater {
        model: block.containers
        delegate: Item {
            id: deco
            required property var modelData
            required property int index
            readonly property real x0: {
                let x = 0
                for (let i = 0; i < index; i++)
                    x += block.containerWidth(block.containers[i])
                return x
            }
            x: x0
            width: block.containerWidth(modelData)
            height: block.height

            // A quote bar runs through the gap when the block above is in
            // the same quote, so consecutive blocks draw one bar.
            Rectangle {
                visible: deco.modelData.t === "quote"
                y: deco.modelData.first ? block.topSpace : 0
                width: 4 * block.scale
                height: block.height - y
                color: block.alertOf(deco.modelData) ? Theme.alertColor(deco.modelData.alert) : Theme.border
            }

            // List marker on an item's first block, aligned with its first
            // line: a number a space before the text, or the browser's
            // bullet, a disc, ring or square centred 1em before the text and
            // .31em above the baseline.
            readonly property bool marked: modelData.t === "item" && modelData.first && modelData.task === null
            Text {
                visible: deco.marked && deco.modelData.ordered
                anchors.right: parent.right
                anchors.rightMargin: block.em * 0.3125
                y: block.topSpace + block.firstBaseline - baselineOffset
                text: deco.modelData.marker || ""
                color: block.dimmed ? Theme.blur : Theme.foreground
                font.family: Ui.textFont
                font.pixelSize: block.em
            }
            Rectangle {
                readonly property string shape: ({ "•": "disc", "◦": "circle", "▪": "square" })[deco.modelData.marker] || ""
                readonly property color ink: block.dimmed ? Theme.blur : Theme.foreground
                visible: deco.marked && !deco.modelData.ordered
                width: block.em * (shape === "circle" ? 0.4375 : shape === "square" ? 0.34 : 0.375)
                height: width
                radius: shape === "square" ? 0 : width / 2
                x: parent.width - block.em - width / 2
                y: block.topSpace + block.firstBaseline - block.em * 0.3125 - height / 2
                color: shape === "circle" ? "transparent" : ink
                border.width: shape === "circle" ? 1 : 0
                border.color: ink
            }
            // Task checkbox: the browser's 13px box, where a bullet would be.
            Rectangle {
                objectName: "task"
                visible: deco.modelData.t === "item" && deco.modelData.first && deco.modelData.task !== null
                readonly property real size: block.em * 0.8125
                width: size
                height: size
                radius: 2 * block.scale
                x: parent.width - block.em - size / 2
                y: block.topSpace + block.firstBaseline - block.em * 0.4375 - size / 2
                opacity: block.dimmed ? 0.2 : 1
                color: deco.modelData.task ? Theme.accent : "transparent"
                border.color: deco.modelData.task ? Theme.accent : Theme.muted
                border.width: 1.2
                Text {
                    anchors.centerIn: parent
                    visible: deco.modelData.task === true
                    text: "✓"
                    color: Theme.accentText
                    font.pixelSize: parent.size * 0.8
                    font.bold: true
                }
            }
        }
    }

    // A control inside the block that takes clicks itself (a code
    // fence's language box, a footnote's links back), or null.
    function controlAt(x, y) {
        if (backLinks.visible) {
            const link = backLinks.childAt(x - backLinks.x, y - backLinks.y)
            if (link && link.activate)
                return link
        }
        const c = content.item
        return c && c.controlAt ? c.controlAt(x - content.x, y - content.y) : null
    }

    // Where the first line of text sits, for markers in the gutter.
    readonly property real firstBaseline: content.item && content.item.firstBaseline !== undefined ? content.item.firstBaseline : em
    readonly property real firstLineMiddle: content.item && content.item.firstLineMiddle !== undefined ? content.item.firstLineMiddle : em * 0.8

    // An alert's title, an icon and its kind, as Typora's `md-alert-text`;
    // its `[!NOTE]` shows after it while the caret is there.
    Row {
        id: alertLabel
        visible: block.alertTitle
        x: block.indent
        y: block.topSpace + block.firstBaseline - alertName.baselineOffset
        spacing: alertName.font.pixelSize * 0.5
        readonly property color color: block.dimmed ? Theme.blur : block.alertTitle ? Theme.alertColor(block.alert) : Theme.foreground
        Glyph {
            y: alertName.baselineOffset - size * 0.85
            name: "alert" + block.alert.charAt(0).toUpperCase() + block.alert.slice(1)
            size: alertName.font.pixelSize
            color: alertLabel.color
        }
        Text {
            id: alertName
            text: ({ note: qsTr("Note"), tip: qsTr("Tip"), important: qsTr("Important"), warning: qsTr("Warning"), caution: qsTr("Caution") })[block.alert] || ""
            color: alertLabel.color
            font.family: Ui.textFont
            font.pixelSize: block.em * 0.9
            font.bold: true
        }
    }

    // Indent first line of paragraphs: 2em, as Typora's
    // `#write.first-line-indent p`, but not in list items.
    readonly property real paraIndent: Prefs.indentFirstLine && kind === "paragraph" && !labelled && !alertTitle
                                       && containers.every(c => c.t === "quote") ? 2 * em : 0
    // A footnote definition's label, `[^name] :`, as Typora's `md-def-name`.
    readonly property bool labelled: footnote !== null && footnote.first
    readonly property real labelWidth: labelled ? footLabel.implicitWidth : alertTitle ? alertLabel.width + em * 0.3
                                     : headingSource ? headingMarks.implicitWidth : 0
    // Preferences ▸ Editor ▸ Live Rendering: a heading's `#`s show in front
    // of it while the caret is in it.
    readonly property bool headingSource: kind === "heading" && Prefs.showSourceOnFocus && host !== null
                                          && host.focusedRow === index
    Text {
        id: headingMarks
        visible: block.headingSource
        x: block.indent
        y: block.topSpace + block.firstBaseline - baselineOffset
        text: "#".repeat(Math.max(1, Math.min(6, block.level))) + " "
        color: Theme.faint
        font.family: Ui.textFont
        font.pixelSize: [2.25, 1.75, 1.5, 1.25, 1, 1][Math.max(1, Math.min(6, block.level)) - 1] * block.em
        font.bold: true
    }
    // The bold name padded 2ch before and 2.5ch after, `[^` over the one
    // and `] :` over the other, then the gap before the text.
    Item {
        id: footLabel
        visible: block.labelled
        x: block.indent
        y: block.topSpace + block.firstBaseline - footName.baselineOffset
        opacity: 0.8
        readonly property real ch: spaceFont.advanceWidth("0")
        readonly property color faint: block.dimmed ? Theme.blur : Theme.faint
        implicitWidth: 4.5 * ch + footName.implicitWidth + block.em * 0.5
        implicitHeight: footName.implicitHeight
        Text {
            text: "[^"
            color: footLabel.faint
            font.family: Ui.textFont
            font.pixelSize: block.em
        }
        Text {
            id: footName
            x: 2 * footLabel.ch
            text: block.labelled ? block.footnote.label : ""
            color: block.dimmed ? Theme.blur : Theme.foreground
            font.family: Ui.textFont
            font.pixelSize: block.em
            font.bold: true
        }
        Text {
            anchors.right: footName.right
            anchors.rightMargin: -2.5 * footLabel.ch
            text: "] :"
            color: footLabel.faint
            font.family: Ui.textFont
            font.pixelSize: block.em
        }
    }
    // An empty definition says what goes there.
    Text {
        id: footHint
        visible: block.labelled && block.footnote.last && block.textual && block.text === ""
        x: block.indent + block.labelWidth
        y: footLabel.y
        text: qsTr("input description here")
        color: Theme.faint
        font.family: Ui.textFont
        font.pixelSize: block.em
    }

    // After a footnote's definition, a link back to each reference to it.
    readonly property int references: {
        if (!footnote || !footnote.last || !host)
            return 0
        const f = host.footnotes[footnote.label.toLowerCase()]
        return f ? f.refs.length : 0
    }
    readonly property alias backLinks: backLinks
    Row {
        id: backLinks
        visible: block.references > 0 && block.textual && block.editItem !== null
        // A space before each, as Typora writes them.
        spacing: spaceFont.advanceWidth(" ")
        // Where the text ends; below it when the last line has no room.
        readonly property rect end: {
            block.text
            block.width
            const e = block.editItem
            return e && content.item && content.item.placeRevision >= 0 ? e.positionToRectangle(e.length) : Qt.rect(0, 0, 0, 0)
        }
        readonly property real after: content.x + end.x + (footHint.visible ? footHint.width : 0) + spacing
        readonly property bool below: after + implicitWidth > block.width
        x: below ? content.x : after
        // On the last line's baseline (lines are the same height).
        y: block.topSpace + (below ? content.height : end.y + block.firstBaseline - linkFont.ascent)
        FontMetrics {
            id: linkFont
            font.family: Ui.monoFont
            font.pixelSize: block.em
        }
        FontMetrics {
            id: spaceFont
            font.family: Ui.textFont
            font.pixelSize: block.em
        }
        opacity: 0.8
        Repeater {
            model: block.references
            Text {
                required property int index
                // Typora's ↩ links are clicked without Ctrl.
                function activate() {
                    block.host.jumpToFootnoteRef(block.footnote.label.toLowerCase(), index)
                }
                text: "↩"
                color: block.dimmed ? Theme.blur : Theme.link
                font.underline: true
                font.family: Ui.monoFont
                font.pixelSize: block.em
            }
        }
    }

    Loader {
        id: content
        x: block.indent + (block.labelled && !block.textual ? block.labelWidth : 0)
        y: block.topSpace
        opacity: block.footnote ? 0.8 : 1
        width: block.width - x
        sourceComponent: {
            switch (block.kind) {
            case "paragraph":
            case "heading":
            case "linkref": return textBlock
            case "code":
            case "frontmatter": return codeBlock
            case "math": return mathBlock
            case "html": return htmlBlock
            case "table": return tableBlock
            case "hr": return ruleBlock
            case "toc": return tocBlock
            default: return rawBlock
            }
        }
    }

    // Selected alone, a TOC shows its frame and header instead.
    readonly property bool alone: spanned && host !== null && host.span !== null && host.span.aRow === index && host.span.zRow === index
    Rectangle {
        visible: block.spanned && (!block.editItem || block.table !== null) && !(block.kind === "toc" && block.alone)
        x: content.x
        y: content.y
        width: content.width
        height: content.height
        color: Theme.selection
        opacity: 0.6
    }

    // Paragraphs and headings: inline markdown with live preview.
    Component {
        id: textBlock
        Item {
            id: textItem
            readonly property Item editItem: edit
            readonly property bool heading: block.kind === "heading"
            // Roles update one at a time, so the kind can change before the level.
            readonly property int level: Math.max(1, Math.min(6, block.level))
            readonly property real size: heading ? [2.25, 1.75, 1.5, 1.25, 1, 1][level - 1] * block.em : block.em
            // With a fixed line height Qt puts the baseline at 4/5 of the
            // line. A line with an image or formula in it can grow.
            readonly property real firstBaseline: {
                placeRevision
                const grown = hl.placing ? hl.baselineAt(0) : NaN
                return isNaN(grown) ? hl.lineHeight * 0.8 : grown
            }
            readonly property real firstLineMiddle: firstBaseline - fm.ascent * 0.35
            // h1 and h2 are underlined, as in Typora's default theme.
            readonly property bool ruled: heading && level <= 2
            // A paragraph that is one image shows the image, centred. With
            // the caret inside, its source shows above it; until it loads,
            // or when it can't, only the source shows, as in Typora.
            readonly property string image: {
                // Link references, the front matter's image root and the
                // file's folder change where an image points.
                block.editor.inlineRevision
                block.editor.path
                return heading ? "" : block.editor.imageUrl(edit.text)
            }
            // The size an `<img>` tag gives it: `[width, height, zoom]`.
            readonly property var imageSize: image === "" ? null : JSON.parse(block.editor.imageSize(edit.text))
            // What the picture loads: `image`, but while the caret is in the
            // source only once typing pauses, so a half-typed path isn't
            // tried at every key.
            property string loadUrl
            onImageChanged: {
                if (expanded)
                    settle.restart()
                else
                    loadUrl = image
            }
            Component.onCompleted: loadUrl = image
            Timer {
                id: settle
                interval: 400
                onTriggered: textItem.loadUrl = textItem.image
            }
            // A local file that isn't there fails without asking Qt.
            readonly property bool missing: loadUrl.startsWith("file://") && !Desktop.exists(decodeURIComponent(loadUrl.slice(7)))
            // GIFs and WebPs play, as in a browser.
            readonly property bool animated: /\.(gif|webp)([?#].*)?$/i.test(loadUrl)
            readonly property Image picture: animated ? moving : still
            function loaded(status) {
                if (status === Image.Ready && edit.activeFocus && block.host)
                    block.host.imageLoaded(edit)
            }
            readonly property bool failed: image !== "" && (missing || picture.status === Image.Error)
            readonly property bool pictured: image !== "" && picture.status === Image.Ready
            readonly property bool expanded: edit.live || block.found
            readonly property bool sourceShown: !pictured || expanded
            // A Text's own height can't follow its implicit height without
            // a binding loop, so the hint's room is reckoned here.
            readonly property real hintHeight: hint.shown ? hint.implicitHeight + 4 * block.scale : 0
            readonly property real pictureY: expanded ? edit.height + hintHeight + 4 * block.scale : 0
            width: parent ? parent.width : 0
            height: pictured ? pictureY + picture.height
                             : edit.height + hintHeight + (ruled ? 1 : 0)

            FontMetrics {
                id: fm
                font: edit.font
            }

            // Typora's `code` box around each code span, under the text; the
            // highlighter leaves room for its padding and border.
            FontMetrics {
                id: codeFm
                font.family: Ui.monoFont
                font.pixelSize: Math.max(1, Math.round(textItem.size * 0.9))
            }
            readonly property real codePad: 3 * size / 16
            readonly property var codeBoxes: JSON.parse(hl.boxes)
            // The box's piece on each line from UTF-16 `s` to `e`: {x, y, w,
            // h} of the line it is on.
            function codeSegments(s, e) {
                const a = edit.positionToRectangle(s)
                const z = edit.positionToRectangle(e)
                if (a.y === z.y)
                    return [{ x: a.x - codePad, y: a.y, h: a.height, w: z.x - a.x + codePad }]
                const out = []
                let from = a
                let first = true
                for (let i = s + 1; i <= e; i++) {
                    const r = edit.positionToRectangle(i)
                    if (r.y === from.y)
                        continue
                    const end = edit.positionToRectangle(i - 1).x + codeFm.advanceWidth(edit.text.charAt(i - 1))
                    out.push({ x: from.x - (first ? codePad : 0), y: from.y, h: from.height, w: end - from.x + (first ? codePad : 0) })
                    from = r
                    first = false
                }
                out.push({ x: from.x, y: from.y, h: from.height, w: z.x - from.x })
                return out
            }
            Repeater {
                model: textItem.sourceShown ? textItem.codeBoxes : []
                delegate: Repeater {
                    required property var modelData
                    model: {
                        textItem.placeRevision
                        edit.width
                        return textItem.codeSegments(modelData[0], modelData[1])
                    }
                    delegate: Rectangle {
                        required property var modelData
                        // Fixed lines put the baseline at 4/5; grown ones
                        // keep the text at the bottom.
                        readonly property real textBase: modelData.y + (hl.placing ? modelData.h - hl.lineHeight * 0.2 : hl.lineHeight * 0.8)
                        x: edit.x + modelData.x
                        y: edit.y + textBase - codeFm.ascent - 1
                        width: modelData.w
                        height: codeFm.ascent + codeFm.descent + 2
                        radius: 3 * block.scale
                        color: Theme.codeBackground
                        border.color: Qt.alpha(Theme.border, 0.6)
                    }
                }
            }

            TextEdit {
                id: edit
                width: parent.width
                text: block.text
                readOnly: block.locked
                readonly property bool live: activeFocus && !readOnly
                textFormat: TextEdit.PlainText
                wrapMode: TextEdit.Wrap
                selectByMouse: true
                persistentSelection: true
                color: textItem.heading ? [Theme.h1, Theme.h2, Theme.h3, Theme.h4, Theme.h5, Theme.h6][textItem.level - 1]
                                        : (block.quoted && block.alert === "" ? Theme.muted : Theme.foreground)
                selectionColor: Theme.selection
                selectedTextColor: textItem.heading ? color : Theme.foreground
                font.family: Ui.textFont
                font.pixelSize: textItem.size
                font.bold: textItem.heading
                // Room for the image icon before an image's source.
                leftPadding: mark.visible ? mark.width + 4 * block.scale : 0
                cursorDelegate: Caret {}
                Keys.onPressed: event => block.keyPressed(edit, event)
                // Format changes (highlighting, line height) also emit
                // textChanged; only a change of text is an edit.
                property string seen
                onTextChanged: {
                    if (text === seen)
                        return
                    seen = text
                    hl.fixLineHeight()
                    block.editor.edited(block.index, text, cursorPosition)
                    // A span can cross lines, and Qt only repaints the line
                    // that changed.
                    if (text.indexOf("\n") >= 0)
                        hl.refresh()
                    if (block.host && activeFocus)
                        Qt.callLater(() => block.host.emojiSuggest.typed(edit))
                }
                onCursorPositionChanged: {
                    hl.caretMoved(live || block.found ? cursorPosition : -1)
                    if (block.host) {
                        block.host.emojiSuggest.caretMoved(edit)
                        block.host.caretMoved()
                    }
                }
                onLiveChanged: {
                    hl.caretMoved(live || block.found ? cursorPosition : -1)
                    if (block.host)
                        block.host.emojiSuggest.caretMoved(edit)
                }
                Connections {
                    target: block
                    function onFoundChanged() { hl.caretMoved(edit.live || block.found ? edit.cursorPosition : -1) }
                }
                Component.onCompleted: {
                    hl.dim(block.dimmed, -1)
                    hl.attach(edit.textDocument)
                    hl.fixLineHeight()
                }
            }
            InlineHighlighter {
                id: hl
                readonly property bool dimmed: block.dimmed
                onDimmedChanged: dim(dimmed, -1)
                // Typora's line height: 1.6 for text, tighter for headings.
                lineHeight: textItem.size * (textItem.heading ? [1.2, 1.225, 1.43, 1.4, 1.4, 1.4][textItem.level - 1] : 1.6)
                fontSize: textItem.size
                imageShown: textItem.pictured
                firstIndent: block.labelWidth + block.paraIndent
                onFirstIndentChanged: fixLineHeight()
                alertTitle: block.alertTitle
                doc: block.editor.docId
                onAlertTitleChanged: refresh()
                linkDef: block.kind === "linkref"
                onLinkDefChanged: refresh()
                hits: block.hits
                onHitsChanged: refresh()
                onLineHeightChanged: fixLineHeight()
                onFontSizeChanged: refresh()
                // Later, not while the change that loaded the image is
                // still being handled.
                onImageShownChanged: Qt.callLater(() => hl.refresh())
                // Lines with an image or formula in them grow to fit it.
                onPlacingChanged: Qt.callLater(() => hl.fixLineHeight())
                onFormatted: Qt.callLater(textItem.replace)
                // Repaint when the theme, link definitions or options change.
                readonly property string inputs: Theme.revision + "/" + block.editor.inlineRevision
                onInputsChanged: refresh()
            }
            // The pointer on a footnote reference shows the footnote.
            HoverHandler {
                enabled: block.host !== null && block.host.footnoteRefs[block.index] !== undefined
                onPointChanged: {
                    if (!hovered)
                        return
                    const x = point.position.x - edit.x
                    const pos = block.host.textPos(edit, x, point.position.y - edit.y)
                    block.host.footnoteHover(block.index, block.host.footnoteSnap(block.index, edit, x, pos))
                }
                onHoveredChanged: {
                    if (!hovered)
                        block.host.footnoteHover(block.index, -1)
                }
            }

            // Inline images and formulas, over the room left for them.
            // Their places are read again once the text has been laid out.
            property int placeRevision: 0
            function replace() { placeRevision++ }
            Connections {
                target: edit
                function onWidthChanged() { Qt.callLater(textItem.replace) }
                function onContentHeightChanged() { Qt.callLater(textItem.replace) }
            }
            readonly property var objects: JSON.parse(hl.objects)
            Repeater {
                model: textItem.objects.length
                InlineObject {
                    info: textItem.objects[index]
                    highlighter: hl
                    textEdit: edit
                    editor: block.editor
                    em: textItem.size
                    xHeight: fm.xHeight
                    color: edit.color
                    scale: block.scale
                    revision: textItem.placeRevision
                    opacity: block.pictureOpacity
                    onPictureLoaded: if (edit.activeFocus && block.host) block.host.imageLoaded(edit)
                }
            }
            Glyph {
                id: mark
                visible: textItem.image !== "" && textItem.sourceShown
                name: textItem.failed ? "brokenImage" : "image"
                color: textItem.failed ? Theme.error : Theme.faint
                size: textItem.size
                y: textItem.firstLineMiddle - height / 2
            }
            // Under an image's source while the caret is in it.
            Text {
                id: hint
                readonly property bool shown: textItem.image !== "" && textItem.expanded && !textItem.pictured
                visible: shown
                y: edit.height
                leftPadding: edit.leftPadding
                text: textItem.failed ? qsTr("image load failed") : qsTr("image is loading …")
                color: textItem.failed ? Theme.error : Theme.faint
                font.family: Ui.textFont
                font.pixelSize: textItem.size * 0.85
            }
            // Its own size at 100% zoom, or its tag's, never wider than the
            // page.
            readonly property size fitted: Ui.imageSize(picture.implicitWidth, picture.implicitHeight, imageSize, width, block.scale)
            Image {
                id: still
                source: textItem.missing || textItem.animated ? "" : Ui.reloaded(textItem.loadUrl)
                asynchronous: true
                visible: textItem.pictured && !textItem.animated
                y: textItem.pictureY
                width: textItem.fitted.width
                height: textItem.fitted.height
                x: (textItem.width - width) / 2
                opacity: block.pictureOpacity
                smooth: true
                mipmap: true
                onStatusChanged: textItem.loaded(status)
            }
            AnimatedImage {
                id: moving
                source: textItem.missing || !textItem.animated ? "" : Ui.reloaded(textItem.loadUrl)
                visible: textItem.pictured && textItem.animated
                y: textItem.pictureY
                width: textItem.fitted.width
                height: textItem.fitted.height
                x: (textItem.width - width) / 2
                opacity: block.pictureOpacity
                smooth: true
                onStatusChanged: textItem.loaded(status)
            }
            Rectangle {
                visible: textItem.ruled
                anchors.bottom: parent.bottom
                width: parent.width
                height: 1
                color: Theme.border
            }
        }
    }

    // Code fences and front matter: monospace text in a box, coloured by
    // language. While a fence has the caret, its code language box shows
    // below it on the right, as in Typora. A ```math fence shows its
    // formula instead, and with the caret in it, its code with a live
    // preview beneath. Diagram fences (```mermaid, ```flow, ```sequence)
    // likewise show their drawing, and with the caret in them, their code
    // with the drawing in a bordered panel beneath.
    Component {
        id: codeBlock
        Item {
            id: fence
            readonly property Item editItem: code
            // Front matter is Typora's `pre.md-meta-block`: unbordered,
            // 1rem of padding, 85% text 1.45 high, plain grey. A fence has
            // a 1px border, 8px above and 6px below, 4px of padding and its
            // lines' 8px more on the left, and lines 1.6 high.
            readonly property bool meta: block.kind === "frontmatter"
            // Qt rounds the font's pixel size; CSS doesn't, nor its lines.
            readonly property real size: block.em * (meta ? 0.85 : 0.9)
            readonly property real lineHeight: size * (meta ? 1.45 : 1.6)
            readonly property real padTop: (meta ? 16 : 9) * block.scale
            readonly property real padBottom: (meta ? 16 : 7) * block.scale
            readonly property real padLeft: (meta ? 16 : 13) * block.scale
            readonly property real padRight: (meta ? 16 : 9) * block.scale
            onLineHeightChanged: hl.fixLineHeight(lineHeight)
            // With a fixed line height Qt puts the baseline at 4/5 of the
            // line, CSS the font's middle at the line's: inside a box that
            // shows, so the code moves up by the difference.
            readonly property real drop: lineHeight * 0.8 - ((lineHeight - fm.ascent - fm.descent) / 2 + fm.ascent)
            readonly property real firstBaseline: codeView.y + lineHeight * 0.8
            readonly property real firstLineMiddle: codeView.y + lineHeight / 2
            readonly property bool fenced: block.kind === "code"
            readonly property bool active: fenced && (code.activeFocus || langInput.activeFocus || block.found)
            readonly property string info: block.info
            readonly property bool mathFence: Prefs.mathFences && fenced && info.trim().toLowerCase() === "math"
            readonly property string diagramLang: Prefs.diagrams && fenced && ["mermaid", "flow", "sequence"].indexOf(info.trim().toLowerCase()) >= 0
                ? info.trim().toLowerCase() : ""
            readonly property bool diagramFence: diagramLang !== ""
            readonly property alias diagramView: diagram
            readonly property alias codeHighlighter: hl
            readonly property alias languageBox: langBox
            // The code is out of sight, but keeps the caret's place.
            readonly property bool folded: (mathFence || diagramFence) && !active
            // Typora's diagram panel sits 10px below the code. A gantt
            // chart's takes the window's width at the top level while the
            // sidebar is closed: 98% of it folded, less .8em a side (at
            // most 1140px) open.
            readonly property real panelGap: 10 * block.scale
            readonly property bool wide: diagram.gantt && block.containers.length === 0 && !Ui.sidebar && block.host !== null
            readonly property real panelWidth: !wide ? width
                : folded ? block.host.width * 0.98 : Math.min(block.host.width - 1.6 * block.em, 1140 * block.scale)
            // Line numbers: the preference, or `{.numberLines startFrom="N"}`
            // after the language, as Typora reads fence attributes.
            readonly property bool numbered: fenced && (Prefs.codeLineNumbers || /\.number(-?l|L)ines\b/.test(info))
            readonly property int firstNumber: {
                const m = /startFrom=["']?(\d+)/.exec(info)
                return m ? Number(m[1]) : 1
            }
            readonly property var lineStarts: {
                const starts = [0]
                const t = code.text
                for (let i = t.indexOf("\n"); i >= 0; i = t.indexOf("\n", i + 1))
                    starts.push(i + 1)
                return starts
            }
            width: parent ? parent.width : 0
            height: folded ? (diagramFence ? panelGap + diagram.height : formula.height + 20 * block.scale)
                : box.height + (diagramFence ? panelGap + panel.height : 0) + (active ? langBox.height + 8 * block.scale : 0)
            clip: folded && !wide

            // The language box takes clicks itself.
            function controlAt(x, y) {
                return langBox.visible && langBox.contains(langBox.mapFromItem(fence, x, y)) ? langInput : null
            }
            function commitLanguage() {
                if (langInput.text.trim() !== info.trim())
                    block.editor.setInfo(block.index, langInput.text)
            }
            onInfoChanged: if (!langInput.activeFocus) langInput.text = info

            Rectangle {
                id: box
                visible: !fence.folded
                width: parent.width
                height: code.height + fence.padTop + fence.padBottom + (fence.mathFence ? formula.height + 12 * block.scale : 0)
                color: Theme.codeBackground
                border.width: fence.meta ? 0 : 1
                border.color: Qt.alpha(Theme.border, 0.6)
                radius: 3
            }
            FontMetrics {
                id: fm
                font: code.font
            }
            Item {
                id: gutter
                visible: fence.numbered && !fence.folded
                y: codeView.y
                width: fence.numbered ? fm.advanceWidth(String(fence.firstNumber + fence.lineStarts.length - 1)) + 22 * block.scale : 0
                height: code.height
                Repeater {
                    model: fence.numbered ? fence.lineStarts.length : 0
                    delegate: Text {
                        required property int index
                        anchors.right: parent.right
                        anchors.rightMargin: 10 * block.scale
                        // Re-read after the text reflows.
                        y: (code.contentHeight + code.width >= 0 ? code.positionToRectangle(fence.lineStarts[index] || 0).y : 0) + fence.lineHeight * 0.8 - baselineOffset
                        text: fence.firstNumber + index
                        color: Ui.focusMode && index !== hl.keep ? Theme.blur : Theme.faint
                        font: code.font
                    }
                }
                Rectangle {
                    anchors.right: parent.right
                    y: -codeView.y + 1
                    width: 1
                    height: box.height - 2
                    color: Qt.alpha(Theme.border, 0.6)
                }
            }
            // Without Preferences ▸ Markdown ▸ Code Fences ▸ Line Wrapping,
            // long lines scroll sideways, following the caret.
            Flickable {
                id: codeView
                x: gutter.width + (fence.numbered ? 10 * block.scale : fence.padLeft)
                y: fence.padTop - fence.drop
                width: parent.width - x - fence.padRight
                height: code.height
                contentWidth: code.width
                contentHeight: code.height
                clip: !Prefs.codeWrap
                interactive: false
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.horizontal: ScrollBar {
                    policy: Prefs.codeWrap ? ScrollBar.AlwaysOff : ScrollBar.AsNeeded
                }
                function follow(r) {
                    if (Prefs.codeWrap)
                        contentX = 0
                    else if (r.x < contentX)
                        contentX = Math.max(0, r.x - 10 * block.scale)
                    else if (r.x + r.width > contentX + width)
                        contentX = Math.min(contentWidth - width, r.x + r.width - width + 10 * block.scale)
                }
            TextEdit {
                id: code
                readonly property var highlighter: hl
                readOnly: block.locked
                // Unwrapped, as wide as its longest line.
                width: Prefs.codeWrap ? codeView.width : undefined
                opacity: fence.folded ? 0 : 1
                text: block.text
                textFormat: TextEdit.PlainText
                wrapMode: Prefs.codeWrap ? TextEdit.Wrap : TextEdit.NoWrap
                tabStopDistance: fm.advanceWidth(" ") * Prefs.codeIndentSize
                onCursorRectangleChanged: codeView.follow(cursorRectangle)
                onWidthChanged: codeView.follow(cursorRectangle)
                selectByMouse: true
                persistentSelection: true
                color: fence.meta ? Theme.muted : Theme.foreground
                selectionColor: Theme.selection
                selectedTextColor: color
                font.family: Ui.monoFont
                font.pixelSize: fence.size
                cursorDelegate: Caret {}
                Keys.onPressed: event => block.keyPressed(this, event)
                // Format changes (highlighting, line height) also emit
                // textChanged; only a change of text is an edit.
                property string seen
                onTextChanged: {
                    if (text === seen)
                        return
                    seen = text
                    hl.fixLineHeight(fence.lineHeight)
                    block.editor.edited(block.index, text, cursorPosition)
                }
                Component.onCompleted: {
                    hl.dim(Ui.focusMode, -1)
                    hl.attach(code.textDocument)
                    hl.fixLineHeight(fence.lineHeight)
                }
                Text {
                    visible: block.kind === "frontmatter" && code.text === "" && code.preeditText === ""
                    text: qsTr("input YAML Front Matter. ")
                    color: Theme.faint
                    font: code.font
                }
            }
            }
            MathView {
                id: formula
                visible: fence.mathFence
                y: fence.folded ? 10 * block.scale : codeView.y + code.height + 12 * block.scale
                width: parent.width
                height: fence.mathFence ? implicitHeight : 0
                tex: fence.mathFence ? Ui.mathTex(block.text) : ""
                tags: block.mathTags
                em: block.em
                live: fence.mathFence
                keepLast: fence.active
                opacity: block.pictureOpacity
            }
            // The drawing: on its own while folded, else in Typora's
            // bordered panel under the code.
            Rectangle {
                id: panel
                visible: fence.diagramFence
                y: fence.folded ? fence.panelGap : box.height + fence.panelGap
                x: (parent.width - width) / 2
                width: fence.panelWidth
                height: fence.diagramFence ? diagram.height + (fence.folded ? 0 : 2) : 0
                color: "transparent"
                border.width: fence.folded ? 0 : 1
                border.color: Theme.border
                DiagramView {
                    id: diagram
                    x: panel.border.width
                    y: panel.border.width
                    width: panel.width - 2 * x
                    height: fence.diagramFence ? implicitHeight : 0
                    lang: fence.diagramLang
                    code: fence.diagramFence ? block.text : ""
                    em: block.em
                    zoom: block.scale
                    live: fence.diagramFence
                    opacity: block.pictureOpacity
                }
            }
            CodeHighlighter {
                id: hl
                language: fence.meta ? "" : fence.mathFence ? "latex" : block.info
                onLanguageChanged: refresh()
                // Focus mode greys every line but the caret's.
                readonly property int keep: code.activeFocus ? block.lineOf(code.text, code.cursorPosition) : -1
                readonly property var dimming: [Ui.focusMode, keep]
                onDimmingChanged: dim(Ui.focusMode, keep)
                hits: block.hits
                onHitsChanged: refresh()
                // A broken diagram's code, but not while the caret is on
                // its line, as Typora's. (Repainting signals the edit's
                // text, so this reads the block's.)
                errors: {
                    const m = JSON.parse(diagram.mark)
                    return m.length && code.activeFocus
                        && block.lineOf(block.text, code.cursorPosition) === block.lineOf(block.text, m[0][0]) ? "[]" : diagram.mark
                }
                onErrorsChanged: refresh()
                readonly property int revision: Theme.revision
                onRevisionChanged: refresh()
            }

            Rectangle {
                id: langBox
                visible: fence.active
                anchors.right: parent.right
                y: box.height + (fence.diagramFence ? fence.panelGap + panel.height : 0) + 4 * block.scale
                width: Math.max(120 * block.scale, langInput.contentWidth + 20 * block.scale)
                height: langInput.contentHeight + 8 * block.scale
                color: Theme.panel
                border.color: langInput.activeFocus ? Theme.accent : Theme.border
                radius: 3

                TextInput {
                    id: langInput
                    readOnly: block.locked
                    anchors.fill: parent
                    leftPadding: 10 * block.scale
                    rightPadding: 10 * block.scale
                    verticalAlignment: TextInput.AlignVCenter
                    text: fence.info
                    color: Theme.foreground
                    selectionColor: Theme.selection
                    selectedTextColor: Theme.foreground
                    selectByMouse: true
                    clip: true
                    font.family: Ui.uiFont
                    font.pixelSize: block.em * 0.8

                    Text {
                        visible: langInput.text === "" && langInput.preeditText === ""
                        x: langInput.leftPadding
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("code language")
                        color: Theme.faint
                        font: langInput.font
                    }

                    onActiveFocusChanged: {
                        if (activeFocus) {
                            text = fence.info
                        } else {
                            suggestions.close()
                            fence.commitLanguage()
                        }
                    }
                    onTextEdited: suggestions.update()
                    // Enter takes the highlighted suggestion and goes back to
                    // the code; Escape goes back without one.
                    Keys.onPressed: event => {
                        const list = suggestions.opened ? suggestionList : null
                        switch (event.key) {
                        case Qt.Key_Down:
                        case Qt.Key_Up:
                            if (list) {
                                list.currentIndex = (list.currentIndex + (event.key === Qt.Key_Down ? 1 : -1) + list.count) % list.count
                                event.accepted = true
                            }
                            return
                        case Qt.Key_Tab:
                        case Qt.Key_Return:
                        case Qt.Key_Enter:
                            if (list && list.currentIndex >= 0)
                                text = suggestions.matches[list.currentIndex]
                            if (event.key === Qt.Key_Tab && list) {
                                suggestions.close()
                                event.accepted = true
                                return
                            }
                            event.accepted = true
                            fence.commitLanguage()
                            code.forceActiveFocus()
                            return
                        case Qt.Key_Escape:
                            event.accepted = true
                            if (suggestions.opened) {
                                suggestions.close()
                                return
                            }
                            text = fence.info
                            code.forceActiveFocus()
                            return
                        }
                    }
                }

                Popup {
                    id: suggestions
                    property var matches: []
                    // Languages starting with what was typed, then those
                    // containing it.
                    function update() {
                        const t = langInput.text.trim().toLowerCase()
                        if (t === "") {
                            close()
                            return
                        }
                        const all = Desktop.codeLanguages().split("\n")
                        const starts = all.filter(l => l.startsWith(t))
                        matches = starts.concat(all.filter(l => !l.startsWith(t) && l.indexOf(t) >= 0))
                        suggestionList.currentIndex = matches.length > 0 ? 0 : -1
                        if (matches.length > 0 && !(matches.length === 1 && matches[0] === t))
                            open()
                        else
                            close()
                    }
                    y: parent.height + 2
                    x: parent.width - width
                    width: Math.max(parent.width, 160 * block.scale)
                    height: Math.min(8, matches.length) * 24 * block.scale + 2
                    padding: 1
                    focus: false
                    closePolicy: Popup.CloseOnPressOutsideParent
                    background: Rectangle {
                        color: Theme.panel
                        border.color: Theme.border
                        radius: 3
                    }
                    contentItem: ListView {
                        id: suggestionList
                        clip: true
                        model: suggestions.matches
                        boundsBehavior: Flickable.StopAtBounds
                        delegate: Rectangle {
                            id: choice
                            required property string modelData
                            required property int index
                            width: ListView.view.width
                            height: 24 * block.scale
                            color: ListView.isCurrentItem ? Theme.selection : "transparent"
                            Text {
                                x: 10 * block.scale
                                anchors.verticalCenter: parent.verticalCenter
                                text: choice.modelData
                                color: Theme.foreground
                                font.family: Ui.uiFont
                                font.pixelSize: block.em * 0.8
                            }
                            MouseArea {
                                anchors.fill: parent
                                onClicked: {
                                    langInput.text = choice.modelData
                                    suggestions.close()
                                    fence.commitLanguage()
                                    code.forceActiveFocus()
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Display math, typeset. Editing it opens Typora's panel: the source
    // between $$ lines, with a live preview beneath and a "Math" tab with a
    // Finish Editing button above. The source stays in the tree, folded
    // away, so the caret can move into it like any other block.
    Component {
        id: mathBlock
        Item {
            id: mathItem
            readonly property Item editItem: source
            readonly property alias tagFields: tagFields
            readonly property alias formula: rendered
            readonly property bool editing: (source.activeFocus || tagFields.focused) && !block.locked || block.found
            readonly property real pad: 10 * block.scale
            width: parent ? parent.width : 0
            height: editing ? panel.height : rendered.height + 2 * pad

            // The Finish Editing button takes clicks itself.
            function controlAt(x, y) {
                return finish.visible && finish.contains(finish.mapFromItem(mathItem, x, y)) ? finish : null
            }

            MathView {
                id: rendered
                visible: !mathItem.editing
                y: mathItem.pad
                width: parent.width
                height: implicitHeight
                tex: Ui.mathTex(block.text)
                tags: block.mathTags
                em: block.em
                opacity: block.pictureOpacity
            }

            Rectangle {
                visible: mathItem.editing
                width: parent.width
                height: panel.height
                color: Theme.codeBackground
                border.color: Qt.alpha(Theme.border, 0.6)
                radius: 3
            }

            Item {
                id: panel
                width: parent.width
                height: mathItem.editing ? preview.y + preview.height + mathItem.pad : 0
                clip: true

                Text {
                    id: before
                    x: mathItem.pad
                    y: 6 * block.scale
                    text: block.info === "\\[" ? "\\[" : "$$"
                    color: Theme.faint
                    font.family: Ui.monoFont
                    font.pixelSize: block.em * 0.8
                }
                TextEdit {
                    id: source
                    readonly property var highlighter: texHighlighter
                    readOnly: block.locked
                    x: mathItem.pad
                    y: before.y + before.height + 4 * block.scale
                    width: parent.width - 2 * mathItem.pad
                    text: block.text
                    textFormat: TextEdit.PlainText
                    wrapMode: TextEdit.Wrap
                    selectByMouse: true
                    persistentSelection: true
                    color: Theme.foreground
                    selectionColor: Theme.selection
                    selectedTextColor: Theme.foreground
                    font.family: Ui.monoFont
                    font.pixelSize: block.em * 0.9
                    cursorDelegate: Caret {}
                    Keys.onPressed: event => block.keyPressed(this, event)
                    onTextChanged: block.editor.edited(block.index, text, cursorPosition)
                    Component.onCompleted: {
                        texHighlighter.dim(Ui.focusMode, -1)
                        texHighlighter.attach(source.textDocument)
                    }

                    Text {
                        visible: source.length === 0
                        text: qsTr("Input Math TeX here")
                        color: Theme.faint
                        font: source.font
                    }
                }
                CodeHighlighter {
                    id: texHighlighter
                    language: "latex"
                    readonly property int keep: source.activeFocus ? block.lineOf(source.text, source.cursorPosition) : -1
                    readonly property var dimming: [Ui.focusMode, keep]
                    onDimmingChanged: dim(Ui.focusMode, keep)
                    hits: block.hits
                    onHitsChanged: refresh()
                    readonly property int revision: Theme.revision
                    onRevisionChanged: refresh()
                }
                Text {
                    id: after
                    x: mathItem.pad
                    y: source.y + source.height + 4 * block.scale
                    text: block.info === "\\[" ? "\\]" : "$$"
                    color: Theme.faint
                    font.family: Ui.monoFont
                    font.pixelSize: block.em * 0.8
                }
                // The {tag} each $$ line can carry, in Typora's fields
                // beside them; braced once there is one.
                Repeater {
                    id: tagFields
                    readonly property bool focused: count === 2 && (itemAt(0).activeFocus || itemAt(1).activeFocus)
                    readonly property var tags: block.info === "$$" && block.rows ? JSON.parse(block.rows) : ["", ""]
                    function commit() {
                        block.editor.setMathTags(block.index, itemAt(0).text, itemAt(1).text)
                    }
                    model: block.info === "$$" ? [before, after] : []
                    TextInput {
                        required property Item modelData
                        readOnly: block.locked
                        required property int index
                        readonly property bool braced: text !== "" || activeFocus
                        x: modelData.x + modelData.width
                        y: modelData.y
                        leftPadding: tagMetrics.advanceWidth(braced ? " {" : " ")
                        rightPadding: tagMetrics.advanceWidth(braced ? "}" : " ")
                        text: tagFields.tags[index]
                        color: Theme.foreground
                        selectByMouse: true
                        selectionColor: Theme.selection
                        selectedTextColor: Theme.foreground
                        font: before.font
                        onEditingFinished: {
                            tagFields.commit()
                            text = Qt.binding(() => tagFields.tags[index])
                        }
                        Keys.onReturnPressed: source.forceActiveFocus()
                        Keys.onEnterPressed: source.forceActiveFocus()
                        FontMetrics {
                            id: tagMetrics
                            font: before.font
                        }
                        Text {
                            visible: parent.braced
                            x: tagMetrics.advanceWidth(" ")
                            text: "{"
                            color: Theme.faint
                            font: parent.font
                        }
                        Text {
                            visible: parent.braced
                            x: parent.width - parent.rightPadding
                            text: "}"
                            color: Theme.faint
                            font: parent.font
                        }
                    }
                }
                Rectangle {
                    y: after.y + after.height + 6 * block.scale
                    width: parent.width
                    height: 1
                    color: Qt.alpha(Theme.border, 0.6)
                }
                MathView {
                    id: preview
                    y: after.y + after.height + 6 * block.scale + mathItem.pad
                    width: parent.width
                    height: implicitHeight
                    // Only while editing, so a document's formulas typeset once.
                    live: mathItem.editing
                    tex: Ui.mathTex(block.text)
                    tags: block.mathTags
                    em: block.em
                    keepLast: true
                }
            }

            // Typora's tab above the panel's top right corner.
            Rectangle {
                id: tab
                visible: mathItem.editing
                anchors.right: parent.right
                y: -height
                width: tabRow.width + 12 * block.scale
                height: 18 * block.scale
                color: Theme.codeBackground
                radius: 3
                Row {
                    id: tabRow
                    anchors.centerIn: parent
                    spacing: 6 * block.scale
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Math")
                        color: Theme.foreground
                        opacity: 0.5
                        font.family: Ui.textFont
                        font.pixelSize: 12 * block.scale
                    }
                    Rectangle {
                        id: finish
                        // Leaves the block, as Typora's button does.
                        function activate() {
                            block.host.leaveBlock(block.index)
                        }
                        anchors.verticalCenter: parent.verticalCenter
                        width: 18 * block.scale
                        height: 16 * block.scale
                        radius: 2
                        color: "transparent"
                        Glyph {
                            anchors.centerIn: parent
                            name: "check"
                            size: 12 * block.scale
                            color: Theme.foreground
                        }
                    }
                }
            }
        }
    }

    // HTML blocks, as Typora draws them: the markup rendered, or, for the
    // tags it won't draw (scripts, styles and the like), the source, faint.
    // A click opens the source in a panel under Typora's raw block tab,
    // except on a link or a <details> summary, which work as on a page.
    Component {
        id: htmlBlock
        Item {
            id: htmlItem
            readonly property Item editItem: source
            readonly property bool editing: source.activeFocus && !block.locked || block.found
            readonly property string look: block.editor.htmlLook(block.text)
            // The <details> the reader opened or closed, by order.
            property var toggled: []
            readonly property Item page: rendered
            readonly property bool hovered: hover.hovered
            readonly property real pad: 4 * block.scale
            width: parent ? parent.width : 0
            height: editing ? panel.height : Math.max(20 * block.scale, look === "rendered" ? rendered.height : raw.height)

            HoverHandler {
                id: hover
            }

            // Theme colours for the page, opaque, as CSS takes them.
            function css(c) {
                return Qt.rgba(c.r, c.g, c.b, 1).toString()
            }

            // The tab's button and the page's links take clicks themselves.
            function controlAt(x, y) {
                if (tab.visible && button.contains(button.mapFromItem(htmlItem, x, y)))
                    return button
                if (editing || look !== "rendered")
                    return null
                const p = rendered.mapFromItem(htmlItem, x, y)
                const link = rendered.linkAt(p.x, p.y)
                return link === "" ? null : { activate: () => htmlItem.follow(link) }
            }

            function follow(link) {
                if (link.startsWith("typo-details:")) {
                    const n = parseInt(link.substring(13))
                    const t = toggled.slice()
                    const i = t.indexOf(n)
                    if (i >= 0)
                        t.splice(i, 1)
                    else
                        t.push(n)
                    toggled = t
                } else if (block.host) {
                    block.host.openLink(link)
                }
            }

            // Hovering tints the block as Typora's edit panel.
            Rectangle {
                visible: htmlItem.editing || htmlItem.hovered
                width: parent.width
                height: parent.height
                color: Theme.codeBackground
                radius: 3
            }

            Text {
                id: rendered
                visible: !htmlItem.editing && htmlItem.look === "rendered"
                width: parent.width
                text: htmlItem.look === "rendered"
                      ? block.editor.htmlView(block.text, htmlItem.toggled.join(","), width,
                                              [Theme.border, Theme.codeBackground, Theme.mark, Theme.faint, Theme.link, Theme.foreground].map(htmlItem.css).join(","))
                      : ""
                textFormat: Text.RichText
                wrapMode: Text.Wrap
                color: Theme.foreground
                font.family: Ui.textFont
                font.pixelSize: block.em
            }

            // Source Typora shows as is: white space runs as one space, as
            // CSS's pre-line has it, in a faint monospace; comments in the
            // theme's yellow.
            Text {
                id: raw
                visible: !htmlItem.editing && htmlItem.look !== "rendered"
                width: parent.width
                text: block.text.split("\n").map(l => l.replace(/[ \t]+/g, " ").trim()).join("\n")
                textFormat: Text.PlainText
                wrapMode: Text.Wrap
                color: htmlItem.look === "comment" ? Theme.color("color3") : Theme.foreground
                opacity: 0.6
                font.family: Ui.monoFont
                font.pixelSize: block.em * 0.9
            }

            Item {
                id: panel
                width: parent.width
                height: htmlItem.editing ? source.height + 12 * block.scale : 0
                clip: true

                TextEdit {
                    id: source
                    readonly property var highlighter: htmlHighlighter
                    readOnly: block.locked
                    x: htmlItem.pad
                    y: 6 * block.scale
                    width: parent.width - 2 * htmlItem.pad
                    text: block.text
                    textFormat: TextEdit.PlainText
                    wrapMode: TextEdit.Wrap
                    selectByMouse: true
                    persistentSelection: true
                    color: Theme.foreground
                    selectionColor: Theme.selection
                    selectedTextColor: Theme.foreground
                    font.family: Ui.monoFont
                    font.pixelSize: block.em * 0.9
                    cursorDelegate: Caret {}
                    Keys.onPressed: event => block.keyPressed(this, event)
                    onTextChanged: block.editor.edited(block.index, text, cursorPosition)
                    Component.onCompleted: {
                        htmlHighlighter.dim(Ui.focusMode, -1)
                        htmlHighlighter.attach(source.textDocument)
                    }
                }
                CodeHighlighter {
                    id: htmlHighlighter
                    language: "html"
                    readonly property int keep: source.activeFocus ? block.lineOf(source.text, source.cursorPosition) : -1
                    readonly property var dimming: [Ui.focusMode, keep]
                    onDimmingChanged: dim(Ui.focusMode, keep)
                    hits: block.hits
                    onHitsChanged: refresh()
                    readonly property int revision: Theme.revision
                    onRevisionChanged: refresh()
                }
            }

            // Typora's tab above the block's top right corner: the edit
            // button on hover, Finish Editing while editing.
            Rectangle {
                id: tab
                visible: htmlItem.editing || htmlItem.hovered
                anchors.right: parent.right
                y: -height
                width: tabRow.width + 12 * block.scale
                height: 16 * block.scale
                color: Theme.codeBackground
                radius: 3
                Row {
                    id: tabRow
                    anchors.centerIn: parent
                    spacing: 4 * block.scale
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: "HTML"
                        color: Theme.foreground
                        opacity: 0.5
                        font.family: Ui.textFont
                        font.pixelSize: 12 * block.scale
                    }
                    Rectangle {
                        id: button
                        function activate() {
                            if (htmlItem.editing) {
                                block.host.leaveBlock(block.index)
                            } else {
                                source.cursorPosition = 0
                                source.forceActiveFocus()
                            }
                        }
                        anchors.verticalCenter: parent.verticalCenter
                        width: 20 * block.scale
                        height: 16 * block.scale
                        radius: 2
                        color: "transparent"
                        Glyph {
                            anchors.centerIn: parent
                            name: htmlItem.editing ? "check" : "code"
                            size: 12 * block.scale
                            color: Theme.foreground
                        }
                    }
                }
            }
        }
    }

    // Tables, edited cell by cell.
    Component {
        id: tableBlock
        TableBlock {
            owner: block
        }
    }

    // Horizontal rule.
    Component {
        id: ruleBlock
        Item {
            width: parent ? parent.width : 0
            height: 2 * block.scale
            Rectangle {
                anchors.fill: parent
                color: Theme.border
            }
        }
    }

    // Table of contents: the document's headings, as Typora's `.md-toc`.
    // A click on an entry goes to the end of its heading; elsewhere it
    // selects the TOC, which then shows a dashed frame and a header with a
    // delete button.
    Component {
        id: tocBlock
        Item {
            id: toc
            width: parent ? parent.width : 0
            height: Math.max(3.58 * Ui.fontSize, list.height)
            readonly property real em: block.em * 0.9
            readonly property var entries: JSON.parse(block.editor.outline)
            readonly property bool focused: block.alone
            // Typora's indents by level, in ems of the entry's font.
            readonly property var indents: [0, 2, 3.5, 5, 6.5, 8]
            // For scripts.
            readonly property alias deleteButton: deleteButton
            function entry(i) {
                return list.children.filter(c => c.activate)[i]
            }

            function controlAt(x, y) {
                if (header.visible && y < 0) {
                    const p = mapToItem(deleteButton, x, y)
                    return deleteButton.contains(p) ? deleteButton : null
                }
                const p = mapToItem(list, x, y)
                const e = list.childAt(p.x, p.y)
                if (!e || !e.activate)
                    return null
                return p.x >= e.x + e.leftPadding && p.x <= e.x + e.textRight ? e : null
            }

            Column {
                id: list
                width: parent.width
                Repeater {
                    model: toc.entries
                    delegate: Text {
                        id: entry
                        required property var modelData
                        function activate() {
                            block.host.jumpToRow(modelData.row, true)
                        }
                        // Where the entry's text ends: a click past it misses.
                        readonly property real textRight: Math.min(implicitWidth, width)
                        width: list.width
                        leftPadding: toc.indents[Math.min(5, modelData.level - 1)] * toc.em
                        topPadding: toc.em * 0.15
                        bottomPadding: toc.em * 0.15
                        textFormat: Text.RichText
                        wrapMode: Text.Wrap
                        // Links inside a heading are its text here; pictures
                        // show, at most as wide as the entry, as Typora's.
                        text: modelData.html.replace(/<\/?a\b[^>]*>/g, "")
                            .replace(/<img\b[^>]*>/g, img => block.editor.htmlView(img, "", width - leftPadding, ""))
                        color: Theme.link
                        font.family: Ui.textFont
                        font.pixelSize: toc.em
                        font.bold: modelData.level === 1
                        font.underline: hover.hovered
                        Item {
                            x: entry.leftPadding
                            width: entry.textRight - entry.leftPadding
                            height: entry.height
                            HoverHandler {
                                id: hover
                                cursorShape: Qt.PointingHandCursor
                            }
                        }
                    }
                }
            }
            Text {
                visible: toc.entries.length === 0
                text: "[TOC]"
                color: Theme.link
                font.family: Ui.textFont
                font.pixelSize: toc.em * 2
            }

            // The frame and header of a selected TOC.
            Shape {
                visible: toc.focused
                x: -1
                y: -1
                width: toc.width + 2
                height: toc.height + 2
                ShapePath {
                    strokeColor: Theme.border
                    strokeWidth: 1
                    strokeStyle: ShapePath.DashLine
                    fillColor: "transparent"
                    startX: 0.5; startY: 0.5
                    PathLine { x: toc.width + 1.5; y: 0.5 }
                    PathLine { x: toc.width + 1.5; y: toc.height + 1.5 }
                    PathLine { x: 0.5; y: toc.height + 1.5 }
                    PathLine { x: 0.5; y: 0.5 }
                }
            }
            Rectangle {
                id: header
                visible: toc.focused
                y: -height - 1
                width: toc.width
                height: 30 * block.scale
                color: Theme.background
                Rectangle {
                    width: parent.width
                    height: 1
                    color: Theme.border
                }
                Row {
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 6 * block.scale
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: qsTr("Table of Contents")
                        color: Theme.foreground
                        font.family: Ui.textFont
                        font.pixelSize: toc.em
                        font.bold: true
                    }
                    Rectangle {
                        id: deleteButton
                        function activate() {
                            block.host.command("deleteBlock")
                        }
                        anchors.verticalCenter: parent.verticalCenter
                        width: 24 * block.scale
                        height: 20 * block.scale
                        radius: 2 * block.scale
                        color: "transparent"
                        Glyph {
                            anchors.centerIn: parent
                            name: "trash"
                            size: 13 * block.scale
                            color: Theme.muted
                        }
                    }
                }
            }
        }
    }

    // Anything else (link reference definitions) as muted source.
    Component {
        id: rawBlock
        Text {
            width: parent ? parent.width : 0
            text: block.text
            wrapMode: Text.Wrap
            color: block.dimmed ? Theme.blur : Theme.muted
            font.family: Ui.textFont
            font.pixelSize: block.em * 0.9
        }
    }
}
