//! macOS: the system's open and save panels from QML, Finder and Preview
//! through `open`, and Quit as a signal to each typo process.

use std::fs;
use std::io::Read;
use std::os::fd::IntoRawFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};

use super::{IMAGES, IMPORT, MARKDOWN};

/// How the panel for a `Desktop.chooseFile` kind opens: "open",
/// "openMany", "save" or "folder", and its Qt name filters. The panels must
/// open on the GUI thread, so QML opens them.
pub fn dialog(kind: &str, export: Option<&[(&str, &[&str])]>) -> (&'static str, Vec<String>) {
    let filter = |name: &str, exts: &[&str]| format!("{name} ({})", exts.iter().map(|e| format!("*.{e}")).collect::<Vec<_>>().join(" "));
    let any = || "All Files (*)".to_string();
    match kind {
        _ if export.is_some() => ("save", export.unwrap_or_default().iter().map(|(n, e)| filter(n, e)).collect()),
        "save" => ("save", vec![filter("Markdown", MARKDOWN), any()]),
        "folder" | "imagesCopy" | "imagesMove" | "imageFolder" | "imageRoot" => ("folder", Vec::new()),
        // A folder or a file for a preference, named after the colon.
        k if k.starts_with("prefFolder:") => ("folder", Vec::new()),
        k if k.starts_with("prefFile:") => ("open", Vec::new()),
        "images" => ("openMany", vec![filter("Images", IMAGES)]),
        "import" => ("open", vec![filter("Supported Files", IMPORT), any()]),
        "imageCopy" | "imageMove" => ("save", Vec::new()),
        _ => ("open", vec![filter("Markdown", MARKDOWN), any()]),
    }
}

/// Show `path` selected in Finder.
pub fn reveal(path: &Path) {
    let _ = Command::new("open").arg("-R").arg(path).spawn();
}

/// Whether process `pid` is a running typo: the same program as this one,
/// so a reused id isn't taken for a window.
pub fn alive(pid: u32) -> bool {
    let path = |pid: i32| {
        let mut buf = [0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
        let n = unsafe { libc::proc_pidpath(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
        (n > 0).then(|| buf[..n as usize].to_vec())
    };
    // Not 0 or negative, which name groups of processes.
    i32::try_from(pid).is_ok_and(|p| p > 0 && path(p).is_some_and(|p| Some(p) == path(std::process::id() as i32)))
}

/// Ask every window in the group, this one included, to close: each
/// process with a file in the session folder, once however many windows it
/// has.
pub fn broadcast_quit() -> Result<(), String> {
    let mut pids: Vec<u32> = crate::session::entries(&crate::session::dir()).into_iter().map(|(pid, _)| pid).collect();
    pids.push(std::process::id());
    pids.sort_unstable();
    pids.dedup();
    for pid in pids.into_iter().filter(|p| alive(*p)) {
        unsafe { libc::kill(pid as i32, libc::SIGUSR1) };
    }
    Ok(())
}

/// Where the SIGUSR1 handler writes: a socket a thread reads.
static QUIT: AtomicI32 = AtomicI32::new(-1);

extern "C" fn quit_signalled(_: libc::c_int) {
    // Only what is safe in a signal handler: one write.
    let fd = QUIT.load(Ordering::Relaxed);
    if fd >= 0 {
        unsafe { libc::write(fd, [0u8].as_ptr().cast(), 1) };
    }
}

/// Each window's `on_quit`. ponytail: a closed window's stays, and only
/// fails to reach it; prune them if windows ever number in the thousands.
static LISTENERS: Mutex<Vec<Box<dyn Fn() + Send>>> = Mutex::new(Vec::new());

/// Call `quit` on a background thread whenever the group is asked to quit;
/// each window of the process calls it.
pub fn on_quit(quit: impl Fn() + Send + 'static) {
    let mut listeners = LISTENERS.lock().unwrap_or_else(|e| e.into_inner());
    listeners.push(Box::new(quit));
    if listeners.len() > 1 {
        return;
    }
    let Ok((mut rx, tx)) = UnixStream::pair() else { return };
    QUIT.store(tx.into_raw_fd(), Ordering::Relaxed);
    let handler: extern "C" fn(libc::c_int) = quit_signalled;
    unsafe { libc::signal(libc::SIGUSR1, handler as libc::sighandler_t) };
    std::thread::spawn(move || {
        let mut byte = [0u8];
        while rx.read_exact(&mut byte).is_ok() {
            for quit in LISTENERS.lock().unwrap_or_else(|e| e.into_inner()).iter() {
                quit();
            }
        }
    });
}

/// Typeset the PDF on `page` (its size and margins in millimetres) and
/// open it in Preview, which prints it. ponytail: Preview stands in for the
/// print panel; NSPrintOperation would need an AppKit view to print from.
pub fn print(
    _title: &str,
    path: &Path,
    page: ((f64, f64), [f64; 4]),
    render: impl FnOnce(((f64, f64), [f64; 4])) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    let bytes = render(page)?;
    fs::write(path, &bytes).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    let status = Command::new("open").args(["-a", "Preview"]).arg(path).status().map_err(|e| e.to_string())?;
    if status.success() { Ok(()) } else { Err("Preview could not open the PDF".to_string()) }
}
