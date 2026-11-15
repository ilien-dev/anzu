<!--
Title: a Conventional Commit of at most 72 characters, e.g. "feat: export the cover image to EPUB".
Branch: feature/ + 1 to 4 lowercase words joined by hyphens, e.g. feature/epub-cover-image.
One PR = one change. Read CONTRIBUTING.md before opening it.
Comments like this one are removed before the limits are counted.
-->

## What
<!-- At most 280 characters. The one change this PR makes, in one or two sentences. -->

## Why
<!-- At most 280 characters. The problem it solves for the people who use Anzu. Link the issue: "Closes #12". -->

## How
<!-- At most 500 characters. The approach, and the main files or modules it touches. -->

## Evidence
<!--
At most 300 characters of text; images, GIFs and links do not count.
Visual change: a screenshot is required, a GIF is better; before and after if something changed.
Drag the files into this box on GitHub to upload them.
Other changes: the tests that cover it, or how you checked it by hand.
-->

## Checklist
- [ ] This PR does one thing, and nothing else is mixed in
- [ ] I searched open PRs and issues and nobody is already doing this
- [ ] `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass
- [ ] New interface text is in both EN and ES in `crates/anzu/src/i18n.rs`, or there is none
