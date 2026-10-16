//! Open and save dialogs through xdg-desktop-portal's FileChooser, so the
//! desktop's own chooser appears (Omarchy's apps never use Qt's). The calls
//! block until the user answers, so they run off the GUI thread.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use zbus::blocking::{Connection, MessageIterator};
use zbus::zvariant::{OwnedValue, Value};
use zbus::{MatchRule, message};

/// What the dialog is for.
pub enum Pick<'a> {
    Open { folder: Option<&'a Path> },
    /// A file of any kind, for a preference.
    Any { folder: Option<&'a Path> },
    Save { folder: Option<&'a Path>, name: &'a str },
    Folder { folder: Option<&'a Path> },
    /// Insert Local Images…: any number of images.
    Images { folder: Option<&'a Path> },
    /// Copy Image to… and Rename or Move Image to…: where an image goes.
    SaveImage { folder: Option<&'a Path>, name: &'a str },
    /// File → Import…: a file pandoc can read.
    Import { folder: Option<&'a Path> },
    /// File → Export: where the file goes, with the format's filters (a
    /// name and its extensions each).
    Export { folder: Option<&'a Path>, name: &'a str, filters: &'a [(&'a str, &'a [&'a str])] },
}

/// Markdown first, as Typora's own dialogs offer it.
fn filters() -> Vec<(String, Vec<(u32, String)>)> {
    let md = ["md", "markdown", "mdown", "mmd", "mkd", "mkdn", "text", "txt"];
    vec![
        ("Markdown".into(), md.iter().map(|e| (0, format!("*.{e}"))).collect()),
        ("All Files".into(), vec![(0, "*".into())]),
    ]
}

/// The pictures Typora's Insert Local Images… offers.
fn image_filters() -> Vec<(String, Vec<(u32, String)>)> {
    let images = ["png", "jpg", "jpeg", "jfif", "bmp", "svg", "tiff", "webp", "gif"];
    vec![("Images".into(), images.iter().map(|e| (0, format!("*.{e}"))).collect())]
}

/// What File → Import… offers: what pandoc reads.
fn import_filters() -> Vec<(String, Vec<(u32, String)>)> {
    let kinds = [
        "docx", "odt", "rtf", "epub", "html", "htm", "tex", "latex", "rst", "textile", "wiki", "mediawiki", "opml", "org", "ipynb",
        "fb2", "dbk", "xml", "csv", "tsv", "typ", "md",
    ];
    vec![
        ("Supported Files".into(), kinds.iter().map(|e| (0, format!("*.{e}"))).collect()),
        ("All Files".into(), vec![(0, "*".into())]),
    ]
}

/// A folder as the portal takes it: bytes ending in a NUL.
fn folder_bytes(p: &Path) -> Vec<u8> {
    let mut b = p.as_os_str().as_encoded_bytes().to_vec();
    b.push(0);
    b
}

/// What a portal request answered: 0 for yes, then its results.
type Answer = (u32, HashMap<String, OwnedValue>);

/// Make a portal request and wait for the user's answer. `call` makes the
/// D-Bus call, with the handle token its options must carry.
fn request(conn: &Connection, call: impl FnOnce(&str) -> zbus::Result<()>) -> Result<Answer, String> {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let token = format!("typo{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed));
    // Listen on the request object before asking, so a quick answer isn't
    // missed (the portal documents this path).
    let sender = conn.unique_name().map(|n| n.trim_start_matches(':').replace('.', "_")).unwrap_or_default();
    let path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");
    let rule = MatchRule::builder()
        .msg_type(message::Type::Signal)
        .interface("org.freedesktop.portal.Request")
        .and_then(|b| b.member("Response"))
        .and_then(|b| b.path(path.as_str()))
        .map_err(|e| e.to_string())?
        .build();
    let mut answers = MessageIterator::for_match_rule(rule, conn, Some(1)).map_err(|e| e.to_string())?;
    call(&token).map_err(|e| format!("The desktop portal is not available: {e}"))?;
    let msg = answers.next().ok_or("The portal closed without answering")?.map_err(|e| e.to_string())?;
    msg.body().deserialize().map_err(|e| e.to_string())
}

fn session() -> Result<Connection, String> {
    Connection::session().map_err(|e| format!("No session bus: {e}"))
}

const DESKTOP: &str = "org.freedesktop.portal.Desktop";
const DESKTOP_PATH: &str = "/org/freedesktop/portal/desktop";

/// Ask the user for a file (or folder, or images): what they chose,
/// nothing when they cancel.
pub fn pick(title: &str, what: Pick) -> Result<Vec<PathBuf>, String> {
    let conn = session()?;
    let (code, results) = request(&conn, |token| {
        let mut options: HashMap<&str, Value> = HashMap::new();
        options.insert("handle_token", Value::from(token));
        options.insert("modal", Value::from(true));
        let (method, folder) = match what {
            Pick::Open { folder } => {
                options.insert("filters", Value::from(filters()));
                ("OpenFile", folder)
            }
            Pick::Save { folder, name } => {
                options.insert("filters", Value::from(filters()));
                options.insert("current_name", Value::from(name));
                ("SaveFile", folder)
            }
            Pick::Folder { folder } => {
                options.insert("directory", Value::from(true));
                ("OpenFile", folder)
            }
            Pick::Any { folder } => ("OpenFile", folder),
            Pick::Images { folder } => {
                options.insert("filters", Value::from(image_filters()));
                options.insert("multiple", Value::from(true));
                options.insert("accept_label", Value::from("Select"));
                ("OpenFile", folder)
            }
            Pick::SaveImage { folder, name } => {
                options.insert("current_name", Value::from(name));
                ("SaveFile", folder)
            }
            Pick::Import { folder } => {
                options.insert("filters", Value::from(import_filters()));
                options.insert("accept_label", Value::from("Import"));
                ("OpenFile", folder)
            }
            Pick::Export { folder, name, filters } => {
                let filters: Vec<(String, Vec<(u32, String)>)> =
                    filters.iter().map(|(n, exts)| (n.to_string(), exts.iter().map(|e| (0, format!("*.{e}"))).collect())).collect();
                if let Some(first) = filters.first() {
                    options.insert("current_filter", Value::from(first.clone()));
                }
                options.insert("filters", Value::from(filters));
                options.insert("current_name", Value::from(name));
                ("SaveFile", folder)
            }
        };
        if let Some(f) = folder {
            options.insert("current_folder", Value::from(folder_bytes(f)));
        }
        conn.call_method(Some(DESKTOP), DESKTOP_PATH, Some("org.freedesktop.portal.FileChooser"), method, &("", title, options)).map(|_| ())
    })?;
    if code != 0 {
        return Ok(Vec::new());
    }
    let uris: Vec<String> = results
        .get("uris")
        .and_then(|v| Vec::<String>::try_from(v.try_clone().ok()?).ok())
        .unwrap_or_default();
    Ok(uris.iter().filter_map(|u| path_of_uri(u)).collect())
}

/// A printed page, as the print dialog sets it up: its size and margins
/// in millimetres (margins top, right, bottom, left).
#[derive(Clone, Debug, PartialEq)]
pub struct PageSetup {
    pub width: f64,
    pub height: f64,
    pub margins: [f64; 4],
    pub landscape: bool,
}

/// The print dialog: the page the user set up and the token that prints
/// with those settings, or nothing when they cancel.
pub fn prepare_print(title: &str, page: &PageSetup) -> Result<Option<(PageSetup, u32)>, String> {
    let conn = session()?;
    let (code, results) = request(&conn, |token| {
        let mut options: HashMap<&str, Value> = HashMap::new();
        options.insert("handle_token", Value::from(token));
        options.insert("modal", Value::from(true));
        let settings: HashMap<&str, Value> = HashMap::new();
        let mut setup: HashMap<&str, Value> = HashMap::new();
        setup.insert("Width", Value::from(page.width));
        setup.insert("Height", Value::from(page.height));
        setup.insert("MarginTop", Value::from(page.margins[0]));
        setup.insert("MarginRight", Value::from(page.margins[1]));
        setup.insert("MarginBottom", Value::from(page.margins[2]));
        setup.insert("MarginLeft", Value::from(page.margins[3]));
        setup.insert("Orientation", Value::from(if page.landscape { "landscape" } else { "portrait" }));
        conn.call_method(Some(DESKTOP), DESKTOP_PATH, Some("org.freedesktop.portal.Print"), "PreparePrint", &("", title, settings, setup, options)).map(|_| ())
    })?;
    if code != 0 {
        return Ok(None);
    }
    let token = results.get("token").and_then(|v| u32::try_from(v).ok()).ok_or("The print dialog gave no token")?;
    let setup: HashMap<String, OwnedValue> = results
        .get("page-setup")
        .and_then(|v| HashMap::<String, OwnedValue>::try_from(v.try_clone().ok()?).ok())
        .unwrap_or_default();
    let num = |k: &str, default: f64| setup.get(k).and_then(|v| f64::try_from(v).ok()).unwrap_or(default);
    let orientation = setup.get("Orientation").and_then(|v| String::try_from(v.try_clone().ok()?).ok()).unwrap_or_default();
    let landscape = orientation.ends_with("landscape");
    let (w, h) = (num("Width", page.width), num("Height", page.height));
    // The paper's size is given upright; a landscape page turns it.
    let (width, height) = if landscape { (w.max(h), w.min(h)) } else { (w, h) };
    let margins = [num("MarginTop", page.margins[0]), num("MarginRight", page.margins[1]), num("MarginBottom", page.margins[2]), num("MarginLeft", page.margins[3])];
    Ok(Some((PageSetup { width, height, margins, landscape }, token)))
}

/// Print the PDF in `file` with the settings `token` stands for.
pub fn print(title: &str, token: u32, file: &std::fs::File) -> Result<bool, String> {
    let conn = session()?;
    let (code, _) = request(&conn, |handle| {
        let mut options: HashMap<&str, Value> = HashMap::new();
        options.insert("handle_token", Value::from(handle));
        options.insert("modal", Value::from(true));
        options.insert("token", Value::from(token));
        conn.call_method(Some(DESKTOP), DESKTOP_PATH, Some("org.freedesktop.portal.Print"), "Print", &("", title, zbus::zvariant::Fd::from(file), options)).map(|_| ())
    })?;
    Ok(code == 0)
}

/// `file:///a%20b.md` → `/a b.md`.
pub fn path_of_uri(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    // Skip a host part ("file://localhost/…").
    let rest = &rest[rest.find('/')?..];
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        match (bytes[i], bytes.get(i + 1).copied().and_then(hex), bytes.get(i + 2).copied().and_then(hex)) {
            (b'%', Some(h), Some(l)) => {
                out.push((h * 16 + l) as u8);
                i += 3;
            }
            (b, _, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    Some(PathBuf::from(String::from_utf8_lossy(&out).into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uris_become_paths() {
        assert_eq!(path_of_uri("file:///home/a/My%20Notes.md"), Some(PathBuf::from("/home/a/My Notes.md")));
        assert_eq!(path_of_uri("file://localhost/tmp/x.md"), Some(PathBuf::from("/tmp/x.md")));
        assert_eq!(path_of_uri("https://x"), None);
    }
}
