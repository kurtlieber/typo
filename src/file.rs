//! Files on disk: reading them in whatever encoding they use, writing them
//! back the same way without ever leaving a half-written file, noticing
//! when something else changed them, and keeping drafts of unsaved work.

use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use encoding_rs::{Encoding, UTF_8, UTF_16BE, UTF_16LE};

/// A file's text and how it was stored.
#[derive(Debug, PartialEq)]
pub struct Decoded {
    pub text: String,
    pub encoding: &'static Encoding,
    /// Whether it started with a byte order mark, which is kept on save.
    pub bom: bool,
}

/// Decode `bytes`: by their byte order mark, as UTF-8 when they are valid
/// UTF-8, and otherwise by guessing, unless `forced` names the encoding.
pub fn decode(bytes: &[u8], forced: Option<&'static Encoding>) -> Decoded {
    let (sniffed, bom_len) = match Encoding::for_bom(bytes) {
        Some((e, n)) => (Some(e), n),
        None => (None, 0),
    };
    let encoding = forced.or(sniffed).unwrap_or_else(|| {
        if std::str::from_utf8(bytes).is_ok() {
            UTF_8
        } else {
            let mut detector = chardetng::EncodingDetector::new(chardetng::Iso2022JpDetection::Allow);
            detector.feed(bytes, true);
            detector.guess(None, chardetng::Utf8Detection::Allow)
        }
    });
    let bom = bom_len > 0 && sniffed == Some(encoding);
    let body = if bom { &bytes[bom_len..] } else { bytes };
    let (text, _) = encoding.decode_without_bom_handling(body);
    Decoded { text: text.into_owned(), encoding, bom }
}

/// Encode `text` for saving as `encoding` (encoding_rs only decodes
/// UTF-16, so that is done here).
pub fn encode(text: &str, encoding: &'static Encoding, bom: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() + 3);
    if encoding == UTF_16LE || encoding == UTF_16BE {
        let le = encoding == UTF_16LE;
        if bom {
            out.extend_from_slice(if le { &[0xFF, 0xFE] } else { &[0xFE, 0xFF] });
        }
        for u in text.encode_utf16() {
            out.extend_from_slice(&if le { u.to_le_bytes() } else { u.to_be_bytes() });
        }
        return out;
    }
    if bom && encoding == UTF_8 {
        out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    let (bytes, _, _) = encoding.encode(text);
    out.extend_from_slice(&bytes);
    out
}

/// The encoding for a label from the Reopen with Encoding menu.
pub fn encoding_for(label: &str) -> Option<&'static Encoding> {
    Encoding::for_label(label.as_bytes())
}

/// Write atomically: a temporary file beside the target, then a rename,
/// keeping the original's permissions.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name = path.file_name().map_or_else(|| "untitled".into(), |n| n.to_string_lossy().into_owned());
    let tmp = dir.join(format!(".{name}.typo-{}", std::process::id()));
    let result = fs::write(&tmp, bytes).and_then(|_| {
        if let Ok(meta) = fs::metadata(path) {
            fs::set_permissions(&tmp, meta.permissions())?;
        }
        fs::rename(&tmp, path)
    });
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// What a file looked like when typo last read or wrote it.
#[derive(Debug, Clone, PartialEq)]
pub struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
    hash: u64,
}

fn hash(bytes: &[u8]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    h.finish()
}

impl Stamp {
    pub fn of(path: &Path, bytes: &[u8]) -> Stamp {
        let modified = fs::metadata(path).ok().and_then(|m| m.modified().ok());
        Stamp { modified, len: bytes.len() as u64, hash: hash(bytes) }
    }
}

/// How the file on disk compares with a stamp.
#[derive(Debug, PartialEq)]
pub enum Disk {
    Same,
    /// Its content changed; the new stamp and bytes.
    Changed(Stamp, Vec<u8>),
    /// Only its time changed (touched, or rewritten with the same text).
    Touched(Stamp),
    Missing,
}

/// Compare the file with `stamp`, reading it only when its time or size
/// moved.
pub fn check(path: &Path, stamp: &Stamp) -> Disk {
    let Ok(meta) = fs::metadata(path) else { return Disk::Missing };
    if meta.modified().ok() == stamp.modified && meta.len() == stamp.len {
        return Disk::Same;
    }
    let Ok(bytes) = fs::read(path) else { return Disk::Missing };
    let now = Stamp::of(path, &bytes);
    if now.hash == stamp.hash && now.len == stamp.len { Disk::Touched(now) } else { Disk::Changed(now, bytes) }
}

/// typo's settings folder: `$XDG_CONFIG_HOME/typo`.
pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("typo")
}

/// The advanced settings a new or reset `conf.user.json` holds: JSON with
/// comments and trailing commas, as Typora's file allows.
pub const ADVANCED_SETTINGS: &str = r#"/** typo's advanced settings, read when a window opens. */
{
  "autoHideMenuBar": false, // Hide the menu bar until Alt is pressed.

  // What the right-click menu offers to search the selected text with, each
  // [caption, url], %s standing for the text.
  "searchService": [
    ["Search with Google", "https://google.com/search?q=%s"]
  ],

  // Shortcut keys for menu items, by the item's label as the menu shows
  // it ("..." for "…"), taking over from the built-in ones; "" for none.
  "keyBinding": {
    // for example:
    // "Always on Top": "Ctrl+Shift+P"
  }
}
"#;

/// The advanced settings: `conf.user.json` read over the defaults, key by
/// key. A file that won't parse counts for nothing.
pub fn advanced() -> serde_json::Value {
    let mut out = parse_relaxed(ADVANCED_SETTINGS).unwrap_or_default();
    let path = config_dir().join("conf.user.json");
    if let Some(serde_json::Value::Object(user)) = fs::read_to_string(path).ok().and_then(|s| parse_relaxed(&s))
        && let Some(base) = out.as_object_mut()
    {
        base.extend(user);
    }
    out
}

/// JSON with `//` and `/* */` comments and trailing commas.
fn parse_relaxed(text: &str) -> Option<serde_json::Value> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);
            if c == '\\' {
                out.extend(chars.next());
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match (c, chars.peek()) {
            ('"', _) => {
                in_string = true;
                out.push(c);
            }
            ('/', Some('/')) => {
                while chars.next_if(|&c| c != '\n').is_some() {}
            }
            ('/', Some('*')) => {
                chars.next();
                let mut last = ' ';
                for c in chars.by_ref() {
                    if last == '*' && c == '/' {
                        break;
                    }
                    last = c;
                }
                out.push(' ');
            }
            // A comma with only space before the closing bracket goes.
            ('}' | ']', _) => {
                let trimmed = out.trim_end().len();
                if out[..trimmed].ends_with(',') {
                    out.truncate(trimmed - 1);
                }
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    serde_json::from_str(&out).ok()
}

/// `conf.user.json` in the settings folder, written with the defaults when
/// it isn't there or `reset` asks for them.
pub fn advanced_settings(reset: bool) -> Result<PathBuf, String> {
    let path = config_dir().join("conf.user.json");
    if reset || !path.exists() {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        write_atomic(&path, ADVANCED_SETTINGS.as_bytes()).map_err(|e| e.to_string())?;
    }
    Ok(path)
}

/// Where drafts of unsaved documents go: `$XDG_DATA_HOME/typo/drafts`.
pub fn drafts_dir() -> PathBuf {
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    data.join("typo").join("drafts")
}

/// Drafts are kept for a month.
pub fn prune_drafts(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let month = Duration::from_secs(30 * 24 * 3600);
    for e in entries.flatten() {
        let old = e.metadata().ok().and_then(|m| m.modified().ok()).and_then(|t| t.elapsed().ok()).is_some_and(|age| age > month);
        if old {
            let _ = fs::remove_file(e.path());
        }
    }
}

/// A file name made safe: no slashes or control characters.
pub fn safe_name(name: &str) -> String {
    let s: String = name.chars().map(|c| if c == '/' || c.is_control() { '-' } else { c }).collect();
    let s = s.trim().trim_start_matches('.').to_string();
    if s.is_empty() { "Untitled".into() } else { s.chars().take(80).collect() }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_relaxed_json() {
        let v = super::parse_relaxed(
            "/** head */ {\n \"a\": 1, // one\n \"b\": [\"//x\", \"/*y*/\",],\n /* c */ \"c\": {\"d\": \"e\\\" // f\",},\n}\n",
        )
        .unwrap();
        assert_eq!(v, serde_json::json!({"a": 1, "b": ["//x", "/*y*/"], "c": {"d": "e\" // f"}}));
        let d = super::parse_relaxed(super::ADVANCED_SETTINGS).unwrap();
        assert_eq!(d["autoHideMenuBar"], false);
        assert_eq!(d["searchService"][0][0], "Search with Google");
        assert_eq!(d["keyBinding"], serde_json::json!({}));
    }

    use super::*;
    use encoding_rs::{SHIFT_JIS, WINDOWS_1252};

    #[test]
    fn decodes_by_bom_utf8_or_guess() {
        assert_eq!(decode(b"# Hi\n", None), Decoded { text: "# Hi\n".into(), encoding: UTF_8, bom: false });
        assert_eq!(decode(b"\xEF\xBB\xBFx", None), Decoded { text: "x".into(), encoding: UTF_8, bom: true });
        assert_eq!(decode(b"\xFF\xFEx\0", None), Decoded { text: "x".into(), encoding: UTF_16LE, bom: true });
        // Latin-1 text that isn't UTF-8.
        let d = decode(b"caf\xE9 cr\xE8me br\xFBl\xE9e et na\xEFve r\xE9sum\xE9", None);
        assert_eq!(d.encoding, WINDOWS_1252);
        assert_eq!(d.text, "café crème brûlée et naïve résumé");
        // A forced encoding wins.
        assert_eq!(decode("日本".as_bytes(), Some(SHIFT_JIS)).encoding, SHIFT_JIS);
    }

    #[test]
    fn encodes_back_the_same_way() {
        for (bytes, _) in [(&b"\xFF\xFEa\0\n\0"[..], 0), (&b"\xFE\xFF\0a\0\n"[..], 0), (&b"\xEF\xBB\xBFa\n"[..], 0), (&b"caf\xE9\n"[..], 0)] {
            let d = decode(bytes, None);
            assert_eq!(encode(&d.text, d.encoding, d.bom), bytes);
        }
    }

    #[test]
    fn notices_changes_on_disk() {
        let dir = std::env::temp_dir().join(format!("typo-file-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.md");
        write_atomic(&p, b"one").unwrap();
        let stamp = Stamp::of(&p, b"one");
        assert_eq!(check(&p, &stamp), Disk::Same);
        std::thread::sleep(Duration::from_millis(20));
        fs::write(&p, b"two!").unwrap();
        assert!(matches!(check(&p, &stamp), Disk::Changed(_, ref b) if b == b"two!"));
        fs::remove_file(&p).unwrap();
        assert_eq!(check(&p, &stamp), Disk::Missing);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn safe_names() {
        assert_eq!(safe_name(" a/b "), "a-b");
        assert_eq!(safe_name("..."), "Untitled");
    }
}
