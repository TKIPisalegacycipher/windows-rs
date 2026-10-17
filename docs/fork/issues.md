# Issues found

Problems found while building and reviewing the fork. [Part A](#a-defects-in-windows-reactor)
lists defects and design gaps in upstream `windows-reactor` itself.
[Part B](#b-problems-in-the-fork) lists the problems the fork had and records how each was closed.
Parts C and D cover the generator and the checking that has not been done.

Each defect has a confidence level, because most were found by reading code or by Okojo's own
testing, not by reproducing them on current upstream:

| Level | Meaning |
| --- | --- |
| Confirmed in source | The faulty code is present in upstream `master` (`ffe56ded6`) as read for these pages |
| Observed in Okojo | Seen in the running application on the fork, which carried upstream `d5b6a1f68` |
| Suspected | Inferred from the fix and from the code, not reproduced |

No defect below has been reported upstream. That is the first step for most of them. Where the
rebuilt stack (`okojo-rebased`) fixes one, the commit number from [patches.md](patches.md) is given.

## A. Defects in windows-reactor

### U1. `PointerReleased` is delivered after `PointerCaptureLost`

`WinUiAdapter::pointer_event_info` in `native/winui/events.rs` calls
`element.ReleasePointerCapture(&pointer)?` while it builds the `PointerReleased` payload. WinUI
raises `PointerCaptureLost` synchronously inside that call, so the capture-lost event is queued
first. With `capture_pointer_on_press(true)` the application sees `DragCancel`, then `Drop`, and
cannot tell a finished drag from a cancelled one. The `?` also drops the released event if the
release fails.

- Confidence: confirmed in source. The call is still in `pointer_event_info`.
- Fork fix: commit 14. Dispatch first, then release, and report a release error without losing
  the event. No public API change.
- Checked: ordering tests against the native event queue; in Okojo, a calendar drag-to-create
  ends in the editor with the dragged range (the unfixed order was not re-run).

### U2. Button and `DropDownButton` menus open above the button

A `Menu` on a `Button` or `DropDownButton` leaves `FlyoutBase.Placement` at WinUI's default, so
the menu opens upward. Near the top of a window it covers the title bar. The reactor's own
notification-icon menu already uses `BottomEdgeAlignedLeft`.

- Confidence: confirmed in source that no placement is set for these menus; the visible effect
  was not re-run unfixed.
- Fork fix: commit 13. If a configurable `Menu::placement` is preferred, this is its default.
- Related: upstream #5055 (notification-icon menu anchoring) and #4923.

### U3. `ListView` `SelectedIndex` is applied before the item changes of the same update

`SelectedIndex` indexes `Items`, but it was set with the other properties, before the item
inserts and removes of the same update. It named an item of the old list: removing a message and
selecting the next one, across a date header, selected the header. A mounted list set the index
on an empty list. Removing the selected item also drops the native selection while the declared
index stays the same.

- Confidence: suspected on current upstream. Upstream orders `SelectedIndex` after other
  properties in `native/winui/objects.rs`, which does not order it after the items relation.
- Fork fix: commit 35.

### U4. Stale or blank `ListView` rows after keyed changes

XAML keeps a recycled container bound to its last item, so that item's content element stays the
container's child. When another container binds the same element, XAML rejects it and that
container keeps its previous content. Rows showed a removed item's text under the new item's
name. A second mode: XAML can hand a recycled container back to the same item without preparing
it, so rows a moved item jumped over had zero height and were missing from the UI Automation
tree.

- Confidence: observed in Okojo (rows kept stale "Message" text). Not reproduced on current
  upstream, which changed keyed moves in #5114.
- Fork fix: commits 40 and 44.

### U5. `NavigationView` reports "nothing selected" late after a deselect

When the reactor clears a `NavigationView` selection, WinUI finds no container for the null item
and queues `SelectionChanged` at low priority, after the reactor's own-write suppression has
ended. The echo reaches the application as a user event, 0.3 to 1 s after the pick. In Okojo it
caused a full render per folder pick.

- Confidence: observed in Okojo; the code that suppresses own writes is unchanged upstream.
- Fork fix: commit 36. It expects one late event after a reactor deselect and swallows it.

### U6. WinUI access violation in `ScrollIntoView` after keyed `ListView` changes

`ScrollIntoView` after some keyed item changes can crash `Microsoft.UI.Xaml.dll`, with text-only
items and with none of the fork's changes. The repro in the body of commit 40 is four item lists
and the index to scroll to, in a 300 px `ListView` with `item_min_height` 0. Okojo's notes say it
also reproduces on upstream master.

- Confidence: observed in Okojo; the repro has not been run for these pages.
- Action: reduce it to a standalone sample and report it to the WinUI team and to Reactor.
  Reactor could avoid it only by not issuing `ScrollIntoView` after keyed changes.
- Related: on 2026-10-16 the maintainer commented on #5087 that forcing the incremental removal
  and insertion path in a synthetic 101-row `ListView` ended in an undiagnosed native crash. It
  may be the same fault, and a standalone repro would help both of them.

### U7. Closing a window with a `ContentDialog` open can crash the process

Okojo issue #211: closing a window while its About dialog is open can end the process with an
access violation, in hosted `ContentDialog` teardown. A related symptom, a dialog left shown
after its owner closed (#212), was an Okojo bug.

- Confidence: observed in Okojo; the cause is not isolated.
- Fork work: the original stack had two commits that hide a released dialog. By the second one's
  own message the reconciler already hides it first, and neither ties the change to #211, so both
  were dropped from the rebuilt stack. Treat #211 as open.

### U8. A drop can hang when the drag source is the application's own modal loop

`IDropTarget::Drop` waits while XAML runs the drop handler. A drop that completes a deferral
from a UI callback cannot finish when the source is the application's own `DoDragDrop`, because
that loop is blocking the thread the callback needs. The fork fixed this for the new
`DragKind::Other` drop by queuing the event without a deferral (commit 38).

- Confidence: suspected for the existing file and text drops. They read a payload and so need a
  deferral. An in-process drag of text or files onto a Reactor target would hit the same wait.
- Action: reproduce with a one-window sample that starts `DoDragDrop` with text and drops on
  itself.

### U9. A selectable `TextBlock` inside an element with a context menu shows its own flyout

Okojo issue #214: with `.menu()` on an ancestor, a `TextBlock` with text selection enabled shows
WinUI's copy flyout instead. Okojo's notes record it as present on Okojo's `main` branch too, so
it may not come from the fork's change; whether it happens on plain upstream is not known.

- Confidence: observed in Okojo.
- Action: document it with the context-menu proposal, or suppress the inner flyout.

### U10. A forced light theme leaves the reading pane dark

Okojo issue #213: with the app theme forced to light, the reading pane body stayed dark
because the colour-scheme observer reported the system scheme before the forced theme applied.

- Confidence: observed in Okojo. Upstream has since changed the observers (#5071) and window
  background handling (#5074), so this needs re-checking before it is reported.

### U11. Observed properties used without a declaration cleared on the next render

Resolved. A native edit to a control the application never declared a value for (a
`ToggleSwitch` with only `on_toggled`) used to revert on the next render. Upstream #5093 makes
declarations establish ownership and callbacks not, which is the behaviour Okojo's draft
asked for. Recorded here so the draft is not posted.

### U12. Large keyed lists create and re-render every row

Upstream #5085 reports this for `ItemsRepeater` in the third preview. #5087 item 17 asks for
anchored scroll and focus through keyed `ListView` reorders. The maintainer's trace (comment of
2026-10-16) shows a reversal of 101 keyed rows takes the `ReplaceAll` path and resets scroll and
focus; he parked the item until someone supplies an affected application and its expected
behaviour. The fork's `ListView` work
([patches.md](patches.md#8-listview-row-content-and-large-lists)) addresses the same family of
problem for `ListView` and is the largest design question in the stack.

## B. Problems in the fork

The fork had eleven problems. All are closed. Each entry says what was wrong and what closed it.

### F1. The stack no longer rebases cleanly

Upstream `master` was 32 commits ahead of the fork's base, 22 of them in `windows-reactor`,
`windows-window` or `windows-bindgen`, including #5108 (directory module layout), #5110 (lazy
event subscription), #5114 (keyed reorder moves), #5079 (menus), #5093 (property ownership) and
#5071 (window observers). A trial rebase stopped on hand-written conflicts in 36 of 58 commits.

Closed: the stack was rebuilt on `ffe56ded6` as `okojo-rebased`, one proposal at a time and in
dependency order, resolving conflicts by hand and regenerating every generated file rather than
merging it. Each of the 45 commits passed the generator check, the `windows-reactor` tests, a
compile of every crate that depends on Reactor, clippy and the public API snapshot before the next
was started. Rebuilding exposed an ordering dependency that a mechanical rebase would have missed:
the bindgen fix (commit 23) must precede the first `ListView` naming commit, or the regenerated
bindings fail `-D warnings`.

### F2. Some patches duplicated upstream

Commits 3, 14, 16, 23 and 24 of the original stack are on upstream under other changes, and part
of commit 7 is.

Closed: all dropped, and the `FontIcon` half of commit 7 removed (it is commit 15, `SymbolIcon`
only). Upstream's behaviour is used.

### F3. Accelerator modifiers cannot express Control+Shift

#5087 item 14 asks for combined modifiers "including Control+Shift". The original
`AcceleratorModifiers` offered `Control`, `Menu`, `ControlMenu` and `MenuShift`.

Closed: commit 12. `AcceleratorModifiers` is back to upstream's `None` and `Control`; both
`KeyAccelerator::new` and the mapping to `VirtualKeyModifiers` take `InputModifiers`, which gains
`BitOr`. A unit test maps every combination, including Control+Shift and all four modifiers
together. Existing calls compile unchanged because `AcceleratorModifiers` converts into
`InputModifiers`.

### F4. Public types are not `non_exhaustive`

`AcceleratorKey`, `MenuItem`, `PointerEventInfo`, `DragDropPolicy`, `DragKind`, `DroppedData` and
`ThemeBrush` are exhaustive, so every variant or field the fork adds can break downstream code.

Closed: commit 1 marks them `#[non_exhaustive]` (and the `Item` and `Submenu` variants of
`MenuItem`) before any commit adds to them. The drag-drop sample and the live self-test were
adjusted. The attribute is itself a breaking change; [patches.md](patches.md#public-api-impact)
states it, and each proposal should too.

### F5. Resource keys are inserted into XAML unescaped

The earlier version of this page, taken from the draft issue text, said a key containing `'`, `<`
or `&` would fail at XAML load time. That was wrong about the code. `ThemeBrush::Resource` and
`register_theme_colors` validate the key (ASCII letters, digits, `_` and `.` only) and panic on
anything else, with tests for empty and malformed keys, so a bad key never reaches XAML.

Closed as not a defect. The draft text for the theming proposal should say the keys are
validated, and that a panic on a bad key matches how `WindowPolicy` rejects bad sizes.

### F6. `TitleBar` icon theme is unverified

The `TitleBar` takes an `IconSource`, which is not in the visual tree, so a theme brush on it is
resolved when the icon is applied and may not follow later theme changes.

Closed: the limit is stated plainly in the public rustdoc of `Icon::foreground` and in the comment
on `icon_source`, as the original asked ("verify or state it plainly"). It has not been verified
in a running window; that stays in [Part D](#d-verification-debt).

### F7. History was not shaped for review

Seven commits were fixups of earlier ones, two were on hold, and generator fixes were bundled
inside feature commits.

Closed: ten fixups were merged into their parents, two held commits dropped, and the three
generator fixes became their own output-neutral commits (2, 3, 4). See the map in
[patches.md](patches.md#what-happened-to-the-original-58).

### F8. `DragDropPolicy::other` did not do what it was added for

`other` was added so Okojo's message drag could drop on folder rows. It never matched there: XAML
presents virtual `.eml` files as storage items. `format` was added instead and Okojo uses it.
`other` still matched any foreign drag with no files and no text, with no payload to inspect.

Closed: `other` is removed from commit 38. The `DragKind::Other` and `DroppedData::Other` values
remain, reached only through `format`. A unit test pins that `Other` data is not accepted without
a format. Okojo does not call `other`.

### F9. `skip_render` changes when side effects run

`ComponentContext::skip_render()` skips `view` and the reconcile for a message. State changed in a
skipped update shows on the next render, and the effects and window publication declared from it
run then.

Closed: commit 39 now documents the hazard in the rustdoc ("call it only when the message changed
nothing that `view`, an effect or the window publication reads; a skipped update that did change
such state leaves the screen out of date until some later message renders"), and two unit tests
pin the behaviour. Whether upstream wants the API at all stays a design question for the
maintainer.

### F10. Commits carry an AI co-author trailer

57 of 58 original commits ended with `Co-Authored-By: Claude ...`. Upstream ships
`.github/copilot-instructions.md`, so assisted work is expected there, and that file says the
maintainer reviews and commits changes.

Closed by policy. All 45 commits on `okojo-rebased` carry `Co-Authored-By: Claude Sonnet 5.5`, the
model that rebuilt them, and 37 also carry `Claude Opus 5.5`, the model that wrote the original.
The trailers describe how the work was made and stay. Original authors and dates were preserved.
Each upstream issue or pull request should say the patch was written with assistance, and follow
that file's writing rules. [upstreaming.md](upstreaming.md#how-to-post-one) says so.

### F11. Redundant branches

`okojo-local` was a reordered copy of part of the stack, and ten per-feature branches were
ancestors of `okojo-fast-list`.

Closed on 2026-10-17: after checking that each was an ancestor of `okojo-fast-list` (and that
`okojo-local` is identical to the local `okojo` branch), these twelve were deleted from `origin`.
Any can be restored with `git push origin <sha>:refs/heads/<name>`.

| Branch | Tip |
| --- | --- |
| `okojo-accel-letters` | `7804302d7` |
| `okojo-dialog-retire` | `12871a8f1` |
| `okojo-drag-format` | `01b9d70a6` |
| `okojo-drag-image` | `acfde14fc` |
| `okojo-drop-other` | `07d4d2ec3` |
| `okojo-listview-item-click` | `abf0a135c` |
| `okojo-live-setting` | `a4b5992a9` |
| `okojo-local` | `37f87d6fa` |
| `okojo-nav-item-invoked` | `c594a87fe` |
| `okojo-nav-scroll` | `1f9242fe3` |
| `okojo-open-inactive` | `2a7f1e258` |
| `okojo-review-fixes` | `981798f05` |

`okojo`, `okojo-fast-list` and `okojo-infobar-action` stay because Okojo pins SHAs reachable
through them; they can go once Okojo vendors `okojo-rebased`.

## C. Generator bugs found

`tool-reactor` fixes that rode inside feature commits. Each is a defect in the generator and now
has its own output-neutral commit on `okojo-rebased`. The first two are still present on upstream.

| Bug | Symptom | Fork commit | Present on upstream |
| --- | --- | --- | --- |
| Attached enum properties are probed with a hard-coded `::None` variant | Fails to compile for any enum without a `None` variant, such as `AutomationLiveSetting` | 2 | Yes: the string is still in `main.rs` |
| Event args are assumed to expose the payload getter on the default interface | `InvokedItemContainer` lives on the args class's second interface; the generated call does not compile | 3 | Yes: no secondary-interface lookup |
| `?` emitted in `set_content` for a single-child relation with a `native_item` cast | Does not compile in a function returning `Option` | 4 | Not re-checked |

Each should be a pull request on its own, with a test in the generator, since it unblocks
properties and events the schema cannot express today. The generator crate has no unit tests; the
commits rely on regeneration producing no diff.

## D. Verification debt

What the claims in this directory rest on, and what is missing:

| Gap | Detail |
| --- | --- |
| Unit and generated tests only | Every behaviour was checked headless in its commit. Native paths such as the live `SetContextFlyout`, restoring a `TextBox`'s own flyout, scoped accelerators, and the `TitleBar` icon have no run in a real window |
| Live tests not run on the rebuilt stack | The live self-test and integration binaries open windows and need a desktop. They were not run, and nothing was run in the Okojo application |
| Okojo's in-app check is old | It ran on 2026-10-10 against the original `660a63bec`. About half of the stack's commits came after it |
| Several bugs were never seen unfixed | The upward menu and the capture-lost ordering are inferred from code. Okojo confirmed only the fixed behaviour |
| Not run on current upstream | The patches were rebuilt on `ffe56ded6` and pass its headless gates, but no behaviour was observed there |
| No regression test for recycled containers | Commit 40's container handling is native behaviour a headless test cannot reach; its live case is unrun |
| Okojo not built against the stack | The migration notes in [readme.md](readme.md#moving-okojo-to-the-rebuilt-stack) come from reading Okojo's sources |

Before any proposal: rebase that proposal onto current upstream, run the headless gates, run the
live self-test for the touched control on a desktop, and say in the issue what was and was not
seen.
