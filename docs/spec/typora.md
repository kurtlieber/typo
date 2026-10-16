# Typora behaviour spec (what typo mirrors)

Written from black-box observation of Typora 1.14.9 for Linux (run in the
headless sandbox), its shipped help documents, its UI string tables, and the
public support site. No Typora code is copied into typo; this file describes
behaviour in our own words. Reference screenshots are kept outside the repo.

## Window layout

- Menu bar across the top: File, Edit, Paragraph, Format, View, Themes, Help.
- Optional left sidebar (Ctrl+Shift+L), ~260px, resizable, with three views:
  Outline (Ctrl+Shift+1), Articles / file list (Ctrl+Shift+2), File Tree
  (Ctrl+Shift+3). A search box filters files; folder-wide search lives here.
- Editor column centred, max width ~860px, generous top padding (~36px),
  vertical scrollbar at the far right edge of the window.
- Footer (28px, shown on hover / always when "Show Status Bar"): bottom-left
  buttons toggle the sidebar (circle icon) and Source Code Mode (`</>`);
  bottom-right shows the word count, which opens a popover with words,
  characters, lines and reading time (and counts for the selection).
- The window title is the file name (plus "•" when there are unsaved changes).

## Editing model ("seamless live preview")

- Block syntax is consumed on input and never shown again in the rendered view:
  `# ` → heading, `> ` → quote, `- `/`* `/`+ ` → bullet list, `1. ` → ordered
  list, `- [ ] ` → task list, ```` ```lang ```` + Enter → code fence, `$$` +
  Enter → math block, `---`/`***` + Enter → horizontal rule, `| a | b |` +
  Enter → table, `[toc]` + Enter → table of contents, `---` on the first line →
  YAML front matter, `[^1]: ` → footnote definition.
- Inline syntax renders in place and its markers reappear while the caret is
  inside the span: `**strong**`, `*em*`, `` `code` ``, `~~strike~~`,
  `<u>underline</u>`, `==highlight==`, `^sup^`, `~sub~`, `$math$`, links
  `[t](url "title")`, images `![a](src)`, autolinks, footnote refs `[^1]`,
  emoji `:smile:`, inline HTML.
- Enter splits the block; Enter on an empty list item ends the list (or
  outdents a nested one); Shift+Enter inserts a soft line break; Backspace at
  the start of a heading/quote/list item turns it into a plain paragraph, at
  the start of a paragraph merges with the previous block.
- Tab / Shift+Tab (also Ctrl+[ / Ctrl+]) indent / outdent list items.
- Code fences: monospace editor with syntax highlighting, a language box at
  the bottom-right, optional line numbers.
- Math blocks: source editor shown while focused, rendered preview below; only
  the rendering when unfocused. Inline math renders when the caret leaves it.
- Tables: WYSIWYG grid of cells; a toolbar above the focused table (alignment,
  resize, delete); Ctrl+Enter adds a row; context menu adds/removes rows and
  columns; Tab moves between cells.
- Images render inline; the source `![..](..)` shows while the caret is in it.
  Dropping/pasting an image can copy it to `./assets` etc. per preferences.
- Links open with Ctrl+click.
- Diagrams in code fences: `mermaid`, `flow`, `sequence`.
- Source Code Mode (Ctrl+/) swaps the whole view for a plain monospace
  markdown editor with syntax colouring; toggling back re-renders.
- Focus Mode (F8) dims everything but the current block; Typewriter Mode (F9)
  keeps the caret line vertically centred.

## Menus and shortcuts (Linux)

File: New (Ctrl+N), New Window (Ctrl+Shift+N), Open (Ctrl+O), Open Folder,
Open Quickly (Ctrl+P), Reopen Closed File (Ctrl+Shift+T), Open Recent ▸,
Reopen with Encoding ▸, Reload from Disk, Save (Ctrl+S), Save As
(Ctrl+Shift+S), Save All, Move To, Rename, Open File Location, Reveal in
Sidebar, Import, Export ▸ (PDF, HTML, HTML without styles, Image, + pandoc
formats: Word, OpenOffice, RTF, Epub, LaTeX, MediaWiki, reStructuredText,
Textile, OPML), Print (Alt+Shift+P), Preferences (Ctrl+,), Close (Ctrl+W),
Quit (Ctrl+Q).

Edit: Undo (Ctrl+Z), Redo (Ctrl+Y / Ctrl+Shift+Z), Cut, Copy, Paste, Copy as
Plain Text, Copy as Markdown (Ctrl+Shift+C), Copy as HTML Code, Paste as Plain
Text (Ctrl+Shift+V), Select All, Select Line/Sentence (Ctrl+L), Select Styled
Scope (Ctrl+E), Select Word (Ctrl+D), Delete Word (Ctrl+Shift+D), Delete
Line, Jump to Top (Ctrl+Home), Jump to Selection (Ctrl+J), Jump to Bottom
(Ctrl+End), Find (Ctrl+F), Find Next (F3), Find Previous (Shift+F3), Replace
(Ctrl+H), Math Tools ▸, Image Tools ▸, Line Endings ▸ (CRLF / LF), Whitespace
and Line Breaks ▸, Smart Punctuation ▸, Spelling ▸, Emoji & Symbols.

Paragraph: Heading 1–6 (Ctrl+1…6), Paragraph (Ctrl+0), Increase / Decrease
Heading Level (Ctrl+= / Ctrl+-), Table (Ctrl+T), Code Fences (Ctrl+Shift+K),
Math Block (Ctrl+Shift+M), Quote (Ctrl+Shift+Q), Alert ▸ (Note, Tip,
Important, Warning, Caution), Ordered List (Ctrl+Shift+[), Unordered List
(Ctrl+Shift+]), Task List (Ctrl+Shift+X), Task Status ▸, List Indentation ▸
(Indent Ctrl+[, Outdent Ctrl+]), Insert Paragraph Before / After, Link
Reference, Footnotes, Horizontal Line, Table of Contents, YAML Front Matter.

Format: Strong (Ctrl+B), Emphasis (Ctrl+I), Underline (Ctrl+U), Code
(Ctrl+Shift+`), Inline Math, Strike (Alt+Shift+5), Highlight, Superscript,
Subscript, Comment, Hyperlink (Ctrl+K), Image ▸ (Ctrl+Shift+I), Clear Format
(Ctrl+\).

View: Toggle Sidebar (Ctrl+Shift+L), Outline (Ctrl+Shift+1), Articles
(Ctrl+Shift+2), File Tree (Ctrl+Shift+3), Source Code Mode (Ctrl+/), Focus
Mode (F8), Typewriter Mode (F9), Show Status Bar, Toolbar, Toggle Fullscreen
(F11), Always on Top, Actual Size (Ctrl+Shift+0), Zoom In (Ctrl+Shift+=),
Zoom Out (Ctrl+Shift+-), Switch Between Opened Documents (Ctrl+Tab).

Themes: list of installed themes (radio). In typo this lists Omarchy themes.

Help: Quick Start, Markdown Reference, Install and Use Pandoc, Custom Themes,
Use Images in Typora, Data Recovery and Version Control, What's New, Credits,
Change Log, Privacy Policy, Website, Feedback, About.

## Preferences

File ▸ Preferences… (Ctrl+,) toggles a panel that covers the whole window
under the menu bar, in the theme's colours. A 56px header says
"Preferences" (16px, 35px in) with a close button (×, 56px wide, a rule on
its left) at the right; a soft shadow under it. Below, a sidebar (190px)
with a "Search" field and the sections, the selected one on a grey pill
(`--active-file-bg-color`), then the section itself: its name as a 24px
heading, then groups. A group's title is bold on the left (up to 200px) with
its controls beside it; under 1040px wide, the title sits above them.
Searching dims sections without a hit, marks the hits in yellow, and says
`No results found for "…"` when nothing matches. "Global Image Settings…"
opens the panel on Image with that group highlighted for four seconds.

Sections, top to bottom, with each group's controls (Linux build; the macOS
and Windows only ones are left out):

- **Files.** On Launch: Open new file / Restore last closed folders / Restore
  last closed files and folders / Open custom folder (with a folder field).
  Outline: Collapsible Outline on Left Panel. Default File Type / Extension:
  Markdown (.md) / Markdown (.markdown) / Text File (.text) / Custom (a field;
  "Following extensions are supported: md, markdown, mmd, mdown, mkd, mdwn,
  mdtxt, mdtext, txt, text, rmd, rmarkdown."). Save & Recover: Auto Save;
  Save without asking when switch files on side panel; a Recover Unsaved
  Drafts button (opens the drafts folder). Recent Locations: Record recent
  files and folders; Clear Recent Folders and Files (asks). When drop file /
  folder into Typora: When drop folder (Open in Typora / Insert Folder Link),
  When drop markdown file (Open in Typora / Insert File Link), When drop
  files that can be imported (Import File / Insert File Link). File Tree /
  Library ("Choose which files appear in the file tree and file library."):
  Show hidden folders / files; File Tree and File Library each Supported
  Files (Markdown & Plain Text) / All Files / Custom, with Configure… (glob
  patterns, `!` excludes).
- **Editor** ("(Some options will be applied after restart.)"). Indent Size
  on Save (hint: only apply to quotes and lists created from menu bar or
  hybrid view): Auto / 2 / 3 / 4 / 5 / Tab, Pretty indentation, and a
  preview of a quote, list item and sub-list. Auto Pair: Auto pair brackets
  and quotes; Auto pair common Markdown syntax (needs the first). Auto
  Complete: Enable autocomplete for Emojis. Live Rendering: Display source
  for simple blocks (including headings, etc.) on focus. Default Copy
  Behavior (ctrl+c): Copy Markdown source as plain text; Copy or cut the
  whole lines that have cursors on them, if there is no selection. Default
  Line Ending (Line ending for new file): LF (Unix Style) / CRLF (Windows
  Style). Spell Check: Auto Detect Language / Disable Spell Check /
  languages. Typewriter Mode: Always keep caret in middle of screen when
  typewriter mode is enabled; a Turn off Typewriter / Focus Mode button
  ("Typewriter / Focus Mode have been turned off.").
- **Image.** When Insert…: No special action / Copy image to current folder
  (./) / Copy image to ./assets / Copy image to ./${filename}.assets / Upload
  image / Copy image to custom folder (a path field with a folder button and
  Reset to Default; "Please specify a relative path…"); Apply above rules to
  local images; to online images; Allow upload images automatically based on
  YAML settings. Preferred Image Syntax: Use relative path if possible; Add
  ./ for relative path; Auto escape image URL when insert. Image Uploader.
- **Markdown** ("(Following preferences will be applied after restart.)").
  Syntax Preference: Strict Mode; Heading Style (atx (#) / atx with suffix
  (# ⋯ #) / setext (===) / variable-width setext (=====)); Unordered List
  (- / + / *); Ordered List (1. ⋯ 2. ⋯ 3. ⋯ / 1. ⋯ 1. ⋯ 1. ⋯). Syntax
  Support: Auto Links; Inline Math; Subscript; Superscript; Highlight;
  Github Style Alert; Diagrams. Smart Punctuation: Convert on Input /
  Convert on Rendering; Smart Quotes (with quote pairs); Smart Dashes
  (-- to en dash and --- to em dash, or both to em dash); Remap Unicode
  Punctuation on Parse. Code Fences: Display line numbers; Auto wrap long
  lines; Use Shift+Tab to auto indent selected code; Indent Size for Code;
  Default Code Language (None / Last Used / languages) and when it applies.
  Math: Inline Math; LaTeX Math Delimiter \( \) \[ \]; Code Block Math;
  Enable physics package; legacy inline math parsing; Auto Numbering Math
  Equations (No Automatic Equation Numbering / Use AMS Numbering Rules /
  Auto Numbering for All Math Equations); When copy / export as HTML
  (without style): Use Rendered SVG / Use LaTeX Source. Whitespace / Line
  Break: Indent first line of paragraphs; When Writing: Visible `<br/>`,
  Preserve / Ignore sequential whitespace and single line break; Export /
  Print: the same choice.
- **Export.** A list on the left: General, PDF, HTML, HTML (without Styles),
  Image, then the pandoc formats (Word (.docx), OpenOffice, RTF, Epub,
  LaTeX, Media Wiki, reStructuredText, Textile, OPML) with Add / Delete /
  Rename / Move Up / Move Down. General: Default Folder for Exported File
  (Auto / Same folder with current file / Custom location), Pandoc Path
  ((Auto Detect)), After Export (Open exported file location). PDF: Paper
  Size (A5, A4, A3, US Letter, US Legal, Tabloid, Custom; Margin Default /
  Custom top, bottom, left, right in mm, 16 by default; Width x Height in
  mm), Page Break Between Top Headings, Theme, Header and Footer (None, or
  text with `${title}`, `${author}`, `${pageNo}`, `${totalPages}`,
  `${today}`), Author, Append Extra Content (HTML), Read and overwrite export
  settings from YAML front matters, After Export. HTML: Include Outline
  Sidebar, Theme, Append in `<head />`, Append in `<body />`, YAML. Image:
  Width (640px), Image Quality (Auto / Medium / High / Best), Font Size (Use
  custom font size 24px / Use theme font size), Theme. Pandoc formats:
  Arguments and per-format extras (Style Reference for Word, Template, Custom
  CSS and chapter level for Epub).
- **Appearance.** Font Size: Auto ( Recommended ) / Customized (9–24px).
  Zoom: −3 … 6 with Reset to Default; Zoom when using mouse wheel and
  holding Ctrl key. Status Bar: Show status bar. Word Count: Always Show
  Word Count; Reading Speed (words / min, 120 / 240 / 382, Reset to
  Default). Toolbar: Show editor toolbar. Themes: Theme; Open Theme Folder;
  Get Themes.
- **General.** Language ((applied after restart)). Update. Shortcut Keys:
  Custom Shortcut Keys. Dialogs: Reset All Dialog Warnings ("Dialog warnings
  have been reset."). Advanced Settings: Enable Debug, Send Anonymous Usage
  Info, Open Advanced Settings (`conf.user.json`, where `keyBinding` maps
  menu labels to keys), Reset Advanced Settings.
