Typo 2.6.1 lets you select and copy text in the preview.

## Download

| Platform | File | How to start it |
| --- | --- | --- |
| Linux, x86-64 | `typo-2.6.1-linux-x86_64.tar.gz` | Unpack it and run `./install.sh`. Then open Typo from the app launcher, or run `typo`. |
| macOS, Apple silicon | `typo-2.6.1-macos-arm64.zip` | Unzip it and move `Typo.app` to Applications. The app is not notarized, so right-click it and choose **Open** the first time. |
| Windows, x86-64 | `typo-2.6.1-windows-x86_64.zip` | Unzip it and run `typo.exe`. The app is not signed, so Windows SmartScreen can warn you. Choose **More info**, then **Run anyway**. |

There is no Android APK.

In Typo 2.0.0 or later, select **Update** at the top right to install this version. Quire 1.0.0 and 1.1.0 can't install Typo themselves: see the [2.0.0 notes](https://github.com/kurtlieber/typo/releases/tag/v2.0.0) to move from Quire.

## Changed since 2.6.0

- **Copy from the preview.** Drag across the preview to select text, double-click a word or triple-click a paragraph. The selection can run across headings, paragraphs, lists, tables and code blocks. `Cmd+C` (`Ctrl+C` on Linux and Windows) copies it, and `Cmd+A` (`Ctrl+A`) selects the whole note once part of it is selected. On macOS, a right-click offers **Copy**.
- What you copy is plain text: a blank line between paragraphs, `- ` or `1. ` before list items, `- [ ] ` or `- [x] ` before tasks, and a tab between table cells. Links still open on a click.
