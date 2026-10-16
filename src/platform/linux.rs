//! Linux, as Omarchy has it: dialogs and printing through
//! xdg-desktop-portal, the file manager and Quit on the session bus, and
//! processes in `/proc`.

mod bus;
mod portal;

use std::fs;
use std::path::{Path, PathBuf};

pub use bus::{broadcast_quit, on_quit};
use portal::Pick;

/// Ask the user for the file (or folder, or images) a `Desktop.chooseFile`
/// kind wants: what they chose, nothing when they cancel. It blocks until
/// they answer, so it runs off the GUI thread.
pub fn choose(kind: &str, title: &str, folder: Option<&Path>, name: &str, export: Option<&[(&str, &[&str])]>) -> Result<Vec<PathBuf>, String> {
    let what = match kind {
        _ if export.is_some() => Pick::Export { folder, name, filters: export.unwrap_or_default() },
        "save" => Pick::Save { folder, name },
        "folder" | "imagesCopy" | "imagesMove" | "imageFolder" | "imageRoot" => Pick::Folder { folder },
        // A folder or a file for a preference, named after the colon.
        k if k.starts_with("prefFolder:") => Pick::Folder { folder },
        k if k.starts_with("prefFile:") => Pick::Any { folder },
        "images" => Pick::Images { folder },
        "import" => Pick::Import { folder },
        "imageCopy" | "imageMove" => Pick::SaveImage { folder, name },
        _ => Pick::Open { folder },
    };
    portal::pick(title, what)
}

/// Show `path` selected in the file manager, or else open its folder.
pub fn reveal(path: &Path) {
    if bus::show_item(path).is_err() {
        if let Some(folder) = path.parent() {
            let _ = std::process::Command::new("xdg-open").arg(folder).spawn();
        }
    }
}

/// Whether process `pid` is a running typo: the same program as this one,
/// so a reused id isn't taken for a window.
pub fn alive(pid: u32) -> bool {
    let name = |p: &str| fs::read_to_string(format!("/proc/{p}/comm")).ok();
    name(&pid.to_string()).is_some_and(|n| Some(n) == name("self"))
}

/// Print what `render` typesets on the page the print dialog sets up,
/// starting from `page` (its size and margins in millimetres). `path` is
/// where the PDF waits while the dialog takes it.
pub fn print(
    title: &str,
    path: &Path,
    page: ((f64, f64), [f64; 4]),
    render: impl FnOnce(((f64, f64), [f64; 4])) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    let setup = portal::PageSetup { width: page.0.0, height: page.0.1, margins: page.1, landscape: false };
    let Some((setup, token)) = portal::prepare_print(title, &setup)? else { return Ok(()) };
    let bytes = render(((setup.width, setup.height), setup.margins))?;
    fs::write(path, &bytes).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    // The portal reads the open file; the name can go at once.
    let file = fs::File::open(path).map_err(|e| e.to_string());
    let _ = fs::remove_file(path);
    portal::print(title, token, &file?).map(|_| ())
}
