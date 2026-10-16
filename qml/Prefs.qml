pragma Singleton
import QtQuick
import QtCore

// User preferences, with Typora's defaults. The Preferences window edits
// these; they persist in ~/.config/typo/typo.conf under [prefs].
QtObject {
    id: prefs

    // The singleton as a value, for controls that take an object to edit
    // (compiled QML can't use a singleton's name as one).
    readonly property QtObject object: prefs

    // Files
    // 0 new file, 1 restore last closed folders, 2 restore last closed
    // files and folders, 3 open launchFolder
    property int launchAction: 0
    property string launchFolder: ""
    // What new files are saved as: "md", "markdown", "text" or another.
    property string defaultExtension: "md"
    property bool autoSave: false
    // Switching files in the sidebar saves the one left without asking.
    property bool saveOnSwitch: false
    property bool recordRecent: true
    // Dropping a folder, a markdown file or a file pandoc imports: 0 opens
    // (or imports) it, 1 inserts a link to it.
    property int dropFolder: 0
    property int dropMarkdown: 0
    property int dropImportable: 0
    // The file tree and the articles list: "supported" files, "all", or
    // "custom" (supported files plus `…Patterns`, a glob a line, "!" in
    // front to leave matches out).
    property bool showHiddenFiles: false
    property string treeFilter: "supported"
    property string treePatterns: ""
    property string libraryFilter: "supported"
    property string libraryPatterns: ""

    // Editor
    // Quotes and lists made from the menu: the columns a level takes (0
    // auto, -1 a tab), and whether the marker's padding fills them.
    property int indentSize: 2
    property bool prettyIndent: false
    property bool autoPairBrackets: true
    property bool autoPairMarkdown: false
    property bool showSourceOnFocus: false
    property bool emojiAutocomplete: true
    property bool copyMarkdownAsPlain: false
    // Copy or cut with nothing selected takes the caret's lines.
    property bool lineWiseCopy: false
    property bool spellCheck: false
    property string defaultLineEnding: "lf"
    property bool alwaysShowWordCount: true
    property int readingSpeed: 382        // words a minute: 120, 240 or 382
    // What the status bar counts: 0 words, 1 characters, 2 lines, 3 minutes
    property int countMode: 0

    // Markdown
    property bool inlineMath: false
    property bool subscript: false
    property bool superscript: false
    property bool highlight: false
    property bool diagrams: true
    property bool strictMode: false
    // Headings made from the menu: 0 atx, 1 setext, 2 atx with closing
    // hashes, 3 setext underlined to the heading's width.
    property int headingStyle: 0
    property string bulletMarker: "-"
    // Ordered lists made from the menu number every item 1.
    property bool orderedSame: false
    property bool autoLinks: true
    property bool alerts: true
    property bool codeLineNumbers: false
    property bool codeWrap: true
    property int codeIndentSize: 4
    // A new code fence's language: "" none, "__LAST" the last one used;
    // applied to fences typed (1), made from the menu (2), or both (3).
    property string defaultCodeLang: ""
    property int defaultCodeLangWhen: 1
    // Math in HTML without styles and copied HTML: "svg" or "latex".
    property string mathCopyFormat: "svg"
    // "" none, "ams" AMS rules, "all" every display formula
    property string mathNumbering: ""
    // \( \) and \[ \] for math, as well as $ and $$
    property bool latexDelimiters: true
    // ```math code blocks show as display math
    property bool mathFences: true
    property bool indentFirstLine: false
    // When writing, Typora's defaults: soft line breaks show, and so
    // do `<br>` tags.
    property bool preserveLineBreaks: true
    property bool visibleBr: true
    // Exports and printing keep single line breaks and runs of spaces.
    property bool exportLineBreaks: false
    // Smart quotes and dashes, typed in or only shown, with the quote
    // pairs (double then single) and whether -- is an em dash too.
    property bool smartOnRendering: false
    property bool smartQuotes: false
    property bool smartDashes: false
    property bool dashesToEm: false
    property string quotePairs: "“”‘’"
    property bool remapPunctuation: false

    // Appearance
    property bool useCustomFontSize: false
    property int customFontSize: 16
    property bool zoomByWheel: false

    // Export: each format's options as JSON, keyed by format ("general"
    // for the ones they share), and where pandoc is.
    property string exportConfig: "{}"
    readonly property var exportOptions: {
        try {
            return JSON.parse(exportConfig) || {}
        } catch (e) {
            return {}
        }
    }
    property string pandocPath: ""
    function exportOption(format, key, fallback) {
        const o = exportOptions[format]
        return o && o[key] !== undefined ? o[key] : fallback
    }
    function setExportOption(format, key, value) {
        const all = JSON.parse(JSON.stringify(exportOptions))
        all[format] = all[format] || {}
        all[format][key] = value
        exportConfig = JSON.stringify(all)
    }

    // Image
    property int imageInsertAction: 0     // 0 none, 1 copy to ./, 2 ./assets, 3 ./${filename}.assets, 4 custom
    property string imageFolder: ""
    property bool imageRelative: true
    property bool imageCopyLocal: true    // Apply above rules to local images
    property bool imageWithDot: false     // Add ./ for relative path
    property bool imageEscape: false      // Auto escape image URL when insert

    // Typewriter mode moves every caret move's line to the middle, not
    // just typing's.
    property bool caretAlwaysCentred: false
    // "Do not show this again" on the notices that focus and typewriter
    // mode are on.
    property bool noFocusModeNotice: false
    property bool noTypewriterNotice: false
    // File → Export: the format Export with Previous uses, and whether
    // Export and Overwrite with Previous asks first.
    property string lastExport: ""
    property bool noExportOverwriteWarning: false
    // "Do not show this again" on asking before deleting a file in the
    // sidebar, and before moving files dropped on the tree or the list
    // (Typora's noWarnigForDeleteFile, noWarnigForMoveFile and
    // noWarnigForMoveFileToList).
    property bool noWarningForDeleteFile: false
    property bool noWarningForMoveFile: false
    property bool noWarningForMoveFileToList: false

    // The advanced settings (conf.user.json), read when a window opens:
    // keyBinding, searchService and autoHideMenuBar.
    property var advanced: ({})
    // The keys for the menu item labelled `label`, whose own are `builtIn`:
    // keyBinding's for it, else none when keyBinding gave them to another
    // item (Qt fires neither of two items with one key), else its own.
    // Labels match as Typora's are written, "..." for "…".
    function keysFor(label, builtIn) {
        const bindings = advanced.keyBinding || {}
        const name = String(label).replace(/&/g, "").replace(/…/g, "...").trim()
        if (typeof bindings[name] === "string")
            return normalKeys(bindings[name])
        const own = normalKeys(builtIn || "")
        if (own !== "")
            for (const k in bindings)
                if (k !== name && typeof bindings[k] === "string" && normalKeys(bindings[k]) === own)
                    return ""
        return builtIn || ""
    }
    // Keys as Typora (Electron accelerators) or Qt write them, the way Qt
    // writes them: "CmdOrCtrl+shift+p" is "Ctrl+Shift+P".
    function normalKeys(keys) {
        const names = {
            cmdorctrl: "Ctrl", commandorcontrol: "Ctrl", command: "Ctrl", cmd: "Ctrl", control: "Ctrl", ctrl: "Ctrl",
            alt: "Alt", option: "Alt", altgr: "Alt", shift: "Shift", super: "Meta", meta: "Meta",
            plus: "+", space: "Space", tab: "Tab", backspace: "Backspace", delete: "Del", del: "Del", insert: "Ins",
            ins: "Ins", return: "Return", enter: "Return", escape: "Esc", esc: "Esc", up: "Up", down: "Down",
            left: "Left", right: "Right", home: "Home", end: "End", pageup: "PgUp", pgup: "PgUp", pagedown: "PgDown",
            pgdown: "PgDown"
        }
        const text = String(keys).trim()
        if (text === "")
            return ""
        // A "+" key is the one not followed by anything.
        const parts = text.split(/\+(?=.)/).map(p => names[p.toLowerCase()] || (p.length === 1 ? p.toUpperCase()
                                                                                  : p[0].toUpperCase() + p.substring(1)))
        const key = parts.pop()
        return ["Ctrl", "Alt", "Shift", "Meta"].filter(m => parts.indexOf(m) >= 0).concat([key]).join("+")
    }

    // Recently opened files, newest first, and files closed in any window,
    // for Reopen Closed File.
    property var recentFiles: []
    // Folders opened in the sidebar, newest first.
    property var recentFolders: []
    property var closedFiles: []

    property Settings store: Settings {
        category: "prefs"
        property alias launchAction: prefs.launchAction
        property alias launchFolder: prefs.launchFolder
        property alias defaultExtension: prefs.defaultExtension
        property alias autoSave: prefs.autoSave
        property alias saveOnSwitch: prefs.saveOnSwitch
        property alias recordRecent: prefs.recordRecent
        property alias dropFolder: prefs.dropFolder
        property alias dropMarkdown: prefs.dropMarkdown
        property alias dropImportable: prefs.dropImportable
        property alias showHiddenFiles: prefs.showHiddenFiles
        property alias treeFilter: prefs.treeFilter
        property alias treePatterns: prefs.treePatterns
        property alias libraryFilter: prefs.libraryFilter
        property alias libraryPatterns: prefs.libraryPatterns
        property alias indentSize: prefs.indentSize
        property alias prettyIndent: prefs.prettyIndent
        property alias autoPairBrackets: prefs.autoPairBrackets
        property alias autoPairMarkdown: prefs.autoPairMarkdown
        property alias showSourceOnFocus: prefs.showSourceOnFocus
        property alias emojiAutocomplete: prefs.emojiAutocomplete
        property alias copyMarkdownAsPlain: prefs.copyMarkdownAsPlain
        property alias lineWiseCopy: prefs.lineWiseCopy
        property alias spellCheck: prefs.spellCheck
        property alias defaultLineEnding: prefs.defaultLineEnding
        property alias alwaysShowWordCount: prefs.alwaysShowWordCount
        property alias readingSpeed: prefs.readingSpeed
        property alias countMode: prefs.countMode
        property alias inlineMath: prefs.inlineMath
        property alias subscript: prefs.subscript
        property alias superscript: prefs.superscript
        property alias highlight: prefs.highlight
        property alias diagrams: prefs.diagrams
        property alias strictMode: prefs.strictMode
        property alias headingStyle: prefs.headingStyle
        property alias bulletMarker: prefs.bulletMarker
        property alias orderedSame: prefs.orderedSame
        property alias autoLinks: prefs.autoLinks
        property alias alerts: prefs.alerts
        property alias codeLineNumbers: prefs.codeLineNumbers
        property alias codeWrap: prefs.codeWrap
        property alias codeIndentSize: prefs.codeIndentSize
        property alias defaultCodeLang: prefs.defaultCodeLang
        property alias defaultCodeLangWhen: prefs.defaultCodeLangWhen
        property alias mathCopyFormat: prefs.mathCopyFormat
        property alias mathNumbering: prefs.mathNumbering
        property alias latexDelimiters: prefs.latexDelimiters
        property alias mathFences: prefs.mathFences
        property alias indentFirstLine: prefs.indentFirstLine
        property alias preserveLineBreaks: prefs.preserveLineBreaks
        property alias visibleBr: prefs.visibleBr
        property alias exportLineBreaks: prefs.exportLineBreaks
        property alias smartOnRendering: prefs.smartOnRendering
        property alias smartQuotes: prefs.smartQuotes
        property alias smartDashes: prefs.smartDashes
        property alias dashesToEm: prefs.dashesToEm
        property alias quotePairs: prefs.quotePairs
        property alias remapPunctuation: prefs.remapPunctuation
        property alias useCustomFontSize: prefs.useCustomFontSize
        property alias customFontSize: prefs.customFontSize
        property alias zoomByWheel: prefs.zoomByWheel
        property alias exportConfig: prefs.exportConfig
        property alias pandocPath: prefs.pandocPath
        property alias imageInsertAction: prefs.imageInsertAction
        property alias imageFolder: prefs.imageFolder
        property alias imageRelative: prefs.imageRelative
        property alias imageCopyLocal: prefs.imageCopyLocal
        property alias imageWithDot: prefs.imageWithDot
        property alias imageEscape: prefs.imageEscape
        property alias caretAlwaysCentred: prefs.caretAlwaysCentred
        property alias noFocusModeNotice: prefs.noFocusModeNotice
        property alias noTypewriterNotice: prefs.noTypewriterNotice
        property alias lastExport: prefs.lastExport
        property alias noExportOverwriteWarning: prefs.noExportOverwriteWarning
        property alias noWarningForDeleteFile: prefs.noWarningForDeleteFile
        property alias noWarningForMoveFile: prefs.noWarningForMoveFile
        property alias noWarningForMoveFileToList: prefs.noWarningForMoveFileToList
        property alias recentFiles: prefs.recentFiles
        property alias recentFolders: prefs.recentFolders
        property alias closedFiles: prefs.closedFiles
    }

    function toggle(name) {
        prefs[name] = !prefs[name]
    }

    // Every window is its own process, so lists are read back from disk
    // before they change. A one-item list comes back as a plain string.
    function fresh(key) {
        store.sync()
        const v = store.value(key, [])
        return typeof v === "string" ? [v] : Array.from(v || [])
    }
    function refresh() {
        recentFiles = fresh("recentFiles")
        recentFolders = fresh("recentFolders")
        closedFiles = fresh("closedFiles")
    }

    function addRecent(path) {
        if (!path || !recordRecent)
            return
        const list = fresh("recentFiles").filter(p => p !== path)
        list.unshift(path)
        recentFiles = list.slice(0, 20)
    }
    function addRecentFolder(path) {
        if (!path || !recordRecent)
            return
        const list = fresh("recentFolders").filter(p => p !== path)
        list.unshift(path)
        recentFolders = list.slice(0, 8)
    }
    function clearRecent() {
        recentFiles = []
        recentFolders = []
    }

    function addClosed(path) {
        if (!path)
            return
        const list = fresh("closedFiles").filter(p => p !== path)
        list.unshift(path)
        closedFiles = list.slice(0, 20)
    }
    // The most recently closed file that `usable` accepts, taken off the
    // list; "" when there is none.
    function takeClosed(usable) {
        const list = fresh("closedFiles")
        const i = list.findIndex(usable)
        // The ones before it can't be reopened (gone, or open here).
        closedFiles = i < 0 ? [] : list.slice(i + 1)
        return i < 0 ? "" : list[i]
    }
}
