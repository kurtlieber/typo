//! What differs between the desktops typo runs on: file dialogs, printing,
//! revealing a file, telling whether a window's process runs, and Quit
//! reaching every window. Linux (Omarchy) does these through the
//! freedesktop portals, the session bus and `/proc`; macOS through its own
//! tools. No file outside this folder names either.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(test)]
mod tests {
    #[test]
    fn this_process_is_alive() {
        assert!(super::alive(std::process::id()));
        assert!(!super::alive(u32::MAX));
    }
}
