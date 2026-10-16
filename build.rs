use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

fn main() {
    let qml = [
        "qml/Main.qml",
        "qml/MacDialogs.qml",
        "qml/AppMenuBar.qml",
        "qml/TMenu.qml",
        "qml/TMenuItem.qml",
        "qml/TSep.qml",
        "qml/Glyph.qml",
        "qml/ContextMenu.qml",
        "qml/TDialog.qml",
        "qml/TableDialog.qml",
        "qml/EditorView.qml",
        "qml/BlockDelegate.qml",
        "qml/MathView.qml",
        "qml/DiagramView.qml",
        "qml/InlineObject.qml",
        "qml/FootnoteTip.qml",
        "qml/EmojiSuggest.qml",
        "qml/TableBlock.qml",
        "qml/Caret.qml",
        "qml/Footer.qml",
        "qml/Sidebar.qml",
        "qml/FileList.qml",
        "qml/FileTree.qml",
        "qml/RenameField.qml",
        "qml/FolderSearch.qml",
        "qml/OutlineView.qml",
        "qml/QuickOpen.qml",
        "qml/FindBar.qml",
        "qml/Notice.qml",
        "qml/ZoomHint.qml",
        "qml/SourceView.qml",
        "qml/PrefsPanel.qml",
        "qml/PrefGroup.qml",
        "qml/PrefCheck.qml",
        "qml/PrefHint.qml",
        "qml/PrefChoice.qml",
        "qml/PrefButton.qml",
        "qml/PrefField.qml",
        "qml/PrefsFiles.qml",
        "qml/PrefsEditor.qml",
        "qml/PrefsImage.qml",
        "qml/PrefsMarkdown.qml",
        "qml/PrefsExport.qml",
        "qml/PrefsAppearance.qml",
        "qml/PrefsGeneral.qml",
    ];
    let singletons = ["qml/Ui.qml", "qml/Prefs.qml"];
    let files: Vec<QmlFile> = qml
        .into_iter()
        .map(QmlFile::from)
        .chain(singletons.into_iter().map(|f| QmlFile::from(f).singleton(true)))
        .collect();
    CxxQtBuilder::new_qml_module(QmlModule::new("Typo").qml_files(files))
        .qt_module("Gui")
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .qt_module("Test")
        .include_dir("cpp")
        .cpp_file("cpp/typo_glue.h")
        .cpp_file("cpp/typo_math.cpp")
        .files(["src/qt/desktop.rs", "src/qt/editor.rs", "src/qt/folder.rs", "src/qt/glue.rs", "src/qt/highlighter.rs", "src/qt/theme.rs"])
        .build();
}
