import QtCore
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Typo

ApplicationWindow {
    id: win

    readonly property alias editor: model
    readonly property alias view: view
    readonly property var recentFiles: Prefs.recentFiles
    property string folder: ""
    readonly property string folderName: folder === "" ? "" : folder.substring(folder.lastIndexOf("/") + 1)

    // The block with the caret and its heading level (0 for a paragraph,
    // -1 for anything else), for the menus.
    readonly property int currentRow: view.currentRow
    readonly property int blockLevel: {
        const item = currentRow >= 0 ? view.itemAt(currentRow) : null
        if (!item)
            return -1
        return item.kind === "heading" ? item.level : item.kind === "paragraph" ? 0 : -1
    }
    // The heading the outline marks: as Typora's, the one the page shows
    // (`EditorView.headingInView`), looked for again a moment after the
    // view scrolls or the document changes, not where the caret is.
    property int currentHeading: -1
    Timer {
        id: headingTimer
        interval: 100
        onTriggered: {
            if (!Ui.sourceMode)
                win.currentHeading = view.headingInView(JSON.parse(model.outline).map(h => h.row))
        }
    }
    Connections {
        target: view
        function onContentYChanged() { headingTimer.restart() }
        function onHeightChanged() { headingTimer.restart() }
        function onContentHeightChanged() { headingTimer.restart() }
    }
    Connections {
        target: model
        function onOutlineChanged() { headingTimer.restart() }
    }

    width: 1100
    height: 760
    minimumWidth: 420
    minimumHeight: 300
    visible: true
    title: (model.modified ? "• " : "") + model.title + " - typo"
    color: Theme.background
    flags: Ui.alwaysOnTop ? (Qt.Window | Qt.WindowStaysOnTopHint) : Qt.Window
    font.family: Ui.uiFont
    font.pixelSize: 14

    palette {
        window: Theme.panel
        windowText: Theme.foreground
        base: Theme.background
        alternateBase: Theme.codeBackground
        text: Theme.foreground
        button: Theme.panel
        buttonText: Theme.foreground
        brightText: Theme.foreground
        highlight: Theme.accent
        highlightedText: Theme.accentText
        light: Theme.codeBackground
        midlight: Theme.codeBackground
        mid: Theme.border
        dark: Theme.border
        shadow: Theme.panel
        placeholderText: Theme.faint
        toolTipBase: Theme.panel
        toolTipText: Theme.foreground
        link: Theme.link
        linkVisited: Theme.link
    }

    ContextMenu {
        id: contextMenu
        app: win
    }

    Editor {
        id: model
        onFailed: message => toast.show(message)
        onContentChanged: stats.restart()
        onPathChanged: {
            Prefs.addRecent(path)
            // With no folder open, the sidebar shows the file's own, as
            // Typora mounts it.
            if (path !== "" && win.folder === "")
                win.folder = win.folderOf(path)
        }
    }
    // The sidebar follows its folder on disk.
    onFolderChanged: Folder.watch(folder)

    // Preferences ▸ Markdown: the optional inline syntax.
    // Remapping is forced on while smart punctuation converts on input.
    readonly property bool remapPunctuation: Prefs.remapPunctuation || !Prefs.smartOnRendering && (Prefs.smartQuotes || Prefs.smartDashes)
    readonly property string inlineOptions: [Prefs.inlineMath, Prefs.highlight, Prefs.superscript, Prefs.subscript, Prefs.latexDelimiters,
        Prefs.autoLinks, Prefs.alerts, Prefs.strictMode, remapPunctuation].join()
    onInlineOptionsChanged: applyInlineOptions()
    readonly property string smartOptions: [Prefs.smartQuotes, Prefs.smartDashes, Prefs.dashesToEm, Prefs.quotePairs,
        Prefs.smartOnRendering].join()
    onSmartOptionsChanged: applySmart()
    function applySmart() {
        model.setSmart(Prefs.smartQuotes, Prefs.smartDashes, Prefs.dashesToEm, Prefs.quotePairs, Prefs.smartOnRendering)
    }
    // Preferences ▸ Markdown: Visible <br/>, math copied as HTML, and the
    // language new code fences get.
    readonly property var writingOptions: [Prefs.visibleBr, Prefs.mathCopyFormat, Prefs.defaultCodeLang,
        Prefs.defaultCodeLangWhen]
    onWritingOptionsChanged: applyWriting()
    function applyWriting() {
        model.setWriting(!Prefs.visibleBr, Prefs.mathCopyFormat === "svg", Prefs.defaultCodeLang, Prefs.defaultCodeLangWhen)
    }
    function applyInlineOptions() {
        model.setInlineOptions(Prefs.inlineMath, Prefs.highlight, Prefs.superscript, Prefs.subscript, Prefs.latexDelimiters,
                               Prefs.autoLinks, Prefs.alerts, Prefs.strictMode, remapPunctuation)
    }

    // Preferences ▸ Files: which files the sidebar shows.
    readonly property var folderFilters: [Prefs.showHiddenFiles, Prefs.treeFilter, Prefs.treePatterns, Prefs.libraryFilter,
        Prefs.libraryPatterns]
    onFolderFiltersChanged: applyFolderFilters()
    function applyFolderFilters() {
        Folder.setFilters(Prefs.showHiddenFiles, Prefs.treeFilter, Prefs.treePatterns, Prefs.libraryFilter, Prefs.libraryPatterns)
    }

    // Preferences ▸ Editor and ▸ Markdown: how new lists, quotes and
    // headings are written. Indent size Auto is 2, as in Typora, and Tab
    // nests by 4 columns.
    readonly property var syntaxOptions: [Prefs.bulletMarker, Prefs.orderedSame, Prefs.indentSize, Prefs.prettyIndent,
        Prefs.headingStyle]
    onSyntaxOptionsChanged: applySyntax()
    readonly property bool newCrlf: Prefs.defaultLineEnding === "crlf"
    onNewCrlfChanged: model.setNewLineEnding(newCrlf)
    function applySyntax() {
        const indent = Prefs.indentSize < 0 ? 4 : Math.max(2, Prefs.indentSize)
        model.setSyntax(Prefs.bulletMarker, Prefs.orderedSame, indent, Prefs.prettyIndent && Prefs.indentSize >= 0,
                        Prefs.headingStyle)
    }

    // Preferences ▸ Image: what inserting an image file does.
    readonly property string imageOptions: [Prefs.imageInsertAction, Prefs.imageFolder, Prefs.imageCopyLocal,
        Prefs.imageRelative, Prefs.imageWithDot, Prefs.imageEscape].join()
    onImageOptionsChanged: applyImageOptions()
    function applyImageOptions() {
        model.setImageOptions(Prefs.imageInsertAction, Prefs.imageFolder, Prefs.imageCopyLocal, Prefs.imageRelative,
            Prefs.imageWithDot, Prefs.imageEscape)
    }

    // Insert files at the selection (Insert Local Images…, a drop, a
    // paste). Copying images into a folder that isn't there yet asks
    // first, as Typora does.
    function insertFiles(paths) {
        const list = paths.filter(p => p !== "").join("\n")
        if (list === "")
            return
        const s = view.selection()
        const folder = model.missingImageFolder(list)
        const insert = create => {
            const error = view.insertFiles(list, create, s)
            if (error !== "")
                toast.show(qsTr("Failed to Copy Image to Target Folder.") + " " + error)
        }
        if (folder === "") {
            insert(false)
            return
        }
        dialog.ask(qsTr("Copy Image to…"),
            qsTr("typo is trying to copy the newly inserted image to folder %1, but the folder does not exist. Create it now?").arg(folder),
            [qsTr("Yes"), qsTr("No")], i => insert(i === 0))
    }

    // Files dropped on the page, as Typora takes them: a folder opens in
    // the sidebar, a markdown or text file in place of the document and a
    // file pandoc reads is imported, unless Preferences ▸ Files says to
    // insert a link to it instead; anything else is inserted where it was
    // dropped.
    readonly property var textFiles: ["md", "markdown", "mmd", "mkd", "mdwn", "mdown", "mdx", "mdtxt", "mdtext", "apib",
        "rmarkdown", "rmd", "qmd", "txt", "text"]
    readonly property var importable: ["docx", "odt", "rtf", "epub", "html", "htm", "tex", "latex", "rst", "textile", "wiki",
        "mediawiki", "opml", "org", "ipynb", "fb2", "dbk"]
    function dropFiles(paths) {
        const first = paths[0]
        const name = first.substring(first.lastIndexOf("/") + 1)
        const ext = name.indexOf(".") > 0 ? name.substring(name.lastIndexOf(".") + 1).toLowerCase() : ""
        if (Desktop.isFolder(first)) {
            if (Prefs.dropFolder === 0)
                openFolder(first)
            else
                insertFiles(paths)
        } else if (ext === "" || textFiles.indexOf(ext) >= 0) {
            if (Prefs.dropMarkdown === 0)
                openFile(first)
            else
                insertFiles(paths)
        } else if (importable.indexOf(ext) >= 0 && Prefs.dropImportable === 0) {
            if (requirePandoc())
                importFile(first)
        } else {
            insertFiles(paths)
        }
    }

    // Counting words parses the whole document, so wait for a pause.
    Timer {
        id: stats
        interval: 200
        onTriggered: model.refreshStats()
    }

    // Omarchy rewrites the theme files on a switch; follow it.
    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: Theme.poll()
    }

    // conf.user.json's autoHideMenuBar hides the menu bar until Alt is
    // tapped; a command from it, or another tap, hides it again.
    property bool menuShown: false
    function altTapped() {
        if (Prefs.advanced.autoHideMenuBar)
            menuShown = !menuShown
    }
    menuBar: AppMenuBar {
        id: menus
        app: win
        visible: Ui.mac || !Prefs.advanced.autoHideMenuBar || win.menuShown
    }
    // The advanced settings, read again (as a new window would).
    function loadAdvanced() {
        Prefs.advanced = JSON.parse(Desktop.advanced())
    }

    RowLayout {
        anchors.fill: parent
        spacing: 0

        Sidebar {
            id: sidebar
            app: win
            visible: Ui.sidebar
            Layout.preferredWidth: Ui.sidebarWidth
            Layout.fillHeight: true
        }
        // Drag to resize the sidebar.
        Rectangle {
            visible: Ui.sidebar
            Layout.fillHeight: true
            implicitWidth: 1
            color: Theme.border
            MouseArea {
                anchors.fill: parent
                anchors.leftMargin: -3
                anchors.rightMargin: -3
                cursorShape: Qt.SplitHCursor
                property real startX
                property int startWidth
                onPressed: mouse => {
                    startX = mapToItem(null, mouse.x, 0).x
                    startWidth = Ui.sidebarWidth
                }
                onPositionChanged: mouse => {
                    const x = mapToItem(null, mouse.x, 0).x
                    Ui.sidebarWidth = Math.max(160, Math.min(win.width / 2, startWidth + x - startX))
                }
            }
        }
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            EditorView {
                id: view
                editor: model
                visible: !Ui.sourceMode
                anchors.fill: parent
                // Typora moves the page down a little under its find bar,
                // and below a notice.
                anchors.topMargin: notice.visible ? notice.height : findBar.visible ? 20 : 0
                anchors.bottomMargin: footer.visible ? footer.height : 0
                onMenuRequested: (x, y) => contextMenu.show(x, y)
                onFilesPasted: paths => win.insertFiles(paths)
                onFilesDropped: paths => win.dropFiles(paths)
                popup: imagePopup
                onImageLoaded: edit => imagePopup.offer(edit, win.imageHere())
                onCaretMoved: {
                    const img = imagePopup.shown ? win.imageHere() : null
                    if (imagePopup.shown && (!img || img.url !== imagePopup.image.url))
                        imagePopup.close()
                }
                WheelZoom {}
                ImagePopup {
                    id: imagePopup
                    view: view
                    onChosen: (cmd, arg) => win.run(cmd, arg)
                }
            }
            SourceView {
                id: sourceView
                app: win
                visible: Ui.sourceMode
                anchors.fill: parent
                anchors.topMargin: notice.visible ? notice.height : findBar.visible ? 20 : 0
                anchors.bottomMargin: footer.visible ? footer.height : 0
                WheelZoom {}
            }
            FindBar {
                id: findBar
                app: win
                view: view
                source: sourceView
                z: 10
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
            }
            Notice {
                id: notice
                z: 9
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
            }
            ZoomHint {
                id: zoomHint
                app: win
                z: 11
                anchors.right: parent.right
                anchors.rightMargin: 8
                anchors.top: parent.top
            }
            Footer {
                id: footer
                app: win
                visible: Ui.statusBar
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
            }
        }
    }

    // A short message at the bottom of the window.
    Rectangle {
        id: toast
        property alias text: toastText.text
        function show(message) {
            text = message
            visible = true
            toastTimer.restart()
        }
        visible: false
        z: 100
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 44
        width: Math.min(toastText.implicitWidth + 32, win.width - 40)
        height: toastText.implicitHeight + 16
        radius: 6
        color: Theme.panel
        border.color: Theme.border
        Text {
            id: toastText
            anchors.centerIn: parent
            width: Math.min(implicitWidth, win.width - 72)
            wrapMode: Text.Wrap
            color: Theme.foreground
            font.pixelSize: 13
        }
        Timer {
            id: toastTimer
            interval: 4000
            onTriggered: toast.visible = false
        }
    }

    function notify(message) {
        toast.show(message)
    }

    // File ▸ Preferences…: over everything under the menu bar.
    PrefsPanel {
        id: prefsPanel
        app: win
        anchors.fill: parent
        z: 50
    }
    // Recover Unsaved Drafts: the folder unsaved work is backed up to.
    function openDrafts() {
        Qt.openUrlExternally(fileUrl(Desktop.draftsFolder()))
    }
    function openThemeFolder() {
        Qt.openUrlExternally(fileUrl(Desktop.themesFolder()))
    }
    function fileUrl(path) {
        return "file://" + path.split("/").map(encodeURIComponent).join("/")
    }
    // conf.user.json, made with the defaults if it isn't there.
    function openAdvancedSettings() {
        const path = Desktop.advancedSettings(false)
        if (path !== "")
            Qt.openUrlExternally(fileUrl(path))
        else
            toast.show(qsTr("Could not write the advanced settings file."))
    }
    function resetAdvancedSettings() {
        if (Desktop.advancedSettings(true) === "")
            toast.show(qsTr("Could not write the advanced settings file."))
    }
    function ask(title, text, buttons, callback, options) {
        dialog.ask(title, text, buttons, callback, options)
    }

    QuickOpen {
        id: quickOpen
        app: win
        side: sidebar
    }
    // Typora's shortcut for the sidebar's Search.
    Shortcut {
        sequence: "Ctrl+Shift+F"
        onActivated: sidebar.openSearch()
    }
    // The Mac's ⌘`: the next window, in the order they opened. macOS's own
    // key, so Typora's menus don't list it.
    Shortcut {
        enabled: Ui.mac
        sequence: "Ctrl+`"
        onActivated: nextWindow()
    }
    function nextWindow() {
        Desktop.raiseWindow((Desktop.frontWindow() + 1) % Desktop.windowTitles().split("\n").length)
    }

    // Commands that work the same in Source Code Mode; the rest act on its
    // text there (SourceView.command), or do nothing.
    readonly property var everywhere: ["new", "newWindow", "open", "openFolder", "openPath", "reopenClosed", "save",
        "saveAs", "saveAll", "moveTo", "revealInSidebar", "deleteFile", "clearRecent", "reload", "encoding", "openLocation", "close", "quit", "lineEnding", "pref", "smartTiming", "preferences", "sidebar",
        "quickOpen", "newFileInFolder", "refreshFolder", "searchFolder", "sidebarView", "sourceMode", "focusMode",
        "typewriter", "statusBar", "fullscreen", "alwaysOnTop", "zoom", "theme", "find", "replace", "replaceNext", "findNext",
        "findPrevious", "wordCount", "switchDocument", "help", "website", "about", "export", "exportPrevious", "exportOverwrite", "print", "import"]

    // Every menu item, shortcut and button runs a command here.
    function run(cmd, arg) {
        if (Ui.sourceMode && !everywhere.includes(cmd)) {
            sourceView.command(cmd, arg)
            return
        }
        switch (cmd) {
        case "new":
            if (discardable)
                model.newDocument()
            else
                Desktop.spawn("")
            return
        case "newWindow": Desktop.spawn(""); return
        case "open": Desktop.chooseFile("open", qsTr("Open"), startFolder(), ""); return
        case "openFolder": Desktop.chooseFile("folder", qsTr("Open Folder"), startFolder(), ""); return
        case "insertImages": Desktop.chooseFile("images", qsTr("Insert Local Images"), startFolder(), ""); return
        case "openPath": openFile(arg); return
        case "reopenClosed": {
            const path = Prefs.takeClosed(p => p !== model.path && Desktop.exists(p))
            if (path !== "")
                openFile(path)
            return
        }
        case "save": save(null); return
        case "saveAs": saveAs(null); return
        // Every window saves, as Typora's; this one alone without the bus.
        case "saveAll":
            if (!Desktop.saveAll())
                save(null)
            return
        // The file goes where the user picks, the document with it; an
        // untitled one is saved there.
        case "moveTo":
            settle()
            if (model.path === "")
                saveAs(null)
            else
                Desktop.chooseFile("moveTo", qsTr("Move To"), startFolder(), fileName(model.path))
            return
        // The sidebar's files, the current one shown, as Typora's: out of a
        // folder search, and onto the files view from the outline, or the
        // view `arg` names (the Mac's Reveal in Library and in File Tree).
        case "revealInSidebar":
            sidebar.closeSearch()
            if (model.path === "")
                return
            Ui.sidebar = true
            if (arg === 1 || arg === 2)
                Ui.sidebarView = arg
            else if (Ui.sidebarView === 0)
                Ui.sidebarView = Ui.fileTree ? 2 : 1
            sidebar.reveal(model.path)
            return
        // The Mac's File ▸ Delete…: the current file to the Trash, as the
        // sidebar's Delete.
        case "deleteFile":
            if (model.path !== "")
                sidebar.remove(model.path)
            return
        case "export": exportAs(arg); return
        case "import":
            if (requirePandoc())
                Desktop.chooseFile("import", qsTr("Import"), startFolder(), "")
            return
        case "exportPrevious":
            if (Prefs.lastExport !== "")
                exportAs(Prefs.lastExport)
            return
        case "exportOverwrite": exportOverwrite(); return
        case "print":
            settle()
            model.print(exportSettings("pdf"))
            return
        case "clearRecent": Prefs.clearRecent(); return
        case "reload":
        case "encoding":
            if (model.path === "")
                return
            confirmReload(() => model.reload(cmd === "encoding" ? arg : ""))
            return
        case "openLocation":
            if (model.path !== "")
                Qt.openUrlExternally("file://" + folderOf(model.path))
            return
        case "close": win.close(); return
        case "quit":
            if (!Desktop.quitAll())
                win.close()
            return
        case "lineEnding": model.setLineEnding(arg === "crlf"); return
        case "finalNewline": model.setFinalNewline(!model.finalNewline()); return
        case "pref": Prefs.toggle(arg); return
        case "smartTiming": Prefs.smartOnRendering = arg; return
        case "preferences":
            if (arg === "image")
                prefsPanel.show(2, "image-setting-group")
            else
                Ui.prefsOpen = !Ui.prefsOpen
            return

        case "cut": view.cut(); return
        case "copy": view.copy(""); return
        case "copyPlain": view.copy("plain"); return
        case "copyMarkdown": view.copy("markdown"); return
        case "copyHtml": view.copy("html"); return
        case "paste": view.paste(false); return
        case "pastePlain": view.paste(true); return
        case "selectAll": view.selectAll(); return
        case "selectBlock": view.selectUnit("block"); return
        case "selectLine": view.selectUnit("line"); return
        case "selectScope": view.selectUnit("scope"); return
        case "selectWord": view.selectUnit("word"); return
        case "deleteBlock": view.deleteUnit("block"); return
        case "deleteLine": view.deleteUnit("line"); return
        case "deleteScope": view.deleteUnit("scope"); return
        case "deleteWord": view.deleteUnit("word"); return
        case "jumpSelection": view.jumpSelection(); return
        case "deleteSelected": view.deleteSelection(); return
        case "openLink": {
            const s = view.selection()
            view.followLink(s.zRow, s.zPos)
            return
        }
        case "copyLink": {
            const s = view.selection()
            Desktop.setClipboard(model.linkAt(s.zRow, s.zPos), "", "")
            return
        }
        // Math's own copies (the context menu's Copy / Paste As…): the
        // TeX, MathML as text and as a code block for rich text, or the
        // formula typeset at twice the text's size.
        case "copyMathTex": {
            const m = mathHere()
            if (m)
                Desktop.setClipboard(m.tex.trim(), "", "")
            return
        }
        case "copyMathML": {
            const m = mathHere()
            const tags = m && m.display ? view.mathRows[m.row] || [] : []
            const mml = m ? Desktop.mathML(Ui.mathTex(m.tex), m.display, tags.join("\n")) : ""
            if (mml !== "") {
                const escaped = mml.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
                Desktop.setClipboard(mml, "<pre class='md-fences' lang='xml'>" + escaped + "</pre>", "")
            } else if (m) {
                toast.show(qsTr("Failed to convert the formula to MathML."))
            }
            return
        }
        case "copyMathImage": {
            const m = mathHere()
            if (m && !Desktop.copyMathImage(Ui.mathTex(m.tex), m.display, Ui.fontSize * 1.18 * 2))
                toast.show(qsTr("Failed to copy the formula as an image."))
            return
        }
        case "copyCode": {
            const edit = view.editAt(view.selection().zRow)
            if (edit)
                Desktop.setClipboard(edit.text, "", "")
            return
        }
        // Out of code, Typora outdents.
        case "autoIndentCode": {
            const edit = view.editAt(view.selection().zRow)
            if (edit && edit.highlighter)
                view.codeAutoIndent(edit, arg === "whole")
            else
                view.command("outdent")
            return
        }
        case "jumpTop": view.jumpTop(); return
        case "jumpBottom": view.jumpBottom(); return
        case "jumpToRow": view.jumpToRow(arg); return

        case "sidebar": Ui.sidebar = !Ui.sidebar; return
        case "quickOpen": quickOpen.show(); return
        case "newFileInFolder": sidebar.newFile(folder); return
        case "refreshFolder": sidebar.refresh(); return
        case "searchFolder": sidebar.openSearch(); return
        case "sidebarView":
            if (Ui.sidebar && Ui.sidebarView === arg)
                Ui.sidebar = false
            else {
                Ui.sidebarView = arg
                Ui.sidebar = true
            }
            return
        case "sourceMode": toggleSource(); return
        case "focusMode":
            Ui.focusMode = !Ui.focusMode
            modeNotice("focus")
            return
        case "typewriter":
            Ui.typewriter = !Ui.typewriter
            modeNotice("typewriter")
            return
        case "statusBar": Ui.statusBar = !Ui.statusBar; return
        case "wordCount":
            if (footer.visible)
                footer.toggleCount()
            return
        case "fullscreen":
            win.visibility = win.visibility === Window.FullScreen ? Window.Windowed : Window.FullScreen
            return
        case "alwaysOnTop": Ui.alwaysOnTop = !Ui.alwaysOnTop; return
        // The Mac's Window menu.
        case "minimize": win.showMinimized(); return
        case "maximize": win.visibility === Window.Maximized ? win.showNormal() : win.showMaximized(); return
        case "front": Desktop.raiseAll(); return
        // Typora's zoom steps are Chromium's zoom levels: each one 1.2
        // times the last. The panel shows the size it came to.
        case "zoom": {
            const level = arg === 0 ? 0 : Math.max(-7, Math.min(8, Math.round(Math.log(Ui.zoom) / Math.log(1.2)) + arg))
            Ui.zoom = Math.pow(1.2, level)
            zoomHint.show()
            return
        }
        case "theme":
            Ui.theme = arg
            Theme.choose(arg)
            return
        case "find": findBar.show(false); return
        case "replace": findBar.show(true); return
        case "replaceNext":
            if (findBar.visible && findBar.replacing)
                findBar.replaceOne()
            else
                findBar.show(true)
            return
        case "findNext": findBar.next(false); return
        case "findPrevious": findBar.next(true); return
        case "undo":
        case "redo":
            model.command(cmd, 0, 0, 0, 0, 0, "")
            return
        case "heading":
            view.command(arg === 0 ? "paragraph" : "heading", arg)
            return
        case "headingUp":
        case "headingDown":
        case "quote":
        case "indent":
        case "outdent":
        case "mathBlock":
        case "toc":
        case "frontMatter":
        case "paragraphBefore":
        case "paragraphAfter":
        case "clearFormat":
        case "footnote":
            view.command(cmd)
            return
        // Typora asks for the size first; the table goes where the
        // selection was when the dialog opened.
        case "table": {
            const s = view.selection()
            tableDialog.ask((columns, rows) => {
                view.select(s.aRow, s.aPos, s.zRow, s.zPos)
                view.command("table", columns * 100 + rows)
            })
            return
        }
        // A link or image takes its address from the clipboard.
        case "link":
        case "image":
            view.command(cmd, 0, Desktop.clipboardText())
            return
        case "tableRow":
        case "tableColumn":
        case "tableMove":
        case "tableDelete":
        case "tableAlign":
        case "tableCopy":
        case "tableFormat":
            view.tableCommand(cmd, arg)
            return
        // The image at the caret (Format ▸ Image and the context menu).
        case "zoomImage":
        case "imageSyntax":
            view.command(cmd, 0, arg)
            return
        case "revealImage": {
            const img = imageHere()
            if (img && img.path !== "")
                Desktop.showInFolder(img.path)
            else if (img && /^https?:/i.test(img.url))
                Qt.openUrlExternally(img.url)
            return
        }
        case "copyImage": {
            const img = imageHere()
            if (img && img.path !== "")
                Desktop.setClipboardImage(img.path)
            return
        }
        case "copyImageToFolder": {
            const img = imageHere()
            if (img)
                relocateImage(img, "", false)
            return
        }
        case "moveImageToFolder": {
            const img = imageHere()
            if (img && img.path !== "")
                confirmMove(() => relocateImage(img, "", true))
            return
        }
        case "copyImageTo": {
            const img = imageHere()
            if (!img || img.path === "")
                return
            imageTarget = img
            Desktop.chooseFile("imageCopy", qsTr("Copy and update image source to…"), startFolder(), fileName(img.path))
            return
        }
        case "moveImageTo": {
            const img = imageHere()
            if (!img || img.path === "")
                return
            confirmMove(() => {
                imageTarget = img
                Desktop.chooseFile("imageMove", qsTr("Copy and update image source to…"), folderOf(img.path), fileName(img.path))
            })
            return
        }
        case "deleteImage": {
            const img = imageHere()
            if (img)
                deleteImage(img)
            return
        }
        // Typora takes the image out first, then asks about its file; undo
        // puts both back.
        case "deleteImageFile": {
            const img = imageHere()
            if (!img || !deleteImage(img) || img.path === "" || !Desktop.exists(img.path))
                return
            shouldContinue(qsTr("Delete Image File"), qsTr("Delete original image file?"), "noWarningForDeleteImage", () => {
                if (!model.trashImageFile(img.path))
                    toast.show(qsTr("Failed to delete %1").arg(img.path))
            })
            return
        }
        case "copyAllImages":
            Desktop.chooseFile("imagesCopy", qsTr("Select Container Folder"), model.pastedImageFolder(startFolder()), "")
            return
        case "moveAllImages":
            confirmMove(() => Desktop.chooseFile("imagesMove", qsTr("Select Container Folder"), model.pastedImageFolder(startFolder()), ""))
            return
        case "reloadImages": Ui.imageReload++; return
        // When Insert Local Image: the document's own folder for inserted
        // images, and the folder its image paths start from, kept in its
        // front matter; on again asks for the folder, off takes it out.
        case "imageFolderSetting":
        case "imageRootSetting": {
            const name = cmd === "imageFolderSetting" ? "copy" : "root"
            const s = view.selection()
            if (model.imageSetting(name) !== "")
                model.setImageSetting(name, "", s.zRow, s.zPos)
            else if (name === "copy" && model.path === "")
                dialog.ask(qsTr("Document must be saved first."),
                    qsTr("typo will copy newly inserted image to folder by selected relative path, so please save the file first."),
                    [qsTr("OK")], () => {})
            else
                Desktop.chooseFile(name === "copy" ? "imageFolder" : "imageRoot", qsTr("Select Container Folder"), startFolder(), "")
            return
        }
        case "codeBlock": view.command("codeFence"); return
        case "alert": view.command("alert", 0, arg); return
        case "horizontalLine": view.command("hr"); return
        case "list": view.command(arg + "List"); return
        case "task": view.command({ done: "taskDone", todo: "taskTodo" }[arg] || "taskToggle"); return
        case "style": view.command(arg === "math" ? "inlineMath" : arg); return
        case "help": {
            // ponytail: Typora's own guides, online, as typo follows it; write
            // typo's when they part ways.
            const page = { "quick-start": "Quick-Start", "markdown-reference": "Markdown-Reference", shortcuts: "Shortcut-Keys" }[arg]
            Qt.openUrlExternally("https://support.typora.io/" + page + "/")
            return
        }
        case "website": Qt.openUrlExternally("https://github.com/kurtlieber/typo"); return
        case "switchDocument":
            if (Ui.mac)
                nextWindow()
            else
                Desktop.focusNextWindow()
            return
        case "about":
            toast.show(qsTr("typo — a native Typora-style markdown editor for Omarchy"))
            return
        }
        toast.show(qsTr("Not available yet: %1").arg(cmd + (arg !== undefined ? " " + arg : "")))
    }

    function open(path) {
        model.open(path)
    }

    // The image at the caret, as the editor describes it (imageAt), with
    // where the caret is, so a chooser's answer finds it again; or null.
    // In a table cell, `cell` is set and the cell edits it (TableBlock.format).
    function imageHere() {
        const s = view.selection()
        const t = view.tableAt()
        const e = t ? t.editItem : null
        const info = e ? model.cellImageAt(e.text, e.cursorPosition) : model.imageAt(s.zRow, s.zPos)
        if (info === "")
            return null
        const img = JSON.parse(info)
        img.row = s.zRow
        img.pos = s.zPos
        img.cell = e !== null
        return img
    }
    // Delete Image: from its cell, or the document; false when it isn't there.
    function deleteImage(img) {
        return img.cell ? view.command("deleteImage") : model.deleteImage(img.row, img.pos)
    }
    // The formula at the caret (Editor.mathAt), or null.
    function mathHere() {
        const s = view.selection()
        const info = model.mathAt(s.zRow, s.zPos)
        if (info === "")
            return null
        const m = JSON.parse(info)
        if (m.fence && !Prefs.mathFences)
            return null
        m.row = s.zRow
        return m
    }
    // The image Copy Image to… or Rename or Move Image to… asked about.
    property var imageTarget: null
    function relocateImage(img, target, move) {
        let error
        if (img.cell) {
            const r = JSON.parse(model.copyImageFile(img.src, target, move))
            if (r.src !== "")
                view.command("imageSrc", 0, r.src)
            error = r.error
        } else {
            error = model.relocateImage(img.row, img.pos, target, move)
        }
        if (error !== "")
            toast.show(qsTr("Failed to Copy Image to Target Folder.") + " " + error)
        imageMoved()
    }
    // The picture a copy or move points at isn't offered for, as in Typora.
    function imageMoved() {
        const img = imageHere()
        if (img)
            imagePopup.lastUrl = img.url
    }
    function relocateAllImages(folder, move) {
        const s = view.selection()
        const result = JSON.parse(model.relocateAllImages(folder, move, s.zRow, s.zPos))
        imageMoved()
        if (result.error !== "")
            toast.show(qsTr("Failed to Copy Image to Target Folder.") + " " + result.error)
        else if (result.count > 0)
            toast.show((move ? qsTr("Images moved to %1.") : qsTr("Images copied to %1.")).arg(fileName(folder)))
    }
    // Moving an image deletes its file where it was, so Typora asks first.
    function confirmMove(then) {
        shouldContinue(qsTr("Move Image"), qsTr("Move image will delete the image file at original path, are you sure to continue?"),
            "noWarningForMoveImage", then)
    }
    // Typora's warnings with "Do not show this again": `then` runs on OK,
    // or at once after OK with the box ticked, which sets `Prefs[never]`.
    function shouldContinue(title, text, never, then) {
        if (Prefs[never])
            return then()
        dialog.ask(title, text, [qsTr("OK"), qsTr("Cancel")], (i, checked) => {
            if (i !== 0)
                return
            if (checked)
                Prefs[never] = true
            then()
        }, { check: qsTr("Do not show this again") })
    }

    // Source Code Mode in and out, as Typora: the find bar closes, and the
    // caret goes to the line of its block (or the block of its line), kept
    // as far down the view as it was.
    function toggleSource() {
        findBar.close()
        if (!Ui.sourceMode) {
            const row = Math.max(0, view.currentRow)
            const y = view.rowY(row)
            Ui.sourceMode = true
            sourceView.load(row, y)
        } else {
            const at = sourceView.leave()
            Ui.sourceMode = false
            view.showRow(at.row, at.y)
        }
    }
    // Turning focus or typewriter mode on says so at the top of the page,
    // as Typora does (and at startup); turning it off takes that away.
    function modeNotice(mode) {
        const on = mode === "focus" ? Ui.focusMode : Ui.typewriter
        if (!on) {
            notice.close()
            return
        }
        const name = mode === "focus" ? qsTr("Focus Mode") : qsTr("Typewriter Mode")
        notice.show(mode === "focus" ? "noFocusModeNotice" : "noTypewriterNotice",
                    qsTr("<b>%1</b> has been turned on.").arg(name), qsTr("You could turn it off from `View` menu."))
    }

    // Back to typing, in whichever view shows the document.
    function focusPage() {
        if (Ui.sourceMode)
            sourceView.edit.forceActiveFocus()
        else
            view.forceActiveFocus()
    }
    // The document as Source Code Mode has it, before anything reads it.
    function settle() {
        if (Ui.sourceMode)
            sourceView.sync()
    }

    // Files. Typora loads a file into this window only when it holds a
    // "discardable untitled" document; anything else gets a new window.
    readonly property bool discardable: model.path === "" && !model.modified

    function openFile(path) {
        if (path === "" || Desktop.absolutePath(path) === model.path)
            return
        if (Desktop.isFolder(path))
            openFolder(path)
        else if (discardable)
            open(path)
        else
            Desktop.spawn(path)
    }
    function openFolder(path) {
        folder = Desktop.absolutePath(path)
        Prefs.addRecentFolder(folder)
        Ui.sidebar = true
        if (Ui.sidebarView === 0)
            Ui.sidebarView = Ui.fileTree ? 2 : 1
    }
    // A file picked in the sidebar or Quick Open replaces the document in
    // this window, once unsaved changes are saved or let go; then `then`
    // runs. An empty path only settles the changes.
    function openInPlace(path, then) {
        if (path !== "" && path === model.path) {
            if (then)
                then()
            return
        }
        settle()
        const go = () => {
            if (path !== "" && !model.open(path))
                return
            if (then)
                then()
        }
        // Preferences ▸ Files: "Save without asking when switch files on
        // side panel".
        if (model.modified && Prefs.saveOnSwitch && model.path !== "" && model.fileExists())
            save(ok => { if (ok) go() })
        else if (model.modified)
            confirmClose(ok => { if (ok) go() })
        else
            go()
    }
    function folderOf(path) {
        return path.substring(0, path.lastIndexOf("/"))
    }
    function fileName(path) {
        return path.substring(path.lastIndexOf("/") + 1)
    }
    function startFolder() {
        return model.path !== "" ? folderOf(model.path) : folder
    }
    // The first heading names an untitled document's file.
    function suggestedName() {
        if (model.path !== "")
            return model.title
        // The outline may still be waiting for a pause in typing.
        if (stats.running) {
            stats.stop()
            model.refreshStats()
        }
        const first = JSON.parse(model.outline)[0]
        const name = first ? first.title.replace(/[\/\\:*?"<>|]/g, " ").trim() : ""
        return (name !== "" ? name.substring(0, 80) : qsTr("Untitled")) + "." + Prefs.defaultExtension
    }

    // File → Export: ask where, then write it in the background with a
    // notice at the top while it runs, as Typora does.
    function exportName() {
        return suggestedName().replace(/\.[^.\/]*$/, "")
    }
    function exportAs(format) {
        settle()
        if (!["pdf", "html", "html-plain", "image"].includes(format) && !requirePandoc())
            return
        Desktop.chooseFile("export:" + format, qsTr("Export"), exportFolder(), exportName())
    }
    // Where the export dialog starts (Preferences ▸ Export ▸ General ▸
    // Default Folder for Exported File): where this window last exported
    // to, else the document's folder or the custom one when chosen, else
    // the folder open beside it.
    property string lastExportFolder: ""
    function exportFolder() {
        if (lastExportFolder !== "")
            return lastExportFolder
        const general = Prefs.exportOptions.general || {}
        if (general.exportFolder === "same" && model.path !== "")
            return folderOf(model.path)
        if (general.exportFolder === "custom")
            return general.customExportPath || documentsFolder()
        return folder !== "" ? folder : startFolder()
    }
    function documentsFolder() {
        return decodeURIComponent(String(StandardPaths.writableLocation(StandardPaths.DocumentsLocation)).replace(/^file:\/\//, ""))
    }
    // Import… and the exports pandoc writes need it installed.
    function requirePandoc() {
        if (Desktop.hasPandoc(Prefs.pandocPath))
            return true
        dialog.ask(qsTr("Require Pandoc to Continue..."),
                   qsTr("Import or some export features requires Pandoc (≥ v2.0) to be installed."),
                   [qsTr("Follow Instructions to Install/Update Pandoc"), qsTr("Cancel")], i => {
            if (i === 0)
                Qt.openUrlExternally("https://pandoc.org/installing.html")
        })
        return false
    }
    // File → Import…: into this window when it holds nothing, else into a
    // new one, as Typora does.
    function importFile(path) {
        if (!discardable) {
            Desktop.spawn("--import=" + path)
            return
        }
        notice.progress(qsTr("Importing..."))
        model.importFile(path)
    }
    // The last export of this document, which Export and Overwrite with
    // Previous writes again.
    property var lastExport: null
    function exportOverwrite() {
        const last = lastExport
        if (!last || last.document !== model.path) {
            if (Prefs.lastExport !== "")
                exportAs(Prefs.lastExport)
            return
        }
        if (Prefs.noExportOverwriteWarning) {
            exportTo(last.format, last.path)
            return
        }
        dialog.ask(qsTr("Export"),
                   qsTr("\"Export and Overwrite with Previous\" will overwrite file at %1, would you continue?").arg(last.path),
                   [qsTr("Continue"), qsTr("Cancel")], i => {
            if (i === 0)
                exportTo(last.format, last.path)
        })
    }
    function exportTo(format, path) {
        settle()
        notice.progress(qsTr("Exporting in progress..."))
        model.exportTo(format, path, exportSettings(format))
    }
    function exportSettings(format) {
        return JSON.stringify({
            numbering: Prefs.mathNumbering,
            mathFences: Prefs.mathFences,
            diagrams: Prefs.diagrams,
            lineBreaks: Prefs.exportLineBreaks,
            mathTex: Prefs.mathCopyFormat === "latex",
            indentFirstLine: Prefs.indentFirstLine,
            monoFont: monoProbe.fontInfo.family,
            scale: Screen.devicePixelRatio,
            options: Prefs.exportOptions[format] || {},
            pandocPath: Prefs.pandocPath,
            opened: folder,
            collapsibleOutline: Ui.outlineCollapsible,
            themeFont: Ui.fontSize / Ui.zoom,
            documents: documentsFolder()
        })
    }
    // After Export, as the format's options say: what the command wrote,
    // when it failed or is to be shown, else the notice; then the file's
    // folder, or the file, opened.
    function afterExport(format, path, ran) {
        const o = Prefs.exportOptions[format] || {}
        const general = Prefs.exportOptions.general || {}
        const reveal = Ui.mac ? qsTr("Reveal in Finder") : qsTr("Reveal in File Explorer")
        const lastLine = s => s.trim().split("\n").pop()
        if (ran.ran && (!ran.ok || o.showOutput)) {
            notice.close()
            // The command as set, then what it said: the last lines,
            // so the dialog fits on the screen.
            const pre = s => "<pre>" + s.replace(/\s+$/, "").split("\n").slice(-20).join("\n")
                .replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;") + "</pre>"
            // What it wrote to stderr only when it failed; its stdout
            // unless that is empty and stderr isn't.
            const error = ran.ok ? "" : ran.error
            let text = qsTr("Command Arguments:") + pre(o.runCommandStr || "(empty)") + qsTr("Console Output:")
            if (error !== "")
                text += pre(error)
            if (ran.output !== "" || error === "")
                text += pre(ran.output || "(empty)")
            dialog.wide = true
            dialog.ask(ran.ok ? qsTr("Export Success!") : qsTr("Run command after export failed"), text,
                       [reveal, qsTr("Close")], i => {
                if (i === 0)
                    Desktop.showInFolder(path)
            })
        } else {
            const done = qsTr("Export operation finished successfully.")
            notice.tell(ran.ran ? lastLine(ran.output) || done : done, "", reveal, () => Desktop.showInFolder(path))
        }
        const location = o.openExportLocation !== undefined ? o.openExportLocation : !!general.openExportLocation
        if (location)
            Desktop.showInFolder(path)
        if (o.openExportFile)
            Qt.openUrlExternally(fileUrl(path))
        if ((location || o.openExportFile) && ran.ok !== false)
            noticeTimeout.restart()
    }
    Timer {
        id: noticeTimeout
        interval: 2000
        onTriggered: if (!notice.busy) notice.close()
    }
    // What the desktop's monospace font is called, for exports to name.
    Text {
        id: monoProbe
        visible: false
        font.family: Ui.monoFont
    }
    Connections {
        target: model
        function onExported(format, path, error, command) {
            if (error !== "") {
                notice.tell(qsTr("Export Failed"), error, "", null)
                return
            }
            Prefs.lastExport = format
            win.lastExport = { format: format, path: path, document: model.path }
            win.lastExportFolder = folderOf(path)
            win.afterExport(format, path, JSON.parse(command))
        }
        function onImported(error) {
            notice.close()
            if (error !== "")
                toast.show(error)
        }
        function onPrinted(error) {
            if (error !== "")
                toast.show(error)
        }
    }

    // Save, through Save As when untitled; `then(ok)` runs afterwards.
    property var afterSave: null
    function save(then) {
        settle()
        if (model.path === "") {
            saveAs(then)
            return
        }
        const ok = model.save()
        if (then)
            then(ok)
    }
    function saveAs(then) {
        settle()
        afterSave = then || null
        Desktop.chooseFile("save", qsTr("Save As"), startFolder(), suggestedName())
    }

    // The Mac's own file panels; on Linux the portal's open from Rust.
    Loader {
        active: Ui.mac
        source: "MacDialogs.qml"
    }

    Connections {
        target: Desktop
        function onQuitRequested() {
            win.quitting = true
            win.close()
        }
        // Save All from any window. ponytail: an untitled document that
        // wasn't touched isn't asked about, which Typora's would be.
        function onSaveRequested() {
            win.settle()
            if (model.modified)
                win.save(null)
        }
        function onFileChosen(kind, path, error) {
            if (error !== "")
                toast.show(error)
            if (kind === "save") {
                const then = win.afterSave
                win.afterSave = null
                const ok = path !== "" && model.saveAs(path)
                if (then)
                    then(ok)
            } else if (kind === "moveTo") {
                const from = model.path
                if (path === "" || from === "" || path === from)
                    return
                // The panel asked before replacing a file there.
                const result = Folder.moveFile(from, path, true)
                if (result.startsWith("!")) {
                    toast.show(result.substring(1))
                    return
                }
                sidebar.moved(from, path)
                if (model.modified)
                    model.save()
            } else if (kind === "import") {
                if (path !== "")
                    win.importFile(path)
            } else if (kind.startsWith("prefFolder:") || kind.startsWith("prefFile:")) {
                // A preference's name, or export/<format>/<option>.
                const name = kind.substring(kind.indexOf(":") + 1).split("/")
                if (path === "")
                    return
                if (name[0] === "export")
                    Prefs.setExportOption(name[1], name[2], path)
                else
                    Prefs[name[0]] = path
            } else if (kind.startsWith("export:")) {
                if (path !== "")
                    win.exportTo(kind.substring(7), path)
            } else if (kind === "images") {
                insertFiles(path.split("\n"))
            } else if (kind === "imageCopy" || kind === "imageMove") {
                const img = win.imageTarget
                win.imageTarget = null
                if (img && path !== "")
                    win.relocateImage(img, path, kind === "imageMove")
            } else if (kind === "imagesCopy" || kind === "imagesMove") {
                if (path !== "")
                    win.relocateAllImages(path, kind === "imagesMove")
            } else if (kind === "imageFolder" || kind === "imageRoot") {
                const s = view.selection()
                if (path !== "")
                    model.setImageSetting(kind === "imageFolder" ? "copy" : "root", path, s.zRow, s.zPos)
            } else if (path !== "") {
                if (kind === "folder")
                    win.openFolder(path)
                else
                    win.openFile(path)
            }
        }
    }

    TDialog {
        id: dialog
    }
    TableDialog {
        id: tableDialog
    }

    // Closing an edited document asks first, unless auto save can just
    // save it. A draft is kept either way.
    property bool closeConfirmed: false
    onClosing: close => {
        settle()
        // An open question is answered first; asking another would lose it.
        if (dialog.opened && !closeConfirmed) {
            close.accepted = false
            return
        }
        if (model.modified && !closeConfirmed) {
            close.accepted = false
            confirmClose(ok => {
                if (ok) {
                    closeConfirmed = true
                    win.close()
                }
            })
            return
        }
        Prefs.addClosed(model.path)
        if (!model.modified)
            model.discardDraft()
        // A window closed while others stay, or while the Mac's typo
        // stays open, isn't part of the session the next launch brings back.
        if (!quitting && (Desktop.otherWindowsOpen() || Ui.mac && !Prefs.quitAfterWindowClose))
            Desktop.forgetWindow()
    }
    function confirmClose(proceed) {
        if (Prefs.autoSave && model.path !== "" && model.fileExists()) {
            save(proceed)
            return
        }
        backupDraft()
        // Quit asks in every window; bring this one forward to ask.
        win.requestActivate()
        dialog.ask(qsTr("Save"),
                   qsTr("Do you want to save the changes made to this document ?<br> Your changes will be lost if you don't save them."),
                   [qsTr("Save"), model.path !== "" ? qsTr("Discard Changes") : qsTr("Discard"), qsTr("Cancel")],
                   i => i === 0 ? save(proceed) : proceed(i === 1))
    }
    function confirmReload(reload) {
        settle()
        if (!model.modified) {
            reload()
            return
        }
        dialog.ask("", qsTr("Reload from disk? Unsaved changes will be discarded."), [qsTr("Reload"), qsTr("Cancel")],
                   i => { if (i === 0) reload() })
    }
    function backupDraft() {
        settle()
        if (model.modified)
            model.backupDraft(Qt.formatDateTime(new Date(), "yyyy-MM-dd hh.mm.ss"))
    }

    // Unsaved work is copied to the drafts folder now and then.
    Timer {
        interval: 30000
        running: model.modified
        repeat: true
        onTriggered: win.backupDraft()
    }

    // Auto save: shortly after an edit, and when the window loses focus.
    Timer {
        id: autoSave
        interval: 2000
        onTriggered: {
            win.settle()
            if (Prefs.autoSave && model.modified && model.path !== "")
                model.save()
        }
    }
    Connections {
        target: model
        function onContentChanged() {
            if (Prefs.autoSave)
                autoSave.restart()
        }
    }
    onActiveChanged: {
        if (active)
            return
        settle()
        if (Prefs.autoSave && model.modified && model.path !== "")
            model.save()
    }

    // Another program changed the file: reload when there is nothing to
    // lose, otherwise ask.
    Timer {
        interval: 1500
        running: model.path !== "" && !dialog.opened
        repeat: true
        onTriggered: {
            win.settle()
            const state = model.checkDisk()
            if (state === "changed") {
                if (!model.modified)
                    model.acceptDisk(true)
                else
                    dialog.ask("", qsTr("File content is changed by external applications. Reload content from disk ?") + " "
                               + qsTr("You could undo this operation later via `Edit` → `Undo`."),
                               [qsTr("Reload"), qsTr("Cancel")], i => model.acceptDisk(i === 0))
            } else if (state === "missing") {
                toast.show(qsTr("%1 was moved or deleted. Save to keep it.").arg(model.title))
            }
        }
    }

    // Preferences ▸ Files ▸ On Launch, when typo starts with nothing to
    // open: the windows open when it last closed (their folders, or their
    // documents and folders), or the custom folder. This window takes the
    // first; the others get windows of their own.
    function launch() {
        const action = Prefs.launchAction
        const last = JSON.parse(Desktop.takeSession())
        if (action === 3) {
            if (Desktop.isFolder(Prefs.launchFolder))
                openFolder(Prefs.launchFolder)
            return
        }
        if (action !== 1 && action !== 2)
            return
        const seen = {}
        const windows = []
        for (const w of last) {
            const entry = { path: action === 2 ? w.path : "", folder: w.folder }
            const key = entry.path + "\n" + entry.folder
            if ((entry.path !== "" || entry.folder !== "") && !seen[key]) {
                seen[key] = true
                windows.push(entry)
            }
        }
        windows.forEach((w, i) => {
            if (i > 0) {
                Desktop.spawnWindow(w.path, w.folder)
                return
            }
            if (w.folder !== "")
                openFolder(w.folder)
            if (w.path !== "")
                open(w.path)
        })
    }
    // What the session keeps of this window.
    readonly property string sessionEntry: model.path + "\n" + folder
    onSessionEntryChanged: Desktop.recordWindow(model.path, folder)
    // Quit closes every window, and they all stay in the session.
    property bool quitting: false

    // Command line: `typo [file]`, plus developer flags used by bin/shot:
    // --size=WxH, --screenshot=PATH (render, save, quit), --sidebar[=VIEW],
    // --cursor=ROW:POS, --menu=N (open the Nth menu), --theme=ID, --folder=DIR,
    // --script=FILE (run a test script, see tests/e2e/README.md). The
    // process's, or what the Mac's later windows open with (desktop.rs,
    // open_window).
    property var args: Qt.application.arguments
    function flag(name) {
        for (const a of args.slice(1))
            if (a === "--" + name)
                return "1"
            else if (a.startsWith("--" + name + "="))
                return a.substring(name.length + 3)
        return ""
    }

    Component.onCompleted: {
        Desktop.listenForQuit()
        loadAdvanced()
        Desktop.watchAlt()
        applyInlineOptions()
        applySmart()
        applyWriting()
        applyImageOptions()
        applyFolderFilters()
        applySyntax()
        model.setNewLineEnding(Prefs.defaultLineEnding === "crlf")
        if (flag("theme") !== "")
            Ui.theme = flag("theme")
        if (Ui.theme !== "")
            Theme.choose(Ui.theme)
        const size = flag("size")
        if (size !== "") {
            const [w, h] = size.split("x").map(Number)
            win.width = w
            win.height = h
        }
        // --sidebar, or --sidebar=outline|list|tree for that view.
        if (flag("sidebar") !== "") {
            Ui.sidebar = true
            const views = { outline: 0, list: 1, tree: 2 }
            if (views[flag("sidebar")] !== undefined)
                Ui.sidebarView = views[flag("sidebar")]
        }
        // --folder=PATH opens a folder in the sidebar, as a restored
        // window has it.
        if (flag("folder") !== "" && Desktop.isFolder(flag("folder")))
            openFolder(flag("folder"))
        const files = args.slice(1).filter(a => !a.startsWith("--"))
        if (files.length > 0)
            openFile(files[0])
        else if (flag("import") !== "")
            importFile(flag("import"))
        else if (flag("folder") === "" && !Desktop.otherWindowsOpen())
            launch()
        Desktop.recordWindow(model.path, folder)
        // Typora's editor says so when it starts in either mode, typewriter
        // last.
        if (Ui.focusMode)
            modeNotice("focus")
        if (Ui.typewriter)
            modeNotice("typewriter")
        const cursor = flag("cursor")
        if (cursor !== "") {
            const [row, pos] = cursor.split(":").map(Number)
            Qt.callLater(() => view.focusRow(row, pos, pos))
        }
        const menu = flag("menu")
        if (menu !== "")
            Qt.callLater(() => {
                const item = win.menuBar.itemAt(Number(menu))
                item.menu.popup(item, 0, item.height)
            })
        if (flag("screenshot") !== "")
            shot.start()
        if (flag("script") !== "") {
            script.lines = Desktop.readFile(flag("script")).split("\n")
            script.start()
        }
    }

    // Runs one script line per tick, so the UI settles between steps.
    // Typed text goes one key per tick too, as a person types: an Enter
    // moves the caret to a new block only once the view has caught up.
    Timer {
        id: script
        property var lines: []
        property int at: 0
        property int idle: 0
        property var keys: []
        interval: 10
        repeat: true
        onTriggered: {
            if (idle > 0) {
                idle--
                return
            }
            if (keys.length > 0) {
                const k = keys.shift()
                if (k.chord) {
                    if (!Desktop.sendKey(k.chord))
                        console.error("script: bad key", k.chord)
                } else
                    Desktop.typeText(k.text)
                return
            }
            if (at >= lines.length) {
                stop()
                // A test ends without the unsaved-changes prompt.
                win.closeConfirmed = true
                Qt.quit()
                return
            }
            win.step(lines[at++])
        }
    }

    // Ctrl and the wheel zoom, a step a notch, when Preferences ▸
    // Appearance says so; on the page itself, which would scroll first.
    component WheelZoom: WheelHandler {
        property real turned: 0
        enabled: Prefs.zoomByWheel
        acceptedModifiers: Qt.ControlModifier
        target: null
        onWheel: e => {
            turned += e.angleDelta.y
            while (Math.abs(turned) >= 120) {
                win.run("zoom", turned > 0 ? 1 : -1)
                turned -= Math.sign(turned) * 120
            }
        }
    }

    function step(line) {
        const space = line.indexOf(" ")
        const op = space < 0 ? line : line.substring(0, space)
        const rest = space < 0 ? "" : line.substring(space + 1)
        switch (op) {
        case "":
        case "#":
            return
        // Text to type, with \n for Enter and \t for Tab.
        case "type":
            for (const part of rest.split(/(\\n|\\t)/))
                if (part === "\\n")
                    script.keys.push({ chord: "Return" })
                else if (part === "\\t")
                    script.keys.push({ chord: "Tab" })
                else
                    for (const c of part)
                        script.keys.push({ text: c })
            return
        case "key":
            for (const chord of rest.split(" "))
                script.keys.push({ chord: chord })
            return
        case "click": {
            const [x, y, mods] = rest.split(" ")
            Desktop.click(Number(x), Number(y), mods || "")
            return
        }
        // Click or drag at text positions: ROW POS [MODS], ROW POS ROW POS.
        case "clickText": {
            const [row, pos, mods] = rest.split(" ")
            const p = textPoint(Number(row), Number(pos))
            Desktop.click(p.x, p.y, mods || "")
            return
        }
        // Turn the wheel at a window point: X Y DY [MODS], DY in eighths of
        // a degree (120 a notch, positive away from the user).
        case "wheel": {
            const [x, y, dy, mods] = rest.split(" ")
            Desktop.wheel(Number(x), Number(y), Number(dy), mods || "")
            return
        }
        // Move the pointer onto a text position: ROW POS.
        case "hoverText": {
            const [row, pos] = rest.split(" ").map(Number)
            const p = textPoint(row, pos)
            Desktop.hover(p.x, p.y)
            return
        }
        // Click the middle of the item an expression names.
        case "clickItem": {
            const item = eval(rest)
            if (!item) {
                console.error("script: no item", rest)
                return
            }
            const p = item.mapToItem(null, item.width / 2, item.height / 2)
            Desktop.click(p.x, p.y, "")
            return
        }
        // Click in a table's cell: ROW R C POS [MODS].
        case "clickCell": {
            const [row, r, c, pos, mods] = rest.split(" ")
            const edit = view.itemAt(Number(row)).table.cellItem(Number(r), Number(c)).edit
            const p = pointIn(edit, Number(pos))
            Desktop.click(p.x, p.y, mods || "")
            return
        }
        case "drag": {
            const [r1, p1, r2, p2] = rest.split(" ").map(Number)
            const a = textPoint(r1, p1)
            const z = textPoint(r2, p2)
            Desktop.drag(a.x, a.y, z.x, z.y)
            return
        }
        // A drag a step at a time: press, move or release the left button
        // at the window point an expression gives.
        case "mouse": {
            const gap = rest.indexOf(" ")
            const what = ["press", "move", "release"].indexOf(rest.substring(0, gap))
            const p = eval(rest.substring(gap + 1))
            if (what < 0 || !p)
                console.error("script: no mouse step", rest)
            else
                Desktop.mouse(what, p.x, p.y)
            return
        }
        // Save the clipboard's text to PATH, or put TEXT on it.
        case "saveClipboard":
            Desktop.writeFile(rest, Desktop.clipboardText())
            return
        case "clipboard":
            Desktop.setClipboard(rest.replace(/\\n/g, "\n"), "", "")
            return
        // Files as a file manager copies them (paths, space apart), or the
        // picture in a file.
        case "clipboardFiles":
            Desktop.setClipboardFiles(rest.split(" ").join("\n"))
            return
        case "clipboardImage":
            if (!Desktop.setClipboardImage(rest))
                console.warn("script: no picture in " + rest)
            return
        // Drop files (paths, space apart) at text position ROW POS.
        case "dropFiles": {
            const parts = rest.split(" ")
            const p = textPoint(Number(parts[0]), Number(parts[1]))
            if (!Desktop.dropFiles(p.x, p.y, parts.slice(2).join("\n")))
                console.warn("script: the drop was refused")
            return
        }
        // Files dragged in from another program: STAGE (over, drop or
        // leave) PATH[|PATH…] EXPR, at the window point EXPR gives.
        case "dragFiles": {
            const [stage, paths] = rest.split(" ", 2)
            const p = stage === "leave" ? { x: 0, y: 0 } : eval(rest.substring(stage.length + paths.length + 2))
            if (!Desktop.dragFiles(["over", "drop", "leave"].indexOf(stage), p.x, p.y, (paths || "").split("|").join("\n")) && stage === "drop")
                console.warn("script: the drop was refused")
            return
        }
        // HTML as a browser would copy it, with its text as the plain form.
        case "clipboardHtml":
            Desktop.setClipboard(Desktop.plainText(rest), rest, "")
            return
        case "wait":
            script.idle = Math.ceil(Number(rest) / script.interval)
            return
        case "cmd": {
            const [c, a] = rest.split(" ")
            run(c, a === undefined ? undefined : (isNaN(Number(a)) ? a : Number(a)))
            return
        }
        case "dump":
            settle()
            Desktop.writeFile(rest, model.markdown())
            return
        // The next file chooser's answer, given in advance (empty cancels).
        case "choose":
            Desktop.fakeChoice(rest.replace(/\\n/g, "\n"))
            return
        // Press the Nth button of the open dialog.
        case "answer":
            if (dialog.opened)
                dialog.choose(Number(rest))
            else
                console.error("script: no dialog to answer")
            return
        // Write a file as another program would.
        case "write": {
            const space = rest.indexOf(" ")
            Desktop.writeFile(rest.substring(0, space), rest.substring(space + 1).replace(/\\n/g, "\n"))
            return
        }
        case "shot":
            Desktop.screenshot(rest)
            return
        // A JavaScript expression, logged with its value (for debugging).
        case "eval":
            console.log("eval:", rest, "=", JSON.stringify(eval(rest)))
            return
        case "check":
            if (!eval(rest))
                console.error("script: check failed:", rest)
            return
        case "caret": {
            const [row, pos] = rest.split(" ").map(Number)
            view.select(row, pos, row, pos)
            return
        }
        }
        console.error("script: unknown step", line)
    }

    // The window point at UTF-16 position `pos` in `row`, mid-line.
    function textPoint(row, pos) {
        return pointIn(view.itemAt(row).editItem, pos)
    }
    function pointIn(edit, pos) {
        const r = edit.positionToRectangle(pos)
        return edit.mapToItem(null, r.x + 0.5, r.y + r.height / 2)
    }

    Timer {
        id: shot
        interval: 600
        onTriggered: {
            if (!Desktop.screenshot(win.flag("screenshot")))
                console.error("could not save the screenshot")
            win.closeConfirmed = true
            Qt.quit()
        }
    }
}
