//! The Rust half of the `image://diagram/` provider (the C++ half is in
//! cpp/typo_math.cpp). QML asks for
//! `image://diagram/<lang>/<pixels per point>/<gantt width>/<colours>/<URI-encoded source>`,
//! the colours as `diagram::Colors::parse` reads them, and Qt calls
//! `render_diagram` on a worker thread. A diagram that fails comes back
//! empty, and `error` says why, for the fence to show instead, and
//! `error_mark` where in its code.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::{LazyLock, Mutex};

use cxx_qt_lib::QString;

use super::editor::utf16_of;
use super::math::decode;

/// Why each failed diagram failed, and the bytes of its code at fault, by
/// its language and source.
type Errors = HashMap<(String, String), (String, Option<Range<usize>>)>;
static ERRORS: LazyLock<Mutex<Errors>> = LazyLock::new(Default::default);

/// The PNG for the diagram `id` names, or nothing when it fails.
pub fn render_diagram(id: &QString) -> Vec<u8> {
    let id = id.to_string();
    let mut parts = id.splitn(5, '/');
    let lang = parts.next().unwrap_or_default().to_string();
    let scale = parts.next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(1.0);
    let width = parts.next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let colors = crate::diagram::Colors::parse(parts.next().unwrap_or_default());
    let src = decode(parts.next().unwrap_or_default());
    let result = crate::diagram::render(&lang, &src, &colors, scale, width);
    let mut errors = ERRORS.lock().unwrap_or_else(|e| e.into_inner());
    match result {
        Ok(png) => {
            errors.remove(&(lang, src));
            png
        }
        Err(failed) => {
            if errors.len() > 200 {
                errors.clear();
            }
            errors.insert((lang, src), failed);
            Vec::new()
        }
    }
}

/// What is wrong with the `lang` diagram `src`, once its image has loaded;
/// empty when nothing.
pub fn error(lang: &str, src: &str) -> String {
    let errors = ERRORS.lock().unwrap_or_else(|e| e.into_inner());
    errors.get(&(lang.to_string(), src.to_string())).map(|e| e.0.clone()).unwrap_or_default()
}

/// The code `error` is about, as find's `[[start, end]]` in UTF-16; `[]`
/// when nothing is wrong or merman can't say where.
pub fn error_mark(lang: &str, src: &str) -> String {
    let errors = ERRORS.lock().unwrap_or_else(|e| e.into_inner());
    match errors.get(&(lang.to_string(), src.to_string())) {
        Some((_, Some(r))) => format!("[[{},{}]]", utf16_of(src, r.start), utf16_of(src, r.end)),
        _ => "[]".to_string(),
    }
}
