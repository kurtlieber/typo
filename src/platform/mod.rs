//! What differs between the desktops typo runs on: file dialogs, printing,
//! revealing a file, telling whether a window's process runs, and Quit
//! reaching every window. Linux (Omarchy) does these through the
//! freedesktop portals, the session bus and `/proc`; macOS through its own
//! tools. Only this folder names either's APIs.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

/// What the open dialogs offer, Markdown first, as Typora's own do.
const MARKDOWN: &[&str] = &["md", "markdown", "mdown", "mmd", "mkd", "mkdn", "text", "txt"];
/// The pictures Typora's Insert Local Images… offers.
const IMAGES: &[&str] = &["png", "jpg", "jpeg", "jfif", "bmp", "svg", "tiff", "webp", "gif"];
/// What File → Import… offers: what pandoc reads.
const IMPORT: &[&str] = &[
    "docx", "odt", "rtf", "epub", "html", "htm", "tex", "latex", "rst", "textile", "wiki", "mediawiki", "opml", "org", "ipynb", "fb2",
    "dbk", "xml", "csv", "tsv", "typ", "md",
];

#[cfg(test)]
mod tests {
    #[test]
    fn this_process_is_alive() {
        assert!(super::alive(std::process::id()));
        assert!(!super::alive(u32::MAX));
    }
}
