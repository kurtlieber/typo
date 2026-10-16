//! Strut fonts: tiny TrueType fonts made here at startup whose glyphs draw
//! nothing and take no width. An inline image or formula is drawn over the
//! text where its source is hidden; to make the text leave room for it, the
//! highlighter sets one character of the source in a strut font, sized to
//! the object's height and letter-spaced to its width. Each font splits its
//! em differently between ascent and descent, so a formula that reaches
//! below the baseline gets room there too.

/// How finely the fonts split the em: font `n` has an ascent of `n / STEPS`
/// of it and the rest is descent.
pub const STEPS: u16 = 20;

const UNITS_PER_EM: i16 = 1000;

/// The family name of strut font `n`.
pub fn family(n: u16) -> String {
    format!("Typo Strut {n}")
}

/// The strut font and pixel size that make room for `above` pixels over
/// the baseline and `below` under it, wasting as little as the steps allow.
pub fn choose(above: f64, below: f64) -> (u16, f64) {
    let (above, below) = (above.max(0.0), below.max(0.0));
    (0..=STEPS)
        .filter_map(|n| {
            let up = f64::from(n) / f64::from(STEPS);
            let size = match (above > 0.0, below > 0.0) {
                (true, true) if n > 0 && n < STEPS => (above / up).max(below / (1.0 - up)),
                (true, false) if n > 0 => above / up,
                (false, true) if n < STEPS => below / (1.0 - up),
                (false, false) => 0.0,
                _ => return None,
            };
            Some((n, size))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
        .unwrap_or((STEPS, 0.0))
}

/// Strut font `n` (0 to `STEPS`): every character in the Basic
/// Multilingual Plane maps to one empty glyph of no advance.
pub fn font(n: u16) -> Vec<u8> {
    let n = n.min(STEPS) as i32;
    let ascent = (i32::from(UNITS_PER_EM) * n / i32::from(STEPS)) as i16;
    let descent = ascent - UNITS_PER_EM;
    let family = family(n as u16);
    let postscript: String = family.chars().filter(|c| !c.is_whitespace()).collect();
    let tables: [(&[u8; 4], Vec<u8>); 10] = [
        (b"OS/2", os2(ascent, descent)),
        (b"cmap", cmap()),
        (b"glyf", Vec::new()),
        (b"head", head(ascent, descent)),
        (b"hhea", hhea(ascent, descent)),
        (b"hmtx", hmtx()),
        (b"loca", loca()),
        (b"maxp", maxp()),
        (b"name", name(&[(1, &family), (2, "Regular"), (3, &family), (4, &family), (6, &postscript)])),
        (b"post", post()),
    ];
    assemble(&tables)
}

/// Big-endian writer.
#[derive(Default)]
struct W(Vec<u8>);

impl W {
    fn u16(&mut self, v: u16) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    fn i16(&mut self, v: i16) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    fn zeros(&mut self, n: usize) -> &mut Self {
        self.0.resize(self.0.len() + n, 0);
        self
    }
    fn done(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.0)
    }
}

/// The glyphs: 0 is `.notdef` and 1 is the one every character maps to.
const GLYPHS: u16 = 2;

fn head(ascent: i16, descent: i16) -> Vec<u8> {
    W::default()
        .u32(0x0001_0000) // version
        .u32(0x0001_0000) // font revision
        .u32(0) // checksum adjustment, filled in by `assemble`
        .u32(0x5F0F_3CF5) // magic
        .u16(0b11) // baseline and left side bearing at 0
        .u16(UNITS_PER_EM as u16)
        .zeros(16) // created, modified
        .i16(0)
        .i16(descent)
        .i16(0)
        .i16(ascent) // bounding box
        .u16(0) // style
        .u16(1) // smallest readable size
        .i16(2) // direction hint
        .i16(0) // short loca offsets
        .i16(0) // glyph data format
        .done()
}

fn hhea(ascent: i16, descent: i16) -> Vec<u8> {
    W::default()
        .u32(0x0001_0000)
        .i16(ascent)
        .i16(descent)
        .i16(0) // line gap
        .u16(0) // widest advance
        .zeros(6) // side bearings, extent
        .i16(1) // caret slope rise
        .zeros(4) // caret slope run, offset
        .zeros(8) // reserved
        .i16(0) // metric data format
        .u16(GLYPHS) // horizontal metrics
        .done()
}

fn os2(ascent: i16, descent: i16) -> Vec<u8> {
    W::default()
        .u16(4) // version
        .i16(0) // average width
        .u16(400) // weight
        .u16(5) // width
        .u16(0) // embedding: installable
        .zeros(20) // sub/superscript and strikeout metrics
        .i16(0) // family class
        .zeros(10) // panose
        .u32(1) // Unicode ranges: Basic Latin, so Qt takes it for Latin
        .zeros(12)
        .zeros(4) // vendor
        .u16(0x40 | 0x80) // regular; use the typo metrics
        .u16(0x20) // first character
        .u16(0xFFFD) // last character
        .i16(ascent)
        .i16(descent)
        .i16(0) // typo line gap
        .u16(ascent as u16)
        .u16((-descent) as u16) // Windows ascent and descent
        .u32(1) // code pages: Latin 1
        .u32(0)
        .i16(0) // x-height
        .i16(0) // cap height
        .u16(0x20) // default character
        .u16(0x20) // break character
        .u16(1) // context
        .done()
}

/// Every character maps to glyph 1. Format 13, made for many-to-one
/// mappings, does that for all of Unicode in one group; FreeType picks it
/// as the UCS-4 table. A format 4 table maps printable ASCII the same way
/// for readers that only know format 4.
fn cmap() -> Vec<u8> {
    // Format 4: U+0020–U+007E, each to glyph 1 through a glyph id array.
    let ascii: Vec<u16> = (0x20..=0x7E).collect();
    let mut f4 = W::default();
    let seg_count = 2u16;
    let length = 16 + 8 * seg_count + 2 * ascii.len() as u16;
    f4.u16(4).u16(length).u16(0);
    f4.u16(seg_count * 2).u16(4).u16(1).u16(0);
    f4.u16(0x7E).u16(0xFFFF); // end codes
    f4.u16(0); // reserved pad
    f4.u16(0x20).u16(0xFFFF); // start codes
    f4.i16(0).i16(1); // deltas
    // The first segment's glyphs are in the array right after the offsets:
    // 4 bytes on (its own offset and the last segment's).
    f4.u16(4).u16(0);
    for _ in &ascii {
        f4.u16(1);
    }
    let f4 = f4.done();
    // Format 13: one group, U+0000–U+10FFFF all to glyph 1.
    let f13 = W::default().u16(13).u16(0).u32(28).u32(0).u32(1).u32(0).u32(0x10FFFF).u32(1).done();
    // Records sorted by platform and encoding: Unicode 2.0 full repertoire
    // (format 13), then Windows BMP (format 4).
    let header = 4 + 8 * 2;
    let mut out = W::default();
    out.u16(0).u16(2);
    out.u16(0).u16(4).u32(header);
    out.u16(3).u16(1).u32(header + f13.len() as u32);
    let mut out = out.done();
    out.extend_from_slice(&f13);
    out.extend_from_slice(&f4);
    out
}

fn hmtx() -> Vec<u8> {
    let mut w = W::default();
    for _ in 0..GLYPHS {
        w.u16(0).i16(0);
    }
    w.done()
}

fn loca() -> Vec<u8> {
    // Every glyph is empty: all offsets 0.
    vec![0; 2 * (GLYPHS as usize + 1)]
}

fn maxp() -> Vec<u8> {
    W::default()
        .u32(0x0001_0000)
        .u16(GLYPHS)
        .zeros(8) // points and contours
        .u16(2) // zones
        .zeros(16) // twilight points … component depth
        .done()
}

fn name(records: &[(u16, &str)]) -> Vec<u8> {
    let strings: Vec<Vec<u8>> =
        records.iter().map(|(_, s)| s.encode_utf16().flat_map(|u| u.to_be_bytes()).collect()).collect();
    let mut w = W::default();
    w.u16(0).u16(records.len() as u16).u16(6 + 12 * records.len() as u16);
    let mut offset = 0u16;
    for ((id, _), s) in records.iter().zip(&strings) {
        w.u16(3).u16(1).u16(0x0409).u16(*id).u16(s.len() as u16).u16(offset);
        offset += s.len() as u16;
    }
    let mut out = w.done();
    for s in strings {
        out.extend_from_slice(&s);
    }
    out
}

fn post() -> Vec<u8> {
    W::default()
        .u32(0x0003_0000) // no glyph names
        .u32(0) // italic angle
        .i16(-100)
        .i16(50) // underline
        .zeros(20) // fixed pitch, memory hints
        .done()
}

fn checksum(data: &[u8]) -> u32 {
    data.chunks(4).fold(0u32, |sum, c| {
        let mut word = [0u8; 4];
        word[..c.len()].copy_from_slice(c);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

/// The offset table, the table directory, then the tables, each padded to
/// four bytes; then `head`'s checksum adjustment.
fn assemble(tables: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let count = tables.len() as u16;
    let power = 1u16 << (15 - count.leading_zeros());
    let mut w = W::default();
    w.u32(0x0001_0000).u16(count).u16(power * 16).u16(power.trailing_zeros() as u16).u16(count * 16 - power * 16);
    let mut offset = 12 + 16 * u32::from(count);
    let mut head_at = 0;
    for (tag, data) in tables {
        if *tag == b"head" {
            head_at = offset as usize;
        }
        w.0.extend_from_slice(*tag);
        w.u32(checksum(data)).u32(offset).u32(data.len() as u32);
        offset += (data.len() as u32).next_multiple_of(4);
    }
    let mut out = w.done();
    for (_, data) in tables {
        out.extend_from_slice(data);
        out.resize(out.len().next_multiple_of(4), 0);
    }
    let adjust = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
    out[head_at + 8..head_at + 12].copy_from_slice(&adjust.to_be_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use typst::foundations::Bytes;
    use typst::layout::Abs;
    use typst::text::{Font, FontVariant, FontVariations};

    #[test]
    fn fonts_parse_with_the_metrics_asked_for() {
        for n in [0, 7, STEPS] {
            let fonts: Vec<Font> = Font::iter(Bytes::new(font(n))).collect();
            assert_eq!(fonts.len(), 1, "font {n}");
            assert_eq!(fonts[0].info().family, family(n));
            let font = fonts[0].clone().instantiate(FontVariant::default(), Abs::pt(10.0), &FontVariations::default());
            let up = f64::from(n) / f64::from(STEPS);
            assert!((font.metrics().ascender.get() - up).abs() < 1e-6, "font {n}");
            assert!((font.metrics().descender.get() + (1.0 - up)).abs() < 1e-6, "font {n}");
            // Any character, Latin or not, is the empty glyph.
            for c in ['!', '$', 'é', 'α', '中'] {
                assert_eq!(font.ttf().glyph_index(c).map(|g| g.0), Some(1), "{c} in font {n}");
                assert_eq!(font.x_advance(1).map(|a| a.get()), Some(0.0));
            }
        }
    }

    #[test]
    fn chooses_the_tightest_split() {
        // All above: the font with no descent, at the object's height.
        assert_eq!(choose(40.0, 0.0), (STEPS, 40.0));
        assert_eq!(choose(0.0, 10.0), (0, 10.0));
        // 15 over, 5 under: three quarters up, 20 high.
        let (n, px) = choose(15.0, 5.0);
        assert_eq!(n, 15);
        assert!((px - 20.0).abs() < 1e-9);
        // An awkward split costs a little room, never too little.
        let (n, px) = choose(10.0, 3.0);
        let up = f64::from(n) / f64::from(STEPS);
        assert!(px * up >= 10.0 - 1e-9 && px * (1.0 - up) >= 3.0 - 1e-9);
        assert!(px < 13.0 * 1.05);
        assert_eq!(choose(0.0, 0.0), (STEPS, 0.0));
    }
}
