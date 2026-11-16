---
version: 1
slug: "site-src-pages-index-astro"
primary_target: "site/src/pages/index.astro"
related_targets: []
---

# Anzu product site — home

Mode: Persuade. The visitor is a Kindle reader on Windows, often not technical, who wants
their books as EPUB. Success: they understand what Anzu does within one viewport, see it
working, trust that it is careful, and download it.

Constraints: real facts only (PRODUCT.md, README). No stats, testimonials or invented
claims. EN and ES. Light and dark follow the system. The demo is the real app, compiled to
WebAssembly from `crates/anzu-web`; nothing in the page imitates the app's interface.

## Direction contract

THESIS: The product proves itself: the first viewport carries the real Anzu window,
running, at its true size. It refuses the hero-headline-plus-screenshot template and any
HTML lookalike of the app.

OWN-WORLD: "Amber console", the app's own world. JetBrains Mono for every voice, ink
palette with amber as the only accent, nearly square corners (3 px), 1 px lines, no
shadows except the window's. Dark: charcoal #0E0F0C, panel #161812, amber #FFB000. Light
"Cold paper": #F3F5F7, white panels, ink #14171A, toasted amber #B25E00; amber is never
a ground or a large surface, only the primary action's fill, as the app's own
`primary_button` does. The header band is dark ink in both themes.

STORY: Kindle book downloads → Anzu checks the tool → you tick books → it asks to close
Kindle → unlock and convert → the EPUB lands in Documents\EPUB. Then: who does what, where
files go, the name, download.

FIRST VIEWPORT: Dark band header (logo, nav, EN/ES, Download). Headline and one-paragraph
overview left-aligned above the window; primary action "Download for Windows" beside the
copy. The real window (1180×760 logical, Windows 11 caption in HTML) dominates the lower
viewport; on narrow screens it scales as a whole.

FORM: Proposal 1 "La pista" from the canvas (first of five), the three-stage track
DRM → unlocked → EPUB as the page's spine: it marks the files beside the window, runs
down the flow section in its three colours, and closes the page. Signature interaction:
the scripted real window, choreographed with an HTML prelude (Kindle download drops
locked files in) and coda (each EPUB travels to its folder), driven by the demo's
events. seed:1eeb2f1b1ee10055

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
