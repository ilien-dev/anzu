# Anzu: technical overview

The [README](../README.md) explains Anzu in plain words. This page is for anyone who wants
the details: how it installs and updates, when it refuses to unlock, what the converter
handles, where it keeps its files, and how the code is laid out.

Read the [responsible use notice](../README.md#use-it-responsibly) first.

## How the parts fit together

| Part | Who does it |
|---|---|
| Detecting the Kindle app and the books | Anzu |
| Checking the latest release of the unlock tool and its compatibility | Anzu |
| Downloading the tool and verifying its SHA-256 hash | Anzu |
| Removing the DRM | [Satsuoni](https://github.com/Satsuoni/DeDRM_tools)'s tool, as an external process |
| Reading the KFX format and writing the EPUB | Anzu, with its own code |

Anzu contains no decryption code, and it does not need Calibre: the KFX to EPUB
converter is part of Anzu.

## Install and updates

The release is a single file, `anzu-<version>-windows-x64-setup.exe`. It is `anzu.exe`
itself: under a name that contains `setup` it shows the install window, copies itself into
the chosen folder (`%LOCALAPPDATA%\Programs\Anzu` by default), creates the shortcuts and
registers Anzu in the apps of Windows under `HKCU`. No administrator rights are needed. The
language chosen in the installer is the one Anzu opens in.

The installer is not signed yet, so Windows SmartScreen may warn the first time.

Uninstalling from Settings › Apps asks first, and asks separately whether to delete the
data folder.

While it is open, Anzu checks for a new version 30 seconds after starting and then every
hour, with one small request to GitHub. When there is one it offers to download it, checks
its SHA-256 digest, and on “Restart and update” closes, lets the new version take its place
(`--update`) and opens again. Nothing is downloaded or installed unless you ask.

## The window

- **System status:** whether it is ready to unlock, with the Kindle version, the tool
  version and whether the checksum was verified.
- **Your shelf:** the books sit on shelves by what they still need (to unlock, ready for
  EPUB, in EPUB) and sort by title, ASIN or size. Each book's status is a three-stage
  track: with DRM, unlocked, EPUB ready.
- **Side panel:** the ticked books, the destination folder and a button that says what is
  about to happen, for example “Unlock and convert 2 books”.

A locked book shows the title the Kindle app's catalog has for it; one with no title there
shows its ASIN until it is unlocked. If the Kindle app is open, Anzu asks for permission
before closing it.

The theme follows the system or is set to light or dark. The interface is in English (the
default) and Spanish. Both settings are behind the cog in the header, together with the
version and “Check for updates”.

## Command line

An already unlocked book can be converted from the terminal:

```
anzu convert book.kfx-zip -o C:\Books
```

## When unlocking is blocked

Anzu refuses to unlock, and explains why, when:

- the Kindle app is newer than the latest published tool;
- it cannot tell for certain whether they are compatible;
- there is no connection to check;
- the download comes without a SHA-256 hash, or the hash does not match.

The cached tool is checked again on every use: if the published hash changes, or the hash
of any of its executables does, it is downloaded again.

Books that are already unlocked can always be converted.

## What the converter covers

Text books: chapters, headings, italics and bold, internal and external links, images,
cover, table of contents and metadata. Lists and tables keep their text but lose their
structure. Comics, fixed layout and dictionaries are not supported. Anything the converter
does not recognise leaves a warning in the log; it does not stop the conversion.

## Where it stores things

In `%LOCALAPPDATA%\anzu\`:

| Path | Contents |
|---|---|
| `tool\<version>\` | The downloaded tool and its hash manifest |
| `unlocked\<ASIN>.kfx-zip` | Unlocked copies of your books |
| `keys\anzu.k4i`, `work\` | **Secrets of your Amazon account** |
| `settings.json` | Destination folder, theme, language, titles and converted books |
| `updates\` | The installer of a new version, while it waits for the restart |

`keys\` and `work\` are as sensitive as a password: do not share them or upload them
anywhere. The unlock tool is a third-party program and may leave data of its own outside
this folder; Anzu has no control over that.

The tool prints account secrets while it works. Anzu only lets the progress lines it knows
through to the log (an allowlist), hides any long string inside them, and replaces
everything else with a notice.

Anzu never writes to the Kindle app's folder.

## Privacy and network

Anzu has no account, no telemetry and no ads. It makes three kinds of request, all to
GitHub:

| Request | When | What it gets |
|---|---|---|
| `api.github.com/repos/Satsuoni/DeDRM_tools/releases` | Before each unlock | The latest releases of the unlock tool, to check compatibility |
| The tool's release asset | When the cached tool is missing or out of date | The tool's archive, verified against its SHA-256 hash |
| `api.github.com/repos/ilien-dev/anzu/releases/latest` | 30 s after start, then every hour | Whether a new version of Anzu exists |

None of them carries your books, your keys or anything about your library. Converting
books that are already unlocked needs no connection at all.

## Known limitations

Recorded at version 0.1.0; see the [changelog](../CHANGELOG.md) for what has changed since.

- The full path from the window (unlock and convert by pressing the button) has not been
  tested from start to finish; each part has been tested separately, with a real book.
- The EPUB files have not been validated with `epubcheck`; their internal integrity is
  checked.
- A run in progress cannot be cancelled. If the tool stops responding, Anzu stops it after
  ten minutes without activity.
- The tool unlocks every DRM-protected book in the folder, not only the selected ones;
  books that were already unlocked are not redone.
- The compatibility rule is inferred from the name of the tool's executable. If a future
  release changes that name, Anzu will block until it is updated.

## Build

```
cargo build --release
cargo test --workspace
```

The executable ends up in `target\release\anzu.exe`.

## Structure

| Crate | Responsibility |
|---|---|
| `anzu-kfx` | Binary Ion reader and KFX container reader; book model; EPUB 3 writer |
| `anzu-core` | Kindle detection, GitHub releases, compatibility, verified download, running the tool, library |
| `anzu` | Interface (`egui`) and command line |
| `anzu-web` | The interface compiled to WebAssembly for the demo on the website |

The website lives in the `site/` submodule ([ilien-dev/anzu-site](https://github.com/ilien-dev/anzu-site)).
`PRODUCT.md` and `.impeccable/` hold the product context and the design direction of the
interface. `CONTRIBUTING.md` explains how to propose a change.

The logo is defined once, on a 64 x 64 grid, in `crates/anzu/src/ui/mark.rs`: the window,
the window icon and the icon embedded in the executable (`crates/anzu/build.rs`) all come
from it. The brand files and their rules are in `assets/brand/`.

## Credits

- **Satsuoni**, for the [unlock tool](https://github.com/Satsuoni/DeDRM_tools).
- **John Howell** (jhowell), whose KFX Input plugin is the reference for the KFX format.
- **Inter** and **Space Grotesk**, the interface typefaces, under the SIL Open Font License
  1.1 (`crates/anzu/assets/fonts/Inter-OFL.txt` and `SpaceGrotesk-OFL.txt`).
