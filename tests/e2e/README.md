# End-to-end tests

Each `NAME.script` drives the real app offscreen; `bin/e2e` runs it on an
empty document with the caret in the first block, saves the document as
markdown with `dump @OUT@`, and compares that with `NAME.md`.

Script steps, one per line:

| step | does |
| --- | --- |
| `type TEXT` | types TEXT one key per tick; `\n` presses Enter, `\t` Tab |
| `key CHORD…` | presses chords such as `Ctrl+B`, `Shift+Return`, `Up` |
| `click X Y [MODS]` | left-clicks at window coordinates, holding MODS (`Shift`, `Ctrl+Shift`…); `Right` in MODS clicks the right button |
| `clickText ROW POS [MODS]` | clicks at UTF-16 position POS in block ROW |
| `clickCell ROW R C POS [MODS]` | clicks at POS in cell (R, C) of the table in block ROW; row 0 is the header |
| `wheel X Y DY [MODS]` | turns the wheel at window coordinates, DY in eighths of a degree (120 a notch, positive away from the user), holding MODS |
| `hoverText ROW POS` | moves the pointer onto UTF-16 position POS in block ROW, no button held |
| `clickItem EXPR` | clicks the middle of the item a JavaScript expression names (`view.itemAt(0).table.button("trash")`) |
| `drag ROW POS ROW POS` | drags with the left button between two text positions |
| `mouse press\|move\|release EXPR` | a step of a drag with the left button, at the window point a JavaScript expression gives (`view.itemAt(0).table.handlePoint("row", 2)`); a move before a press hovers |
| `clipboard TEXT` | puts TEXT on the clipboard (`\n` for a line break) |
| `clipboardHtml HTML` | puts HTML on the clipboard, with its text as the plain form |
| `clipboardFiles PATH…` | puts files (space apart) on the clipboard, as a file manager copies them |
| `clipboardImage PATH` | puts the picture in PATH on the clipboard |
| `dropFiles ROW POS PATH…` | drops files (space apart) at UTF-16 position POS in block ROW |
| `dragFiles over\|drop\|leave PATH[\|PATH…] EXPR` | files dragged in from another program, over or dropped at the window point a JavaScript expression gives (`sidebar.fileTree.pointOf("@DIR@/notes/box")`), or taken away |
| `saveClipboard PATH` | writes the clipboard's text to PATH |
| `caret ROW POS` | puts the caret at UTF-16 position POS in block ROW |
| `eval EXPR` | logs a JavaScript expression's value (debugging) |
| `check EXPR` | fails the test unless a JavaScript expression is true (`Desktop.exists("@DIR@/a.svg")`) |
| `cmd NAME [ARG]` | runs a menu command, as `app.run(NAME, ARG)` |
| `wait MS` | lets the UI settle |
| `until MS EXPR` | waits until a JavaScript expression is true, MS at most (a `check` after it tells) |
| `dump PATH` | writes the document's markdown to PATH |
| `write PATH TEXT` | writes TEXT to PATH (`\n` for a line break), as another program would |
| `choose PATH` | answers the next file chooser with PATH (`\n` between several); an empty PATH cancels it |
| `answer N` | presses button N of the open dialog (0 is the default; the last is Cancel) |
| `shot PATH` | saves a screenshot |
| `serve FOLDER` | serves FOLDER's files over HTTP on 127.0.0.1; `@PORT@` in later steps is its port |
| `# …` | a comment |

`@OUT@` is the test's output file, `@DIR@` the folder it is in and
`@REPO@` the checkout (for fixtures such as `hidpi.png`, a 144-DPI PNG). A
pasted picture's time-stamped name reads `image-STAMP` in the output.

Each test runs in a folder of its own, with throwaway settings and data
directories, so one test never sees what another left behind, and recent
files and drafts never touch the real ones.

`bin/e2e --update NAME` rewrites `NAME.md` from the current output, and
`KEEP=1 bin/e2e NAME` keeps each test's log and `shot @SHOT@` screenshot.
