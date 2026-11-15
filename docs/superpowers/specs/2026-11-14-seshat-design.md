# Seshat: Kindle to EPUB in a single app

> Historical record. Written when the project was called Seshat; it was later renamed Anzu and parts of the design changed.

Date: 2026-11-14. Status: approved by the user (direction and interface).

## Intent

A desktop app for Windows, written in Rust, that converts the Kindle books the user
has bought to EPUB without the user having to look for tools, check versions or
touch folders. The user opens the app, sees their books, picks a destination
folder and presses a button.

## Goals

- Detect the Microsoft Store Kindle app, its version and its books on its own.
- Before each unlock, query the latest release of `Satsuoni/DeDRM_tools`,
  download it if needed and verify its hash.
- Block unlocking when the tool does not support the installed Kindle version,
  and explain why.
- Show the state of each book: with DRM, unlocked, converted.
- Convert DRM-free KFX to EPUB 3 with its own Rust code.
- A single `.exe`, instant start.

## Out of scope

- Writing its own decryption code. Unlocking is done by Satsuoni's archiver,
  which the app downloads and runs as an external process.
- Comics, fixed layout, Print Replica, dictionaries.
- Classic MOBI/AZW3, macOS, Linux.
- Library management (editing metadata, sending to devices).

## What would make the project fail

- Running the unlock with incompatible versions, or without being able to tell.
- Running a downloaded binary whose hash does not match.
- Closing the Kindle app without the user's confirmation.
- Touching or deleting the original files in the Kindle folder.

## Facts verified on the user's machine (2026-11-14)

- Package: `AMZNKindle.AmazonKindleReadingApp`, version `1.1.5634.0`,
  family `AMZNKindle.AmazonKindleReadingApp_m1sc522ngdk36`. Process: `Kindle`.
- Content: `%LOCALAPPDATA%\Packages\<family>\LocalState\Classic\Content\`,
  with `book_asset.db` and one `<ASIN>_EBOK` folder per book. Inside:
  `<ASIN>_EBOK.azw` (encrypted KFX, header `\xeaDRMION\xee`), an
  `amzn1.drm-voucher.v1.<uuid>.voucher`, a `.phl`, an `.azw.md` and several
  `.azw.res`.
- Test book: `B0GRCGCPMZ`.
- API: `GET https://api.github.com/repos/Satsuoni/DeDRM_tools/releases`.
  Every release has `prerelease: true`, so `/releases/latest` is no use: the
  first one in the list is taken. Each asset carries a `digest`
  (`sha256:...`).
- `v10.0.35` (2026-11-08): `DeDRM_tools.zip`, sha256
  `5c85cce954b17836c2b421d6ae8b313f0aaeb190a9a3f81881636dbc80e25bc3`. It contains
  `MSIXKFXArchiver_x64_1_5634.exe` and `MSIXKFXArchiver1_23620_final.exe`.
  Release note: "Support for 1.5634 MSIX Kindle added".
- Known invocation of the archiver (according to `jaltez/kindle2epub`, with an
  earlier version): `archiver.exe <content_folder> <output_folder> <path.k4i>`.
  It produces one DRM-free `.kfx-zip` per book. **Not verified for the x64
  version**: it has to be confirmed in the source code of Satsuoni's repository
  before implementing the unlock step.

## Approaches considered

1. **Fork of `kindle2epub` (Python).** Quick, but the user ruled it out: they
   want their own tool, and a fast one.
2. **Rust with its own decryption.** Not viable: the keys are in the Kindle app,
   protected by the TPM. Besides, it is circumvention code that is not going to
   be written.
3. **Rust that orchestrates Satsuoni's archiver and converts with its own
   code.** Chosen. It automates everything the user asked for and keeps the
   delicate part in the tool that already solves it.

## User decisions

- Rust as the language, a project from scratch, in `%USERPROFILE%\Documents\Projects\`.
- Graphical interface with the three approved screens: library, blocked,
  progress. Mockup: (a private mockup, not included in this repository)
- Automatic download of the tool, a latest-release check before each unlock,
  blocking if it is not compatible.
- Automatic detection of the books and of which ones are unlocked; a
  destination folder the user can choose.

## Author decisions (open to correction)

- Name `seshat`. License GPL-3.0-or-later (compatible with the sources of
  knowledge about the KFX format).
- GUI with `eframe`/`egui`. Dark theme as in the mockup.
- Cargo workspace with three crates:
  - `seshat-kfx`: binary Ion reader, KFX container, `.kfx-zip`, and EPUB 3
    generator. No Windows dependencies. With tests.
  - `seshat-core`: Kindle detection, GitHub releases, compatibility, verified
    download, running the archiver, library state.
  - `seshat` (binary): `egui` interface. It also accepts
    `seshat convert <file> -o <folder>` to use the converter without the GUI.
- Compatibility rule, inferred from the name of the executable in the zip:
  - `MSIXKFXArchiver_x64_1_<N>.exe` supports Kindle `1.1.<M>` with `M <= N`.
  - `MSIXKFXArchiver1_<N>_final.exe` supports Kindle `1.0.<M>` with `M <= N`.
  - Any other case, or a name that cannot be interpreted: blocked.
- Integrity: the zip is verified against the `digest` given by the GitHub API.
  Without a `digest`, it is not run.
- App data in `%LOCALAPPDATA%\seshat\`: cached tool, unlocked copies
  (`unlocked\<ASIN>.kfx-zip`), `.k4i` keyfile, settings and log. Nothing is ever
  written to the Kindle folder.
- State of a book: "converted" if the EPUB recorded in the settings exists;
  "unlocked" if its `.kfx-zip` exists; otherwise, "with DRM".
- Kindle version: obtained with `Get-AppxPackage` via PowerShell.
- The app asks for confirmation before closing Kindle.
- Book title: the ASIN is shown until the book is unlocked; if the `.azw.md`
  turns out to be readable without DRM, it is used for the title from the start.

## KFX to EPUB converter (initial scope)

Reflowable text novels: chapters, paragraphs, headings, emphasis, internal
links, images, cover, table of contents, metadata (title, author, language,
ASIN) and basic styles as CSS. Anything not recognised is skipped with a
warning in the log; it must never abort the conversion of a whole book.

## Verification

- Unit tests of the Ion reader and of the container with hand-built data.
- Real test: unlock the test book, convert it and validate the result with
  `epubcheck`. Compare chapters and word count with the Calibre + KFX Input
  conversion if it is available.

## Left for the implementer

- Confirm the arguments and output of the x64 archiver.
- Source of the shared symbol table `YJ_symbols` needed to read KFX.
- Detail of the mapping from KFX styles to CSS.
