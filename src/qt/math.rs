//! The Rust half of the `image://math/` provider (the C++ half is
//! cpp/typo_math.cpp). QML asks for
//! `image://math/<d|i>/<em in pixels>/<rrggbb>/<URI-encoded TeX>`, `d` for
//! display math and `i` for inline, and Qt calls `render_math` on a worker
//! thread. A formula that fails comes back as one clear pixel, and `error`
//! says why, for the block to show instead. Inline math also notes where
//! its baseline is, for the text to line up with it, and display math of
//! several rows where each row's is, for its number to.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use cxx_qt_lib::QString;

/// Why each failed formula failed, by its TeX.
static ERRORS: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(Default::default);

/// Where each inline formula's baseline is, as a part of its height.
static BASELINES: LazyLock<Mutex<HashMap<String, f64>>> = LazyLock::new(Default::default);

/// Where each display formula's rows have their baselines, by its TeX.
static ROWS: LazyLock<Mutex<HashMap<String, Vec<f64>>>> = LazyLock::new(Default::default);

/// The pixels of the formula `id` names, premultiplied RGBA.
pub fn render_math(id: &QString, width: &mut i32, height: &mut i32) -> Vec<u8> {
    let id = id.to_string();
    let mut parts = id.splitn(4, '/');
    let display = parts.next() == Some("d");
    let em = parts.next().and_then(|s| s.parse::<f64>().ok()).unwrap_or(16.0);
    let color = parts.next().unwrap_or_default().to_string();
    let tex = decode(parts.next().unwrap_or_default());
    let result = crate::math::render(&tex, display, em, &color);
    if !display && let Ok(image) = &result {
        let mut baselines = BASELINES.lock().unwrap_or_else(|e| e.into_inner());
        if baselines.len() > 2000 {
            baselines.clear();
        }
        baselines.insert(tex.clone(), image.baseline / f64::from(image.height.max(1)));
    }
    if let Ok(image) = &result
        && !image.rows.is_empty()
    {
        let mut rows = ROWS.lock().unwrap_or_else(|e| e.into_inner());
        if rows.len() > 500 {
            rows.clear();
        }
        rows.insert(tex.clone(), image.rows.clone());
    }
    let mut errors = ERRORS.lock().unwrap_or_else(|e| e.into_inner());
    match result {
        Ok(image) => {
            errors.remove(&tex);
            *width = image.width as i32;
            *height = image.height as i32;
            image.rgba
        }
        Err(message) => {
            // Errors are small, but a long session makes many.
            if errors.len() > 500 {
                errors.clear();
            }
            errors.insert(tex, message);
            *width = 1;
            *height = 1;
            vec![0; 4]
        }
    }
}

/// What is wrong with `tex`, once its image has loaded; empty when nothing.
pub fn error(tex: &str) -> String {
    ERRORS.lock().unwrap_or_else(|e| e.into_inner()).get(tex).cloned().unwrap_or_default()
}

/// Where inline formula `tex`'s baseline is, as a part of its height, once
/// its image has loaded; -1 before.
pub fn baseline(tex: &str) -> f64 {
    BASELINES.lock().unwrap_or_else(|e| e.into_inner()).get(tex).copied().unwrap_or(-1.0)
}

/// Where display formula `tex`'s rows have their baselines, once its image
/// has loaded; empty for one row.
pub fn rows(tex: &str) -> Vec<f64> {
    ROWS.lock().unwrap_or_else(|e| e.into_inner()).get(tex).cloned().unwrap_or_default()
}

/// Undo `encodeURIComponent`. Qt has already decoded some of the escapes,
/// but never `%25`, so a `%` still in the id always starts an escape.
pub fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex(bytes[i + 1]), hex(bytes[i + 2]))
        {
            out.push((h * 16 + l) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn decodes_uri_escapes() {
        assert_eq!(decode(r"\frac%7Ba%7D%7Bb%7D"), r"\frac{a}{b}");
        assert_eq!(decode("50%25%20%E2%86%92"), "50% →");
        // Qt's partial decoding leaves plain text as it is.
        assert_eq!(decode(r"\alpha{x}"), r"\alpha{x}");
        assert_eq!(decode("100%"), "100%");
    }
}
