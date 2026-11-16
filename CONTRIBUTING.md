# Contributing to Anzu

Thanks for wanting to help. Contributing to Anzu does not only mean pull requests:
a clear issue that reports a bug or proposes an improvement is a contribution too.

Before anything else, read [`PRODUCT.md`](PRODUCT.md). Anzu does one job: it converts
books downloaded with the Microsoft Store Kindle app to EPUB, for people who are not
technical. **A contribution that drifts away from that purpose will be rejected**, however
well it is made.

## Nothing is guaranteed

Opening an issue or a pull request does not guarantee that it will be accepted or merged.
Every change is reviewed before it goes in, with no exceptions, including the
maintainer's own. A change can be declined for its scope, its approach or the cost of
maintaining it.

## Before you start

- **Search first.** Look through the [open pull requests](https://github.com/ilien-dev/anzu/pulls)
  and the [issues](https://github.com/ilien-dev/anzu/issues), open and closed. Someone may
  already be working on the same feature, or it may have been discussed and declined.
  If you find it, add to that conversation instead of starting a new one.
- **For anything bigger than a small fix, open an issue first** and wait for an answer.
  It is the cheapest way to find out whether the change fits before you write it.

## Issues

Use the templates: **Bug report** or **Feature request**. One issue per problem or idea.

- A bug report says what you did, what you expected, what happened, and your Windows,
  Kindle app and Anzu versions. A screenshot helps.
- A feature request says the problem first, and only then the solution you have in mind.
- Never paste anything from your Amazon account, your keys, or the files under
  `%LOCALAPPDATA%\anzu\keys` and `%LOCALAPPDATA%\anzu\work`.

## Pull requests

### One thing at a time

Each pull request does **one thing**. If you want to add a new output format, the PR adds
that format and nothing else: no change to the settings menu, no visual tweak elsewhere,
no unrelated cleanup. Those go in their own pull requests.

Keep it as simple as possible. The smaller and more focused a PR is, the easier it is to
see what it is about and to decide whether it can go in. A PR that mixes several changes
will be asked to split, or closed.

### Branch name

```
feature/<1 to 4 words>
```

All lowercase, words of letters and digits joined by single hyphens, at most four words.
This holds for every kind of change, fixes and docs included.

- `feature/epub-cover-image` ✓
- `feature/fix-progress-bar` ✓
- `feature/Cover-Image` ✗ (uppercase)
- `fix/progress` ✗ (does not start with `feature/`)
- `feature/add-the-new-cover-image` ✗ (five words)

### Title

The title is a [Conventional Commit](https://www.conventionalcommits.org/) of at most 72
characters: `feat: ...`, `fix: ...`, `docs: ...`, `refactor: ...`, `ci: ...`, `chore: ...`.
PRs are squash-merged, the title becomes the commit on `main`, and the version number is
computed from it, so it has to describe the change exactly.

### Description

Start from the template GitHub fills in for you
([`.github/pull_request_template.md`](.github/pull_request_template.md)). Every section is
required and has a limit:

| Section | Limit | What goes in it |
|---|---|---|
| **What** | 280 characters | The one change this PR makes. |
| **Why** | 280 characters | The problem it solves. Link the issue (`Closes #12`). |
| **How** | 500 characters | The approach, and the main files it touches. |
| **Evidence** | 300 characters of text | Screenshots or a GIF; how you checked it. |
| **Checklist** | — | Every box ticked. |

**Evidence.** For any visual change a screenshot is required, and a GIF is better; show
before and after when something changed. Drag the files into the description box on
GitHub to upload them. Images, GIFs and links do not count toward the limit. For a change
with nothing to see, name the tests that cover it or say how you checked it.

A CI check verifies the branch name, the title length and the description. You can run
it yourself before opening the PR:

```
node .github/scripts/check-pr.js --branch feature/epub-cover-image \
  --title "feat: export the cover image to EPUB" --body-file pr.md
```

### Code

- `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`
  must pass. CI runs both on Windows.
- Code, comments and docs are in English. Every text of the interface goes in
  `crates/anzu/src/i18n.rs`, in both English and Spanish.
- Do not edit the version in `Cargo.toml` or `CHANGELOG.md`; the release workflow does it.

### Review

A pull request is merged only after an approval from someone with write access to the
repository (for now, the maintainer) and with every check green. A new push dismisses an
earlier approval. Answer review comments in the PR; keep extra changes for another PR.

## Working with an AI agent

The rules are the same. Agents that read `.claude/skills` or `.agents/skills` have a
`uncontributing` skill that walks through them step by step.
