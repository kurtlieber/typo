import QtQuick
import QtQuick.Controls
import Typo

// Preferences ▸ Markdown.
Column {
    id: section

    required property var app

    width: parent ? parent.width : 0

    PrefGroup {
        title: qsTr("Syntax Preference")
        hint: qsTr("(only applies to styles created from menubar or hybrid view)")
        PrefCheck {
            text: qsTr("Strict Mode")
            key: "strictMode"
            hint: qsTr("(more strict to GFM spec)")
        }
        Item { width: 1; height: 6 }
        PrefChoice {
            label: qsTr("Heading Style")
            labelWidth: 160
            key: "headingStyle"
            options: [[0, qsTr("atx (#)")], [2, qsTr("atx with suffix (# ⋯ #)")], [1, qsTr("setext (===)")],
                [3, qsTr("variable-width setext (=====)")]]
            boxWidth: 260
        }
        PrefChoice {
            label: qsTr("Unordered List")
            labelWidth: 160
            key: "bulletMarker"
            options: [["-", "-"], ["+", "+"], ["*", "*"]]
            boxWidth: 260
        }
        PrefChoice {
            label: qsTr("Ordered List")
            labelWidth: 160
            key: "orderedSame"
            options: [[false, "1. ⋯ 2. ⋯ 3. ⋯"], [true, "1. ⋯ 1. ⋯ 1. ⋯"]]
            boxWidth: 260
        }
    }

    PrefGroup {
        title: qsTr("Syntax Support")
        hint: qsTr("(Following preferences will be applied after restart.)")
        PrefCheck {
            text: qsTr("Auto Links (e.g: https://typora.io)")
            key: "autoLinks"
            hint: qsTr("When enabled, typo will auto recognize and highlight URLs (e.g: https://typora.io or mailto:hi@typora.io) in text.")
        }
        PrefCheck {
            text: qsTr("Inline Math (e.g: $\\LaTeX$)")
            key: "inlineMath"
        }
        PrefCheck {
            text: qsTr("Subscript (e.g: H~2~O)")
            key: "subscript"
        }
        PrefCheck {
            text: qsTr("Superscript (e.g: X^2^)")
            key: "superscript"
        }
        PrefCheck {
            text: qsTr("Highlight (e.g: ==key==)")
            key: "highlight"
        }
        PrefCheck {
            text: qsTr("Github Style Alert")
            key: "alerts"
        }
        PrefCheck {
            text: qsTr("Diagrams (Sequence, Flowchart and Mermaid)")
            key: "diagrams"
        }
    }

    PrefGroup {
        title: qsTr("Smart Punctuation")
        PrefChoice {
            key: "smartOnRendering"
            options: [[false, qsTr("Convert on Input")], [true, qsTr("Convert on Rendering")]]
            hint: qsTr("<b>Convert on Input</b>: Input ASCII dashes or quotes, converted and saved as curly quotes or unicode dashes.<br/><b>Convert on Rendering</b>: Input and saved as ASCII dashes or quotes, render as curly quotes or unicode dashes.")
        }
        Item { width: 1; height: 4 }
        Row {
            spacing: 12
            PrefCheck {
                width: implicitWidth
                anchors.verticalCenter: parent.verticalCenter
                text: qsTr("Smart Quotes")
                key: "smartQuotes"
            }
            PrefChoice {
                anchors.verticalCenter: parent.verticalCenter
                enabled: Prefs.smartQuotes
                key: "quotePairs"
                options: [["“”‘’", "“abc” ‘abc’"], ["„“‚‘", "„abc“ ‚abc‘"], ["„”‚’", "„abc” ‚abc’"], ["«»‹›", "«abc» ‹abc›"],
                    ["»«›‹", "»abc« ›abc‹"], ["「」『』", "「abc」『abc』"]]
                boxWidth: 180
            }
        }
        PrefCheck {
            text: qsTr("Smart Dashes")
            key: "smartDashes"
        }
        PrefChoice {
            visible: Prefs.smartDashes
            key: "dashesToEm"
            options: [[false, qsTr("Convert -- to – (en dash), and --- to — (em dash)")], [true, qsTr("Convert -- and --- to — (em dash)")]]
            boxWidth: 380
        }
        PrefCheck {
            readonly property bool forced: !Prefs.smartOnRendering && (Prefs.smartQuotes || Prefs.smartDashes)
            text: qsTr("Remap Unicode Punctuation on Parse")
            key: "remapPunctuation"
            checked: Prefs.remapPunctuation || forced
            enabled: !forced
            hint: qsTr("Recognize unicode punctuations as ASCII ones when parse markdown, e.g:<pre>》 blockquote → &gt; blockquote<br/>[link](src ’title’) → [link](src 'title')</pre>It will be turned on automatically if smart quotes/dashes will be converted on input.")
        }
    }

    PrefGroup {
        title: qsTr("Code Fences")
        PrefCheck {
            text: qsTr("Display line numbers for code fences")
            key: "codeLineNumbers"
        }
        PrefCheck {
            text: qsTr("Auto wrap long lines")
            key: "codeWrap"
        }
        PrefCheck {
            text: qsTr("Use Shift+Tab to auto indent selected code")
            key: "shiftTabAutoIndent"
            hint: qsTr("When disabled, Shift+Tab will outdent selected code or current line. When enabled, Shift+Tab will auto apply indentation for selected code or current line in code blocks.")
        }
        Item { width: 1; height: 6 }
        PrefChoice {
            label: qsTr("Indent Size for Code")
            labelWidth: 230
            key: "codeIndentSize"
            options: [[2, "2"], [3, "3"], [4, "4"], [5, "5"]]
            boxWidth: 100
        }
        Row {
            id: language
            readonly property bool searchHit: Ui.prefsHit(languageLabel.text)
            height: 30
            Text {
                id: languageLabel
                width: 230
                anchors.verticalCenter: parent.verticalCenter
                text: qsTr("Default Code Language")
                color: Theme.foreground
                font.family: Ui.uiFont
                font.pixelSize: 16
                Rectangle {
                    visible: language.searchHit
                    z: -1
                    width: parent.contentWidth
                    height: parent.height
                    color: Qt.alpha(Theme.mark, 0.48)
                }
            }
            // Typora's field with suggestions: (None), Last Used, then the
            // languages code fences know.
            ComboBox {
                id: languages
                width: 260
                height: 28
                anchors.verticalCenter: parent.verticalCenter
                editable: true
                font.family: Ui.uiFont
                font.pixelSize: 14
                readonly property var specials: [qsTr("(None)"), qsTr("Last Used")]
                model: specials.concat(Desktop.codeLanguages().split("\n").filter(s => s !== ""))
                Component.onCompleted: editText = shown()
                function shown() {
                    return Prefs.defaultCodeLang === "" ? "" : Prefs.defaultCodeLang === "__LAST" ? specials[1] : Prefs.defaultCodeLang
                }
                Connections {
                    target: Prefs
                    function onDefaultCodeLangChanged() {
                        if (!languages.activeFocus)
                            languages.editText = languages.shown()
                    }
                }
                function store(text) {
                    const t = text.trim()
                    Prefs.defaultCodeLang = t === specials[0] ? "" : t === specials[1] ? "__LAST" : t
                    editText = shown()
                }
                onActivated: index => store(textAt(index))
                onAccepted: store(editText)
                onActiveFocusChanged: if (!activeFocus) store(editText)
                Text {
                    visible: languages.editText === "" && !languages.activeFocus
                    x: 8
                    anchors.verticalCenter: parent.verticalCenter
                    text: languages.specials[0]
                    color: Theme.faint
                    font: languages.font
                }
            }
        }
        PrefChoice {
            label: qsTr("Apply Default Code Language When")
            labelWidth: 230
            key: "defaultCodeLangWhen"
            options: [[1, qsTr("When add code fences via Markdown")], [2, qsTr("When add code fences via Menubar")],
                [3, qsTr("Both")]]
            boxWidth: 260
        }
    }

    PrefGroup {
        title: qsTr("Math")
        PrefCheck {
            text: qsTr("Inline Math (e.g: $\\LaTeX$)")
            key: "inlineMath"
        }
        PrefCheck {
            text: qsTr("LaTeX Math Delimiter \\( \\) \\[ \\]")
            key: "latexDelimiters"
        }
        PrefCheck {
            text: qsTr("Code Block Math (```math code block)")
            key: "mathFences"
        }
        Item { width: 1; height: 8 }
        PrefChoice {
            label: qsTr("Auto Numbering Math Equations")
            labelWidth: 230
            key: "mathNumbering"
            options: [["", qsTr("No Automatic Equation Numbering")], ["ams", qsTr("Use AMS Numbering Rules")],
                ["all", qsTr("Auto Numbering for All Math Equations")]]
            boxWidth: 300
        }
        PrefChoice {
            label: qsTr("When copy / export as HTML (without style)")
            labelWidth: 230
            key: "mathCopyFormat"
            options: [["svg", qsTr("Use Rendered SVG")], ["latex", qsTr("Use LaTeX Source")]]
            boxWidth: 300
            hint: qsTr("Take effects when export as HTML (without styles) or copy as HTML code, or copy without theme style.<br/>When <b>Use LaTeX Source</b> is selected, typo will put the raw LaTeX source (e.g: $X^2$),<br/>When <b>Use Rendered SVG</b> is selected, the rendered SVG will be used, if you paste to other app, SVG may get ignored.")
        }
    }

    PrefGroup {
        title: qsTr("Whitespace / Line Break")
        PrefCheck {
            text: qsTr("Indent first line of paragraphs")
            key: "indentFirstLine"
            hint: qsTr("These options can also be changed under \"Edit→Whitespace and Line Breaks\" menu")
        }
        Item { width: 1; height: 6 }
        Row {
            readonly property bool searchHit: Ui.prefsHit(writing.text)
            Text {
                id: writing
                width: 140
                height: 30
                verticalAlignment: Text.AlignVCenter
                text: qsTr("When Writing")
                color: Theme.foreground
                font.family: Ui.uiFont
                font.pixelSize: 16
            }
            Column {
                PrefCheck {
                    width: implicitWidth
                    text: qsTr("Visible <br/>")
                    key: "visibleBr"
                }
                PrefChoice {
                    key: "preserveLineBreaks"
                    options: [[true, qsTr("Preserve sequential whitespace and single line break")], [false, qsTr("Ignore sequential whitespace and single line break")]]
                    boxWidth: 340
                }
            }
        }
        PrefChoice {
            label: qsTr("Export / Print")
            labelWidth: 140
            key: "exportLineBreaks"
            options: [[true, qsTr("Preserve sequential whitespace and single line break")], [false, qsTr("Ignore sequential whitespace and single line break")]]
            boxWidth: 340
        }
    }
}
