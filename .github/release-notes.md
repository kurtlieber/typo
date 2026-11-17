Typo 2.6.0 adds a right-click menu to the file tabs, with find and replace, split panes and note windows.

## Download

| Platform | File | How to start it |
| --- | --- | --- |
| Linux, x86-64 | `typo-2.6.0-linux-x86_64.tar.gz` | Unpack it and run `./install.sh`. Then open Typo from the app launcher, or run `typo`. |
| macOS, Apple silicon | `typo-2.6.0-macos-arm64.zip` | Unzip it and move `Typo.app` to Applications. The app is not notarized, so right-click it and choose **Open** the first time. |
| Windows, x86-64 | `typo-2.6.0-windows-x86_64.zip` | Unzip it and run `typo.exe`. The app is not signed, so Windows SmartScreen can warn you. Choose **More info**, then **Run anyway**. |

There is no Android APK.

In Typo 2.0.0 or later, select **Update** at the top right to install this version. Quire 1.0.0 and 1.1.0 can't install Typo themselves: see the [2.0.0 notes](https://github.com/kurtlieber/typo/releases/tag/v2.0.0) to move from Quire.

## Changed since 2.5.0

Right-click a tab for its menu:

- **Close**, **Close others**, **Close tabs after** and **Close all**.
- **Move to new window** and **Open in new window** open the note in a window of its own, with its title, its editor and the edit and preview toggle. Move closes the tab. The main window saves each edit, and a change made there or by sync shows in the note window at once. Note windows don't open again when Typo starts. A locked note stays in the main window.
- **Split right** and **Split down** show the note beside the active note, or under it. Selecting a tab opens it in the pane you used last, and each pane has a close button.
- **Find…** (`Ctrl+F`) and **Replace…** (`Ctrl+H`, `⌥⌘F` on macOS) open a bar at the top right of the note. It finds plain text, with **Aa** to match case. **All** replaces every match in one step, which one undo takes back. In the preview, find switches the note to source.
- **Copy path** copies the note's absolute path, or a Markdown link to it: `[Name](Folder/Name.md)`.
- **Delete file** moves the note to `.trash`, as **Delete note** in the file tree does.
