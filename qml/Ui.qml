pragma Singleton
import QtQuick
import QtCore

// App-wide view state and metrics. Everything the user can toggle from the
// View menu lives here and persists in ~/.config/typo/typo.conf.
QtObject {
    id: ui

    // The singleton as a value (see Prefs.object).
    readonly property QtObject object: ui

    // Typora's default theme sets body text in Open Sans at 16px, unless
    // Preferences ▸ Appearance ▸ Font Size is customized.
    property real zoom: 1.0
    readonly property real fontSize: (Prefs.useCustomFontSize ? Prefs.customFontSize : 16) * zoom
    readonly property bool mac: Qt.platform.os === "osx" || Qt.platform.os === "macos"
    readonly property string textFont: pick(["Open Sans", "Noto Sans", "Clear Sans", "Helvetica Neue", "Liberation Sans"])
    // fontconfig resolves "monospace" to the font Omarchy has set; the Mac
    // has no such alias.
    readonly property string monoFont: mac ? "Menlo" : "monospace"
    // Modifiers that make a key a command rather than typing. On the Mac,
    // Option types characters (é, ™, –).
    readonly property int notTyping: Qt.ControlModifier | Qt.MetaModifier | (mac ? 0 : Qt.AltModifier)
    readonly property string uiFont: textFont

    property bool sidebar: false
    property int sidebarWidth: 270
    // 0 outline, 1 articles (file list), 2 file tree
    property int sidebarView: 0
    // Which the Files tab shows, kept while the Outline tab is up.
    property bool fileTree: false
    onSidebarViewChanged: if (sidebarView > 0) fileTree = sidebarView === 2
    // The articles list's order (natural, name, modified or created), and
    // whether it keeps each folder's files together.
    property string fileSort: "natural"
    property bool fileSortDescending: false
    property bool groupByFolder: true
    // The file tree's, kept apart as Typora keeps them: with grouping, its
    // folders come before its files.
    property string treeSort: "natural"
    property bool treeSortDescending: false
    property bool treeGroupByFolder: true
    // The outline: carets that fold headings away, or flat; and the
    // heading the caret is under in bold.
    property bool outlineCollapsible: false
    property bool outlineHighlight: true
    property bool statusBar: true
    property bool sourceMode: false
    property bool focusMode: false
    property bool typewriter: false
    property bool alwaysOnTop: false
    // Theme picked in the Themes menu; empty follows Omarchy.
    property string theme: ""

    // The Preferences panel: open or not, the section shown, what its
    // Search field holds, the group "Global Image Settings…" points at,
    // and whether it is wide enough to set group titles beside their
    // controls.
    property bool prefsOpen: false
    property int prefsSection: 0
    property string prefsQuery: ""
    property string prefsHighlight: ""
    property bool prefsWide: true
    // Whether a preference's label holds what Search asks for.
    function prefsHit(text) {
        const q = prefsQuery.trim().toLowerCase()
        return q !== "" && String(text).toLowerCase().indexOf(q) >= 0
    }

    // The size an image shows at, in page pixels: its own size at 100%
    // zoom, or the CSS `width` and `height` its tag gives (`size`, as
    // `[width, height, zoom]`), keeping its proportions when only one is
    // given, times the tag's `zoom`; never wider than `room`, as Typora's
    // `max-width: 100%`. Percent widths are of `room`.
    function imageSize(naturalWidth, naturalHeight, size, room, scale) {
        function length(v, of) {
            const m = /^\s*([0-9.]+)\s*(px|%)?\s*$/i.exec(v || "")
            if (!m)
                return NaN
            const n = parseFloat(m[1])
            return m[2] === "%" ? n * of / 100 : n * scale
        }
        let w = length(size && size[0], room)
        let h = length(size && size[1], NaN)
        if (isNaN(w) && isNaN(h)) {
            w = naturalWidth * scale
            h = naturalHeight * scale
        } else if (isNaN(h)) {
            h = naturalWidth > 0 ? w * naturalHeight / naturalWidth : 0
        } else if (isNaN(w)) {
            w = naturalHeight > 0 ? h * naturalWidth / naturalHeight : 0
        }
        const z = /^\s*([0-9.]+)\s*(%)?\s*$/.exec((size && size[2]) || "")
        const zoom = z && parseFloat(z[1]) > 0 ? parseFloat(z[1]) / (z[2] ? 100 : 1) : 1
        w *= zoom
        h *= zoom
        if (w > room && w > 0) {
            h *= room / w
            w = room
        }
        return Qt.size(w, h)
    }

    // Reload All Images: each picture's URL takes a new fragment, so Qt
    // loads it again instead of taking it from its cache.
    property int imageReload: 0
    function reloaded(url) {
        if (imageReload === 0 || url === "" || url.startsWith("data:") || url.startsWith("image:") || url.indexOf("#") >= 0)
            return url
        return url + "#reload-" + imageReload
    }

    // What each `\label` in the document's display math refers to, for
    // `\eqref` and `\ref` (EditorView keeps it current).
    property var mathLabels: ({})
    // TeX to typeset: references written out as what they refer to,
    // `???` for a label no formula has, as MathJax shows them.
    function mathTex(tex) {
        if (tex.indexOf("ref") < 0)
            return tex
        const labels = mathLabels
        return tex.replace(/\\(eqref|ref)(?![A-Za-z])\s*\{([^{}]*)\}/g, (m, cmd, label) => {
            const n = labels[label.trim()] || "???"
            return "\\text{" + (cmd === "eqref" ? "(" + n + ")" : n) + "}"
        })
    }

    function pick(families) {
        const installed = Qt.fontFamilies()
        for (const f of families)
            if (installed.indexOf(f) >= 0)
                return f
        return "sans-serif"
    }

    property Settings store: Settings {
        category: "view"
        property alias zoom: ui.zoom
        property alias sidebar: ui.sidebar
        property alias sidebarWidth: ui.sidebarWidth
        property alias sidebarView: ui.sidebarView
        property alias fileTree: ui.fileTree
        property alias fileSort: ui.fileSort
        property alias fileSortDescending: ui.fileSortDescending
        property alias groupByFolder: ui.groupByFolder
        property alias treeSort: ui.treeSort
        property alias treeSortDescending: ui.treeSortDescending
        property alias treeGroupByFolder: ui.treeGroupByFolder
        property alias outlineCollapsible: ui.outlineCollapsible
        property alias outlineHighlight: ui.outlineHighlight
        property alias statusBar: ui.statusBar
        property alias focusMode: ui.focusMode
        property alias typewriter: ui.typewriter
        property alias theme: ui.theme
    }
}
