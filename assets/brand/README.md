# Anzu brand

The Anzu logo is a solid block that turns into lines of text, the way a locked Kindle book
becomes an EPUB. The block is peach, the colour the app gives a book with DRM, and the lines
are mint, the colour of a book in EPUB. It always sits on an ink tile, like the app icon.

The drawing is defined once, on a 64 x 64 grid, in `crates/anzu/src/ui/mark.rs`. The app,
the `.exe` icon, the site (`site/src/components/Mark.astro`) and every file here use the
same numbers. If the shape changes, change `mark.rs` first.

The full brand book is `brand-book.html`.

## Files

| Folder | What | Use |
|---|---|---|
| `logo/anzu-symbol.svg` | The symbol in one ink (`currentColor`) | Engraving, one-colour print, icons in code |
| `logo/anzu-symbol-small.svg` | Small cut: three thicker lines | Any symbol under 22 px |
| `logo/anzu-symbol-color.svg` | Peach and mint on the ink tile | Default, on any ground |
| `logo/anzu-symbol-on-dark.svg` | Peach and mint, no tile | Dark grounds only |
| `logo/anzu-symbol-black.svg`, `-white.svg` | One ink, no tile | When colour is not available |
| `logo/anzu-horizontal*.svg` | Tile and name side by side | Headers, documents |
| `logo/anzu-stacked*.svg` | Tile above the name | Square spaces |
| `logo/anzu-wordmark.svg` | The name alone, outlined | Where the symbol is already shown |
| `logo/png/` | Symbol 64 to 1024 px, horizontal 800 and 1600 px, transparent | Places that take no SVG |
| `web/` | Favicons, app icons, manifest, `og-image.png`, `head-snippet.html` | Websites |
| `social/github-social-preview.png` | 1280 x 640 | Repository settings, Social preview |
| `social/github-avatar.png` | 500 x 500 | Organisation or profile picture |
| `motion/anzu-animated.svg` | The block settles, the lines slide out (1.2 s) | Splash, loading, README |
| `tokens.json` | Colours, fonts, the tile rules | Code |

## Colours

| Name | HEX | RGB | Use |
|---|---|---|---|
| Peach | `#FFB4A2` | 255 180 162 | The block. In the app: a book with DRM |
| Mint | `#C8F0A0` | 200 240 160 | The lines. In the app: a book in EPUB |
| Ink | `#15161A` | 21 22 26 | The tile, text on light |
| Chip | `#26282E` | 38 40 46 | The tile on dark grounds (app header) |
| Paper | `#F4F5F7` | 244 245 247 | Light grounds |
| Night | `#0E0F12` | 14 15 18 | Dark grounds |

Contrast: peach on ink 10.6:1, mint on ink 14.1:1, peach on chip 8.6:1, mint on chip
11.5:1. Peach and mint on paper are 1.6:1 and 1.2:1, so on light grounds the symbol always
goes on its ink tile, never loose.

## Type

The name is Space Grotesk Bold with -0.045 em tracking, as in the app header. Text is Inter.
In lockups the name is outlined, so it needs no font installed.

## Sizes and space

- The small cut is used when the symbol is under 22 px, that is a tile of 32 px or less:
  the favicon, small icons, the app header.
- Minimum: a 16 px tile.
- Clear space around the tile: one quarter of the tile.
- In the horizontal lockup the name's capitals are 7/16 of the tile tall and the gap is a
  quarter of the tile.

## Do not

- Put the peach and mint symbol loose on a light ground.
- Recolour the block and the lines with other state colours, or swap them.
- Stretch, rotate or outline the symbol.
- Set the name in another face, or add effects.

Before public use as a trademark, do a professional trademark search.

Identity made with [kothar](https://github.com/ilien-dev/kothar).
