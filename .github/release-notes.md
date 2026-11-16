Typo 2.2.1 makes sync use less processor time when your notes are encrypted.

## Download

| Platform | File | How to start it |
| --- | --- | --- |
| Linux, x86-64 | `typo-2.2.1-linux-x86_64.tar.gz` | Unpack it and run `./install.sh`. Then open Typo from the app launcher, or run `typo`. |
| macOS, Apple silicon | `typo-2.2.1-macos-arm64.zip` | Unzip it and move `Typo.app` to Applications. The app is not notarized, so right-click it and choose **Open** the first time. |
| Windows, x86-64 | `typo-2.2.1-windows-x86_64.zip` | Unzip it and run `typo.exe`. The app is not signed, so Windows SmartScreen can warn you. Choose **More info**, then **Run anyway**. |

There is no Android APK.

In Typo 2.0.0 or later, select **Update** at the top right to install this version. Quire 1.0.0 and 1.1.0 can't install Typo themselves: see the [2.0.0 notes](https://github.com/kurtlieber/typo/releases/tag/v2.0.0) to move from Quire.

## Changed since 2.2.0

- **Sync with Encrypt before upload on** works out the encryption keys once and keeps them in memory while Typo runs. Before, every sync worked them out again from the passphrase, which takes about a fifth of a second of processor time. With **On save**, Typo syncs every few seconds while you type, so on a laptop this adds up.
- The keys are never written to disk. A changed passphrase, or another remote folder, gets its own keys.
- The sync format is the same, so 2.2.1 syncs with every earlier version.
- `typo sync` starts anew on each run, so it works the keys out once per run, as before.
