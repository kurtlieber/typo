import QtQuick
import QtQuick.Controls
import Typo

// The editor's right-click menu, laid out as Typora's: what can be done
// with an image, clipboard buttons, Copy / Paste As…, style and block
// buttons, then Paragraph and Insert. What shows depends on where it
// opened: an image, a link, a list or quote, a code block, a table.
TMenu {
    id: menu

    required property var app

    // Where it opened, set by `show`.
    property string kind: "paragraph"
    property int level: 0
    property var context: []
    property string link: ""
    property bool selected: false
    // The image it opened on (Main.imageHere), or null.
    property var image: null
    readonly property bool hasImage: image !== null
    // A local image whose file is there; Typora offers copying and moving
    // only an image that loaded.
    readonly property bool localImage: hasImage && image.path !== ""
    readonly property bool loadedFile: localImage && Desktop.exists(image.path)
    readonly property string imageFolder: localImage ? image.folder : ""
    // The formula it opened in (Main.mathHere), or null. In inline math or
    // a math block the copy menu has math's copies only; a ```math fence's
    // formula can be copied as an image too.
    property var math: null
    readonly property bool inMath: math !== null && !math.fence

    readonly property bool normal: kind === "paragraph" || kind === "heading"
    readonly property bool codeLike: ["code", "math", "html", "frontmatter"].indexOf(kind) >= 0
    readonly property var names: ({
        code: qsTr("Code Fences"), math: qsTr("Math Block"), html: qsTr("HTML Block"), table: qsTr("Table"),
        hr: qsTr("Horizontal Line"), frontmatter: qsTr("YAML Front Matter"), toc: qsTr("Table of Contents"),
        linkref: qsTr("Link Reference")
    })
    readonly property string blockName: kind === "heading" ? qsTr("Heading %1").arg(level)
                                       : kind === "paragraph" ? qsTr("Paragraph") : (names[kind] || qsTr("Paragraph"))

    function run(cmd, arg) {
        menu.close()
        app.run(cmd, arg)
    }

    // Open at window point (x, y) for the selection's block.
    function show(x, y) {
        const view = app.view
        const s = view.selection()
        const item = view.itemAt(s.zRow)
        kind = item ? item.kind : "paragraph"
        level = item ? item.level : 0
        context = app.editor.contextAt(s.zRow, s.zPos).split(",")
        link = app.editor.linkAt(s.zRow, s.zPos)
        image = app.imageHere()
        math = app.mathHere()
        selected = !view.collapsed(s)
        // Text, not an image or a formula.
        searchText = selected && !hasImage && math === null
            ? Desktop.plainText(app.editor.selectionHtml(s.aRow, s.aPos, s.zRow, s.zPos)).trim() : ""
        // (x, y) is in the scene; the menu's parent starts below the menu bar.
        const at = parent.mapFromItem(null, x, y)
        popup(at.x, at.y)
    }

    // A row of icon buttons, as Typora puts its styles in the menu.
    component ButtonRow: Item {
        id: row
        // [{ glyph or label, tip, cmd, arg, active, enabled }]
        property var buttons: []
        property bool wide: false
        implicitWidth: 220
        implicitHeight: 30
        height: visible ? implicitHeight : 0
        Row {
            anchors.centerIn: parent
            spacing: 5
            Repeater {
                model: row.buttons
                delegate: AbstractButton {
                    id: button
                    required property var modelData
                    width: row.wide ? 89 : 42
                    height: 22
                    enabled: modelData.enabled !== false
                    hoverEnabled: true
                    onClicked: menu.run(modelData.cmd, modelData.arg)
                    onHoveredChanged: {
                        if (hovered)
                            menu.showTip(button, modelData.tip)
                        else
                            tip.close()
                    }
                    background: Rectangle {
                        radius: 3
                        color: button.hovered && button.enabled ? Theme.accent
                             : button.modelData.active ? Theme.selection : "transparent"
                        border.color: Theme.border
                    }
                    contentItem: Item {
                        readonly property color ink: !button.enabled ? Theme.faint
                                                   : button.hovered ? Theme.accentText : Theme.foreground
                        Glyph {
                            anchors.centerIn: parent
                            visible: !!button.modelData.glyph
                            name: button.modelData.glyph || ""
                            color: parent.ink
                            size: 14
                        }
                        Text {
                            anchors.centerIn: parent
                            visible: !!button.modelData.label
                            text: button.modelData.label || ""
                            color: parent.ink
                            font.pixelSize: 13
                            font.bold: button.modelData.label === "B"
                            font.italic: button.modelData.label === "I"
                            font.family: button.modelData.label === "I" ? "serif" : menu.font.family
                        }
                    }
                }
            }
        }
    }

    function showTip(button, text) {
        tip.parent = button
        tip.text = text
        tip.open()
    }

    ToolTip {
        id: tip
        delay: 400
        y: parent ? parent.height + 6 : 0
        x: parent ? (parent.width - width) / 2 : 0
        contentItem: Text {
            text: tip.text
            color: Theme.foreground
            font.pixelSize: 12
        }
        background: Rectangle {
            color: Theme.panel
            border.color: Theme.border
            radius: 3
        }
    }

    onClosed: tip.close()

    // Search the selected text with conf.user.json's searchService, as
    // Typora offers it: the first service, the second too when there are
    // two, the others under Search With… when there are more.
    property string searchText: ""
    readonly property var services: (Prefs.advanced.searchService || []).filter(s => Array.isArray(s) && s.length >= 2)
    readonly property bool searching: searchText !== "" && services.length > 0
    function searchUrl(i) {
        return String(services[i][1]).replace("%s", encodeURIComponent(searchText))
    }
    function search(i) {
        menu.close()
        Qt.openUrlExternally(searchUrl(i))
    }
    TMenuItem {
        text: menu.services.length > 0 ? menu.services[0][0] : ""
        visible: menu.searching
        onTriggered: menu.search(0)
    }
    TMenuItem {
        text: menu.services.length === 2 ? menu.services[1][0] : ""
        visible: menu.searching && menu.services.length === 2
        onTriggered: menu.search(1)
    }
    TMenu {
        id: searchMenu
        title: qsTr("Search With…")
        Instantiator {
            model: menu.services.length > 2 ? menu.services.slice(1) : []
            delegate: TMenuItem {
                required property var modelData
                required property int index
                text: modelData[0]
                onTriggered: menu.search(index + 1)
            }
            onObjectAdded: (index, object) => searchMenu.insertItem(index, object)
            onObjectRemoved: (index, object) => searchMenu.removeItem(object)
        }
    }
    TSep { visible: menu.searching }

    TMenuItem {
        text: qsTr("Open Image Location…")
        visible: menu.localImage
        onTriggered: menu.run("revealImage")
    }
    TMenuItem {
        text: qsTr("Open Image in Browser…")
        visible: menu.hasImage && /^https?:/i.test(menu.image.url)
        onTriggered: menu.run("revealImage")
    }
    TMenuItem {
        text: qsTr("Copy to %1").arg(menu.imageFolder)
        visible: menu.loadedFile && menu.imageFolder !== ""
        onTriggered: menu.run("copyImageToFolder")
    }
    TMenuItem {
        text: qsTr("Copy Image to…")
        visible: menu.loadedFile
        onTriggered: menu.run("copyImageTo")
    }
    TMenuItem {
        text: qsTr("Move to %1").arg(menu.imageFolder)
        visible: menu.loadedFile && menu.imageFolder !== ""
        onTriggered: menu.run("moveImageToFolder")
    }
    TMenuItem {
        text: qsTr("Rename or Move Image to…")
        visible: menu.loadedFile
        onTriggered: menu.run("moveImageTo")
    }
    // A HiDPI picture's own size, as Typora offers it.
    TMenuItem {
        text: qsTr("Zoom Image to %1").arg(menu.hasImage ? Math.round(100 / menu.image.ratio) + "%" : "")
        visible: menu.hasImage && menu.image.ratio !== 1
        onTriggered: menu.run("zoomImage", Math.round(100 / menu.image.ratio) + "%")
    }
    TMenu {
        id: zoomMenu
        title: qsTr("Zoom Image")
        TMenuItem { text: "25%"; onTriggered: menu.run("zoomImage", "25%") }
        TMenuItem { text: "33%"; onTriggered: menu.run("zoomImage", "33%") }
        TMenuItem { text: "50%"; onTriggered: menu.run("zoomImage", "50%") }
        TMenuItem { text: "67%"; onTriggered: menu.run("zoomImage", "67%") }
        TMenuItem { text: "80%"; onTriggered: menu.run("zoomImage", "80%") }
        TMenuItem { text: "100%"; onTriggered: menu.run("zoomImage", "100%") }
        TMenuItem { text: "150%"; onTriggered: menu.run("zoomImage", "150%") }
        TMenuItem { text: "200%"; onTriggered: menu.run("zoomImage", "200%") }
    }
    TMenu {
        id: syntaxMenu
        title: qsTr("Switch Image Syntax")
        TMenuItem {
            text: qsTr("Markdown Style")
            checkable: true
            checked: menu.hasImage && menu.image.syntax === "markdown"
            onTriggered: menu.run("imageSyntax", "markdown")
        }
        TMenuItem {
            text: qsTr("HTML Style")
            checkable: true
            checked: menu.hasImage && menu.image.syntax === "html"
            onTriggered: menu.run("imageSyntax", "html")
        }
    }
    TMenuItem {
        text: qsTr("Delete Image File")
        visible: menu.localImage
        onTriggered: menu.run("deleteImageFile")
    }
    TMenuItem {
        text: qsTr("Delete Image")
        visible: menu.hasImage && !menu.localImage
        onTriggered: menu.run("deleteImage")
    }
    TSep { visible: menu.hasImage }

    TMenuItem {
        text: qsTr("Open Link")
        hint: qsTr("Ctrl+Click")
        visible: menu.link !== ""
        onTriggered: menu.run("openLink")
    }
    TSep { visible: menu.link !== "" }

    ButtonRow {
        buttons: [
            { glyph: "cut", tip: qsTr("Cut"), cmd: "cut", enabled: menu.selected },
            { glyph: "copy", tip: qsTr("Copy"), cmd: "copy", enabled: menu.selected },
            { glyph: "paste", tip: qsTr("Paste"), cmd: "paste" },
            { glyph: "trash", tip: qsTr("Delete Selected"), cmd: "deleteSelected", enabled: menu.selected }
        ]
    }
    TMenuItem {
        text: qsTr("Copy Link Address")
        visible: menu.link !== ""
        onTriggered: menu.run("copyLink")
    }
    TMenuItem {
        text: qsTr("Copy Image Content")
        visible: menu.loadedFile
        onTriggered: menu.run("copyImage")
    }
    TMenu {
        title: qsTr("Copy / Paste As…")
        TMenuItem { text: qsTr("Copy as Markdown"); hint: "Ctrl+Shift+C"; visible: !menu.inMath; enabled: menu.selected; onTriggered: menu.run("copyMarkdown") }
        TMenuItem { text: qsTr("Copy as HTML Code"); visible: !menu.inMath; enabled: menu.selected; onTriggered: menu.run("copyHtml") }
        TMenuItem { text: qsTr("Copy without Theme Styling"); visible: !menu.inMath; enabled: menu.selected; onTriggered: menu.run("copy") }
        TMenuItem { text: qsTr("Copy as Plain Text"); visible: !menu.inMath; enabled: menu.selected; onTriggered: menu.run("copyPlain") }
        TSep { visible: !menu.inMath && menu.math !== null }
        TMenuItem { text: qsTr("Copy as Image"); visible: menu.math !== null && menu.math.display; onTriggered: menu.run("copyMathImage") }
        TMenuItem { text: qsTr("Copy as MathML"); visible: menu.inMath; onTriggered: menu.run("copyMathML") }
        TMenuItem { text: qsTr("Copy as Tex"); visible: menu.inMath; onTriggered: menu.run("copyMathTex") }
        TSep { visible: !menu.inMath }
        TMenuItem { text: qsTr("Paste as Plain Text"); hint: "Ctrl+Shift+V"; visible: !menu.inMath; onTriggered: menu.run("pastePlain") }
    }
    TMenuItem {
        text: qsTr("Copy Code Content")
        visible: menu.kind === "code"
        onTriggered: menu.run("copyCode")
    }
    TMenuItem { text: qsTr("Auto Indent Selected Code"); visible: menu.kind === "code"; onTriggered: menu.run("autoIndentCode", "selected") }
    TMenuItem { text: qsTr("Auto Indent Whole Code"); visible: menu.kind === "code"; onTriggered: menu.run("autoIndentCode", "whole") }

    TSep {}

    ButtonRow {
        visible: !menu.codeLike
        buttons: [
            { label: "B", tip: qsTr("Strong") + "  Ctrl+B", cmd: "style", arg: "strong", active: menu.context.indexOf("strong") >= 0 },
            { label: "I", tip: qsTr("Emphasis") + "  Ctrl+I", cmd: "style", arg: "emphasis", active: menu.context.indexOf("em") >= 0 },
            { glyph: "code", tip: qsTr("Code") + "  Ctrl+Shift+`", cmd: "style", arg: "code", active: menu.context.indexOf("code") >= 0 },
            { glyph: "link", tip: qsTr("Hyperlink") + "  Ctrl+K", cmd: "link", active: menu.context.indexOf("link") >= 0 }
        ]
    }
    ButtonRow {
        visible: menu.normal
        buttons: [
            { glyph: "quote", tip: qsTr("Quote") + "  Ctrl+Shift+Q", cmd: "quote" },
            { glyph: "ordered", tip: qsTr("Ordered List") + "  Ctrl+Shift+[", cmd: "list", arg: "ordered" },
            { glyph: "bullet", tip: qsTr("Unordered List") + "  Ctrl+Shift+]", cmd: "list", arg: "bullet" },
            { glyph: "task", tip: qsTr("Task List"), cmd: "list", arg: "task" }
        ]
    }
    ButtonRow {
        visible: menu.normal && (menu.context.indexOf("list") >= 0 || menu.context.indexOf("quote") >= 0)
        wide: true
        buttons: [
            { glyph: "left", tip: qsTr("Outdent") + "  Shift+Tab", cmd: "outdent" },
            { glyph: "right", tip: qsTr("Indent") + "  Tab", cmd: "indent" }
        ]
    }

    TSep { visible: !menu.codeLike }

    TMenu {
        id: tableMenu
        title: qsTr("Table")
        TMenuItem { text: qsTr("Add Row Above"); onTriggered: menu.run("tableRow", "above") }
        TMenuItem { text: qsTr("Add Row Below"); hint: "Ctrl+Enter"; onTriggered: menu.run("tableRow", "below") }
        TMenuItem { text: qsTr("Add Column Before"); onTriggered: menu.run("tableColumn", "before") }
        TMenuItem { text: qsTr("Add Column After"); onTriggered: menu.run("tableColumn", "after") }
        TSep {}
        TMenuItem { text: qsTr("Move Row Up"); hint: "Alt+↑"; onTriggered: menu.run("tableMove", "up") }
        TMenuItem { text: qsTr("Move Row Down"); hint: "Alt+↓"; onTriggered: menu.run("tableMove", "down") }
        TMenuItem { text: qsTr("Move Column Left"); hint: "Alt+←"; onTriggered: menu.run("tableMove", "left") }
        TMenuItem { text: qsTr("Move Column Right"); hint: "Alt+→"; onTriggered: menu.run("tableMove", "right") }
        TSep {}
        TMenuItem { text: qsTr("Delete Row"); hint: "Ctrl+Shift+Backspace"; onTriggered: menu.run("tableDelete", "row") }
        TMenuItem { text: qsTr("Delete Column"); onTriggered: menu.run("tableDelete", "column") }
        TSep {}
        TMenuItem { text: qsTr("Copy Table"); onTriggered: menu.run("tableCopy") }
        TMenuItem { text: qsTr("Prettify Source Code"); onTriggered: menu.run("tableFormat") }
        TSep {}
        TMenuItem { text: qsTr("Delete Table"); onTriggered: menu.run("tableDelete", "table") }
    }
    TMenu {
        id: paragraphMenu
        // Named after the block it would change, as Typora does.
        title: menu.blockName
        TMenuItem {
            text: qsTr("Heading 1"); hint: "Ctrl+1"; enabled: menu.normal; checkable: true
            checked: menu.kind === "heading" && menu.level === 1
            onTriggered: menu.run("heading", 1)
        }
        TMenuItem {
            text: qsTr("Heading 2"); hint: "Ctrl+2"; enabled: menu.normal; checkable: true
            checked: menu.kind === "heading" && menu.level === 2
            onTriggered: menu.run("heading", 2)
        }
        TMenuItem {
            text: qsTr("Heading 3"); hint: "Ctrl+3"; enabled: menu.normal; checkable: true
            checked: menu.kind === "heading" && menu.level === 3
            onTriggered: menu.run("heading", 3)
        }
        TMenuItem {
            text: qsTr("Heading 4"); hint: "Ctrl+4"; enabled: menu.normal; checkable: true
            checked: menu.kind === "heading" && menu.level === 4
            onTriggered: menu.run("heading", 4)
        }
        TMenuItem {
            text: qsTr("Heading 5"); hint: "Ctrl+5"; enabled: menu.normal; checkable: true
            checked: menu.kind === "heading" && menu.level === 5
            onTriggered: menu.run("heading", 5)
        }
        TMenuItem {
            text: qsTr("Heading 6"); hint: "Ctrl+6"; enabled: menu.normal; checkable: true
            checked: menu.kind === "heading" && menu.level === 6
            onTriggered: menu.run("heading", 6)
        }
        TSep {}
        TMenuItem {
            text: qsTr("Paragraph")
            hint: "Ctrl+0"
            enabled: menu.normal
            checkable: true
            checked: menu.kind === "paragraph"
            onTriggered: menu.run("heading", 0)
        }
    }
    TMenu {
        title: qsTr("Insert")
        TMenuItem { text: qsTr("Image"); hint: Prefs.keysFor("Insert Image", "Ctrl+Shift+I"); visible: !menu.codeLike; onTriggered: menu.run("image") }
        TSep { visible: !menu.codeLike }
        TMenuItem { text: qsTr("Footnotes"); enabled: menu.normal; onTriggered: menu.run("footnote") }
        TMenuItem { text: qsTr("Link Reference"); enabled: menu.normal; onTriggered: menu.run("linkReference") }
        TMenuItem { text: qsTr("Horizontal Line"); enabled: menu.normal; onTriggered: menu.run("horizontalLine") }
        TMenuItem { text: qsTr("Table"); hint: Prefs.keysFor("Insert Table", "Ctrl+T"); enabled: menu.normal; onTriggered: menu.run("table") }
        TMenuItem { text: qsTr("Code Fences"); hint: "Ctrl+Shift+K"; enabled: menu.normal; onTriggered: menu.run("codeBlock") }
        TMenuItem { text: qsTr("Math Block"); hint: "Ctrl+Shift+M"; enabled: menu.normal; onTriggered: menu.run("mathBlock") }
        TMenuItem { text: qsTr("Table of Contents"); enabled: menu.normal; onTriggered: menu.run("toc") }
        TMenuItem { text: qsTr("YAML Front Matter"); enabled: menu.normal; onTriggered: menu.run("frontMatter") }
        TSep {}
        TMenuItem { text: qsTr("Paragraph (before)"); enabled: menu.kind !== "frontmatter"; onTriggered: menu.run("paragraphBefore") }
        TMenuItem { text: qsTr("Paragraph (after)"); onTriggered: menu.run("paragraphAfter") }
    }
    TSep { visible: !menu.normal }
    TMenuItem {
        text: qsTr("Delete %1").arg(menu.blockName)
        visible: !menu.normal
        onTriggered: menu.run("deleteBlock")
    }

    // Submenu entries are made by the menu, so show or hide them here.
    onAboutToShow: {
        for (let i = 0; i < count; i++) {
            const item = itemAt(i)
            if (item && item.subMenu === tableMenu)
                item.visible = kind === "table"
            else if (item && item.subMenu === paragraphMenu)
                item.visible = kind !== "html" && kind !== "math"
            else if (item && item.subMenu === zoomMenu)
                item.visible = hasImage
            else if (item && item.subMenu === syntaxMenu)
                item.visible = hasImage && image.syntax !== "ref"
            else if (item && item.subMenu === searchMenu)
                item.visible = searching && services.length > 2
        }
        fit()
    }
}
