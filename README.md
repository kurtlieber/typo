# typo

A native markdown editor for Omarchy Linux, mirroring Typora's seamless
live-preview editing. Rust + Qt 6 / QML. Follows the live Omarchy theme.

```sh
cargo run --release -- notes.md
```

Building needs Qt 6 and the `typst` command (`pacman -S qt6-base
qt6-declarative typst`); typst generates the LaTeX command table for math.

To install it as a package, so `typo` is on the PATH and in the Omarchy
launcher (Super + Space):

```sh
bin/install
```

On a Mac (Apple silicon), with `brew install qt typst` (and `pandoc` for
Import… and the exports it writes), this builds `typo.app` and its `.dmg`
and copies the app to /Applications:

```sh
bin/install-mac
```

There the menus are the Mac's and the keys Typora Mac's (⌘ for Ctrl, plus
its own chords, such as ⌥⌘C for Code Fences).

## Shortcuts

Typora's, from the menus. Preferences ▸ General ▸ Advanced Settings opens
`~/.config/typo/conf.user.json`, where `keyBinding` changes them by menu
label, as in Typora:

```json
{ "keyBinding": { "Strong": "Ctrl+Alt+B", "Underline": "" } }
```

Alt shows the menu bar when it is hidden; Ctrl+click follows a link.

### File

| Command | Keys |
| --- | --- |
| New | `Ctrl+N` |
| New Window | `Ctrl+Shift+N` |
| Open… | `Ctrl+O` |
| Open Quickly… | `Ctrl+P` |
| Reopen Closed File | `Ctrl+Shift+T` |
| Save | `Ctrl+S` |
| Save As… | `Ctrl+Shift+S` |
| Print… | `Alt+Shift+P` |
| Preferences… | `Ctrl+,` |
| Close | `Ctrl+W` |
| Quit | `Ctrl+Q` |

### Edit

| Command | Keys |
| --- | --- |
| Undo | `Ctrl+Z` |
| Redo | `Ctrl+Y` |
| Cut | `Ctrl+X` |
| Copy | `Ctrl+C` |
| Paste | `Ctrl+V` |
| Copy as Markdown | `Ctrl+Shift+C` |
| Paste as Plain Text | `Ctrl+Shift+V` |
| Select All | `Ctrl+A` |
| Select Line / Sentence | `Ctrl+L` |
| Select Styled Scope | `Ctrl+E` |
| Select Word | `Ctrl+D` |
| Delete Word | `Ctrl+Shift+D` |
| Jump to Top | `Ctrl+Home` |
| Jump to Selection | `Ctrl+J` |
| Jump to Bottom | `Ctrl+End` |
| Find | `Ctrl+F` |
| Find Next | `F3` |
| Find Previous | `Shift+F3` |
| Replace | `Ctrl+H` |
| Search in the sidebar | `Ctrl+Shift+F` |

### Paragraph

| Command | Keys |
| --- | --- |
| Heading 1 | `Ctrl+1` |
| Heading 2 | `Ctrl+2` |
| Heading 3 | `Ctrl+3` |
| Heading 4 | `Ctrl+4` |
| Heading 5 | `Ctrl+5` |
| Heading 6 | `Ctrl+6` |
| Paragraph | `Ctrl+0` |
| Increase Heading Level | `Ctrl+=` |
| Decrease Heading Level | `Ctrl+-` |
| Insert Table | `Ctrl+T` |
| Add Row Below | `Ctrl+Return` |
| Delete Row | `Ctrl+Shift+Backspace` |
| Code Fences | `Ctrl+Shift+K` |
| Math Block | `Ctrl+Shift+M` |
| Quote | `Ctrl+Shift+Q` |
| Ordered List | `Ctrl+Shift+[` |
| Unordered List | `Ctrl+Shift+]` |
| Task List | `Ctrl+Shift+X` |
| Indent | `Ctrl+[` |
| Outdent | `Ctrl+]` |

### Format

| Command | Keys |
| --- | --- |
| Strong | `Ctrl+B` |
| Emphasis | `Ctrl+I` |
| Underline | `Ctrl+U` |
| Code | `` Ctrl+Shift+` `` |
| Strike | `Alt+Shift+5` |
| Hyperlink | `Ctrl+K` |
| Insert Image | `Ctrl+Shift+I` |
| Clear Format | `Ctrl+\` |

### View

| Command | Keys |
| --- | --- |
| Toggle Sidebar | `Ctrl+Shift+L` |
| Outline | `Ctrl+Shift+1` |
| Articles | `Ctrl+Shift+2` |
| File Tree | `Ctrl+Shift+3` |
| Source Code Mode | `Ctrl+/` |
| Focus Mode | `F8` |
| Typewriter Mode | `F9` |
| Toggle Fullscreen | `F11` |
| Actual Size | `Ctrl+Shift+0` |
| Zoom In | `Ctrl+Shift+=` |
| Zoom Out | `Ctrl+Shift+-` |
| Switch Between Opened Documents | `Ctrl+Tab` |

`third_party/mitex` holds Typst files from
[mitex](https://github.com/mitex-rs/mitex) under the Apache License 2.0.
