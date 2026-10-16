import QtQuick
import QtQuick.Controls
import Typo

// Preferences ▸ Image. Uploading isn't something typo does, so Typora's
// "Upload image" and its online-image rules are left out.
Column {
    id: section

    required property var app

    width: parent ? parent.width : 0

    // Typora's checks on the custom folder: one is needed, not the root,
    // and an absolute one must be there.
    function folderError(path) {
        const p = String(path).trim()
        if (p === "")
            return qsTr("Please input a folder path.")
        if (p === "/")
            return qsTr("Cannot use root path.")
        if (p.startsWith("/") && !Desktop.isFolder(p))
            return qsTr("Folder not exist at %1, please input another path.").arg(p)
        return ""
    }

    PrefGroup {
        title: qsTr("When Insert…")
        groupId: "image-setting-group"
        PrefChoice {
            key: "imageInsertAction"
            options: [[0, qsTr("No special action")], [1, qsTr("Copy image to current folder (./)")],
                [2, qsTr("Copy image to ./assets")], [3, qsTr("Copy image to ./${filename}.assets")],
                [4, qsTr("Copy image to custom folder")]]
            boxWidth: 300
            onPicked: v => {
                if (v === 4) {
                    if (Prefs.imageFolder.trim() === "")
                        Prefs.imageFolder = "./${filename}"
                    folder.input.forceActiveFocus()
                }
            }
        }
        PrefField {
            id: folder
            visible: Prefs.imageInsertAction === 4
            key: "imageFolder"
            boxWidth: Math.min(420, parent.width - 70)
            input.rightPadding: 58
            property string error: ""
            onEdited: v => error = section.folderError(v)
            AbstractButton {
                parent: folder.input
                anchors.right: parent.right
                anchors.rightMargin: 28
                width: 28
                height: parent.height
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Select custom folder")
                onClicked: Desktop.chooseFile("prefFolder:imageFolder", qsTr("Select custom folder"),
                                              Prefs.imageFolder.startsWith("/") ? Prefs.imageFolder : "", "")
                contentItem: Glyph { name: "folder"; size: 14; color: Theme.muted }
                background: null
            }
            AbstractButton {
                parent: folder.input
                anchors.right: parent.right
                width: 28
                height: parent.height
                ToolTip.visible: hovered
                ToolTip.text: qsTr("Reset to Default")
                onClicked: {
                    folder.error = ""
                    Prefs.imageFolder = "./${filename}"
                }
                contentItem: Glyph { name: "close"; size: 12; color: Theme.muted }
                background: null
            }
        }
        Text {
            visible: folder.visible && folder.error !== ""
            width: parent.width - 16
            wrapMode: Text.Wrap
            text: folder.error
            color: Theme.error
            font.family: Ui.uiFont
            font.pixelSize: 12
            topPadding: 4
        }
        Text {
            visible: folder.visible
            width: parent.width - 16
            wrapMode: Text.Wrap
            text: qsTr("Please specify a relative path to current folder which begins with './' or '../', or an absolute folder path. (`${filename}` representing for currents filename.)")
            color: Theme.muted
            font.family: Ui.uiFont
            font.pixelSize: 12
            lineHeight: 1.2
            topPadding: 4
            bottomPadding: 6
        }
        PrefCheck {
            text: qsTr("Apply above rules to local images")
            key: "imageCopyLocal"
        }
    }

    PrefGroup {
        title: qsTr("Preferred Image Syntax")
        PrefCheck {
            text: qsTr("Use relative path if possible")
            key: "imageRelative"
        }
        PrefCheck {
            text: qsTr("Add ./ for relative path")
            key: "imageWithDot"
            hint: qsTr("When enabled, typo will use pattern like ./assets/image.png instead of assets/image.png when generating relative path for inserted images.")
        }
        PrefCheck {
            text: qsTr("Auto escape image URL when insert")
            key: "imageEscape"
            hint: qsTr("For example, if you insert a file at 'img/example file.png', it will become 'img/example%20file.png'. Enable this for better compatibility with other Markdown engines, or disable this for better readability.")
        }
    }
}
