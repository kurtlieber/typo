Typo 2.6.3 fixes a sync bug that made conflict copies of notes you edited on one device only.

## Download

| Platform | File | How to start it |
| --- | --- | --- |
| Linux, x86-64 | `typo-2.6.3-linux-x86_64.tar.gz` | Unpack it and run `./install.sh`. Then open Typo from the app launcher, or run `typo`. |
| macOS, Apple silicon | `typo-2.6.3-macos-arm64.zip` | Unzip it and move `Typo.app` to Applications. The app is not notarized, so right-click it and choose **Open** the first time. |
| Windows, x86-64 | `typo-2.6.3-windows-x86_64.zip` | Unzip it and run `typo.exe`. The app is not signed, so Windows SmartScreen can warn you. Choose **More info**, then **Run anyway**. |

There is no Android APK.

In Typo 2.0.0 or later, select **Update** at the top right to install this version. Quire 1.0.0 and 1.1.0 can't install Typo themselves: see the [2.0.0 notes](https://github.com/kurtlieber/typo/releases/tag/v2.0.0) to move from Quire.

## Changed since 2.6.2

- **No false conflicts while you type.** When another device synced in the moment between Typo reading the remote file list and downloading a note, Typo recorded the older version as the one it had. The next sync then took that older text for an edit on this device: it kept the older text, sent it up over your newer one, and moved your newer text to a `(conflict <device>)` copy. Typo now records the version it actually downloaded. A device that syncs often, such as `typo sync` on a timer, hit this most while you typed on another device.
- **A Mac syncs under its own name.** Conflict copies and the sync record named every Mac `desktop`. Typo now uses the Mac's local host name, from System Settings, General, Sharing.
