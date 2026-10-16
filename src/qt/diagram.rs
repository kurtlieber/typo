//! The Rust half of the `image://diagram/` provider (the C++ half is in
//! cpp/typo_math.cpp). QML asks for
//! `image://diagram/<lang>/<pixels per point>/<colours>/<URI-encoded source>`,
//! the colours as `diagram::Colors::parse` reads them, and Qt calls
//! `render_diagram` on a worker thread. A diagram that fails comes back
//! empty, and `error` says why, for the fence to show instead.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use cxx_qt_lib::QString;

use super::math::decode;

/// Why each failed diagram failed, by its language and source.
static ERRORS: LazyLock<Mutex<HashMap<(String, String), String>>> = LazyLock::new(Default::default);

/// The PNG for the diagram `id` names, or nothing when it fails.
pub fn render_diagram(id: &QString) -> Vec<u8> {
    let id = id.to_string();
    let mut parts = id.splitn(4, '/');
    let lang = parts.next().unwrap_or_default().to_string();
    let scale = parts.next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(1.0);
    let colors = crate::diagram::Colors::parse(parts.next().unwrap_or_default());
    let src = decode(parts.next().unwrap_or_default());
    let result = crate::diagram::render(&lang, &src, &colors, scale);
    let mut errors = ERRORS.lock().unwrap_or_else(|e| e.into_inner());
    match result {
        Ok(png) => {
            errors.remove(&(lang, src));
            png
        }
        Err(message) => {
            if errors.len() > 200 {
                errors.clear();
            }
            errors.insert((lang, src), message);
            Vec::new()
        }
    }
}

/// What is wrong with the `lang` diagram `src`, once its image has loaded;
/// empty when nothing.
pub fn error(lang: &str, src: &str) -> String {
    let errors = ERRORS.lock().unwrap_or_else(|e| e.into_inner());
    errors.get(&(lang.to_string(), src.to_string())).cloned().unwrap_or_default()
}
