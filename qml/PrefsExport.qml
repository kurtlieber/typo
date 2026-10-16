import QtQuick
import QtQuick.Controls
import Typo

// Preferences ▸ Export: the formats on the left, as File ▸ Export lists
// them, and the chosen one's options on the right. Every format's options
// live in Prefs.exportConfig under its name; "general" holds the shared
// ones.
Column {
    id: section

    required property var app
    property string current: "general"
    readonly property var formats: [
        ["general", qsTr("General")], ["pdf", "PDF"], ["html", "HTML"], ["html-plain", qsTr("HTML (without Styles)")],
        ["image", qsTr("Image")], ["docx", "Word (.docx)"], ["odt", "OpenOffice"], ["rtf", "RTF"], ["epub", "Epub"],
        ["latex", "LaTeX"], ["mediawiki", "Media Wiki"], ["rst", "reStructuredText"], ["textile", "Textile"], ["opml", "OPML"]]
    readonly property bool pandoc: !["general", "pdf", "html", "html-plain", "image"].includes(current)
    // The labels a format's options have, for Search to mark it in the list.
    readonly property var labels: {
        const after = [qsTr("After Export"), qsTr("Open exported file location"), qsTr("Open exported file"), qsTr("Run command")]
        const yaml = [qsTr("Read and overwrite export settings from YAML front matters")]
        const pandoc = [qsTr("Arguments"), qsTr("Extra Arguments")].concat(after)
        return {
            general: [qsTr("Default Folder for Exported File"), qsTr("Pandoc Path"), qsTr("After Export"), qsTr("Open exported file location")],
            pdf: [qsTr("Paper Size"), qsTr("Margin"), qsTr("Width x Height"), qsTr("Theme"), qsTr("Page Break Between Top Headings"),
                qsTr("Header"), qsTr("Footer"), qsTr("Author"), qsTr("Append Extra Content (HTML)")].concat(yaml, after),
            html: [qsTr("Include Outline Sidebar"), qsTr("Theme"), qsTr("Append in <head />"), qsTr("Append in <body />")].concat(yaml, after),
            "html-plain": [qsTr("Append in <head />"), qsTr("Append in <body />")].concat(yaml, after),
            image: [qsTr("Width"), qsTr("Image Quality"), qsTr("Font Size"), qsTr("Use custom font size"), qsTr("Use theme font size"),
                qsTr("Theme")].concat(after),
            docx: pandoc.concat([qsTr("Style Reference")]),
            odt: pandoc.concat([qsTr("Template")]),
            epub: pandoc.concat([qsTr("Custom CSS"), qsTr("Chapter Level in Outline")])
        }
    }

    width: parent ? parent.width : 0

    function opt(key, fallback) {
        return Prefs.exportOption(current, key, fallback)
    }
    function set(key, value) {
        Prefs.setExportOption(current, key, value)
    }
    // What pandoc is run with: Typora's arguments for the format, and the
    // files the options name.
    function pandocArguments(format) {
        const target = format === "mediawiki" ? "mediawiki" : format
        let args = "-f native -s -o ${outputPath} -t " + target
        const o = Prefs.exportOptions[format] || {}
        const quoted = s => JSON.stringify(s)
        if (format === "docx" && (o["reference-doc"] || "").trim() !== "")
            args += " --reference-doc " + quoted(o["reference-doc"])
        if (format === "odt" && (o.template || "").trim() !== "")
            args += " --template " + quoted(o.template)
        if (format === "epub") {
            if ((o.css || "").trim() !== "")
                args += " --css " + quoted(o.css)
            args += " --toc-depth=" + (o["toc-depth"] || 3)
        }
        return args
    }
    // The themes an export can take, the one in use first.
    readonly property var themes: [["", qsTr("Current Theme")]].concat(Theme.themes.map(t => t.split("|")))

    // A group inside the options: its title over its controls.
    component Group: Column {
        id: group
        property string title
        readonly property bool searchHit: Ui.prefsHit(title)
        width: parent ? parent.width : 0
        spacing: 4
        bottomPadding: 12
        Text {
            text: group.title
            color: Theme.foreground
            font.family: Ui.uiFont
            font.pixelSize: 14
            Rectangle {
                visible: group.searchHit
                z: -1
                width: parent.contentWidth
                height: parent.height
                color: Qt.alpha(Theme.mark, 0.48)
            }
        }
    }
    // A line of text with a menu of ready-made values beside it, as
    // Typora's header and footer fields.
    component Presets: Row {
        id: presets
        property string key
        property var choices: []
        property string placeholder
        width: parent ? parent.width : 0
        spacing: 4
        TextField {
            id: presetText
            width: presets.width - presetMenu.width - 4
            height: 28
            text: section.opt(presets.key, "")
            placeholderText: presets.placeholder
            selectByMouse: true
            font.family: Ui.uiFont
            font.pixelSize: 13
            onEditingFinished: section.set(presets.key, text)
        }
        ComboBox {
            id: presetMenu
            width: 36
            height: 28
            displayText: ""
            model: [qsTr("Custom")].concat(presets.choices, [qsTr("None")])
            indicator: Glyph {
                anchors.centerIn: parent
                name: "moreH"
                size: 14
                color: Theme.muted
            }
            popup.width: 260
            onActivated: index => {
                if (index === 0) {
                    presetText.forceActiveFocus()
                    return
                }
                section.set(presets.key, index === count - 1 ? "" : presets.choices[index - 1])
            }
        }
    }
    // A file the format reads, typed in or picked.
    component FileField: TextField {
        id: fileField
        property string key
        width: parent ? parent.width : 0
        height: 28
        rightPadding: 32
        text: section.opt(key, "")
        selectByMouse: true
        font.family: Ui.uiFont
        font.pixelSize: 13
        onEditingFinished: section.set(key, text.trim())
        AbstractButton {
            anchors.right: parent.right
            width: 30
            height: parent.height
            onClicked: Desktop.chooseFile("prefFile:export/" + section.current + "/" + fileField.key, qsTr("Select"), "", "")
            contentItem: Glyph { name: "folder"; size: 14; color: Theme.muted }
            background: null
        }
    }
    // A box of HTML added to the output.
    component Snippet: TextArea {
        property string key
        width: parent ? parent.width : 0
        height: 80
        text: section.opt(key, "")
        wrapMode: TextEdit.NoWrap
        font.family: Ui.monoFont
        font.pixelSize: 12
        selectByMouse: true
        onEditingFinished: section.set(key, text)
        background: Rectangle {
            color: Theme.background
            border.color: parent.activeFocus ? Theme.accent : Theme.border
        }
    }

    Row {
        width: parent.width
        spacing: 8

        // The formats, with the selected one in bold on a pill.
        Column {
            id: list
            width: 160
            Repeater {
                model: section.formats
                delegate: Item {
                    id: entry
                    required property var modelData
                    readonly property string format: modelData[0]
                    readonly property bool searchHit: Ui.prefsHit(modelData[1])
                        || (section.labels[format] || section.labels.docx.slice(0, 6)).some(l => Ui.prefsHit(l))
                    width: list.width
                    height: 20
                    Rectangle {
                        anchors.fill: parent
                        radius: 2
                        color: section.current === entry.format ? Qt.alpha(Theme.foreground, Theme.dark ? 0.12 : 0.07)
                             : entry.searchHit ? Qt.alpha(Theme.mark, 0.48) : "transparent"
                    }
                    Text {
                        x: 8
                        width: parent.width - 16
                        anchors.verticalCenter: parent.verticalCenter
                        elide: Text.ElideRight
                        text: entry.modelData[1]
                        color: Theme.foreground
                        font.family: Ui.uiFont
                        font.pixelSize: 12
                        font.bold: section.current === entry.format
                    }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: section.current = entry.format
                    }
                }
            }
        }

        Rectangle {
            width: parent.width - list.width - 8
            height: detail.height + 16
            radius: 6
            color: Qt.alpha(Theme.foreground, Theme.dark ? 0.08 : 0.05)

            Column {
                id: detail
                x: 16
                y: 8
                width: parent.width - 32

                // General
                Column {
                    width: parent.width
                    visible: section.current === "general"
                    Group {
                        title: qsTr("Default Folder for Exported File")
                        PrefChoice {
                            options: [["", qsTr("Auto")], ["same", qsTr("Same folder with current file")], ["custom", qsTr("Custom location")]]
                            value: Prefs.exportOption("general", "exportFolder", "")
                            onPicked: v => Prefs.setExportOption("general", "exportFolder", v)
                            boxWidth: Math.min(300, detail.width)
                        }
                        TextField {
                            id: exportFolder
                            visible: Prefs.exportOption("general", "exportFolder", "") === "custom"
                            width: Math.min(400, detail.width)
                            height: 28
                            rightPadding: 32
                            text: Prefs.exportOption("general", "customExportPath", "")
                            placeholderText: section.app.editor.path !== "" ? section.app.folderOf(section.app.editor.path) : ""
                            selectByMouse: true
                            font.family: Ui.uiFont
                            font.pixelSize: 13
                            onEditingFinished: Prefs.setExportOption("general", "customExportPath", text.trim())
                            AbstractButton {
                                anchors.right: parent.right
                                width: 30
                                height: parent.height
                                ToolTip.visible: hovered
                                ToolTip.text: qsTr("Select custom folder")
                                onClicked: Desktop.chooseFile("prefFolder:export/general/customExportPath", qsTr("Select custom folder"),
                                                              exportFolder.text, "")
                                contentItem: Glyph { name: "folder"; size: 14; color: Theme.muted }
                                background: null
                            }
                        }
                    }
                    Group {
                        title: qsTr("Pandoc Path")
                        TextField {
                            id: pandocPath
                            width: Math.min(400, detail.width)
                            height: 28
                            rightPadding: 32
                            text: Prefs.pandocPath
                            placeholderText: qsTr("(Auto Detect)")
                            selectByMouse: true
                            font.family: Ui.uiFont
                            font.pixelSize: 13
                            onEditingFinished: Prefs.pandocPath = text.trim()
                            AbstractButton {
                                anchors.right: parent.right
                                width: 30
                                height: parent.height
                                onClicked: Desktop.chooseFile("prefFile:pandocPath", qsTr("Select"), "", "")
                                contentItem: Glyph { name: "folder"; size: 14; color: Theme.muted }
                                background: null
                            }
                        }
                    }
                    Group {
                        title: qsTr("After Export")
                        PrefCheck {
                            text: qsTr("Open exported file location")
                            checked: Prefs.exportOption("general", "openExportLocation", false)
                            onToggled: Prefs.setExportOption("general", "openExportLocation", checked)
                        }
                    }
                }

                // PDF
                Column {
                    width: parent.width
                    visible: section.current === "pdf"
                    Group {
                        title: qsTr("Paper Size")
                        PrefChoice {
                            options: [["A5", "A5"], ["A4", "A4"], ["A3", "A3"], ["Letter", qsTr("US Letter")], ["Legal", qsTr("US Legal")],
                                ["Tabloid", qsTr("Tabloid")], ["custom", qsTr("Custom")]]
                            value: Prefs.exportOption("pdf", "paper", "A4")
                            onPicked: v => Prefs.setExportOption("pdf", "paper", v)
                            boxWidth: 200
                        }
                        Row {
                            visible: Prefs.exportOption("pdf", "paper", "A4") === "custom"
                            spacing: 6
                            Text {
                                width: 110
                                height: 28
                                verticalAlignment: Text.AlignVCenter
                                text: qsTr("Width x Height")
                                color: Theme.foreground
                                font.family: Ui.uiFont
                                font.pixelSize: 13
                            }
                            PrefField {
                                numeric: true; from: 20; to: 2000; boxWidth: 64; suffix: "×"
                                input.text: Prefs.exportOption("pdf", "paperWidth", 210)
                                onEdited: v => Prefs.setExportOption("pdf", "paperWidth", v)
                            }
                            PrefField {
                                numeric: true; from: 20; to: 2000; boxWidth: 64; suffix: "mm"
                                input.text: Prefs.exportOption("pdf", "paperHeight", 297)
                                onEdited: v => Prefs.setExportOption("pdf", "paperHeight", v)
                            }
                        }
                        Row {
                            spacing: 6
                            Text {
                                width: 110
                                height: 28
                                verticalAlignment: Text.AlignVCenter
                                text: qsTr("Margin")
                                color: Theme.foreground
                                font.family: Ui.uiFont
                                font.pixelSize: 13
                            }
                            PrefChoice {
                                options: [[false, qsTr("Default")], [true, qsTr("Custom")]]
                                value: Prefs.exportOption("pdf", "customMargin", false)
                                onPicked: v => Prefs.setExportOption("pdf", "customMargin", v)
                                boxWidth: 140
                            }
                        }
                        Grid {
                            visible: Prefs.exportOption("pdf", "customMargin", false)
                            columns: 2
                            columnSpacing: 16
                            rowSpacing: 4
                            Repeater {
                                model: [["marginTop", qsTr("Margin Top")], ["marginBottom", qsTr("Margin Bottom")],
                                    ["marginLeft", qsTr("Margin Left")], ["marginRight", qsTr("Margin Right")]]
                                PrefField {
                                    required property var modelData
                                    label: modelData[1]
                                    labelWidth: 110
                                    numeric: true; from: 0; to: 200; boxWidth: 56; suffix: "mm"
                                    input.text: Prefs.exportOption("pdf", modelData[0], 16)
                                    onEdited: v => Prefs.setExportOption("pdf", modelData[0], v)
                                }
                            }
                        }
                    }
                    Group {
                        title: qsTr("Theme")
                        PrefChoice {
                            options: section.themes
                            value: Prefs.exportOption("pdf", "exportTheme", "")
                            onPicked: v => Prefs.setExportOption("pdf", "exportTheme", v)
                            hint: qsTr("Change this option to specify a separated theme for export.")
                            boxWidth: 240
                        }
                    }
                    Group {
                        title: qsTr("Page Break")
                        PrefCheck {
                            text: qsTr("Page Break Between Top Headings")
                            checked: Prefs.exportOption("pdf", "splitPages", false)
                            onToggled: Prefs.setExportOption("pdf", "splitPages", checked)
                        }
                    }
                    Group {
                        title: qsTr("Header")
                        Presets {
                            key: "header"
                            placeholder: qsTr("None")
                            choices: ["${title}", "${author}", "${pageNo}", "${today}"]
                        }
                    }
                    Group {
                        title: qsTr("Footer")
                        Presets {
                            key: "footer"
                            placeholder: qsTr("None")
                            choices: ["${title}", "No. ${pageNo} / ${totalPages}", "Written in typo", "CONFIDENTIAL"]
                        }
                    }
                    Group {
                        title: qsTr("Author")
                        TextField {
                            width: parent.width
                            height: 28
                            text: Prefs.exportOption("pdf", "author", "")
                            placeholderText: qsTr("Empty")
                            selectByMouse: true
                            font.family: Ui.uiFont
                            font.pixelSize: 13
                            onEditingFinished: Prefs.setExportOption("pdf", "author", text)
                        }
                    }
                    Group {
                        title: qsTr("Append Extra Content (HTML)")
                        Snippet { key: "appendBody" }
                    }
                }

                // HTML
                Column {
                    width: parent.width
                    visible: section.current === "html"
                    Group {
                        title: qsTr("Outline")
                        PrefCheck {
                            text: qsTr("Include Outline Sidebar")
                            checked: Prefs.exportOption("html", "sidebar", false)
                            onToggled: Prefs.setExportOption("html", "sidebar", checked)
                        }
                    }
                    Group {
                        title: qsTr("Theme")
                        PrefChoice {
                            options: section.themes
                            value: Prefs.exportOption("html", "exportTheme", "")
                            onPicked: v => Prefs.setExportOption("html", "exportTheme", v)
                            hint: qsTr("Change this option to specify a separated theme for export.")
                            boxWidth: 240
                        }
                    }
                }

                // HTML and HTML without styles: what goes in <head> and <body>.
                Column {
                    width: parent.width
                    visible: section.current === "html" || section.current === "html-plain"
                    Group {
                        title: qsTr("Append in <head />")
                        Snippet { key: "appendHead" }
                    }
                    Group {
                        title: qsTr("Append in <body />")
                        Snippet { key: "appendBody" }
                    }
                }

                // PDF and HTML: what the front matter may change.
                Group {
                    visible: ["pdf", "html", "html-plain"].includes(section.current)
                    title: qsTr("General")
                    PrefCheck {
                        text: qsTr("Read and overwrite export settings from YAML front matters")
                        checked: section.opt("allowPerFileSetting", false)
                        onToggled: section.set("allowPerFileSetting", checked)
                    }
                }

                // Image
                Column {
                    width: parent.width
                    visible: section.current === "image"
                    Group {
                        title: qsTr("Width")
                        Row {
                            spacing: 8
                            PrefField {
                                numeric: true; from: 100; to: 10000; boxWidth: 70; suffix: "px"
                                input.text: Prefs.exportOption("image", "imageWidth", 640)
                                onEdited: v => Prefs.setExportOption("image", "imageWidth", v)
                            }
                            PrefButton {
                                text: qsTr("Reset to Default")
                                onClicked: Prefs.setExportOption("image", "imageWidth", 640)
                            }
                        }
                    }
                    Group {
                        title: qsTr("Image Quality")
                        PrefChoice {
                            options: [[0, qsTr("Auto")], [1, qsTr("Medium")], [2, qsTr("High")], [3, qsTr("Best")]]
                            value: Prefs.exportOption("image", "quality", 0)
                            onPicked: v => Prefs.setExportOption("image", "quality", v)
                            boxWidth: 160
                        }
                    }
                    Group {
                        title: qsTr("Font Size")
                        Row {
                            spacing: 8
                            RadioButton {
                                text: qsTr("Use custom font size")
                                checked: !Prefs.exportOption("image", "useThemeFontSize", false)
                                onClicked: Prefs.setExportOption("image", "useThemeFontSize", false)
                                font.family: Ui.uiFont
                                font.pixelSize: 13
                            }
                            RadioButton {
                                text: qsTr("Use theme font size")
                                checked: Prefs.exportOption("image", "useThemeFontSize", false)
                                onClicked: Prefs.setExportOption("image", "useThemeFontSize", true)
                                font.family: Ui.uiFont
                                font.pixelSize: 13
                            }
                        }
                        Row {
                            visible: !Prefs.exportOption("image", "useThemeFontSize", false)
                            spacing: 8
                            PrefField {
                                numeric: true; from: 8; to: 96; boxWidth: 56; suffix: "px"
                                input.text: Prefs.exportOption("image", "imageFontSize", 24)
                                onEdited: v => Prefs.setExportOption("image", "imageFontSize", v)
                            }
                            PrefButton {
                                text: qsTr("Reset to Default")
                                onClicked: Prefs.setExportOption("image", "imageFontSize", 24)
                            }
                        }
                    }
                    Group {
                        title: qsTr("Theme")
                        PrefChoice {
                            options: section.themes
                            value: Prefs.exportOption("image", "exportTheme", "")
                            onPicked: v => Prefs.setExportOption("image", "exportTheme", v)
                            hint: qsTr("Change this option to specify a separated theme for export.")
                            boxWidth: 240
                        }
                    }
                }

                // The formats pandoc writes.
                Column {
                    width: parent.width
                    visible: section.pandoc
                    Group {
                        title: qsTr("Arguments")
                        TextField {
                            width: parent.width
                            height: 28
                            enabled: false
                            text: section.pandocArguments(section.current)
                            font.family: Ui.monoFont
                            font.pixelSize: 12
                        }
                    }
                    Group {
                        title: qsTr("Extra Arguments")
                        TextField {
                            width: parent.width
                            height: 28
                            text: section.opt("extraArguments", "")
                            selectByMouse: true
                            font.family: Ui.monoFont
                            font.pixelSize: 12
                            onEditingFinished: section.set("extraArguments", text.trim())
                        }
                    }
                    Group {
                        visible: section.current === "docx"
                        title: qsTr("Style Reference")
                        FileField { key: "reference-doc" }
                    }
                    Group {
                        visible: section.current === "odt"
                        title: qsTr("Template")
                        FileField { key: "template" }
                    }
                    Group {
                        visible: section.current === "epub"
                        title: qsTr("Custom CSS")
                        FileField { key: "css" }
                    }
                    Group {
                        visible: section.current === "epub"
                        title: qsTr("Chapter Level in Outline")
                        PrefChoice {
                            options: [[1, "1"], [2, "2"], [3, "3"], [4, "4"], [5, "5"], [6, "6"]]
                            value: Prefs.exportOption("epub", "toc-depth", 3)
                            onPicked: v => Prefs.setExportOption("epub", "toc-depth", v)
                            boxWidth: 80
                        }
                    }
                }

                // What happens once a format is written.
                Group {
                    visible: section.current !== "general"
                    title: qsTr("After Export")
                    PrefCheck {
                        text: qsTr("Open exported file location")
                        checked: section.opt("openExportLocation", Prefs.exportOption("general", "openExportLocation", false))
                        onToggled: section.set("openExportLocation", checked)
                    }
                    PrefCheck {
                        text: qsTr("Open exported file")
                        checked: section.opt("openExportFile", false)
                        onToggled: section.set("openExportFile", checked)
                    }
                    PrefCheck {
                        text: qsTr("Run command")
                        checked: section.opt("runCommand", false)
                        onToggled: section.set("runCommand", checked)
                    }
                    Presets {
                        visible: section.opt("runCommand", false)
                        key: "runCommandStr"
                        placeholder: qsTr("Command")
                        choices: ['xclip -selection clipboard -i "${outputPath}"', 'git add "${outputPath}"']
                    }
                    PrefCheck {
                        visible: section.opt("runCommand", false)
                        text: qsTr("Show command output")
                        checked: section.opt("showOutput", false)
                        onToggled: section.set("showOutput", checked)
                    }
                }
            }
        }
    }
}
