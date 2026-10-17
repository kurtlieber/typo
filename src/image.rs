//! Images: where the source of a markdown image points, and where an
//! inserted one goes. Typora resolves a local path against the document's
//! folder, or against the folder its front matter names in
//! `typora-root-url`, and loads web and data URLs as they are. Inserting an
//! image file can copy it into a folder first (Preferences ▸ Image, or
//! `typora-copy-images-to` in the front matter), and writes its path
//! relative to the document when asked to.

use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::md::{BlockKind, Document};

/// The `typora-root-url` the document's front matter sets, as written.
pub fn root_setting(doc: &Document) -> Option<String> {
    front_matter(doc, "typora-root-url")
}

/// The `typora-copy-images-to` the document's front matter sets.
pub fn copy_setting(doc: &Document) -> Option<String> {
    front_matter(doc, "typora-copy-images-to")
}

/// A setting the document's front matter makes, as written.
pub fn front_matter(doc: &Document, name: &str) -> Option<String> {
    let first = doc.blocks.first().filter(|b| b.kind == BlockKind::FrontMatter)?;
    first.text.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim().eq_ignore_ascii_case(name).then(|| value.trim().trim_matches(['"', '\'']).to_string())
    })
    .filter(|v| !v.is_empty())
}

/// Preferences ▸ Image: what happens to an image file inserted, dropped or
/// pasted, and how its path is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InsertOptions {
    /// When Insert…: 0 nothing, copy to 1 `./`, 2 `./assets`,
    /// 3 `./${filename}.assets` or 4 `folder`.
    pub action: i32,
    pub folder: String,
    /// Apply above rules to local images.
    pub copy_local: bool,
    /// Use relative path if possible.
    pub relative: bool,
    /// Add ./ for relative path.
    pub with_dot: bool,
    /// Auto escape image URL when insert.
    pub escape: bool,
}

impl Default for InsertOptions {
    fn default() -> Self {
        InsertOptions { action: 0, folder: String::new(), copy_local: true, relative: true, with_dot: false, escape: false }
    }
}

/// Where inserted images are copied to: the folder `typora-copy-images-to`
/// (`setting`) names, relative to the document's folder, or else the one
/// the preferences name. `${filename}` stands for the document's name
/// without its extension. None when images stay where they are, or when
/// the folder is relative and the document untitled. The flag says the
/// folder came from the front matter.
pub fn storage(setting: Option<&str>, opts: &InsertOptions, file: Option<&Path>) -> Option<(PathBuf, bool)> {
    let folder = file.and_then(Path::parent);
    let stem = file.and_then(Path::file_stem).map(|s| s.to_string_lossy().into_owned());
    let named = |s: &str| -> Option<String> {
        if s.contains("${filename}") { Some(s.replace("${filename}", stem.as_deref()?)) } else { Some(s.to_string()) }
    };
    // Uploading isn't something typo does.
    if let (Some(s), Some(folder)) = (setting, folder)
        && !s.eq_ignore_ascii_case("upload")
        && !s.eq_ignore_ascii_case("ipic")
    {
        return Some((normalize(&folder.join(named(s)?)), true));
    }
    let path = match opts.action {
        1 => folder?.to_path_buf(),
        2 => folder?.join("assets"),
        3 => folder?.join(format!("{}.assets", stem?)),
        4 if !opts.folder.trim().is_empty() => {
            let custom = named(opts.folder.trim())?;
            if custom.starts_with('/') { PathBuf::from(custom) } else { folder?.join(custom) }
        }
        _ => return None,
    };
    Some((normalize(&path), false))
}

/// Whether inserting the local image at `path` copies it into `folder`:
/// when the preferences apply the rules to local images, and it isn't
/// there already.
pub fn needs_copy(path: &Path, folder: &Path, opts: &InsertOptions) -> bool {
    opts.copy_local && path.parent() != Some(folder)
}

/// Copy the image at `path` into `folder`, which must exist, under its own
/// name, or a new one when that is taken (Typora adds `-` and a time
/// stamp). Characters a file name can't hold are dropped.
pub fn copy_into(path: &Path, folder: &Path) -> std::io::Result<PathBuf> {
    if !path.is_file() {
        return Err(std::io::Error::other("Image src is not a file"));
    }
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut target = folder.join(clean_name(&name));
    if target.exists() {
        target = unique(&target);
    }
    std::fs::copy(path, &target)?;
    Ok(target)
}

/// A file name with what Typora won't put in one taken out.
pub fn clean_name(name: &str) -> String {
    let kept: String = name
        .chars()
        .filter(|c| !matches!(c, '\u{200b}' | '\0' | '~' | '\u{19}' | '\u{7f}' | '\u{9f}' | '\\' | '/' | '<' | '>' | ':' | '"' | '|' | '?' | '*'))
        .map(|c| if matches!(c, '\t' | '\r' | '\n') { ' ' } else { c })
        .collect();
    kept.trim_end_matches([' ', '.']).to_string()
}

/// `path` made unique: `name-<milliseconds>-<count>.ext`.
fn unique(path: &Path) -> PathBuf {
    static COUNT: AtomicU64 = AtomicU64::new(0);
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or_default();
    let n = COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    path.with_file_name(format!("{stem}-{millis}-{n}{ext}"))
}

/// How an inserted image's path is written in the markdown, as Typora's
/// resolveImagePath: relative to `typora-root-url` (`root`) as an absolute
/// path from it, or relative to the document's folder when the preferences
/// ask for that, or else as it is; then escaped when asked.
pub fn insert_src(path: &str, file: Option<&Path>, root: Option<&Path>, opts: &InsertOptions) -> String {
    if let Some(rest) = path.strip_prefix("//") {
        return format!("https://{rest}");
    }
    if path.starts_with("data:") || path.split_once("://").is_some_and(|(scheme, _)| !scheme.contains(['/', '\\'])) {
        return path.to_string();
    }
    if path.is_empty() {
        return String::new();
    }
    let folder = file.and_then(Path::parent);
    let base = root.or(folder.filter(|_| opts.relative));
    let written = match base {
        Some(base) => {
            let mut r = relative(base, Path::new(path), opts.with_dot);
            if root.is_some() && !r.starts_with("./") {
                r.insert(0, '/');
            }
            r
        }
        None => path.to_string(),
    };
    if opts.escape { escape_url(&written) } else { written }
}

/// The path from folder `from` to `to`, as Node's `path.relative`, with
/// `./` before it when `dot` is set and it doesn't climb out; `./` for the
/// folder itself (a dropped folder linked from a document in it).
pub fn relative(from: &Path, to: &Path, dot: bool) -> String {
    let (from, to) = (normalize(from), normalize(to));
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let same = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = vec!["..".to_string(); from.len() - same];
    parts.extend(to[same..].iter().map(|c| c.as_os_str().to_string_lossy().into_owned()));
    let r = parts.join("/");
    if r.is_empty() {
        return "./".into();
    }
    if dot && !r.starts_with("./") && !r.starts_with("../") { format!("./{r}") } else { r }
}

/// JavaScript's encodeURI, with `'` and `#` escaped too.
fn escape_url(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.replace('\\', "/").bytes() {
        if b.is_ascii_alphanumeric() || b";,/?:@&=+$-_.!~*()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// What kind of file Typora takes `path` for by its extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Media {
    Image,
    Video,
    Audio,
    Other,
}

pub fn media(path: &str) -> Media {
    let ext = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    let ext = ext.as_str();
    if ["png", "jpg", "jpeg", "jfif", "svg", "gif", "bmp", "webp", "tiff"].contains(&ext) {
        Media::Image
    } else if ["mp4", "webm", "mkv", "flv", "avi", "mov", "wmv", "rm", "rmvb", "m4p", "mpg", "mpeg", "mpv", "ogg"].contains(&ext) {
        Media::Video
    } else if ["mp3", "vox", "wav", "wma", "w4a", "wave", "flac", "aac"].contains(&ext) {
        Media::Audio
    } else {
        Media::Other
    }
}

/// An inserted image's alt text: its file name without the extension (or
/// a leading dot).
pub fn alt_of(path: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let name = name.split(['?', '#']).next().unwrap_or(name);
    let name = name.strip_prefix('.').unwrap_or(name);
    match name.rfind('.') {
        Some(i) => name[..i].to_string(),
        None => name.to_string(),
    }
}

/// The folder local images resolve against when the front matter sets
/// `typora-root-url` to `setting`: relative to the document's folder, or as
/// it is when the document is untitled.
pub fn root(setting: &str, file: Option<&Path>) -> PathBuf {
    let setting = Path::new(setting);
    match file.and_then(Path::parent) {
        Some(folder) if setting.is_relative() => normalize(&folder.join(setting)),
        _ => setting.to_path_buf(),
    }
}

/// The URL to load for an image whose source is `src`, in the document at
/// `file` (None when untitled) whose images resolve against `root` when the
/// front matter sets one.
pub fn url(src: &str, file: Option<&Path>, root: Option<&Path>) -> String {
    let src = src.trim();
    if src.is_empty() || has_scheme(src, &["data:", "http://", "https://"]) {
        return src.to_string();
    }
    if let Some(rest) = src.strip_prefix("//") {
        return format!("http://{rest}");
    }
    let src = decode_path(src);
    if has_scheme(&src, &["file://", "ftp://"]) {
        return src;
    }
    // A local path: a `#fragment` names nothing in a file, and a `?query`
    // would be taken as part of the URL rather than the name.
    let mut path = src.as_str();
    if let Some(hash) = path.rfind('#').filter(|&i| !path[i + 1..].contains(['.', '/', '\\'])) {
        path = &path[..hash];
    }
    let absolute = path.starts_with('/') || path.starts_with('\\');
    let full = match root {
        Some(root) => root.join(path.trim_start_matches(['/', '\\'])),
        None if absolute => PathBuf::from(format!("/{}", path.trim_start_matches(['/', '\\']))),
        None => {
            let folder = file.and_then(Path::parent).map(Path::to_path_buf);
            folder.or_else(|| std::env::current_dir().ok()).unwrap_or_default().join(path)
        }
    };
    file_url(&normalize(&full))
}

/// The file an image whose source is `src` shows, when it is a local one.
pub fn local_path(src: &str, file: Option<&Path>, root: Option<&Path>) -> Option<PathBuf> {
    let url = url(src, file, root);
    let path = url.strip_prefix("file://")?;
    Some(PathBuf::from(percent_decode(path, false).unwrap_or_else(|| path.to_string())))
}

fn has_scheme(s: &str, schemes: &[&str]) -> bool {
    schemes.iter().any(|p| s.len() >= p.len() && s.is_char_boundary(p.len()) && s[..p.len()].eq_ignore_ascii_case(p))
}

/// Remove `.` and `..` from a path without asking the file system, as
/// Node's `path.resolve` does.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

/// Undo percent escapes in a written path, as Typora does: everything when
/// the source is a bare file name, but not the escapes of characters that
/// mean something in a URL (`%2F`, `%23`, …) once it has a folder in it.
fn decode_path(src: &str) -> String {
    let has_folder = |s: &str| s.contains(['/', '\\']);
    if !has_folder(src)
        && let Some(all) = percent_decode(src, false)
        && !has_folder(&all)
    {
        return all;
    }
    percent_decode(src, true).unwrap_or_else(|| src.to_string())
}

/// `None` when an escape is malformed or the bytes aren't UTF-8.
fn percent_decode(s: &str, keep_reserved: bool) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        let hex = |b: Option<&u8>| b.and_then(|&b| (b as char).to_digit(16));
        let byte = (hex(bytes.get(i + 1))? * 16 + hex(bytes.get(i + 2))?) as u8;
        // decodeURI leaves these escaped, except that Typora restores `@`.
        if keep_reserved && b";/?:&=+$,#".contains(&byte) {
            out.extend_from_slice(&bytes[i..i + 3]);
        } else {
            out.push(byte);
        }
        i += 3;
    }
    String::from_utf8(out).ok()
}

/// A `file://` URL for an absolute path, escaped so Qt reads it back as the
/// same path.
pub fn file_url(path: &Path) -> String {
    let mut url = String::from("file://");
    for b in path.to_string_lossy().bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~".contains(&b) {
            url.push(b as char);
        } else {
            url.push_str(&format!("%{b:02X}"));
        }
    }
    url
}

/// How many of a picture's pixels make a point, from its file's DPI as
/// Typora reads it (a PNG's pHYs, a JPEG's JFIF density, over 72): 1 when it
/// says none or the file can't be read.
pub fn pixel_ratio(path: &Path) -> f64 {
    use std::io::Read;
    let mut data = Vec::new();
    // ponytail: the header is read from the first MB; a pHYs after a bigger
    // ICC profile is missed.
    let read = std::fs::File::open(path).and_then(|f| f.take(1 << 20).read_to_end(&mut data));
    if read.is_err() {
        return 1.0;
    }
    let u16_at = |i: usize| data.get(i..i + 2).map(|b| u16::from_be_bytes([b[0], b[1]]) as usize);
    let ratio = if data.starts_with(b"\x89PNG") {
        let mut i = 8;
        loop {
            let Some(len) = data.get(i..i + 4).map(|b| u32::from_be_bytes(b.try_into().unwrap()) as usize) else { break 1.0 };
            match data.get(i + 4..i + 8) {
                Some(b"pHYs") => {
                    let ppm = data.get(i + 8..i + 12).map_or(0, |b| u32::from_be_bytes(b.try_into().unwrap()));
                    break (0.0254 * ppm as f64).round() / 72.0;
                }
                Some(b"IDAT") | None => break 1.0,
                _ => i += 12 + len,
            }
        }
    } else if data.starts_with(&[0xFF, 0xD8]) {
        let mut i = 2;
        loop {
            if data.get(i) != Some(&0xFF) {
                break 1.0;
            }
            match data.get(i + 1) {
                Some(0xE0) => {
                    break match data.get(i + 11) {
                        Some(0 | 1) => u16_at(i + 12).map_or(1.0, |d| (d as f64 / 72.0).round()),
                        _ => 1.0,
                    };
                }
                Some(0xDA) | None => break 1.0,
                _ => match u16_at(i + 2) {
                    Some(len) => i += 2 + len,
                    None => break 1.0,
                },
            }
        }
    } else {
        1.0
    };
    if ratio > 0.0 { ratio } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(src: &str) -> String {
        url(src, Some(Path::new("/notes/doc.md")), None)
    }

    #[test]
    fn loads_web_and_data_urls_as_written() {
        assert_eq!(at("https://example.com/a b.png"), "https://example.com/a b.png");
        assert_eq!(at("HTTP://x/y.png"), "HTTP://x/y.png");
        assert_eq!(at("data:image/png;base64,AAAA"), "data:image/png;base64,AAAA");
        assert_eq!(at("//cdn.example.com/x.png"), "http://cdn.example.com/x.png");
        assert_eq!(at("file:///tmp/x.png"), "file:///tmp/x.png");
    }

    #[test]
    fn resolves_local_paths_against_the_document() {
        assert_eq!(at("pic.png"), "file:///notes/pic.png");
        assert_eq!(at("./img/pic.png"), "file:///notes/img/pic.png");
        assert_eq!(at("../pic.png"), "file:///pic.png");
        assert_eq!(at("/abs/pic.png"), "file:///abs/pic.png");
    }

    #[test]
    fn escapes_what_a_url_cannot_hold() {
        assert_eq!(at("my pic #1.png"), "file:///notes/my%20pic%20%231.png");
        assert_eq!(at("café.png"), "file:///notes/caf%C3%A9.png");
        // A fragment that isn't part of a file name is dropped.
        assert_eq!(at("pic.png#center"), "file:///notes/pic.png");
    }

    #[test]
    fn decodes_percent_escapes_as_typora_does() {
        assert_eq!(at("my%20pic.png"), "file:///notes/my%20pic.png");
        // An escaped slash stays part of the name.
        assert_eq!(at("a%2Fb.png"), "file:///notes/a%252Fb.png");
        assert_eq!(at("img/a%2Fb%40c.png"), "file:///notes/img/a%252Fb%40c.png");
        assert_eq!(at("img/50%.png"), "file:///notes/img/50%25.png");
    }

    #[test]
    fn a_root_url_takes_over_local_paths() {
        let root = Path::new("/site/static");
        assert_eq!(url("/img/a.png", Some(Path::new("/notes/doc.md")), Some(root)), "file:///site/static/img/a.png");
        assert_eq!(url("img/a.png", None, Some(root)), "file:///site/static/img/a.png");
        assert_eq!(url("https://x/a.png", None, Some(root)), "https://x/a.png");
    }

    fn opts(action: i32, folder: &str) -> InsertOptions {
        InsertOptions { action, folder: folder.into(), ..InsertOptions::default() }
    }

    #[test]
    fn picks_the_folder_images_are_copied_to() {
        let doc = Some(Path::new("/notes/day one.md"));
        let at = |setting: Option<&str>, o: InsertOptions, file| storage(setting, &o, file).map(|(p, front)| (p.to_string_lossy().into_owned(), front));
        assert_eq!(at(None, opts(0, ""), doc), None);
        assert_eq!(at(None, opts(1, ""), doc), Some(("/notes".into(), false)));
        assert_eq!(at(None, opts(2, ""), doc), Some(("/notes/assets".into(), false)));
        assert_eq!(at(None, opts(3, ""), doc), Some(("/notes/day one.assets".into(), false)));
        assert_eq!(at(None, opts(4, "../pics/${filename}"), doc), Some(("/pics/day one".into(), false)));
        assert_eq!(at(None, opts(4, "/srv/img"), None), Some(("/srv/img".into(), false)));
        // Relative folders need a saved document.
        assert_eq!(at(None, opts(2, ""), None), None);
        assert_eq!(at(None, opts(4, "${filename}"), None), None);
        // The front matter wins.
        assert_eq!(at(Some("./img"), opts(2, ""), doc), Some(("/notes/img".into(), true)));
        assert_eq!(at(Some("upload"), opts(2, ""), doc), Some(("/notes/assets".into(), false)));
        let doc = crate::md::parse("---\ntypora-copy-images-to: '../shared'\n---\n");
        assert_eq!(copy_setting(&doc).as_deref(), Some("../shared"));
    }

    #[test]
    fn copies_under_a_free_name() {
        let dir = std::env::temp_dir().join(format!("typo-image-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("in")).unwrap();
        std::fs::create_dir_all(dir.join("out")).unwrap();
        let src = dir.join("in/a~b.png");
        std::fs::write(&src, b"png").unwrap();
        let first = copy_into(&src, &dir.join("out")).unwrap();
        assert_eq!(first, dir.join("out/ab.png"));
        let second = copy_into(&src, &dir.join("out")).unwrap();
        let name = second.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("ab-") && name.ends_with(".png"), "{name}");
        assert_eq!(std::fs::read(&second).unwrap(), b"png");
        assert!(copy_into(&dir.join("in/none.png"), &dir.join("out")).is_err());
        assert!(!needs_copy(&first, &dir.join("out"), &InsertOptions::default()));
        assert!(needs_copy(&src, &dir.join("out"), &InsertOptions::default()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn writes_the_inserted_path() {
        let doc = Some(Path::new("/notes/doc.md"));
        let relative = InsertOptions::default();
        let absolute = InsertOptions { relative: false, ..InsertOptions::default() };
        assert_eq!(insert_src("/notes/assets/a b.png", doc, None, &relative), "assets/a b.png");
        assert_eq!(insert_src("/pics/a.png", doc, None, &relative), "../pics/a.png");
        assert_eq!(insert_src("/notes/assets/a.png", doc, None, &InsertOptions { with_dot: true, ..relative.clone() }), "./assets/a.png");
        assert_eq!(insert_src("/notes/assets/a b.png", doc, None, &absolute), "/notes/assets/a b.png");
        assert_eq!(insert_src("/notes/a b#1.png", None, None, &relative), "/notes/a b#1.png");
        assert_eq!(insert_src("/notes/a b#1é.png", doc, None, &InsertOptions { escape: true, ..relative.clone() }), "a%20b%231%C3%A9.png");
        assert_eq!(insert_src("/site/static/img/a.png", doc, Some(Path::new("/site/static")), &absolute), "/img/a.png");
        assert_eq!(insert_src("https://x.y/a.png", doc, None, &relative), "https://x.y/a.png");
        assert_eq!(insert_src("//x.y/a.png", doc, None, &relative), "https://x.y/a.png");
    }

    #[test]
    fn names_and_kinds() {
        assert_eq!(alt_of("/a/b/my.cat.PNG"), "my.cat");
        assert_eq!(alt_of("/a/.hidden"), "hidden");
        assert_eq!(alt_of("https://x/y/z.gif?s=2"), "z");
        assert_eq!(media("a.JPG"), Media::Image);
        assert_eq!(media("a.mp4"), Media::Video);
        assert_eq!(media("a.flac"), Media::Audio);
        assert_eq!(media("a.pdf"), Media::Other);
        assert_eq!(clean_name("a:b?c*.png. "), "abc.png");
        assert_eq!(relative(Path::new("/a/b"), Path::new("/a/b"), true), "./");
    }

    #[test]
    fn reads_the_root_from_front_matter() {
        let doc = crate::md::parse("---\ntitle: x\nTypora-Root-URL: ../static\n---\n\n# a\n");
        assert_eq!(root_setting(&doc).as_deref(), Some("../static"));
        assert_eq!(root("../static", Some(Path::new("/site/notes/doc.md"))), PathBuf::from("/site/static"));
        assert_eq!(root("/www", Some(Path::new("/site/notes/doc.md"))), PathBuf::from("/www"));
        assert_eq!(root_setting(&crate::md::parse("# a\n")), None);
    }

    #[test]
    fn reads_a_pictures_pixel_ratio() {
        let dir = std::env::temp_dir().join(format!("typo-ratio-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = |name: &str, bytes: &[u8]| {
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap();
            pixel_ratio(&path)
        };
        let chunk = |kind: &[u8], body: &[u8]| [&(body.len() as u32).to_be_bytes()[..], kind, body, &[0; 4]].concat();
        let png = |ppm: u32| {
            let phys = [&ppm.to_be_bytes()[..], &ppm.to_be_bytes(), &[1]].concat();
            [&b"\x89PNG\r\n\x1a\n"[..], &chunk(b"IHDR", &[0; 13]), &chunk(b"pHYs", &phys), &chunk(b"IDAT", &[])].concat()
        };
        assert_eq!(file("a.png", &png(5669)), 2.0);
        assert_eq!(file("b.png", &png(3780)), 96.0 / 72.0);
        assert_eq!(file("c.png", &png(0)), 1.0);
        let jpeg = |units: u8, density: u16| {
            let d = density.to_be_bytes();
            [&[0xFF, 0xD8, 0xFF, 0xE0, 0, 16][..], b"JFIF\0", &[1, 1, units, d[0], d[1], d[0], d[1], 0, 0]].concat()
        };
        assert_eq!(file("a.jpg", &jpeg(1, 144)), 2.0);
        assert_eq!(file("b.jpg", &jpeg(1, 72)), 1.0);
        assert_eq!(file("c.jpg", &jpeg(2, 57)), 1.0);
        assert_eq!(file("d.jpg", &[0xFF, 0xD8, 0xFF, 0xE1, 0, 2, 0xFF, 0xE0, 0, 16, b'J', b'F', b'I', b'F', 0, 1, 1, 1, 1, 32, 1, 32]), 4.0);
        assert_eq!(file("e.svg", b"<svg/>"), 1.0);
        assert_eq!(pixel_ratio(&dir.join("none.png")), 1.0);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
