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

Okojo still vendors `okojo-fast-list`. To move it:

1. In Okojo, run `vendor/sync-reactor.sh <fork-path>` with the fork checked out at
   `okojo-rebased`. It refuses a dirty fork, repackages the three crates and rewrites
   `vendor/REACTOR_FORK`.
2. Fix what the rebuild changed. A scan of Okojo's sources found:

| Change in the rebuild | Where Okojo is affected |
| --- | --- |
| `PointerEventInfo` is `#[non_exhaustive]` | Three struct literals in `src/ui/calendar/card.rs` tests. Build from `PointerEventInfo::default()` and assign fields |
| `DroppedData`, `DragKind`, `AcceleratorKey` are `#[non_exhaustive]` | Matches on `DroppedData` (`src/ui/compose/attach_view.rs`) need a wildcard arm |
| `MenuItem` variants are `#[non_exhaustive]` | Okojo's patterns already end in `..`; no change expected |
| `AcceleratorModifiers::Menu`, `ControlMenu`, `MenuShift` are gone | Okojo uses none of them. Use `InputModifiers` for Alt and other combinations |
| `DragDropPolicy::other` is gone | Okojo does not call it; it uses `format` and `hide_image` |
| `CalendarDatePicker::date`, and the colour-scheme and menu fixes, come from upstream | Check the three `CalendarDatePicker` call sites against upstream's signature |

3. Update the patch table in Okojo's `vendor/README.md`.

This was derived from reading Okojo's sources; Okojo was not built against the new stack.

### Conventions to follow

Upstream's [contributing guide](../contributing.md) asks for an issue or a comment on an existing
issue before a pull request, so that maintainers agree on an approach first. Upstream's
[`.github/copilot-instructions.md`](../../.github/copilot-instructions.md) sets the code and
writing rules: ASCII punctuation, 100-column wrap, no filler vocabulary, commit subjects of the
form `` `windows-reactor` add X ``, generated files never edited by hand. These pages follow the
same writing rules.
