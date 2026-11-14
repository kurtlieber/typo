Typo 2.1.0 adds JetBrains Mono as a fourth editor font.

## Download

| Platform | File | How to start it |
| --- | --- | --- |
| Linux, x86-64 | `typo-2.1.0-linux-x86_64.tar.gz` | Unpack it and run `./install.sh`. Then open Typo from the app launcher, or run `typo`. |
| macOS, Apple silicon | `typo-2.1.0-macos-arm64.zip` | Unzip it and move `Typo.app` to Applications. The app is not notarized, so right-click it and choose **Open** the first time. |
| Windows, x86-64 | `typo-2.1.0-windows-x86_64.zip` | Unzip it and run `typo.exe`. The app is not signed, so Windows SmartScreen can warn you. Choose **More info**, then **Run anyway**. |

There is no Android APK.

In Typo 2.0.0, select **Update** at the top right to install this version. Quire 1.0.0 and 1.1.0 can't install Typo themselves: see the [2.0.0 notes](https://github.com/kurtlieber/typo/releases/tag/v2.0.0) to move from Quire.

## New since 2.0.0

- **JetBrains Mono** is an editor font. To use it, open Settings, then Editor, and select it. Source mode and the code in the preview use it. As with the other fonts, the editor turns its ligatures off, so `!=` and `->` show as typed.
- Settings, Editor shows the four fonts in two rows.

Typo bundles JetBrains Mono 2.304 from [JetBrains](https://github.com/JetBrains/JetBrainsMono), under the SIL Open Font License, so it doesn't need to be installed on the system.
