Typo 2.4.0 renames folders from the file tree.

## Download

| Platform | File | How to start it |
| --- | --- | --- |
| Linux, x86-64 | `typo-2.4.0-linux-x86_64.tar.gz` | Unpack it and run `./install.sh`. Then open Typo from the app launcher, or run `typo`. |
| macOS, Apple silicon | `typo-2.4.0-macos-arm64.zip` | Unzip it and move `Typo.app` to Applications. The app is not notarized, so right-click it and choose **Open** the first time. |
| Windows, x86-64 | `typo-2.4.0-windows-x86_64.zip` | Unzip it and run `typo.exe`. The app is not signed, so Windows SmartScreen can warn you. Choose **More info**, then **Run anyway**. |

There is no Android APK.

In Typo 2.0.0 or later, select **Update** at the top right to install this version. Quire 1.0.0 and 1.1.0 can't install Typo themselves: see the [2.0.0 notes](https://github.com/kurtlieber/typo/releases/tag/v2.0.0) to move from Quire.

## Changed since 2.3.0

- **Rename folder** is on a folder's right-click menu. The folder's row turns into a field with its name selected: type the new name and press `Enter`, or `Escape` to leave it.
- Everything in the folder moves with it, folders inside it too. Open notes keep their tabs, carets and undo history, and pins stay.
- Links that spell out the folder, such as `[[Work/Plan]]`, follow it when **Update links when renaming** is on (Settings). Links by note name alone, such as `[[Plan]]`, still find the note.
- A name another folder already has leaves both as they are, with a notice.
- A new test checks Typo's Markdown against all 672 examples of the GitHub Flavored Markdown spec. 583 match. The other 89 are differences Typo makes on purpose, each listed with its reason, such as raw HTML showing as text.
