pragma Singleton
import QtQuick
import QtCore
import Typo

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
    property bool quitAfterWindowClose: false
    property int readingSpeed: 382        // words a minute: 120, 240 or 382
    // What the status bar counts: 0 words, 1 characters, 2 lines, 3 minutes
    property int countMode: 0

    // Markdown
    property bool inlineMath: false
    // Inline math from `$` to the next `$`, as Typora once read it.
    property bool legacyInlineMath: false
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
    property bool shiftTabAutoIndent: false
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
    // Formulas read the physics package's commands (src/physics.rs).
    property bool mathPhysics: false
    onMathPhysicsChanged: Desktop.setMathPhysics(mathPhysics)
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
    // Likewise before deleting an image's file and before moving one
    // (Typora's noWarningForDeleteImage and noWarningForMoveImage).
    property bool noWarningForDeleteImage: false
    property bool noWarningForMoveImage: false

    // The advanced settings (conf.user.json), read when a window opens:
    // keyBinding, searchService and autoHideMenuBar.
    property var advanced: ({})
    // Typora Mac's keys where they aren't Linux's, by label. On the Mac Qt's
    // Ctrl is ⌘, Meta ⌃ and Alt ⌥. Test scripts press Linux's keys, so they
    // keep them unless they set macKeys.
    property bool macKeys: (Qt.platform.os === "osx" || Qt.platform.os === "macos")
                           && !Qt.application.arguments.some(a => a.startsWith("--script="))
    readonly property var macTable: ({
        "Open Quickly...": "Ctrl+Shift+O", "Page Setup...": "Ctrl+Shift+P", "Print...": "Ctrl+P", "Save All": "Ctrl+Alt+S",
        "Redo": "Ctrl+Shift+Z",
        "Select Paragraph / Block": "Ctrl+Alt+P", "Delete Paragraph / Block": "Ctrl+Alt+Shift+P",
        "Delete Line / Sentence": "Ctrl+Alt+Shift+L", "Delete Styled Scope": "Ctrl+Alt+Shift+E",
        "Jump to Top": "Ctrl+Up", "Jump to Bottom": "Ctrl+Down",
        "Find Next": "Ctrl+G", "Find Previous": "Ctrl+Shift+G", "Replace": "Ctrl+Alt+F",
        "Replace Next": "Ctrl+Alt+E", "Paste and Match Style": "Ctrl+Alt+Shift+V",
        "Insert Table": "Ctrl+Alt+T", "Move Row Up": "Ctrl+Meta+Up", "Move Row Down": "Ctrl+Meta+Down",
        "Move Column Left": "Ctrl+Meta+Left", "Move Column Right": "Ctrl+Meta+Right",
        "Code Fences": "Ctrl+Alt+C", "Math Block": "Ctrl+Alt+B", "Quote": "Ctrl+Alt+Q",
        "Ordered List": "Ctrl+Alt+O", "Unordered List": "Ctrl+Alt+U", "Task List": "Ctrl+Alt+X",
        "Toggle Task Status": "Meta+X", "Indent": "Ctrl+]", "Outdent": "Ctrl+[",
        "Link Reference": "Ctrl+Alt+L", "Footnotes": "Ctrl+Alt+R", "Horizontal Line": "Ctrl+Alt+-",
        "Code": "Meta+`", "Inline Math": "Meta+M", "Strike": "Shift+Meta+`", "Highlight": "Ctrl+Shift+H",
        "Comment": "Meta+-", "Insert Image": "Ctrl+Meta+I",
        "Outline": "Ctrl+Meta+1", "Articles": "Ctrl+Meta+2", "File Tree": "Ctrl+Meta+3",
        "Toggle Fullscreen": "Ctrl+Meta+F", "Switch Between Opened Documents": "Meta+Tab"
    })
    // The keys for the menu item labelled `label`, whose own are `builtIn`
    // (or the Mac table's): keyBinding's for it, else none when keyBinding
    // gave them to another item (Qt fires neither of two items with one
    // key), else its own. Labels match as Typora's are written, "..." for "…".
    function keysFor(label, builtIn) {
        const bindings = advanced.keyBinding || {}
        const name = String(label).replace(/&/g, "").replace(/…/g, "...").trim()
        if (typeof bindings[name] === "string")
            return normalKeys(bindings[name])
        const keys = macKeys && macTable[name] !== undefined ? macTable[name] : builtIn || ""
        const own = normalKeys(keys)
        if (own !== "")
            for (const k in bindings)
                if (k !== name && typeof bindings[k] === "string" && normalKeys(bindings[k]) === own)
                    return ""
        return keys
    }
    // Keys as the Mac shows them: "Ctrl+Shift+C" is "⇧⌘C".
    function macText(keys) {
        const parts = String(keys).split(/\+(?=.)/)
        const key = parts.pop()
        const names = { Up: "↑", Down: "↓", Left: "←", Right: "→", Return: "↩", Enter: "↩", Backspace: "⌫",
                        Del: "⌦", Tab: "⇥", Esc: "⎋" }
        return [["Meta", "⌃"], ["Alt", "⌥"], ["Shift", "⇧"], ["Ctrl", "⌘"]]
            .filter(m => parts.indexOf(m[0]) >= 0).map(m => m[1]).join("") + (names[key] || key)
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
    // Folders opened in the sidebar, newest first, and those pinned in its
    // Recent Locations, which come first and stay.
    property var recentFolders: []
    property var pinnedFolders: []
    property var closedFiles: []

    // What persists. A window writes only the key that changed and reads the
    // others back every second, so a preference changed in one window
    // reaches every open one (each has a Prefs of its own: its process's, or
    // on the Mac its engine's).
    readonly property var keys: [
        "launchAction", "launchFolder", "defaultExtension", "autoSave", "saveOnSwitch", "recordRecent",
        "dropFolder", "dropMarkdown", "dropImportable", "showHiddenFiles", "treeFilter", "treePatterns",
        "libraryFilter", "libraryPatterns", "indentSize", "prettyIndent", "autoPairBrackets",
        "autoPairMarkdown", "showSourceOnFocus", "emojiAutocomplete", "copyMarkdownAsPlain", "lineWiseCopy",
        "spellCheck", "defaultLineEnding", "alwaysShowWordCount", "quitAfterWindowClose", "readingSpeed", "countMode", "inlineMath", "legacyInlineMath",
        "subscript", "superscript", "highlight", "diagrams", "strictMode", "headingStyle", "bulletMarker",
        "orderedSame", "autoLinks", "alerts", "codeLineNumbers", "codeWrap", "shiftTabAutoIndent",
        "codeIndentSize", "defaultCodeLang", "defaultCodeLangWhen", "mathCopyFormat", "mathNumbering", "latexDelimiters",
        "mathFences", "mathPhysics", "indentFirstLine", "preserveLineBreaks", "visibleBr", "exportLineBreaks",
        "smartOnRendering", "smartQuotes", "smartDashes", "dashesToEm", "quotePairs", "remapPunctuation",
        "useCustomFontSize", "customFontSize", "zoomByWheel", "exportConfig", "pandocPath",
        "imageInsertAction", "imageFolder", "imageRelative", "imageCopyLocal", "imageWithDot",
        "imageEscape", "caretAlwaysCentred", "noFocusModeNotice", "noTypewriterNotice", "lastExport",
        "noExportOverwriteWarning", "noWarningForDeleteFile", "noWarningForMoveFile",
        "noWarningForMoveFileToList", "noWarningForDeleteImage", "noWarningForMoveImage", "recentFiles", "recentFolders", "pinnedFolders", "closedFiles"
    ]
    property Settings store: Settings { category: "prefs" }
    property bool loading: false
    property Timer poll: Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: prefs.reload()
    }
    Component.onCompleted: {
        reload()
        for (const k of keys)
            prefs[k + "Changed"].connect(() => { if (!loading) store.setValue(k, prefs[k]) })
    }

    // The stored value of `key`, as its property's type; undefined when
    // there is none. INI keeps it all as text, a one-item list as a plain
    // string and an empty one as nothing.
    function stored(key) {
        const v = store.value(key, "\u0001")
        if (v === "\u0001")
            return undefined
        switch (typeof prefs[key]) {
        case "boolean": return v === true || v === "true"
        case "number": return Number(v)
        case "string": return String(v)
        }
        return v === undefined || v === "" ? [] : typeof v === "string" ? [v] : Array.from(v)
    }
    function reload() {
        store.sync()
        loading = true
        for (const k of keys) {
            const v = stored(k)
            if (v !== undefined && JSON.stringify(v) !== JSON.stringify(prefs[k]))
                prefs[k] = v
        }
        loading = false
    }

    function toggle(name) {
        prefs[name] = !prefs[name]
    }

    // Lists are read back from disk before they change, as another window
    // may have changed them since the last poll.
    function fresh(key) {
        store.sync()
        return stored(key) || []
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
    function pinFolder(path, pinned) {
        const list = fresh("pinnedFolders").filter(p => p !== path)
        if (pinned)
            list.push(path)
        pinnedFolders = list
    }
    function removeRecentFolder(path) {
        recentFolders = fresh("recentFolders").filter(p => p !== path)
        pinFolder(path, false)
    }
    function clearRecent() {
        recentFiles = []
        recentFolders = []
        pinnedFolders = []
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
