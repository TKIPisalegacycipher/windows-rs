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
| [patches.md](patches.md) | Every commit in the stack: what it changes, why, and its status against upstream |
| [upstreaming.md](upstreaming.md) | Which changes to propose, in what order, and the case for each |
| [issues.md](issues.md) | Defects found in the library, and the problems in the fork and how each was closed |

Facts in these pages were checked on 2026-10-17 against upstream `master` at `ffe56ded6` and the
fork branch `okojo-rebased` at `47c5b855d`. Both move quickly. Re-check before acting on a status.

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

`okojo-rebased` is upstream `ffe56ded6` plus 45 commits. The first four prepare the ground (a
`#[non_exhaustive]` change and three generator fixes). The other 41 expose WinUI behaviour or fix
a defect, and fall into five kinds of work:

| Kind | Examples |
| --- | --- |
| Missing WinUI properties and events | `IsHitTestVisible`, `Cursor`, `AutoSuggestBox` `QuerySubmitted`, `ListView` `ItemClick`, `InfoBar` `ActionButton` |
| Theming and icons | Any theme brush by resource key, app light/dark colour dictionaries, icon foregrounds, icons in menus |
| Input | More accelerator keys, any modifier combination, focus-scoped accelerators, element context menus, drag formats |
| Correctness fixes | Pointer events out of order, wrong `ListView` row selected, stale rows, a late `NavigationView` echo |
| Performance of large lists | Skip a render, replace many keyed rows at once, build row content only when a row is shown |

[patches.md](patches.md) lists all 45 with their reasons. Where they stand against upstream:

| Status against current upstream | Commits |
| --- | --- |
| Preparation: `#[non_exhaustive]` and generator fixes | 4 |
| Requested by the maintainer in upstream issue #5087 | 8 |
| Not on upstream and not requested (needs an issue first) | 27 |
| `ListView` row-content rework that must be proposed as a unit | 6 |

The original stack, `okojo-fast-list`, had 58 commits. The rebuild dropped 7 and merged 10 more
into their parents; [patches.md](patches.md#what-happened-to-the-original-58) maps every one.

## Branches

All branches are on `origin` (`TKIPisalegacycipher/windows-rs`). `master` mirrors upstream.

| Branch | Tip | Use |
| --- | --- | --- |
| `okojo-rebased` | `47c5b855d` | The current stack: 45 commits on upstream `ffe56ded6`. Propose from here |
| `okojo-fork-docs` | this branch | The pages you are reading, on top of `okojo-rebased` |
| `okojo-fast-list` | `dcfb63c07` | The old 58-commit stack on `d5b6a1f68`. Okojo still vendors it. Superseded |
| `okojo-infobar-action` | `7e9aea584` | A prefix of `okojo-fast-list` (51 commits). A pending Okojo PR pins its tip. Superseded |
| `okojo` | `14f2210a2` | An early integration point, an ancestor of `okojo-fast-list`. Superseded |

Twelve other branches were deleted on 2026-10-17; see [issues.md](issues.md#f11-redundant-branches)
for the list and how to restore any of them. Okojo pins fork SHAs, so the three superseded branches
stay until Okojo vendors `okojo-rebased`. Deleting them would not lose commits, since all are
reachable from `okojo-fast-list`, but nothing needs it yet.

The local `okojo` branch in the main checkout is not one of these. It is `origin/okojo` plus 14
commits that exist nowhere else, all superseded by this stack. Steps to retire it:

1. `git tag backup/okojo-local-37f87d6fa 37f87d6fa` so the 14 SHAs stay reachable.
2. `git switch okojo-rebased` in the main checkout (it is on `okojo`, so it cannot be deleted).
3. `git branch -D okojo`.

The tag is local; nothing on `origin` needs to change.

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
cargo public-api -p windows-reactor -sss --color never    # compare with public-api.txt
```

Measured on `okojo-rebased` at `47c5b855d`: regeneration left the tree clean, the
`windows-reactor` test targets passed (366 + 6 + 77 passed, 19 ignored, 0 failed, plus doctests),
the `reactor-gallery` and `reactor-workbench` tests passed, clippy reported nothing new, formatting
was clean for every touched crate, the public API snapshot matched, and all 113 workspace crates
that depend on Reactor compile (`cargo check --all-targets`). Every commit in the stack was gated
the same way when it was rebuilt. The ignored tests drive a real WinUI window and run only on a
desktop.

The live checks need a desktop session and Windows App Runtime 2.5. They are
`cargo run -p test-reactor-selftest` (add `-- --headless` for CI mode) and the
`test-reactor-integration` binaries. Upstream's workflow lists them. They have not been run on this
stack. Do not run them from an unattended agent: they open windows.

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

### Moving Okojo to the rebuilt stack

Okojo still vendors `okojo-fast-list`. These are the exact steps, from reading Okojo's sources
(`main`, 2026-10-17). Okojo has not been built against the new stack.

1. Check out `okojo-rebased` in a clean worktree of the fork. `vendor/sync-reactor.sh` refuses a
   dirty tree and records the branch and commit, so do not run it from `okojo-fork-docs`.
2. In Okojo run `vendor/sync-reactor.sh <fork-path>`. It repackages `windows-reactor`,
   `windows-window` and `windows-reactor-setup` and rewrites `vendor/REACTOR_FORK`.
3. Rewrite the three `PointerEventInfo` literals in the `only_the_primary_button_drags` test in
   `src/ui/calendar/card.rs`. `PointerEventInfo` is `#[non_exhaustive]`, which forbids struct
   expressions outside the crate, including `..Default::default()` and `..p`. Build with
   `let mut p = PointerEventInfo::default(); p.is_left_button_pressed = true;`, and for the two
   copies `let mut q = p; q.is_left_button_pressed = false; q.is_right_button_pressed = true;`
   (and the middle button likewise).
4. Run `cargo test` and clippy in Okojo. Nothing else in Okojo's sources needs an edit:

| Change in the rebuild | Okojo |
| --- | --- |
| `DroppedData`, `DragKind`, `AcceleratorKey`, `ThemeBrush`, `MenuItem` are `#[non_exhaustive]` | No change. The one `DroppedData` match (`attach_view.rs`) has a `_` arm, the `ThemeBrush` match (`theme.rs`) binds `other`, and every `MenuItem` pattern ends in `..` |
| `AcceleratorModifiers::Menu`, `ControlMenu`, `MenuShift` are gone | No change. Okojo uses only `None` and `Control` |
| `DragDropPolicy::other` is gone | No change. Okojo uses `format` and `hide_image` |
| `MenuItem::icon` refuses image icons | No change. Every menu icon is `Icon::font(...)` |
| `CalendarDatePicker::date` is upstream's | No change. `.date(Some(..))` matches `impl Into<Option<DateTime>>`. The comment in `src/ui/snooze.rs` about master clearing a picker's date is out of date after upstream #5093 |

5. Review what upstream changed in the new base that Okojo gets for free: `windows-reactor-setup`
   (632 lines; runtime staging now fails on download, extraction and copy errors and keeps
   architecture-specific caches, so a previously tolerated `build.rs` problem can now fail the
   build), window background matched to the theme (#5074), lazy event subscription (#5110),
   and keyed reorder moves (#5114). The sibling crates Okojo takes from crates.io
   (`windows-core`, `-future`, `-reference` and the rest) did not change between the two bases,
   so 0.100.0 still builds against the vendored crates.
6. Run Okojo's GUI scripts (`tests/gui/*.ps1`) on a desktop. Menus, drag and drop, list
   selection and the folder pane are the areas the rebuild touched.
7. Rewrite the patch table in Okojo's `vendor/README.md` from [patches.md](patches.md). It lists
   58 commits and "upstream status: not yet proposed".

### Conventions to follow

Upstream's [contributing guide](../contributing.md) asks for an issue or a comment on an existing
issue before a pull request, so that maintainers agree on an approach first. Upstream's
[`.github/copilot-instructions.md`](../../.github/copilot-instructions.md) sets the code and
writing rules: ASCII punctuation, 100-column wrap, no filler vocabulary, commit subjects of the
form `` `windows-reactor` add X ``, generated files never edited by hand. These pages follow the
same writing rules.
