//! The stylesheet of an exported page: typo's own look, in the theme's
//! colours. Sizes and spacing are the editor's (`margins` in
//! src/qt/editor.rs and the block delegates), so a page reads as the
//! document did on screen.

use crate::code::Token;
use crate::md::query::ALERTS;
use crate::palette::{ALERT_COLORS, Rgb, Scheme};

/// Typora's text fonts, then those Omarchy has.
pub const TEXT_FONT: &str = "\"Open Sans\", \"Clear Sans\", \"Helvetica Neue\", Helvetica, Arial, \"Noto Sans\", \"Liberation Sans\", sans-serif";
/// The desktop's monospace font. Naming another first keeps browsers from
/// shrinking it, as they do a lone `monospace`.
pub const MONO_FONT: &str = "ui-monospace, monospace";

/// A tick for a done task, drawn white or black to read on the accent.
fn tick(color: Rgb) -> String {
    format!(
        "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 16 16'%3E%3Cpath d='M3.5 8.5 6.5 11.5 12.5 4.5' \
         fill='none' stroke='%23{}' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'/%3E%3C/svg%3E\")",
        &color.hex()[1..]
    )
}

fn rgba(c: Rgb, alpha: f64) -> String {
    format!("rgba({}, {}, {}, {alpha})", c.0, c.1, c.2)
}

pub fn stylesheet(s: &Scheme) -> String {
    let h = s.headings.map(Rgb::hex);
    let mut css = format!(
        r#":root {{ color-scheme: {mode}; }}
html {{ font-size: 16px; -webkit-text-size-adjust: 100%; text-rendering: optimizeLegibility; }}
body {{ margin: 0; padding: 0; background: {bg}; color: {fg}; font-family: {text}; line-height: 1.6; }}
#write {{ max-width: 860px; margin: 0 auto; padding: 30px 30px 100px; box-sizing: border-box; overflow-wrap: break-word; }}
@media only screen and (min-width: 1400px) {{ #write {{ max-width: 1024px; }} }}
@media only screen and (min-width: 1800px) {{ #write {{ max-width: 1200px; }} }}
#write > :first-child {{ margin-top: 0; }}
#write.first-line-indent p {{ text-indent: 2em; }}
#write.first-line-indent li p, #write.first-line-indent p * {{ text-indent: 0; }}
#write.first-line-indent li {{ margin-left: 2em; }}
::selection {{ background: {selection}; color: {selection_text}; }}
p, blockquote, ul, ol, .md-alert, .md-math-block, .md-diagram {{ margin: 12.8px 0; }}
h1, h2, h3, h4, h5, h6 {{ margin: 16px 0; font-weight: bold; line-height: 1.4; }}
h1, h2 {{ line-height: 1.2; padding-bottom: 0.3em; border-bottom: 1px solid {border}; }}
h1 {{ font-size: 2.25em; color: {h1}; }}
h2 {{ font-size: 1.75em; color: {h2}; }}
h3 {{ font-size: 1.5em; color: {h3}; }}
h4 {{ font-size: 1.25em; color: {h4}; }}
h5 {{ font-size: 1em; color: {h5}; }}
h6 {{ font-size: 1em; color: {h6}; }}
a {{ color: {link}; text-decoration: none; }}
a:hover {{ text-decoration: underline; }}
hr {{ height: 2px; margin: 16px 0; border: 0; background: {border}; }}
blockquote {{ padding: 0 15px; border-left: 4px solid {border}; color: {muted}; }}
blockquote > :first-child, .md-alert > :first-child {{ margin-top: 0; }}
blockquote > :last-child, .md-alert > :last-child {{ margin-bottom: 0; }}
ul, ol {{ padding-left: 30px; }}
li {{ margin: 0; }}
li > ul, li > ol {{ margin: 0; }}
li > p {{ margin: 12.8px 0; }}
.task-list-item {{ position: relative; list-style: none; }}
.task-list-item > input[type=checkbox] {{ appearance: none; -webkit-appearance: none; position: absolute; left: -22px; top: calc(0.8em - 7px);
  width: 14px; height: 14px; margin: 0; box-sizing: border-box; border: 1.2px solid {muted}; border-radius: 3px; background: transparent; }}
.task-list-item > input[type=checkbox]:checked {{ border-color: {accent}; background: {accent} {tick} center / 12px no-repeat; }}
code, kbd, pre, samp, tt {{ font-family: {mono}; }}
code {{ padding: 0 0.2em; border-radius: 3px; font-size: 0.9em; color: {code}; background: {code_background}; }}
kbd {{ padding: 0 0.3em; border: 1px solid {border}; border-radius: 3px; font-size: 0.9em; background: {code_background}; box-shadow: 0 1px 0 {border}; }}
mark {{ padding: 0 0.1em; color: inherit; background: {mark}; }}
pre {{ margin: 15px 0; }}
pre.md-fences {{ padding: 8px 12px; overflow-x: auto; font-size: 0.9em; line-height: 1.45; color: {fg}; background: {code_background};
  border: 1px solid {fence_border}; border-radius: 3px; }}
pre.md-fences code {{ padding: 0; font-size: inherit; color: inherit; background: none; }}
img {{ max-width: 100%; }}
p.md-image-only {{ text-align: center; }}
figure.table-figure {{ margin: 19.2px 0; overflow-x: auto; }}
table {{ border-collapse: collapse; }}
th, td {{ padding: 6px 13px; border: 1px solid {border}; }}
th {{ font-weight: bold; }}
thead tr, tbody tr:nth-child(even) {{ background: {code_background}; }}
.md-math-block {{ position: relative; overflow-x: auto; text-align: center; font-size: 118%; }}
.md-math-inline {{ font-size: 118%; }}
.md-math-tag {{ position: absolute; right: 0; transform: translateY(-50%); }}
.md-math-error {{ color: {error}; font-family: {mono}; font-size: 0.9em; white-space: pre-wrap; }}
.md-diagram {{ margin: 15px 0; text-align: center; }}
.md-diagram svg {{ max-width: 100%; height: auto; }}
.md-alert {{ padding: 0 16px; border-left: 4px solid; }}
.md-alert-text {{ display: inline-flex; align-items: center; gap: 0.4em; font-size: 0.9em; font-weight: bold; }}
.md-alert-text svg {{ width: 16px; height: 16px; fill: none; stroke: currentColor; stroke-width: 1.3; stroke-linecap: round; stroke-linejoin: round; }}
.md-toc {{ margin: 20px 0; font-size: 0.9em; }}
.md-toc-item {{ display: block; padding: 0.15em 0; }}
.md-toc-h1 {{ font-weight: bold; }}
.md-toc-h2 {{ padding-left: 2em; }}
.md-toc-h3 {{ padding-left: 3.5em; }}
.md-toc-h4 {{ padding-left: 5em; }}
.md-toc-h5 {{ padding-left: 6.5em; }}
.md-toc-h6 {{ padding-left: 8em; }}
sup.md-footnote {{ font-size: 0.75em; line-height: 0; }}
sup.md-footnote a {{ padding: 0 3px; border-radius: 3px; background: {code_background}; }}
.footnotes-area {{ margin-top: 2em; font-size: 0.9em; }}
.footnotes-area hr {{ margin: 1em 0; }}
.footnote-line {{ margin: 0.4em 0; }}
.footnote-line > p {{ margin: 0.4em 0; }}
.md-fn-count {{ margin-right: 0.3em; }}
.md-fn-count::after {{ content: "."; }}
a.reversefootnote {{ margin-left: 0.3em; }}
"#,
        mode = if s.dark { "dark" } else { "light" },
        bg = s.background.hex(),
        fg = s.foreground.hex(),
        text = TEXT_FONT,
        mono = MONO_FONT,
        selection = s.selection.hex(),
        selection_text = s.selection_text.hex(),
        border = s.border.hex(),
        fence_border = rgba(s.border, 0.6),
        h1 = h[0],
        h2 = h[1],
        h3 = h[2],
        h4 = h[3],
        h5 = h[4],
        h6 = h[5],
        link = s.link.hex(),
        muted = s.muted.hex(),
        accent = s.accent.hex(),
        tick = tick(s.accent_text),
        code = s.code.hex(),
        code_background = s.code_background.hex(),
        mark = s.mark.hex(),
        error = s.error.hex(),
    );
    for (kind, key) in ALERTS.iter().zip(ALERT_COLORS) {
        let c = s.palette.get(key).hex();
        css.push_str(&format!(".md-alert-{kind} {{ border-left-color: {c}; }}\n.md-alert-text-{kind} {{ color: {c}; }}\n"));
    }
    for t in Token::ALL {
        let (key, italic) = t.style();
        if t == Token::Plain {
            continue;
        }
        let style = if italic { " font-style: italic;" } else { "" };
        css.push_str(&format!(".{} {{ color: {};{style} }}\n", token_class(t), s.palette.get(key).hex()));
    }
    css
}

/// The outline sidebar's styles: a panel down the left, as the editor's
/// side panel, that print leaves out.
pub fn sidebar(s: &Scheme) -> String {
    format!(
        r#".typora-export-sidebar {{ position: fixed; top: 0; bottom: 0; left: 0; width: 260px; box-sizing: border-box; overflow-y: auto;
  padding: 32px 10px 24px; background: {panel}; border-right: 1px solid {border}; font-size: 14px; line-height: 1.5; }}
.typora-export-show-outline .typora-export-content {{ margin-left: 260px; }}
.outline-content ul {{ margin: 0; padding: 0; list-style: none; }}
.outline-content ul ul {{ padding-left: 14px; }}
.outline-item {{ display: flex; align-items: center; padding: 2px 6px; border-radius: 4px; }}
.outline-item:hover {{ background: {hover}; }}
.outline-label {{ flex: 1; min-width: 0; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; color: {fg}; }}
.outline-label:hover {{ text-decoration: none; }}
.outline-item-active > .outline-item > .outline-label {{ color: {accent}; font-weight: bold; }}
.outline-expander {{ flex: none; width: 0; color: {muted}; cursor: pointer; }}
.typora-export-collapse-outline .outline-expander {{ width: 16px; height: 16px; display: flex; align-items: center; justify-content: center; }}
.typora-export-collapse-outline .outline-expander::before {{ content: ""; border-style: solid; border-width: 4px 0 4px 6px;
  border-color: transparent transparent transparent currentColor; transition: transform 0.1s; }}
.typora-export-collapse-outline .outline-item-open > .outline-item > .outline-expander::before {{ transform: rotate(90deg); }}
.typora-export-collapse-outline .outline-item-single > .outline-item > .outline-expander::before {{ border: none; }}
.typora-export-collapse-outline .outline-item-wrapper:not(.outline-item-open) > ul {{ display: none; }}
@media screen and (max-width: 800px) {{
  .typora-export-sidebar {{ display: none; }}
  .typora-export-show-outline .typora-export-content {{ margin-left: 0; }}
}}
@media print {{
  .typora-export-sidebar {{ display: none; }}
  .typora-export-show-outline .typora-export-content {{ margin-left: 0; }}
}}
"#,
        panel = s.panel.hex(),
        border = s.border.hex(),
        hover = rgba(s.foreground, 0.08),
        fg = s.foreground.hex(),
        accent = s.accent.hex(),
        muted = s.muted.hex(),
    )
}

/// The class of a run of highlighted code.
pub fn token_class(t: Token) -> String {
    format!("tk-{t:?}").to_lowercase()
}
