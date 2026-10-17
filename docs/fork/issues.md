# Issues found

Problems found while building and reviewing the fork. [Part A](#a-defects-in-windows-reactor)
lists defects and design gaps in upstream `windows-reactor` itself.
[Part B](#b-problems-in-the-fork) lists what must change in the fork before anything is proposed.
Parts C and D cover the generator and the checking that has not been done.

Each defect has a confidence level, because most were found by reading code or by Okojo's own
testing, not by reproducing them on current upstream:

| Level | Meaning |
| --- | --- |
| Confirmed in source | The faulty code is present in upstream `master` (`ffe56ded6`) as read for these pages |
| Observed in Okojo | Seen in the running application on the fork, which carried upstream `d5b6a1f68` |
| Suspected | Inferred from the fix and from the code, not reproduced |

No defect below has been reported upstream. That is the first step for most of them.

## A. Defects in windows-reactor

### U1. `PointerReleased` is delivered after `PointerCaptureLost`

`WinUiAdapter::pointer_event_info` in `native/winui/events.rs` calls
`element.ReleasePointerCapture(&pointer)?` while it builds the `PointerReleased` payload. WinUI
raises `PointerCaptureLost` synchronously inside that call, so the capture-lost event is queued
first. With `capture_pointer_on_press(true)` the application sees `DragCancel`, then `Drop`, and
cannot tell a finished drag from a cancelled one. The `?` also drops the released event if the
release fails.

- Confidence: confirmed in source. The call is still in `pointer_event_info`.
- Fork fix: `5f2d95a1a`. Dispatch first, then release, and report a release error without losing
  the event. No public API change.
- Checked: ordering tests against the native event queue; in Okojo, a calendar drag-to-create
  ends in the editor with the dragged range (the unfixed order was not re-run).

### U2. Button and `DropDownButton` menus open above the button

A `Menu` on a `Button` or `DropDownButton` leaves `FlyoutBase.Placement` at WinUI's default, so
the menu opens upward. Near the top of a window it covers the title bar. The reactor's own
notification-icon menu already uses `BottomEdgeAlignedLeft`.

- Confidence: confirmed in source that no placement is set for these menus; the visible effect
  was not re-run unfixed.
- Fork fix: `9777d44b1`. If a configurable `Menu::placement` is preferred, this is its default.
- Related: upstream #5055 (notification-icon menu anchoring) and #4923.

### U3. `ListView` `SelectedIndex` is applied before the item changes of the same update

`SelectedIndex` indexes `Items`, but it was set with the other properties, before the item
inserts and removes of the same update. It named an item of the old list: removing a message and
selecting the next one, across a date header, selected the header. A mounted list set the index
on an empty list. Removing the selected item also drops the native selection while the declared
index stays the same.

- Confidence: suspected on current upstream. Upstream orders `SelectedIndex` after other
  properties in `native/winui/objects.rs`, which does not order it after the items relation.
- Fork fix: `df9100551`, `9194fe4cc`, `677bc7dcf` (one fix, three commits).

### U4. Stale or blank `ListView` rows after keyed changes

XAML keeps a recycled container bound to its last item, so that item's content element stays the
container's child. When another container binds the same element, XAML rejects it and that
container keeps its previous content. Rows showed a removed item's text under the new item's
name. A second mode: XAML can hand a recycled container back to the same item without preparing
it, so rows a moved item jumped over had zero height and were missing from the UI Automation
tree.

- Confidence: observed in Okojo (rows kept stale "Message" text). Not reproduced on current
  upstream, which changed keyed moves in #5114.
- Fork fix: `660a63bec`, `7804302d7`, `02f414d18`. `7804302d7` has no regression test.

### U5. `NavigationView` reports "nothing selected" late after a deselect

When the reactor clears a `NavigationView` selection, WinUI finds no container for the null item
and queues `SelectionChanged` at low priority, after the reactor's own-write suppression has
ended. The echo reaches the application as a user event, 0.3 to 1 s after the pick. In Okojo it
caused a full render per folder pick.

- Confidence: observed in Okojo; the code that suppresses own writes is unchanged upstream.
- Fork fix: `dcfb63c07`. It expects one late event after a reactor deselect and swallows it.

### U6. WinUI access violation in `ScrollIntoView` after keyed `ListView` changes

`ScrollIntoView` after some keyed item changes can crash `Microsoft.UI.Xaml.dll`, with text-only
items and with none of the fork's changes. The repro in the body of `660a63bec` is four item
lists and the index to scroll to, in a 300 px `ListView` with `item_min_height` 0. Okojo's notes
say it also reproduces on upstream master.

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
- Fork work: `12871a8f1` and `039255210` hide a released dialog, but by `039255210`'s own
  message the reconciler already hides it first, and neither message ties the change to #211.
  Treat #211 as open.

### U8. A drop can hang when the drag source is the application's own modal loop

`IDropTarget::Drop` waits while XAML runs the drop handler. A drop that completes a deferral
from a UI callback cannot finish when the source is the application's own `DoDragDrop`, because
that loop is blocking the thread the callback needs. The fork fixed this for the new
`DragKind::Other` drop by queuing the event without a deferral (`01b9d70a6`).

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

### F1. The stack no longer rebases cleanly

Upstream `master` is 32 commits ahead of the fork's base, 22 of them in `windows-reactor`,
`windows-window` or `windows-bindgen`. They include #5108 (directory module layout), #5110
(lazy event subscription), #5114 (keyed reorder moves), #5079 (menus), #5093 (property
ownership) and #5071 (window observers). A trial rebase stopped on hand-written conflicts in 36
of 58 commits, with every generated file resolved to upstream's side. The fork must be rebuilt
by stages and regenerated, not mechanically rebased. Lazy event subscription (#5110) changes how
native events are subscribed, so each event this fork adds needs re-checking against it.

### F2. Some patches duplicate upstream

Commits 14, 16, 3, 23 and 24 are on upstream under other changes, and part of 7 is. Carrying
them risks a double implementation. Drop them and keep upstream's behaviour.

### F3. Accelerator modifiers cannot express Control+Shift

Upstream #5087 item 14 asks for "combined modifiers, including Control+Shift". The fork's
`AcceleratorModifiers` offers `Control`, `Menu`, `ControlMenu` and `MenuShift` only. Adding one
variant per combination does not scale. The draft's second option, taking the existing
`VirtualKey` and `InputModifiers` (with a `BitOr`), covers every combination and matches key
events. Use it.

### F4. Public types are not `non_exhaustive`

`AcceleratorKey`, `AcceleratorModifiers`, `MenuItem`, `PointerEventInfo`, `DragDropPolicy`,
`DragKind` and `DroppedData` are exhaustive, so every variant or field the fork adds can break
downstream code. Upstream already marks `ContentDialogResult` `#[non_exhaustive]`. This is a
property of the library; the fork's additions make it visible. Propose the attribute once, before
the feature proposals that add variants and fields.

### F5. Resource keys are inserted into XAML unescaped

`ThemeBrush::Resource(&'static str)` and the colour swatch write the key into a XAML string as
`{ThemeResource <key>}`. A key containing `'`, `<` or `&` fails at XAML load time, and an empty
key cannot be rejected. Upstream's existing eight brushes use fixed keys, so they never hit
this. Escape or validate the key before proposing commits 10, 11, 12 and 27. Not checked: that
such a key fails as described.

### F6. `TitleBar` icon theme is unverified

The `TitleBar` takes an `IconSource`, which is not in the visual tree. A theme brush on it is
resolved when the icon is applied and may not follow later theme changes. The fork documents the
limit; no one has run it. Commit 19 should either verify this or state it plainly.

### F7. History is not shaped for review

Seven commits are fixups of earlier ones and two are on hold (see [patches.md](patches.md)).
Upstream's maintainer authors almost every Reactor change as one self-contained pull request that
cites an issue. Squash the fixups into their parents and split the bundled generator fixes out
(Part C) before proposing.

### F8. `DragDropPolicy::other` does not do what it was added for

`07d4d2ec3` added `other` so Okojo's message drag could drop on folder rows. It never matched
there: XAML presents virtual `.eml` files as storage items. `86c99deab` added `format` instead,
which Okojo uses. `other` still matches any foreign drag that has no files and no text, so a
drag from another application is accepted too, and it carries no payload to inspect. Propose
`format` and `hide_image`, and leave `other` out unless a second use appears.

### F9. `skip_render` changes when side effects run

`ComponentContext::skip_render()` skips `view` and the reconcile for a message. State changed in
a skipped update shows on the next render, and the effects and window publication declared from
it run then. That is a new rule for component authors, and an update that forgets state it
changed leaves the screen stale until some later message. Raise it as a design question with a
measured benefit before a pull request.

### F10. Commits carry an AI co-author trailer

57 of 58 commits end with `Co-Authored-By: Claude ...` (53 Opus 5.5, 4 Sonnet 5.5). Upstream ships
`.github/copilot-instructions.md`, so assisted work is expected there, and that file also says the
maintainer reviews and commits changes. Say in each issue that the patch was written with
assistance, and follow that file's writing rules.

### F11. Redundant branches

`okojo-local` is a reordered copy of part of the stack and shares no history past `a568d350d`.
Ten per-feature branches are ancestors of `okojo-fast-list`. Keep only the ones used as pull
request heads. See the table in [readme.md](readme.md#branches).

## C. Generator bugs found

`tool-reactor` fixes that ride inside feature commits. Each is a defect in the generator, and the
first two are still present on upstream.

| Bug | Symptom | Fork commit | Present on upstream |
| --- | --- | --- | --- |
| Attached enum properties are probed with a hard-coded `::None` variant | Fails to compile for any enum without a `None` variant, such as `AutomationLiveSetting` | `a4b5992a9` | Yes: the string is still in `main.rs` |
| Event args are assumed to expose the payload getter on the default interface | `InvokedItemContainer` lives on the args class's second interface; the generated call does not compile | `c594a87fe` | Yes: no secondary-interface lookup |
| `?` emitted in `set_content` for a single-child relation with a `native_item` cast | Does not compile in a function returning `Option` | `7e9aea584` | Not re-checked |

Each should be a pull request on its own, with a test in the generator, since it unblocks
properties and events the schema cannot express today.

## D. Verification debt

What the claims in this directory rest on, and what is missing:

| Gap | Detail |
| --- | --- |
| Unit and generated tests only | Every behaviour was checked headless in the commit. Native paths such as the live `SetContextFlyout`, restoring a `TextBox`'s own flyout, scoped accelerators, and the `TitleBar` icon have no live test |
| Okojo's in-app check is old | It ran on 2026-10-10 against `660a63bec`. Commits 31 to 58 (about half) were not part of it |
| Several bugs were never seen unfixed | The `MenuBarItem` duplication ("four entries instead of two"), the capture-lost ordering and the upward menu are inferred from code. Okojo confirmed only the fixed behaviour |
| Not run on current upstream | No patch was applied to or tested against `ffe56ded6`. The status in [patches.md](patches.md) comes from reading upstream source and issues |
| Live tests not run for these pages | The generator check and the headless unit tests were run on `dcfb63c07` (see [readme.md](readme.md#build-and-test)); no window was opened |
| No regression test | `7804302d7` (recycled containers), `01b9d70a6` (drop without deferral) |

Before any proposal: rebase that proposal onto current upstream, run the headless gates, run the
live self-test for the touched control on a desktop, and say in the issue what was and was not
seen.
