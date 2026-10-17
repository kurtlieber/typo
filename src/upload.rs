//! Upload Image: an image uploader is a command that takes image files as
//! its last arguments and prints their URLs, as Typora runs one
//! (Preferences ▸ Image ▸ Image Uploader).

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

/// A URL printed without its scheme: a host name and a path.
static HOST: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z0-9]+([\-.][a-z0-9]+)*\.[a-z]{2,5}/").unwrap());

/// The command for `uploader`: Custom Command's `custom`, with
/// `${filepath}` and `${filename}` standing for the document's path and
/// its name without the extension, or a known uploader's own. Empty when
/// there is none.
pub fn command(uploader: &str, custom: &str, doc: Option<&Path>) -> String {
    match uploader {
        "custom" => {
            let path = doc.map(|d| d.to_string_lossy().into_owned()).unwrap_or_default();
            let name = doc.and_then(Path::file_stem).map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            custom.trim().replacen("${filepath}", &path, 1).replacen("${filename}", &name, 1)
        }
        // Typora downloads PicGo-Core; here it is the one on the PATH.
        "picgo-cli" => "picgo u".into(),
        "upic" if cfg!(target_os = "macos") => "/Applications/uPic.app/Contents/MacOS/uPic -o url -u".into(),
        "picsee" if cfg!(target_os = "macos") => "/Applications/Picsee.app/Contents/MacOS/Picsee -u".into(),
        _ => String::new(),
    }
}

/// What a run of the uploader did.
pub struct Uploaded {
    /// The URLs, one for each file, or why there aren't.
    pub urls: Result<Vec<String>, String>,
    pub output: String,
    pub error: String,
}

/// Run `command` by the shell with `files` after it, and read the URLs it
/// prints.
pub fn run(command: &str, files: &[String]) -> Uploaded {
    let quoted: Vec<String> = files.iter().map(|f| format!("'{}'", f.replace('\'', r"'\''"))).collect();
    let line = format!("{command} {}", quoted.join(" "));
    let out = std::process::Command::new("sh").arg("-c").arg(&line).stdin(std::process::Stdio::null()).output();
    let out = match out {
        Ok(out) => out,
        Err(e) => return Uploaded { urls: Err(e.to_string()), output: String::new(), error: String::new() },
    };
    let output = String::from_utf8_lossy(&out.stdout).into_owned();
    let error = String::from_utf8_lossy(&out.stderr).trim_end().to_string();
    let urls = if !out.status.success() {
        Err(format!("Command failed: {line}\n{error}").trim_end().to_string())
    } else if output.trim().is_empty() {
        Err(error.clone())
    } else {
        urls(&output, files.len()).ok_or_else(|| format!("fail to parse result image path from: {output}"))
    };
    Uploaded { urls, output, error }
}

/// The last `n` URLs of the run of them that ends the output, as Typora
/// reads it: blank lines are skipped, and a bare `host.tld/…` gets
/// `http://`.
fn urls(output: &str, n: usize) -> Option<Vec<String>> {
    let mut found = Vec::new();
    for line in output.lines().rev().map(str::trim).filter(|l| !l.is_empty()) {
        let scheme = ["http://", "https://", "ftp://", "file:/"].iter().any(|s| line.starts_with(s));
        if scheme {
            found.push(line.to_string());
        } else if HOST.is_match(line) {
            found.push(format!("http://{line}"));
        } else if !found.is_empty() {
            break;
        }
    }
    found.reverse();
    (found.len() >= n).then(|| found.split_off(found.len() - n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_urls_printed_last() {
        let out = "[PicGo INFO]: Before transform\n[PicGo SUCCESS]:\nhttps://a.io/1.png\n\nimg.b.io/2.png\n";
        assert_eq!(urls(out, 2), Some(vec!["https://a.io/1.png".into(), "http://img.b.io/2.png".into()]));
        assert_eq!(urls(out, 1), Some(vec!["http://img.b.io/2.png".into()]));
        assert_eq!(urls(out, 3), None);
        assert_eq!(urls("https://a.io/1.png\nuploaded\n", 1), Some(vec!["https://a.io/1.png".into()]));
        assert_eq!(urls("done", 1), None);
    }

    #[test]
    fn runs_the_command() {
        let files = vec!["/tmp/it's here.png".to_string()];
        let ran = run("printf 'https://x.io/%s\\n'", &files);
        assert_eq!(ran.urls, Ok(vec!["https://x.io//tmp/it's here.png".into()]));
        assert!(run("echo nothing", &files).urls.unwrap_err().starts_with("fail to parse"));
        assert_eq!(run("echo oops >&2; true", &files).urls, Err("oops".into()));
        assert!(run("exit 3", &files).urls.is_err());
        let doc = Path::new("/notes/day one.md");
        assert_eq!(command("custom", " up --in ${filename} ", Some(doc)), "up --in day one");
        assert_eq!(command("custom", "up ${filepath}", Some(doc)), "up /notes/day one.md");
        assert_eq!(command("", "up", Some(doc)), "");
    }
}
