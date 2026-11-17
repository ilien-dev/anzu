# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

People who buy books on Kindle and want to read them as EPUB on other readers. The app
is meant to be published: whoever opens it may not be technical and does not know the
tools underneath. Most will have 1 to 20 books downloaded at a time; some will have
hundreds, and the list must stay manageable for them.

## Product Purpose

Anzu converts books downloaded with the Microsoft Store Kindle app to EPUB, without
the user having to look for tools, check versions or touch folders. Success: open the
app, see the books with their titles, choose which ones, and get the EPUB files in the
chosen folder.

## Positioning

Anzu refuses to act when it cannot guarantee that it is safe: before each unlock it
checks that the tool is the latest published release, that its hash matches and that
it supports the installed Kindle version. If something does not add up, it explains
why and does nothing. The KFX to EPUB converter is its own code.

## Operating Context

- Windows 10 or 11, with the Amazon Kindle app from the Microsoft Store installed and
  signed in.
- Unlocking is done by Satsuoni's tool, which Anzu downloads and runs as an external
  process. Anzu contains no decryption code.
- The Kindle app must be closed during unlocking; Anzu asks for permission before
  closing it.
- A connection is needed to check the tool. Converting books that are already unlocked
  does not need one.

## Capabilities and Constraints

- Native desktop application for Windows, written in Rust with `egui`. There is no
  HTML and no browser: the `web` value under Platform only means that it does not
  follow the iOS or Android guidelines.
- Book states: with DRM, unlocked, converted.
- A book's title comes from the Kindle app's catalog, or from the book once unlocked;
  a locked book with no title in the catalog shows its ASIN.
- Converts text books. Comics, fixed layout and dictionaries are out of scope.
- The interface is in English by default and also available in Spanish.
- Light and dark theme: it follows the system or the user sets it.
- The content has a maximum width and is centred on large monitors.
- The books sit on shelves by what they still need (to unlock, ready for EPUB, in EPUB) and
  sort by title, ASIN or size. Undecided: search for libraries of hundreds of books;
  cancelling a run in progress.

## Brand Commitments

- Name: Anzu, after the Mesopotamian bird that stole the Tablet of Destinies.
- Logo: an A monogram formed by two open wings over a tablet, each part in its own colour:
  a lilac wing, a peach wing and a butter tablet, on an ink tile.
- Style chosen by the user: “Coloured paper” (proposal 2A of the redesign canvas): ink on
  cool white or light ink on night, four pastels that name the state of a book, Space
  Grotesk headings over Inter text, soft corners and many drawn icons. It replaced “Amber
  console”. The light theme must not be the typical yellowish background.

## Evidence on Hand

- One real book converted end to end (77 chapters, internal EPUB verification with no
  problems).
- Interface and logo proposals approved on canvas.
- There are no testimonials, usage figures or third-party screenshots; do not invent
  any.

## Product Principles

1. When in doubt, do not act, and explain why.
2. Never write to the Kindle folder or close the app without permission.
3. Account secrets are not shown and are not stored in logs.
4. A book that fails does not stop the others.
5. Always say what is going to happen before it happens.
