import QtQuick
import QtQuick.Controls
import Typo

// Typora's menu bar. Every item runs a named command through `app.run`,
// which is also what shortcuts, the context menu and the footer use.
MenuBar {
    id: bar
    required property var app

    // An action that runs a command, optionally checkable. Its keys are
    // its own unless conf.user.json's keyBinding says otherwise.
    component Cmd: Action {
        id: action
        property string cmd
        property var arg
        property string builtIn: ""
        onTriggered: {
            bar.app.menuShown = false
            bar.app.run(cmd, arg)
        }
        Component.onCompleted: {
            builtIn = shortcut ? String(shortcut) : ""
            shortcut = Qt.binding(() => Prefs.keysFor(action.text, action.builtIn))
        }
    }

    // What the caret is in, looked up as a menu opens: the image (as
    // Main.imageHere gives it) and the link's address.
    property var image: null
    property string link: ""
    // The document's own image settings, from its front matter. A new
    // object each time, so the check marks follow it even after a click
    // toggled them and the chooser was cancelled.
    property var imageSettings: ({})
    readonly property bool hasImage: image !== null
    function lookAround() {
        const s = app.view.selection()
        image = app.imageHere()
        link = app.editor.linkAt(s.zRow, s.zPos)
    }

    // On the Mac this is the system's menu bar, and Qt moves About,
    // Preferences… and Quit to the app menu by their titles, so the
    // separators that set them off go (a native menu ignores `visible`).
    Component.onCompleted: {
        if (Ui.mac) {
            fileMenu.removeItem(prefsSep)
            helpMenu.removeItem(aboutSep)
        } else {
            removeMenu(windowMenu)
        }
    }

    background: Rectangle {
        implicitHeight: 28
        color: Theme.panel
    }

    delegate: MenuBarItem {
        id: top
        contentItem: Text {
            text: top.text.replace("&", "")
            color: top.highlighted ? Theme.accentText : Theme.foreground
            font: top.font
            verticalAlignment: Text.AlignVCenter
        }
        background: Rectangle {
            color: top.highlighted ? Theme.accent : "transparent"
            radius: 3
        }
    }

    TMenu {
        id: fileMenu
        title: qsTr("&File")
        Cmd { text: qsTr("New"); cmd: "new"; shortcut: "Ctrl+N" }
        Cmd { text: qsTr("New Window"); cmd: "newWindow"; shortcut: "Ctrl+Shift+N" }
        TSep {}
        Cmd { text: qsTr("Open…"); cmd: "open"; shortcut: "Ctrl+O" }
        Cmd { text: qsTr("Open Folder…"); cmd: "openFolder" }
        TSep {}
        Cmd { text: qsTr("Open Quickly…"); cmd: "quickOpen"; shortcut: "Ctrl+P" }
        Cmd { text: qsTr("Reopen Closed File"); cmd: "reopenClosed"; shortcut: "Ctrl+Shift+T" }
        TMenu {
            id: recentMenu
            title: qsTr("Open Recent")
            // Other windows may have opened files since.
            onAboutToShow: Prefs.refresh()
            Instantiator {
                model: bar.app.recentFiles
                delegate: TMenuItem {
                    required property string modelData
                    text: modelData
                    onTriggered: bar.app.run("openPath", modelData)
                }
                onObjectAdded: (index, object) => recentMenu.insertItem(index, object)
                onObjectRemoved: (index, object) => recentMenu.removeItem(object)
            }
            TSep {}
            Cmd { text: bar.app.recentFiles.length ? qsTr("Clear Items") : qsTr("No Recent Files"); cmd: "clearRecent"; enabled: bar.app.recentFiles.length > 0 }
        }
        TMenu {
            title: qsTr("Reopen with Encoding")
            Cmd { text: qsTr("Auto"); cmd: "encoding"; arg: "auto" }
            TSep {}
            Cmd { text: "UTF-8"; cmd: "encoding"; arg: "UTF-8" }
            Cmd { text: "UTF-16 LE"; cmd: "encoding"; arg: "UTF-16LE" }
            Cmd { text: "UTF-16 BE"; cmd: "encoding"; arg: "UTF-16BE" }
            Cmd { text: "Windows-1252"; cmd: "encoding"; arg: "windows-1252" }
            Cmd { text: "ISO-8859-1"; cmd: "encoding"; arg: "ISO-8859-1" }
            Cmd { text: "GB18030"; cmd: "encoding"; arg: "GB18030" }
            Cmd { text: "Big5"; cmd: "encoding"; arg: "Big5" }
            Cmd { text: "Shift_JIS"; cmd: "encoding"; arg: "Shift_JIS" }
            Cmd { text: "EUC-KR"; cmd: "encoding"; arg: "EUC-KR" }
        }
        Cmd { text: qsTr("Reload from Disk"); cmd: "reload" }
        TSep {}
        Cmd { text: qsTr("Save"); cmd: "save"; shortcut: "Ctrl+S" }
        Cmd { text: qsTr("Save As…"); cmd: "saveAs"; shortcut: "Ctrl+Shift+S" }
        Cmd { text: qsTr("Save All"); cmd: "saveAll" }
        Cmd { text: qsTr("Move To…"); cmd: "moveTo" }
        TSep {}
        Cmd { text: Ui.mac ? qsTr("Reveal in Finder") : qsTr("Open File Location"); cmd: "openLocation" }
        Cmd { text: qsTr("Reveal in Sidebar"); cmd: "revealInSidebar" }
        TSep {}
        Cmd { text: qsTr("Import…"); cmd: "import" }
        TMenu {
            title: qsTr("Export")
            Cmd { text: qsTr("Export with Previous"); cmd: "exportPrevious"; enabled: Prefs.lastExport !== "" }
            Cmd {
                text: qsTr("Export and Overwrite with Previous")
                cmd: "exportOverwrite"
                enabled: bar.app.lastExport !== null && bar.app.lastExport.document === bar.app.editor.path
            }
            TSep {}
            Cmd { text: "PDF"; cmd: "export"; arg: "pdf" }
            Cmd { text: "HTML"; cmd: "export"; arg: "html" }
            Cmd { text: qsTr("HTML (without Styles)"); cmd: "export"; arg: "html-plain" }
            Cmd { text: qsTr("Image"); cmd: "export"; arg: "image" }
            TSep {}
            Cmd { text: "Word (.docx)"; cmd: "export"; arg: "docx" }
            Cmd { text: "OpenOffice"; cmd: "export"; arg: "odt" }
            Cmd { text: "RTF"; cmd: "export"; arg: "rtf" }
            Cmd { text: "Epub"; cmd: "export"; arg: "epub" }
            Cmd { text: "LaTeX"; cmd: "export"; arg: "latex" }
            Cmd { text: "Media Wiki"; cmd: "export"; arg: "mediawiki" }
            Cmd { text: "reStructuredText"; cmd: "export"; arg: "rst" }
            Cmd { text: "Textile"; cmd: "export"; arg: "textile" }
            Cmd { text: "OPML"; cmd: "export"; arg: "opml" }
        }
        Cmd { text: qsTr("Print…"); cmd: "print"; shortcut: "Alt+Shift+P" }
        TSep { id: prefsSep }
        Cmd { text: qsTr("Preferences…"); cmd: "preferences"; shortcut: "Ctrl+," }
        TSep {}
        Cmd { text: qsTr("Close"); cmd: "close"; shortcut: "Ctrl+W" }
        Cmd { text: qsTr("Quit"); cmd: "quit"; shortcut: "Ctrl+Q" }
    }

    TMenu {
        title: qsTr("&Edit")
        onAboutToShow: {
            bar.lookAround()
            fit()
        }
        Cmd { text: qsTr("Undo"); cmd: "undo"; shortcut: "Ctrl+Z"; enabled: bar.app.editor.canUndo }
        Cmd { text: qsTr("Redo"); cmd: "redo"; shortcut: "Ctrl+Y"; enabled: bar.app.editor.canRedo }
        TSep {}
        Cmd { text: qsTr("Cut"); cmd: "cut"; shortcut: "Ctrl+X" }
        Cmd { text: qsTr("Copy"); cmd: "copy"; shortcut: "Ctrl+C" }
        Cmd { text: qsTr("Paste"); cmd: "paste"; shortcut: "Ctrl+V" }
        TSep {}
        TMenuItem {
            text: qsTr("Copy Image Content")
            visible: bar.hasImage
            enabled: bar.hasImage && bar.image.path !== ""
            onTriggered: bar.app.run("copyImage")
        }
        Cmd { text: qsTr("Copy as Plain Text"); cmd: "copyPlain" }
        Cmd { text: qsTr("Copy as Markdown"); cmd: "copyMarkdown"; shortcut: "Ctrl+Shift+C" }
        Cmd { text: qsTr("Copy as HTML Code"); cmd: "copyHtml" }
        Cmd { text: qsTr("Paste as Plain Text"); cmd: "pastePlain"; shortcut: "Ctrl+Shift+V" }
        TSep {}
        TMenu {
            title: qsTr("Selection")
            Cmd { text: qsTr("Select All"); cmd: "selectAll"; shortcut: "Ctrl+A" }
            TSep {}
            Cmd { text: qsTr("Select Paragraph / Block"); cmd: "selectBlock" }
            Cmd { text: qsTr("Select Line / Sentence"); cmd: "selectLine"; shortcut: "Ctrl+L" }
            Cmd { text: qsTr("Select Styled Scope"); cmd: "selectScope"; shortcut: "Ctrl+E" }
            Cmd { text: qsTr("Select Word"); cmd: "selectWord"; shortcut: "Ctrl+D" }
        }
        TMenu {
            title: qsTr("Delete")
            Cmd { text: qsTr("Delete Paragraph / Block"); cmd: "deleteBlock" }
            Cmd { text: qsTr("Delete Line / Sentence"); cmd: "deleteLine" }
            Cmd { text: qsTr("Delete Styled Scope"); cmd: "deleteScope" }
            Cmd { text: qsTr("Delete Word"); cmd: "deleteWord"; shortcut: "Ctrl+Shift+D" }
        }
        TMenu {
            title: qsTr("Jump To")
            Cmd { text: qsTr("Jump to Top"); cmd: "jumpTop"; shortcut: "Ctrl+Home" }
            Cmd { text: qsTr("Jump to Selection"); cmd: "jumpSelection"; shortcut: "Ctrl+J" }
            Cmd { text: qsTr("Jump to Bottom"); cmd: "jumpBottom"; shortcut: "Ctrl+End" }
        }
        TSep {}
        TMenu {
            title: qsTr("Math Tools")
            Cmd { text: qsTr("Refresh All Math Expressions"); cmd: "refreshMath" }
        }
        TMenu {
            title: qsTr("Image Tools")
            Cmd { text: qsTr("Insert Local Images…"); cmd: "insertImages" }
            TSep {}
            Cmd { text: qsTr("Reload All Images"); cmd: "reloadImages" }
            Cmd { text: qsTr("Global Image Settings…"); cmd: "preferences"; arg: "image" }
        }
        TSep {}
        TMenu {
            title: qsTr("Line Endings")
            Cmd { text: qsTr("Windows Line Endings (CRLF)"); cmd: "lineEnding"; arg: "crlf"; checkable: true; checked: bar.app.editor.crlf }
            Cmd { text: qsTr("Unix Line Endings (LF)"); cmd: "lineEnding"; arg: "lf"; checkable: true; checked: !bar.app.editor.crlf }
        }
        TMenu {
            title: qsTr("Whitespace and Line Breaks")
            Cmd { text: qsTr("Indent first line of paragraphs"); cmd: "pref"; arg: "indentFirstLine"; checkable: true; checked: Prefs.indentFirstLine }
            Cmd { text: qsTr("Preserve single line break"); cmd: "pref"; arg: "preserveLineBreaks"; checkable: true; checked: Prefs.preserveLineBreaks }
            Cmd { text: qsTr("Visible <br/>"); cmd: "pref"; arg: "visibleBr"; checkable: true; checked: Prefs.visibleBr }
        }
        TMenu {
            title: qsTr("Smart Punctuation")
            Cmd { text: qsTr("Convert on Input"); cmd: "smartTiming"; arg: false; checkable: true; checked: !Prefs.smartOnRendering }
            Cmd { text: qsTr("Convert on Rendering"); cmd: "smartTiming"; arg: true; checkable: true; checked: Prefs.smartOnRendering }
            TSep {}
            Cmd { text: qsTr("Smart Quotes"); cmd: "pref"; arg: "smartQuotes"; checkable: true; checked: Prefs.smartQuotes }
            Cmd { text: qsTr("Smart Dashes"); cmd: "pref"; arg: "smartDashes"; checkable: true; checked: Prefs.smartDashes }
            TSep {}
            Cmd {
                readonly property bool forced: !Prefs.smartOnRendering && (Prefs.smartQuotes || Prefs.smartDashes)
                text: qsTr("Remap Unicode Punctuation on Parse")
                cmd: "pref"
                arg: "remapPunctuation"
                checkable: true
                checked: Prefs.remapPunctuation || forced
                enabled: !forced
            }
        }
        TSep {}
        TMenu {
            title: qsTr("Find and Replace")
            Cmd { text: qsTr("Find"); cmd: "find"; shortcut: "Ctrl+F" }
            Cmd { text: qsTr("Find Next"); cmd: "findNext"; shortcut: "F3" }
            Cmd { text: qsTr("Find Previous"); cmd: "findPrevious"; shortcut: "Shift+F3" }
            TSep {}
            Cmd { text: qsTr("Replace"); cmd: "replace"; shortcut: "Ctrl+H" }
        }
        TSep {}
        Cmd { text: qsTr("Emoji and Symbols"); cmd: "emoji" }
    }

    TMenu {
        title: qsTr("&Paragraph")
        Cmd { text: qsTr("Heading 1"); cmd: "heading"; arg: 1; shortcut: "Ctrl+1"; checkable: true; checked: bar.app.blockLevel === 1 }
        Cmd { text: qsTr("Heading 2"); cmd: "heading"; arg: 2; shortcut: "Ctrl+2"; checkable: true; checked: bar.app.blockLevel === 2 }
        Cmd { text: qsTr("Heading 3"); cmd: "heading"; arg: 3; shortcut: "Ctrl+3"; checkable: true; checked: bar.app.blockLevel === 3 }
        Cmd { text: qsTr("Heading 4"); cmd: "heading"; arg: 4; shortcut: "Ctrl+4"; checkable: true; checked: bar.app.blockLevel === 4 }
        Cmd { text: qsTr("Heading 5"); cmd: "heading"; arg: 5; shortcut: "Ctrl+5"; checkable: true; checked: bar.app.blockLevel === 5 }
        Cmd { text: qsTr("Heading 6"); cmd: "heading"; arg: 6; shortcut: "Ctrl+6"; checkable: true; checked: bar.app.blockLevel === 6 }
        TSep {}
        Cmd { text: qsTr("Paragraph"); cmd: "heading"; arg: 0; shortcut: "Ctrl+0"; checkable: true; checked: bar.app.blockLevel === 0 }
        TSep {}
        Cmd { text: qsTr("Increase Heading Level"); cmd: "headingUp"; shortcut: "Ctrl+=" }
        Cmd { text: qsTr("Decrease Heading Level"); cmd: "headingDown"; shortcut: "Ctrl+-" }
        TSep {}
        TMenu {
            title: qsTr("Table")
            Cmd { text: qsTr("Insert Table"); cmd: "table"; shortcut: "Ctrl+T" }
            TSep {}
            Cmd { text: qsTr("Add Row Above"); cmd: "tableRow"; arg: "above" }
            Cmd { text: qsTr("Add Row Below"); cmd: "tableRow"; arg: "below"; shortcut: "Ctrl+Return" }
            Cmd { text: qsTr("Add Column Before"); cmd: "tableColumn"; arg: "before" }
            Cmd { text: qsTr("Add Column After"); cmd: "tableColumn"; arg: "after" }
            TSep {}
            Cmd { text: qsTr("Move Row Up"); cmd: "tableMove"; arg: "up" }
            Cmd { text: qsTr("Move Row Down"); cmd: "tableMove"; arg: "down" }
            Cmd { text: qsTr("Move Column Left"); cmd: "tableMove"; arg: "left" }
            Cmd { text: qsTr("Move Column Right"); cmd: "tableMove"; arg: "right" }
            TSep {}
            Cmd { text: qsTr("Delete Row"); cmd: "tableDelete"; arg: "row"; shortcut: "Ctrl+Shift+Backspace" }
            Cmd { text: qsTr("Delete Column"); cmd: "tableDelete"; arg: "column" }
            Cmd { text: qsTr("Delete Table"); cmd: "tableDelete"; arg: "table" }
        }
        Cmd { text: qsTr("Code Fences"); cmd: "codeBlock"; shortcut: "Ctrl+Shift+K" }
        Cmd { text: qsTr("Math Block"); cmd: "mathBlock"; shortcut: "Ctrl+Shift+M" }
        Cmd { text: qsTr("Quote"); cmd: "quote"; shortcut: "Ctrl+Shift+Q" }
        TMenu {
            title: qsTr("Alert")
            Cmd { text: qsTr("Note Block"); cmd: "alert"; arg: "NOTE" }
            Cmd { text: qsTr("Tip Block"); cmd: "alert"; arg: "TIP" }
            Cmd { text: qsTr("Important Block"); cmd: "alert"; arg: "IMPORTANT" }
            Cmd { text: qsTr("Warning Block"); cmd: "alert"; arg: "WARNING" }
            Cmd { text: qsTr("Caution Block"); cmd: "alert"; arg: "CAUTION" }
        }
        TSep {}
        Cmd { text: qsTr("Ordered List"); cmd: "list"; arg: "ordered"; shortcut: "Ctrl+Shift+[" }
        Cmd { text: qsTr("Unordered List"); cmd: "list"; arg: "bullet"; shortcut: "Ctrl+Shift+]" }
        Cmd { text: qsTr("Task List"); cmd: "list"; arg: "task"; shortcut: "Ctrl+Shift+X" }
        TMenu {
            title: qsTr("Task Status")
            Cmd { text: qsTr("Toggle Task Status"); cmd: "task"; arg: "toggle" }
            TSep {}
            Cmd { text: qsTr("Mark as Complete"); cmd: "task"; arg: "done" }
            Cmd { text: qsTr("Mark as Incomplete"); cmd: "task"; arg: "todo" }
        }
        TMenu {
            title: qsTr("List Indentation")
            Cmd { text: qsTr("Indent"); cmd: "indent"; shortcut: "Ctrl+[" }
            Cmd { text: qsTr("Outdent"); cmd: "outdent"; shortcut: "Ctrl+]" }
        }
        TSep {}
        Cmd { text: qsTr("Insert Paragraph Before"); cmd: "paragraphBefore" }
        Cmd { text: qsTr("Insert Paragraph After"); cmd: "paragraphAfter" }
        TSep {}
        Cmd { text: qsTr("Link Reference"); cmd: "linkReference" }
        Cmd { text: qsTr("Footnotes"); cmd: "footnote" }
        TSep {}
        Cmd { text: qsTr("Horizontal Line"); cmd: "horizontalLine" }
        Cmd { text: qsTr("Table of Contents"); cmd: "toc" }
        Cmd { text: qsTr("YAML Front Matter"); cmd: "frontMatter" }
    }

    TMenu {
        title: qsTr("F&ormat")
        Cmd { text: qsTr("Strong"); cmd: "style"; arg: "strong"; shortcut: "Ctrl+B" }
        Cmd { text: qsTr("Emphasis"); cmd: "style"; arg: "emphasis"; shortcut: "Ctrl+I" }
        Cmd { text: qsTr("Underline"); cmd: "style"; arg: "underline"; shortcut: "Ctrl+U" }
        Cmd { text: qsTr("Code"); cmd: "style"; arg: "code"; shortcut: "Ctrl+Shift+`" }
        TSep {}
        Cmd { text: qsTr("Inline Math"); cmd: "style"; arg: "math" }
        Cmd { text: qsTr("Strike"); cmd: "style"; arg: "strike"; shortcut: "Alt+Shift+5" }
        Cmd { text: qsTr("Highlight"); cmd: "style"; arg: "highlight" }
        Cmd { text: qsTr("Superscript"); cmd: "style"; arg: "superscript" }
        Cmd { text: qsTr("Subscript"); cmd: "style"; arg: "subscript" }
        Cmd { text: qsTr("Comment"); cmd: "style"; arg: "comment" }
        TSep {}
        Cmd { text: qsTr("Hyperlink"); cmd: "link"; shortcut: "Ctrl+K" }
        TMenu {
            title: qsTr("Hyperlink Actions")
            onAboutToShow: {
                bar.lookAround()
                fit()
            }
            Cmd { text: qsTr("Open Link"); cmd: "openLink"; enabled: bar.link !== "" }
            Cmd { text: qsTr("Copy Link Address"); cmd: "copyLink"; enabled: bar.link !== "" }
        }
        TMenu {
            id: imageMenu
            title: qsTr("Image")
            // Zoom Image and Switch Image Syntax work on the image at the
            // caret; a referenced image has no other syntax to switch to.
            onAboutToShow: {
                bar.lookAround()
                bar.imageSettings = { copy: bar.app.editor.imageSetting("copy"), root: bar.app.editor.imageSetting("root") }
                for (let i = 0; i < count; i++) {
                    const item = itemAt(i)
                    if (item && (item.subMenu === zoomMenu || item.subMenu === syntaxMenu))
                        item.enabled = bar.hasImage
                    if (item && item.subMenu === syntaxMenu)
                        item.visible = !bar.hasImage || bar.image.syntax !== "ref"
                }
                fit()
            }
            Cmd { text: qsTr("Insert Image"); cmd: "image"; shortcut: "Ctrl+Shift+I" }
            Cmd { text: qsTr("Insert Local Images…"); cmd: "insertImages" }
            TSep {}
            Cmd { text: qsTr("Open Image Location…"); cmd: "revealImage"; enabled: bar.hasImage }
            TMenu {
                id: zoomMenu
                title: qsTr("Zoom Image")
                Cmd { text: "25%"; cmd: "zoomImage"; arg: "25%" }
                Cmd { text: "33%"; cmd: "zoomImage"; arg: "33%" }
                Cmd { text: "50%"; cmd: "zoomImage"; arg: "50%" }
                Cmd { text: "67%"; cmd: "zoomImage"; arg: "67%" }
                Cmd { text: "80%"; cmd: "zoomImage"; arg: "80%" }
                Cmd { text: "100%"; cmd: "zoomImage"; arg: "100%" }
                Cmd { text: "150%"; cmd: "zoomImage"; arg: "150%" }
                Cmd { text: "200%"; cmd: "zoomImage"; arg: "200%" }
            }
            TMenu {
                id: syntaxMenu
                title: qsTr("Switch Image Syntax")
                Cmd {
                    text: qsTr("Markdown Style"); cmd: "imageSyntax"; arg: "markdown"
                    checkable: true; checked: bar.hasImage && bar.image.syntax === "markdown"
                }
                Cmd {
                    text: qsTr("HTML Style"); cmd: "imageSyntax"; arg: "html"
                    checkable: true; checked: bar.hasImage && bar.image.syntax === "html"
                }
            }
            TSep {}
            Cmd { text: qsTr("Delete Image File"); cmd: "deleteImageFile"; enabled: bar.hasImage }
            Cmd { text: qsTr("Copy Image to…"); cmd: "copyImageTo"; enabled: bar.hasImage }
            Cmd { text: qsTr("Rename or Move Image to…"); cmd: "moveImageTo"; enabled: bar.hasImage }
            TSep {}
            Cmd { text: qsTr("Copy All Images to…"); cmd: "copyAllImages" }
            Cmd { text: qsTr("Move All Images to…"); cmd: "moveAllImages" }
            Cmd { text: qsTr("Reload All Images"); cmd: "reloadImages" }
            TSep {}
            TMenu {
                title: qsTr("When Insert Local Image")
                Cmd {
                    text: qsTr("Copy Image File to Folder"); cmd: "imageFolderSetting"
                    checkable: true; checked: !!bar.imageSettings.copy
                }
                Cmd {
                    text: qsTr("Use Image Root Path"); cmd: "imageRootSetting"
                    checkable: true; checked: !!bar.imageSettings.root
                }
                TSep {}
                Cmd { text: qsTr("Global Image Settings…"); cmd: "preferences"; arg: "image" }
            }
        }
        TSep {}
        Cmd { text: qsTr("Clear Format"); cmd: "clearFormat"; shortcut: "Ctrl+\\" }
    }

    TMenu {
        title: qsTr("&View")
        Cmd { text: qsTr("Toggle Sidebar"); cmd: "sidebar"; shortcut: "Ctrl+Shift+L"; checkable: true; checked: Ui.sidebar }
        Cmd { text: qsTr("Outline"); cmd: "sidebarView"; arg: 0; shortcut: "Ctrl+Shift+1"; checkable: true; checked: Ui.sidebar && Ui.sidebarView === 0 }
        Cmd { text: qsTr("Articles"); cmd: "sidebarView"; arg: 1; shortcut: "Ctrl+Shift+2"; checkable: true; checked: Ui.sidebar && Ui.sidebarView === 1 }
        Cmd { text: qsTr("File Tree"); cmd: "sidebarView"; arg: 2; shortcut: "Ctrl+Shift+3"; checkable: true; checked: Ui.sidebar && Ui.sidebarView === 2 }
        TSep {}
        Cmd { text: qsTr("Source Code Mode"); cmd: "sourceMode"; shortcut: "Ctrl+/"; checkable: true; checked: Ui.sourceMode }
        TSep {}
        Cmd { text: qsTr("Focus Mode"); cmd: "focusMode"; shortcut: "F8"; checkable: true; checked: Ui.focusMode }
        Cmd { text: qsTr("Typewriter Mode"); cmd: "typewriter"; shortcut: "F9"; checkable: true; checked: Ui.typewriter }
        TSep {}
        Cmd { text: qsTr("Status Bar"); cmd: "statusBar"; checkable: true; checked: Ui.statusBar }
        TSep {}
        Cmd { text: qsTr("Toggle Fullscreen"); cmd: "fullscreen"; shortcut: "F11"; checkable: true; checked: bar.app.visibility === Window.FullScreen }
        Cmd { text: qsTr("Always on Top"); cmd: "alwaysOnTop"; checkable: true; checked: Ui.alwaysOnTop }
        TSep {}
        Cmd { text: qsTr("Actual Size"); cmd: "zoom"; arg: 0; shortcut: "Ctrl+Shift+0" }
        Cmd { text: qsTr("Zoom In"); cmd: "zoom"; arg: 1; shortcut: "Ctrl+Shift+=" }
        Cmd { text: qsTr("Zoom Out"); cmd: "zoom"; arg: -1; shortcut: "Ctrl+Shift+-" }
        TSep {}
        Cmd { text: qsTr("Switch Between Opened Documents"); cmd: "switchDocument"; shortcut: "Ctrl+Tab" }
    }

    TMenu {
        id: themesMenu
        title: qsTr("&Themes")
        Cmd { text: Ui.mac ? qsTr("Follow System") : qsTr("Follow Omarchy"); cmd: "theme"; arg: ""; checkable: true; checked: Theme.chosen === "" }
        TSep {}
        Instantiator {
            model: Theme.themes
            delegate: TMenuItem {
                required property string modelData
                readonly property string themeId: modelData.split("|")[0]
                text: modelData.split("|")[1]
                checkable: true
                checked: Theme.chosen === themeId
                onTriggered: bar.app.run("theme", themeId)
            }
            onObjectAdded: (index, object) => themesMenu.insertItem(index + 2, object)
            onObjectRemoved: (index, object) => themesMenu.removeItem(object)
        }
    }

    // The Mac's; Linux's window manager has these.
    TMenu {
        id: windowMenu
        title: qsTr("&Window")
        Cmd { text: qsTr("Minimize"); cmd: "minimize"; shortcut: "Ctrl+M" }
        Cmd { text: qsTr("Zoom"); cmd: "maximize" }
        TSep {}
        Cmd { text: qsTr("Bring All to Front"); cmd: "front" }
    }

    TMenu {
        id: helpMenu
        title: qsTr("&Help")
        Cmd { text: qsTr("Quick Start"); cmd: "help"; arg: "quick-start" }
        Cmd { text: qsTr("Markdown Reference"); cmd: "help"; arg: "markdown-reference" }
        Cmd { text: qsTr("Keyboard Shortcuts"); cmd: "help"; arg: "shortcuts" }
        TSep {}
        Cmd { text: qsTr("Website"); cmd: "website" }
        TSep { id: aboutSep }
        Cmd { text: qsTr("About typo"); cmd: "about" }
    }
}
