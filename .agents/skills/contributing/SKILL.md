---
name: contributing
description: Use before opening an issue, creating a branch, committing for a pull request, or writing a pull request title or description in the Anzu repository. Walks through CONTRIBUTING.md (one change per PR, feature/ branch names, Conventional Commit titles, the description format with character limits, evidence) and checks the result with .github/scripts/check-pr.js.
---

# Contributing to Anzu

`CONTRIBUTING.md` is the source of truth; this skill is the order to apply it in. If the
two disagree, follow `CONTRIBUTING.md` and point out the difference.

## 1. Is it in scope, and is it new?

- Read `PRODUCT.md`. Anzu converts books from the Microsoft Store Kindle app to EPUB for
  non-technical people. If the change does not serve that, say so and stop: it will be
  rejected.
- Search for the same work before writing anything:
  ```
  gh pr list --repo ilien-dev/anzu --state all --search "<keywords>"
  gh issue list --repo ilien-dev/anzu --state all --search "<keywords>"
  ```
  If a PR or issue already covers it, report it to the user instead of duplicating it.

## 2. One thing only

Name the single change in one sentence. If the sentence needs an "and", it is two PRs:
for example a new output format **and** a settings change **and** a visual tweak are three
pull requests. Propose the split to the user and do the first one. Leave unrelated fixes
you notice for later; mention them, do not include them.

## 3. Branch

`feature/` followed by 1 to 4 lowercase words (letters and digits) joined by single
hyphens, for every kind of change: `feature/epub-cover-image`, `feature/fix-progress-bar`.
Regex: `^feature/[a-z0-9]+(-[a-z0-9]+){0,3}$`.

## 4. Code

Follow the repository's `CLAUDE.md`/`AGENTS.md`. Before committing:
```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```
Both must pass; report failures with their output.

## 5. Title

A Conventional Commit of at most 72 characters, lowercase after the type, no final
period. It becomes the squash commit on `main` and drives the version bump:
`feat` and `fix` release a new version, `docs`, `ci`, `refactor`, `test`, `chore` do not.
Pick the type that tells the truth about the change.

## 6. Description

Copy `.github/pull_request_template.md` and fill every section. Limits are counted
after HTML comments are removed; delete the template comments anyway.

| Section | Limit | Content |
|---|---|---|
| `## What` | 280 chars | The one change, one or two sentences. |
| `## Why` | 280 chars | The problem for users. `Closes #N` when there is an issue. |
| `## How` | 500 chars | The approach and the main files or modules. |
| `## Evidence` | 300 chars of text | Images, GIFs and links do not count. |
| `## Checklist` | — | Every box ticked, honestly. Untick and explain if one is not true. |

Write plainly: what changed, not how hard it was. No headings beyond these, no summary
at the end.

**Evidence.**
- Visual change: a screenshot is required, a GIF is better, before and after when
  something changed. Capture the Anzu window with `PrintWindow` on its window handle,
  never the whole screen (it can catch the user's other windows). Save the files in a
  temporary folder, not in the repository.
- GitHub has no API to upload images into a PR description. Open the PR, then give the
  user the absolute paths of the files and ask them to drag them into the description.
  Put a placeholder line in Evidence meanwhile, e.g. `Screenshot: to be attached.`
- Non-visual change: name the tests that cover it, or the exact steps used to check it.

## 7. Check before opening

Write the description to a temporary file and run:
```
node .github/scripts/check-pr.js --branch <branch> --title "<title>" --body-file <file>
```
The same check runs in CI as the required status `Branch name and description`. Fix
every problem it lists, then open the PR:
```
gh pr create --title "<title>" --body-file <file>
```
Do not merge. Every PR needs an approval from someone with write access, and the
maintainer decides.

## Issues

Issues are contributions too. Use the forms in `.github/ISSUE_TEMPLATE/` (blank issues
are disabled): one bug or one idea per issue, the problem before the solution, and
search first. Never include anything from the user's Amazon account or from
`%LOCALAPPDATA%\anzu\keys` or `%LOCALAPPDATA%\anzu\work`.
