# Anzu

Español: [README.es.md](README.es.md)

Kindle to EPUB in a single Windows app, written in Rust.

Anzu finds the books you downloaded with the Kindle app, checks that the unlock tool
is the latest release and supports your Kindle version, runs it, and converts the
result to EPUB. You do not have to hunt for downloads or keep track of versions.

The name comes from Anzu, the Mesopotamian bird that stole the Tablet of Destinies.

## Requirements

- Windows 10 or 11.
- The **Amazon Kindle** app from the Microsoft Store, signed in and with your books
  downloaded.
- An internet connection, to check the tool before each unlock.

## Use

Open `anzu.exe`. The window shows:

- **System status:** whether it is ready to unlock, with the Kindle version and the
  tool version.
- **Your books**, in a table you can sort by any column. Each book's status is a
  three-stage track: with DRM, unlocked, EPUB ready.
- **Destination folder** and a button that says what is about to happen, for example
  “Unlock and convert 2 books”.

A book's real title is only known once it is unlocked; until then Anzu shows its ASIN.
If the Kindle app is open, Anzu asks for permission before closing it.

The theme follows the system, or you can set it to light or dark from the header.

The interface is available in English (the default) and Spanish. You switch language
from the header, next to the theme switch.

It can also convert an already unlocked book from the terminal:

```
anzu convert book.kfx-zip -o C:\Books
```

## What it does and what it does not

| Part | Who does it |
|---|---|
| Detecting Kindle and the books | Anzu |
| Checking the latest release and compatibility | Anzu |
| Downloading the tool and verifying its SHA-256 hash | Anzu |
| Removing the DRM | [Satsuoni](https://github.com/Satsuoni/DeDRM_tools)'s tool, as an external process |
| Reading the KFX format and writing the EPUB | Anzu, with its own code |

Anzu contains no decryption code.

### When unlocking is blocked

Anzu refuses to unlock, and explains why, when:

- your Kindle app is newer than the latest published tool;
- it cannot tell for certain whether they are compatible;
- there is no connection to check;
- the download comes without a SHA-256 hash, or the hash does not match.

The cached tool is checked again on every use: if the published hash changes, or the
hash of any of its executables does, it is downloaded again.

Books that are already unlocked can always be converted.

### What the converter covers

Text books: chapters, headings, italics and bold, internal and external links, images,
cover, table of contents and metadata. Lists and tables keep their text but lose their
structure. Comics, fixed layout and dictionaries are not supported. Anything the
converter does not recognise leaves a warning in the log; it does not stop the
conversion.

## Where it stores things

In `%LOCALAPPDATA%\anzu\`:

| Path | Contents |
|---|---|
| `tool\<version>\` | The downloaded tool and its hash manifest |
| `unlocked\<ASIN>.kfx-zip` | Unlocked copies of your books |
| `keys\anzu.k4i`, `work\` | **Secrets of your Amazon account** |
| `settings.json` | Destination folder, theme, language, titles and converted books |

`keys\` and `work\` are as sensitive as a password: do not share them or upload them
anywhere. The unlock tool is a third-party program and may leave data of its own
outside this folder; Anzu has no control over that.

The tool prints account secrets while it works. Anzu only lets the progress lines it
knows through to the log, hides any long string inside them, and replaces everything
else with a notice.

Anzu never writes to the Kindle app's folder.

## Known limitations of version 0.1.0

- The full path from the window (unlock and convert by pressing the button) has not
  been tested from start to finish; each part has been tested separately, with a real
  book.
- The EPUB files have not been validated with `epubcheck`; their internal integrity is
  checked.
- A run in progress cannot be cancelled. If the tool stops responding, Anzu stops it
  after ten minutes without activity.
- The tool unlocks every DRM-protected book in the folder, not only the selected ones;
  books that were already unlocked are not redone.
- The compatibility rule is inferred from the name of the tool's executable. If a
  future release changes that name, Anzu will block until it is updated.
- The executable does not have its own icon in Explorer yet; the window does.

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

`PRODUCT.md` and `.impeccable/` hold the product context and the design direction of
the interface.

## Credits

- **Satsuoni**, for the unlock tool.
- **John Howell** (jhowell), whose KFX Input plugin is the reference for the KFX
  format.
- **JetBrains Mono**, the interface typeface, under the SIL Open Font License 1.1
  (`crates/anzu/assets/fonts/OFL.txt`).

## License and legal notice

GPL-3.0-or-later. Anzu is not endorsed by Amazon.

Removing DRM may be restricted where you live. Use it only with books you have bought,
and to read them on your own devices.
