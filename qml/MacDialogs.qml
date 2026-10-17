// The Mac's own open, save and folder panels, which Desktop.chooseFile asks
// for with dialogWanted (they must open on the GUI thread). The answer goes
// back as Desktop.fileChosen, as the portal's does on Linux.
import QtQuick
import QtQuick.Dialogs
import Typo

Item {
    id: root
    property string kind: ""

    function answer(urls) {
        Desktop.fileChosen(kind, urls.map(u => decodeURIComponent(String(u).substring(7))).join("\n"), "")
    }

    FileDialog {
        id: fileDialog
        onAccepted: answer(selectedFiles)
        onRejected: Desktop.fileChosen(kind, "", "")
    }
    FolderDialog {
        id: folderDialog
        onAccepted: answer([selectedFolder])
        onRejected: Desktop.fileChosen(kind, "", "")
    }

    Connections {
        target: Desktop
        function onDialogWanted(kind, title, mode, folder, file, filters) {
            root.kind = kind
            if (mode === "folder") {
                folderDialog.title = title
                if (folder !== "")
                    folderDialog.currentFolder = folder
                folderDialog.open()
                return
            }
            fileDialog.title = title
            // The portal's labels; empty is the panel's own Open or Save.
            fileDialog.acceptLabel = { images: qsTr("Select"), import: qsTr("Import") }[kind] || ""
            fileDialog.fileMode = mode === "save" ? FileDialog.SaveFile : mode === "openMany" ? FileDialog.OpenFiles : FileDialog.OpenFile
            fileDialog.nameFilters = filters === "" ? [] : filters.split("\n")
            if (folder !== "")
                fileDialog.currentFolder = folder
            if (mode === "save" && file !== "")
                fileDialog.selectedFile = file
            fileDialog.open()
        }
    }
}
