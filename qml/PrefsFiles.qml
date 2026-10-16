import QtQuick
import QtQuick.Controls
import Typo

// Preferences ▸ Files.
Column {
    id: section

    required property var app
    readonly property var extensions: ["md", "markdown", "text"]

    width: parent ? parent.width : 0

    PrefGroup {
        title: qsTr("On Launch")
        PrefChoice {
            key: "launchAction"
            options: [[0, qsTr("Open new file")], [1, qsTr("Restore last closed folders")],
                [2, qsTr("Restore last closed files and folders")], [3, qsTr("Open custom folder")]]
            boxWidth: 300
        }
        PrefField {
            visible: Prefs.launchAction === 3
            key: "launchFolder"
            placeholder: qsTr("Select custom folder")
            boxWidth: 300
            input.rightPadding: 34
            AbstractButton {
                parent: launchFolder.input
                anchors.right: parent.right
                width: 30
                height: parent.height
                onClicked: Desktop.chooseFile("prefFolder:launchFolder", qsTr("Select custom folder"), Prefs.launchFolder, "")
                contentItem: Glyph { name: "folder"; size: 14; color: Theme.muted }
                background: null
            }
            id: launchFolder
        }
    }

    PrefGroup {
        title: qsTr("Outline")
        PrefCheck {
            text: qsTr("Collapsible Outline on Left Panel")
            target: Ui.object
            key: "outlineCollapsible"
        }
    }

    PrefGroup {
        title: qsTr("Default File Type / Extension")
        PrefChoice {
            options: [["md", qsTr("Markdown (.md)")], ["markdown", qsTr("Markdown (.markdown)")],
                ["text", qsTr("Text File (.text)")], ["custom", qsTr("Custom")]]
            value: section.extensions.indexOf(Prefs.defaultExtension) >= 0 ? Prefs.defaultExtension : "custom"
            onPicked: v => {
                if (v !== "custom")
                    Prefs.defaultExtension = v
                else
                    customExtension.input.forceActiveFocus()
            }
            boxWidth: 300
        }
        PrefField {
            id: customExtension
            visible: section.extensions.indexOf(Prefs.defaultExtension) < 0 || input.activeFocus
            placeholder: qsTr("e.g: %1").arg("txt")
            input.text: section.extensions.indexOf(Prefs.defaultExtension) < 0 ? Prefs.defaultExtension : ""
            boxWidth: 300
            onEdited: v => {
                const ext = ((/^\.?[a-z0-9_-]+(\.[a-z0-9_-]+)*/.exec(String(v).toLowerCase().trim()) || [])[0] || "").replace(/^\./, "")
                Prefs.defaultExtension = ext === "" ? "md" : ext
            }
        }
        Text {
            visible: customExtension.visible
            width: parent.width
            wrapMode: Text.Wrap
            text: qsTr("Following extensions are supported: %1")
                .arg("md, markdown, mmd, mdown, mkd, mdwn, mdtxt, mdtext, txt, text, rmd, rmarkdown.")
            color: Theme.muted
            font.family: Ui.uiFont
            font.pixelSize: 12
            topPadding: 2
        }
    }

    PrefGroup {
        title: qsTr("Save & Recover")
        PrefCheck {
            text: qsTr("Auto Save")
            key: "autoSave"
        }
        PrefCheck {
            text: qsTr("Save without asking when\nswitch files on side panel")
            key: "saveOnSwitch"
            // Auto save already does.
            checked: Prefs.saveOnSwitch || Prefs.autoSave
            enabled: !Prefs.autoSave
        }
        PrefButton {
            text: qsTr("Recover Unsaved Drafts")
            onClicked: section.app.openDrafts()
        }
    }

    PrefGroup {
        title: qsTr("Recent Locations")
        PrefCheck {
            text: qsTr("Record recent files and folders")
            key: "recordRecent"
        }
        PrefButton {
            text: qsTr("Clear Recent Folders and Files")
            onClicked: section.app.ask("", qsTr("Clear recent folders and files?"), [qsTr("Clear"), qsTr("Cancel")],
                                       i => { if (i === 0) Prefs.clearRecent() })
        }
    }

    PrefGroup {
        title: qsTr("When drop file / folder into Typora").replace("Typora", "typo")
        PrefChoice {
            label: qsTr("When drop folder")
            labelWidth: 280
            key: "dropFolder"
            options: [[0, qsTr("Open in Typora").replace("Typora", "typo")], [1, qsTr("Insert Folder Link")]]
            boxWidth: 200
        }
        PrefChoice {
            label: qsTr("When drop markdown file")
            labelWidth: 280
            key: "dropMarkdown"
            options: [[0, qsTr("Open in Typora").replace("Typora", "typo")], [1, qsTr("Insert File Link")]]
            boxWidth: 200
        }
        PrefChoice {
            label: qsTr("When drop files that can be imported")
            labelWidth: 280
            key: "dropImportable"
            options: [[0, qsTr("Import File")], [1, qsTr("Insert File Link")]]
            boxWidth: 200
        }
    }

    PrefGroup {
        title: qsTr("File Tree / Library")
        hint: qsTr("Choose which files appear in the file tree and file library.")
        PrefCheck {
            text: qsTr("Show hidden folders / files")
            key: "showHiddenFiles"
        }
        Repeater {
            model: [
                { label: qsTr("File Tree"), mode: "treeFilter", patterns: "treePatterns", title: qsTr("File Tree — Custom Patterns") },
                { label: qsTr("File Library"), mode: "libraryFilter", patterns: "libraryPatterns", title: qsTr("File Library — Custom Patterns") }
            ]
            delegate: Row {
                id: filterRow
                required property var modelData
                spacing: 8
                readonly property bool searchHit: Ui.prefsHit(modelData.label)
                PrefChoice {
                    label: filterRow.modelData.label
                    labelWidth: 140
                    key: filterRow.modelData.mode
                    options: [["supported", qsTr("Supported Files (Markdown & Plain Text)")], ["all", qsTr("All Files")],
                        ["custom", qsTr("Custom")]]
                    boxWidth: 300
                    onPicked: v => {
                        if (v === "custom")
                            patterns.edit(filterRow.modelData)
                    }
                }
                PrefButton {
                    visible: Prefs[filterRow.modelData.mode] === "custom"
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Configure…")
                    onClicked: patterns.edit(filterRow.modelData)
                }
            }
        }
    }

    // The custom patterns of the file tree or library: one glob a line,
    // "!" in front to leave matches out.
    Popup {
        id: patterns
        property var spec: null
        function edit(spec) {
            patterns.spec = spec
            text.text = Prefs[spec.patterns]
            open()
            text.forceActiveFocus()
        }
        parent: Overlay.overlay
        anchors.centerIn: parent
        width: Math.min(520, parent.width - 32)
        modal: true
        focus: true
        closePolicy: Popup.CloseOnEscape
        padding: 18
        Overlay.modal: Rectangle { color: Qt.rgba(0, 0, 0, 0.35) }
        background: Rectangle {
            color: Theme.panel
            border.color: Theme.border
            radius: 6
        }
        contentItem: Column {
            spacing: 10
            Text {
                text: patterns.spec ? patterns.spec.title : ""
                color: Theme.foreground
                font.family: Ui.uiFont
                font.pixelSize: 15
                font.bold: true
            }
            Text {
                width: parent.width
                wrapMode: Text.Wrap
                text: qsTr("Add glob patterns to choose which files are shown. Prefix a pattern with \"!\" to exclude. Example: *.csv or !**/draft-*.md")
                color: Theme.foreground
                font.family: Ui.uiFont
                font.pixelSize: 13
            }
            Text {
                width: parent.width
                wrapMode: Text.Wrap
                text: qsTr("Markdown and text files") + " — " + qsTr("Included by default. To exclude some formats, use a \"!\" pattern.")
                color: Theme.muted
                font.family: Ui.uiFont
                font.pixelSize: 12
            }
            ScrollView {
                width: parent.width
                height: 140
                TextArea {
                    id: text
                    font.family: Ui.monoFont
                    font.pixelSize: 13
                    placeholderText: "*.csv"
                    background: Rectangle {
                        color: Theme.background
                        border.color: text.activeFocus ? Theme.accent : Theme.border
                    }
                }
            }
            Row {
                anchors.right: parent.right
                spacing: 8
                PrefButton {
                    text: qsTr("Cancel")
                    onClicked: patterns.close()
                }
                PrefButton {
                    text: qsTr("OK")
                    onClicked: {
                        Prefs[patterns.spec.patterns] = text.text.split("\n").map(s => s.trim()).filter(s => s !== "").join("\n")
                        patterns.close()
                    }
                }
            }
        }
    }
}
