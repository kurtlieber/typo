Typo 2.3.0 adds nested lists: `Tab` and `Shift+Tab` move a list item a level in or out.

## Download

| Platform | File | How to start it |
| --- | --- | --- |
| Linux, x86-64 | `typo-2.3.0-linux-x86_64.tar.gz` | Unpack it and run `./install.sh`. Then open Typo from the app launcher, or run `typo`. |
| macOS, Apple silicon | `typo-2.3.0-macos-arm64.zip` | Unzip it and move `Typo.app` to Applications. The app is not notarized, so right-click it and choose **Open** the first time. |
| Windows, x86-64 | `typo-2.3.0-windows-x86_64.zip` | Unzip it and run `typo.exe`. The app is not signed, so Windows SmartScreen can warn you. Choose **More info**, then **Run anyway**. |

There is no Android APK.

In Typo 2.0.0 or later, select **Update** at the top right to install this version. Quire 1.0.0 and 1.1.0 can't install Typo themselves: see the [2.0.0 notes](https://github.com/kurtlieber/typo/releases/tag/v2.0.0) to move from Quire.

## Changed since 2.2.1

- **`Tab` on a list item nests it** under the item above it. The item moves to that item's text column, as GitHub Flavored Markdown reads it: two spaces under `- `, three under `1. `. The caret can be anywhere on the line.
- **`Shift+Tab` moves an item back out** to its parent's level.
- The lines under an item, its own sub-items and text, move with it.
- A numbered item takes the number of its new place: a first sub-item starts at 1, and an item moved out counts on from its parent.
- **`Enter` on an empty sub-item moves it out a level.** On a top-level item it still ends the list.
- With several lines selected, `Tab` and `Shift+Tab` move them all, list or not. Before, `Tab` replaced the selection with two spaces.
- A first item, with nothing above it to nest under, stays where it is. Outside lists, and in code blocks, `Tab` still types two spaces.
- The toolbar's list, task and number buttons keep a sub-item's indent. Before, they put a second marker in front of it. Numbers count each level on its own.
- A pasted numbered list with a list inside it keeps the inner list under the item's text.
