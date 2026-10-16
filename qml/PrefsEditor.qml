import QtQuick
import Typo

// Preferences ▸ Editor.
Column {
    id: section

    required property var app

    width: parent ? parent.width : 0

    PrefGroup {
        title: qsTr("Indent Size on Save")
        hint: qsTr("only apply to quotes and lists created from menu bar or hybrid view")
        Row {
            spacing: 16
            Column {
                spacing: 6
                PrefChoice {
                    key: "indentSize"
                    options: [[0, qsTr("Auto")], [2, "2"], [3, "3"], [4, "4"], [5, "5"], [-1, qsTr("Tab")]]
                    boxWidth: 100
                }
                PrefCheck {
                    width: implicitWidth
                    text: qsTr("Pretty indentation")
                    key: "prettyIndent"
                }
            }
            // What a quote and a list come out as.
            Rectangle {
                width: sample.implicitWidth + 16
                height: sample.implicitHeight
                color: "transparent"
                border.color: Theme.foreground
                Text {
                    id: sample
                    x: 8
                    // A tab shows as four columns.
                    text: section.sampleIndent(Prefs.indentSize, Prefs.prettyIndent).replace(/\t/g, "    ")
                    color: Theme.foreground
                    font.family: Ui.monoFont
                    font.pixelSize: 13
                    lineHeight: 1.5
                }
            }
        }
    }
    function sampleIndent(size, pretty) {
        if (size < 0)
            return "> Quote\n- List item\n\t1. sub-bullet\n\t\tline2"
        const n = Math.max(2, size || 2)
        const pad = s => " ".repeat(s)
        if (pretty)
            return ">" + pad(n - 1) + "Quote\n-" + pad(n - 1) + "List item\n" + pad(n) + "1." + pad(n - 2 || 1) + "sub-bullet\n"
                + pad(n + Math.max(3, n)) + "line2"
        return "> Quote\n- List item\n" + pad(n) + "1. sub-bullet\n" + pad(n + 3) + "line2"
    }

    PrefGroup {
        title: qsTr("Auto Pair")
        PrefCheck {
            text: qsTr("Auto pair brackets and quotes")
            key: "autoPairBrackets"
        }
        PrefCheck {
            text: qsTr("Auto pair common Markdown syntax")
            key: "autoPairMarkdown"
            enabled: Prefs.autoPairBrackets
            checked: Prefs.autoPairBrackets && Prefs.autoPairMarkdown
        }
    }

    PrefGroup {
        title: qsTr("Auto Complete")
        PrefCheck {
            text: qsTr("Enable autocomplete for Emojis")
            key: "emojiAutocomplete"
        }
    }

    PrefGroup {
        title: qsTr("Live Rendering")
        PrefCheck {
            text: qsTr("Display source for simple blocks\n(including headings, etc.) on focus")
            key: "showSourceOnFocus"
        }
    }

    PrefGroup {
        title: qsTr("Default Copy Behavior")
        hint: "(ctrl+c)"
        PrefCheck {
            text: qsTr("Copy Markdown source as plain text")
            key: "copyMarkdownAsPlain"
        }
        PrefCheck {
            text: qsTr("Copy or cut the whole lines that have cursors on them, if there is no selection when doing copy / cut.")
            key: "lineWiseCopy"
        }
    }

    PrefGroup {
        title: qsTr("Default Line Ending")
        hint: qsTr("Line ending for new file")
        PrefChoice {
            key: "defaultLineEnding"
            options: [["lf", qsTr("LF (Unix Style)")], ["crlf", qsTr("CRLF (Windows Style)")]]
        }
    }

    PrefGroup {
        title: qsTr("Typewriter Mode")
        PrefCheck {
            text: qsTr("Always keep caret in middle of screen\nwhen typewriter mode is enabled")
            key: "caretAlwaysCentred"
        }
        PrefButton {
            text: qsTr("Turn off Typewriter / Focus Mode")
            onClicked: {
                Ui.focusMode = false
                Ui.typewriter = false
                section.app.notify(qsTr("Typewriter / Focus Mode have been turned off."))
            }
        }
    }
}
