import QtQuick
import QtQuick.Controls
import Typo

// The sidebar's Search: every file in the folder searched as you type,
// with Typora's case, whole word and regular expression toggles. A result
// per file, with how many matches it has and where it is, and its
// matching lines; clicking one opens the file there.
Item {
    id: search

    required property var app
    required property var side
    readonly property string root: app.folder
    property alias query: input.text
    property bool caseSensitive: false
    property bool wholeWord: false
    property bool regex: false
    property string error: ""
    // Files whose lines are folded away, by path.
    property var folded: ({})

    function focusInput() {
        input.forceActiveFocus()
        input.selectAll()
    }
    function run() {
        if (root === "" || query === "") {
            error = ""
            hits.clear()
            return
        }
        const result = JSON.parse(Folder.search(root, query, caseSensitive, wholeWord, regex))
        error = result.error
        hits.clear()
        for (const h of result.hits)
            hits.append({ path: h.path, count: h.count, lines: JSON.stringify(h.lines) })
    }
    function setAll(open) {
        const f = {}
        if (!open)
            for (let i = 0; i < hits.count; i++)
                f[hits.get(i).path] = true
        folded = f
    }
    // A line of text with its matches marked, as rich text.
    function marked(text, ranges) {
        let out = "", at = 0
        for (const [a, b] of ranges) {
            out += side.html(text.substring(at, a))
            out += "<span style='background-color:" + Qt.alpha(Theme.mark, 0.4) + "; color:" + Theme.foreground + "'>"
                + side.html(text.substring(a, b)) + "</span>"
            at = b
        }
        return out + side.html(text.substring(at))
    }

    Timer {
        id: pending
        interval: 300
        onTriggered: search.run()
    }
    onQueryChanged: pending.restart()
    onCaseSensitiveChanged: run()
    onWholeWordChanged: run()
    onRegexChanged: run()
    onRootChanged: run()
    Connections {
        target: Folder
        enabled: search.visible
        function onChanged() { pending.restart() }
    }

    ListModel { id: hits }
    readonly property alias results: hits

    Item {
        id: panel
        width: parent.width
        height: 50
        TextField {
            id: input
            x: 16
            y: 8
            width: parent.width - 32
            height: 28
            placeholderText: qsTr("Search")
            placeholderTextColor: Theme.faint
            color: Theme.foreground
            selectionColor: Theme.selection
            selectedTextColor: Theme.selectionText
            font.pixelSize: 13
            leftPadding: 2
            rightPadding: options.width + 4
            background: Rectangle {
                color: "transparent"
                Rectangle {
                    y: parent.height - 1
                    width: parent.width
                    height: 1
                    color: input.activeFocus ? Theme.accent : Theme.border
                }
            }
            Keys.onEscapePressed: search.side.closeSearch()
            onAccepted: search.run()
        }
        Row {
            id: options
            anchors.right: input.right
            anchors.verticalCenter: input.verticalCenter
            spacing: 2
            Repeater {
                model: [
                    { key: "caseSensitive", label: "Aa", hint: qsTr("Case Sensitive") },
                    { key: "wholeWord", label: "ab", hint: qsTr("Whole Word") },
                    { key: "regex", label: ".*", hint: qsTr("Regular Expression") }
                ]
                delegate: AbstractButton {
                    id: option
                    required property var modelData
                    readonly property bool on: search[modelData.key]
                    width: 20
                    height: 18
                    hoverEnabled: true
                    focusPolicy: Qt.NoFocus
                    onClicked: search[modelData.key] = !search[modelData.key]
                    ToolTip.visible: hovered
                    ToolTip.delay: 600
                    ToolTip.text: modelData.hint
                    background: Rectangle {
                        radius: 3
                        color: option.on ? Theme.selection : option.hovered ? search.side.hoverColor : "transparent"
                    }
                    contentItem: Text {
                        text: option.modelData.label
                        font.pixelSize: 11
                        font.bold: option.modelData.key === "caseSensitive"
                        font.underline: option.modelData.key === "wholeWord"
                        color: option.on ? Theme.selectionText : Theme.muted
                        opacity: option.on || option.hovered ? 1 : 0.7
                        horizontalAlignment: Text.AlignHCenter
                        verticalAlignment: Text.AlignVCenter
                    }
                }
            }
        }
    }

    ListView {
        id: results
        anchors.top: panel.bottom
        anchors.bottom: parent.bottom
        width: parent.width
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {}
        model: hits
        delegate: Column {
            id: hit
            required property string path
            required property int count
            required property string lines
            readonly property var matches: JSON.parse(lines)
            readonly property bool open: !search.folded[path]
            readonly property string name: path.substring(path.lastIndexOf("/") + 1)
            readonly property string folder: path.substring(0, path.lastIndexOf("/"))
            width: results.width
            bottomPadding: 6

            AbstractButton {
                id: summary
                width: parent.width
                height: open ? 46 : 28
                readonly property bool open: hit.open
                hoverEnabled: true
                onClicked: search.side.open(hit.path)
                TapHandler {
                    acceptedButtons: Qt.RightButton
                    onTapped: search.side.showMenu(hit.path, "search")
                }
                background: Rectangle {
                    color: summary.hovered ? search.side.hoverColor : "transparent"
                }
                contentItem: Item {
                    AbstractButton {
                        x: 4
                        y: 6
                        width: 18
                        height: 20
                        focusPolicy: Qt.NoFocus
                        onClicked: {
                            const f = Object.assign({}, search.folded)
                            f[hit.path] = hit.open
                            search.folded = f
                        }
                        contentItem: Glyph {
                            name: hit.open ? "caretDown" : "caretRight"
                            size: 11
                            color: Theme.muted
                            opacity: 0.7
                        }
                        background: null
                    }
                    Text {
                        x: 24
                        y: 6
                        width: parent.width - x - countText.width - 24
                        height: 20
                        verticalAlignment: Text.AlignVCenter
                        textFormat: Text.StyledText
                        text: "<b>" + search.side.html(hit.name.replace(/\.[^.]*$/, "")) + "</b><font color='" + Theme.faint + "'>"
                            + search.side.html(hit.name.substring(hit.name.replace(/\.[^.]*$/, "").length)) + "</font>"
                        color: Theme.foreground
                        font.pixelSize: 13
                        elide: Text.ElideRight
                    }
                    Text {
                        id: countText
                        anchors.right: parent.right
                        anchors.rightMargin: 16
                        y: 6
                        height: 20
                        verticalAlignment: Text.AlignVCenter
                        text: hit.count
                        color: Theme.faint
                        font.pixelSize: 12
                    }
                    Text {
                        visible: hit.open
                        x: 24
                        y: 26
                        width: parent.width * 0.9 - x
                        text: search.side.tilde(hit.folder)
                        color: Theme.faint
                        font.pixelSize: 12
                        elide: Text.ElideMiddle
                    }
                }
            }
            Repeater {
                model: hit.open ? hit.matches : []
                delegate: AbstractButton {
                    id: line
                    required property var modelData
                    width: hit.width
                    height: 22
                    hoverEnabled: true
                    onClicked: search.side.openAt(hit.path, modelData.line)
                    background: Rectangle {
                        color: line.hovered ? search.side.activeColor : "transparent"
                    }
                    contentItem: Text {
                        leftPadding: 24
                        rightPadding: 8
                        verticalAlignment: Text.AlignVCenter
                        textFormat: Text.RichText
                        text: search.marked(line.modelData.text, line.modelData.ranges)
                        color: Theme.muted
                        font.pixelSize: 13
                        elide: Text.ElideRight
                        clip: true
                    }
                }
            }
            Rectangle {
                width: parent.width
                height: 1
                color: Theme.border
                opacity: 0.6
            }
        }
    }

    Text {
        anchors.top: panel.bottom
        anchors.topMargin: 8
        x: 16
        width: parent.width - 32
        visible: search.error !== ""
        text: search.error
        color: Theme.error
        font.pixelSize: 12
        wrapMode: Text.Wrap
    }
    Text {
        anchors.centerIn: parent
        visible: search.error === "" && search.query !== "" && hits.count === 0 && !pending.running
        text: qsTr("No Results")
        color: Theme.faint
        font.pixelSize: 13
    }
}
