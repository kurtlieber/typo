Typo 2.2.0 adds `typo sync`, which syncs a notes folder with no window, for a server without a desktop.

## Download

| Platform | File | How to start it |
| --- | --- | --- |
| Linux, x86-64 | `typo-2.2.0-linux-x86_64.tar.gz` | Unpack it and run `./install.sh`. Then open Typo from the app launcher, or run `typo`. |
| macOS, Apple silicon | `typo-2.2.0-macos-arm64.zip` | Unzip it and move `Typo.app` to Applications. The app is not notarized, so right-click it and choose **Open** the first time. |
| Windows, x86-64 | `typo-2.2.0-windows-x86_64.zip` | Unzip it and run `typo.exe`. The app is not signed, so Windows SmartScreen can warn you. Choose **More info**, then **Run anyway**. |

There is no Android APK.

In Typo 2.0.0 or 2.1.0, select **Update** at the top right to install this version. Quire 1.0.0 and 1.1.0 can't install Typo themselves: see the [2.0.0 notes](https://github.com/kurtlieber/typo/releases/tag/v2.0.0) to move from Quire.

## New since 2.1.0

- **`typo sync`** syncs a notes folder once and exits, with no window. It reads the sync settings from Typo's state file, as the app does, and syncs the Documents folder unless you name another with `--folder`. Run it from a timer, such as a systemd timer every 5 minutes, to keep the same notes on a server. It keeps both versions of a note that changed on two devices, and moves a note deleted on another device to `.trash`, as the app does.
- `typo --help` lists the new command.
