# The Okojo fork of windows-rs

This is a working fork of [microsoft/windows-rs](https://github.com/microsoft/windows-rs). It
carries a stack of patches to one crate, `windows-reactor`, plus one patch to `windows-bindgen`.
The Okojo mail and calendar client needs those patches today. The goal is to send what is worth
keeping back to upstream, one small justified change at a time, and then stop carrying the rest.

Nothing here is a release or a supported product. If you want `windows-rs`, use the upstream
repository and its [documentation index](../readme.md).

| Page | Read it to learn |
| --- | --- |
| This page | What the fork is, why it exists, how the branches and tooling fit together |
| [patches.md](patches.md) | Every fork commit: what it changes, why, and its status against upstream |
| [upstreaming.md](upstreaming.md) | Which changes to propose, in what order, and the case for each |
| [issues.md](issues.md) | Defects and design problems found in the library and in the fork itself |

Facts in these pages were checked on 2026-10-16 against upstream `master` at `ffe56ded6` and the
fork branch `okojo-fast-list` at `dcfb63c07`. Both move quickly. Re-check before acting on a
status.

## Background

### windows-rs

`windows-rs` is Microsoft's collection of Rust crates for Windows. The best known are `windows`
and `windows-sys`, which project the Win32 and WinRT APIs into Rust. The repository is one Cargo
workspace. Libraries live in `crates/libs`, generators and CI helpers in `crates/tools`, tests in
`crates/tests`, and samples in `crates/samples`. The categorized index is
[docs/readme.md](../readme.md).

### windows-reactor

[`windows-reactor`](../crates/windows-reactor.md) (Reactor) is a preview crate in that workspace.
It is a declarative UI library for [WinUI 3](https://learn.microsoft.com/windows/apps/winui/),
Microsoft's native Windows UI framework. An application is a set of components. Each component
owns Rust state and handles typed messages, and its `view` method describes the controls for the
current state:

```text
native event -> typed message -> update state -> describe the next view
```

Reactor does not use XAML files. It builds native WinUI controls directly through COM bindings,
and keeps them in step with the declared view.

### Okojo

Okojo is a Rust replacement for the new Outlook app, with Microsoft 365 and Gmail accounts. It is
a large real application: a folder tree, date-grouped message lists with multi-select, a calendar
with drag-to-create, context menus, dialogs, drag and drop between panes, keyboard accelerators
and screen-reader support. Reactor is its only UI layer. Where Reactor lacks a WinUI property or
event Okojo needs, the gap is either worked around in the application or fixed in this fork.

Okojo vendors three crates from the fork (`windows-reactor`, `windows-window` and
`windows-reactor-setup`) as `cargo package` output, and points `[patch.crates-io]` at them. Its
`vendor/README.md` and `vendor/REACTOR_FORK` record the exact fork commit in use.

## What the fork contains

On `okojo-fast-list`, 58 commits sit on upstream `d5b6a1f68` ("unify native event dispatch",
#5061, 2026-10-10). They fall into five kinds of work:

| Kind | Examples |
| --- | --- |
| Missing WinUI properties and events | `IsHitTestVisible`, `Cursor`, `AutoSuggestBox` `QuerySubmitted`, `ListView` `ItemClick`, `InfoBar` `ActionButton` |
| Theming and icons | Any theme brush by resource key, app light/dark colour dictionaries, icon foregrounds, icons in menus |
| Input | More accelerator keys, focus-scoped accelerators, element context menus, drag formats |
| Correctness fixes | Pointer events out of order, wrong `ListView` row selected, stale rows, a late `NavigationView` echo |
| Performance of large lists | Skip a render, replace many keyed rows at once, build row content only when a row is shown |

[patches.md](patches.md) lists all 58 with their reasons. The short version of where they stand:

| Status against current upstream | Commits |
| --- | --- |
| Landed on upstream under another change (drop) | 2 |
| Partly landed | 1 |
| Fixed upstream a different way (drop, re-verify) | 3 |
| Requested by the maintainer in upstream issue #5087 | 8 |
| Not on upstream and not requested (needs an issue first) | 28 |
| `ListView` row-content rework that must be proposed as a unit | 7 |
| On hold: little case beyond Okojo | 2 |
| Fixups that belong inside another commit | 7 |

That is all 58. The status of each commit and the reasoning is in [patches.md](patches.md).

## Branches

All branches are on `origin` (`TKIPisalegacycipher/windows-rs`). `master` mirrors upstream.

| Branch | Tip | Use |
| --- | --- | --- |
| `okojo-fast-list` | `dcfb63c07` | The complete stack, 58 commits. This is the branch Okojo vendors |
| `okojo-infobar-action` | `7e9aea584` | A prefix of `okojo-fast-list` (51 commits). The InfoBar commit `7e9aea584` is its tip, and a pending Okojo PR pins it |
| `okojo`, `okojo-review-fixes` | `14f2210a2`, `981798f05` | Earlier integration points, both ancestors of `okojo-fast-list` |
| `okojo-accel-letters`, `okojo-dialog-retire`, `okojo-drag-format`, `okojo-drag-image`, `okojo-drop-other`, `okojo-listview-item-click`, `okojo-live-setting`, `okojo-nav-item-invoked`, `okojo-nav-scroll`, `okojo-open-inactive` | various | One branch per feature, each an ancestor of `okojo-fast-list`. Useful as PR heads, redundant as history |
| `okojo-local` | `37f87d6fa` | A reordered rebuild of part of the stack. 10 of its 14 unique commits are patch-equivalent to commits in `okojo-fast-list`; 4 differ in content. Not an input to anything |
| `okojo-fork-docs` | this branch | The pages you are reading, on top of `okojo-fast-list` |

None of these branches has an open pull request, upstream or on the fork, and no issue has been
filed upstream for any of the work. Okojo's `docs/upstream-drafts/` holds 18 draft issues that
have not been posted.

## Working in the fork

### Build and test

The commands are the ones upstream's `test_reactor` workflow runs. Everything below is headless.

```text
cargo run -p tool-reactor --quiet          # regenerate; must leave no diff
cargo test -p tool-reactor --quiet
cargo test -p windows-reactor --all-features --quiet
cargo clippy -p windows-reactor --all-targets
```

Measured on `okojo-fast-list` at `dcfb63c07` for these pages: regeneration left the tree clean,
and the `windows-reactor` test targets passed (341 + 4 + 77 passed, 16 ignored, 0 failed, plus
doctests). The ignored tests drive a real WinUI window and run only on a desktop.

The live checks need a desktop session and Windows App Runtime 2.5. They are
`cargo run -p test-reactor-selftest` (add `-- --headless` for CI mode) and the
`test-reactor-integration` binaries. Upstream's workflow lists them. Do not run them from an
unattended agent: they open windows.

### How a change is made

Reactor is mostly generated. A property or event usually needs edits in three places and then a
regeneration:

| File | Role |
| --- | --- |
| `crates/tools/reactor/src/schema.toml` | Which WinUI members become Reactor properties, events and relations, and how |
| `crates/tools/reactor/src/bindings.txt` | The WinUI methods the native bindings must expose |
| `crates/tools/reactor/src/main.rs` | The generator. Changed when a pattern is not yet supported |

`cargo run -p tool-reactor` rewrites `generated_declarations.rs`, `generated.rs`,
`native/generated.rs`, `native/bindings.rs` and the live coverage test. Never hand-edit those.
CI regenerates and rejects any diff.

Hand-written behaviour lives in `crates/libs/reactor/src`: `declaration.rs` (public value types),
`reconcile.rs` (diffs the declared tree against the live one), `ir.rs` (mutations and
validation), and `native/winui/` (the adapter that applies mutations to WinUI objects and turns
WinUI events into queued messages). Tests are layered:

| Layer | Where | Needs a desktop |
| --- | --- | --- |
| Unit tests with a recording adapter | `src/tests` | No |
| Native WinUI tests | `src/tests/native`, marked `#[ignore]` | Yes |
| Live self-test | `crates/tests/libs/reactor_selftest` | Yes |
| Integration binaries | `crates/tests/libs/reactor_integration` | Yes |
| Public API snapshot | `crates/libs/reactor/public-api.txt` | No (needs `cargo-public-api`) |

A change that alters the public API must also update `public-api.txt`; upstream's workflow
compares it byte for byte.

### Updating Okojo's vendored copy

1. In the fork, rebase onto `upstream/master`, rerun `cargo run -p tool-reactor`, resolve
   conflicts, run the gates above, push.
2. In Okojo, run `vendor/sync-reactor.sh <fork-path>`. It refuses a dirty fork, repackages the
   three crates and rewrites `vendor/REACTOR_FORK`.
3. Update the patch table in Okojo's `vendor/README.md`.

Step 1 is the expensive one. A trial rebase of the 58 commits onto current upstream stops on
hand-written conflicts in 36 of them, even with every generated file taken from upstream. See
[upstreaming.md](upstreaming.md) for what that means for the plan.

### Conventions to follow

Upstream's [contributing guide](../contributing.md) asks for an issue or a comment on an existing
issue before a pull request, so that maintainers agree on an approach first. Upstream's
[`.github/copilot-instructions.md`](../../.github/copilot-instructions.md) sets the code and
writing rules: ASCII punctuation, 100-column wrap, no filler vocabulary, commit subjects of the
form `` `windows-reactor` add X ``, generated files never edited by hand. These pages follow the
same writing rules.
