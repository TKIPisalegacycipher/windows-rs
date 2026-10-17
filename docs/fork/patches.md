# Fork patch catalogue

Every commit that `okojo-fast-list` (`dcfb63c07`) carries on top of upstream `d5b6a1f68`, grouped
by what it is for. Read [readme.md](readme.md) first for the background.

## How to read the tables

`#` is the commit's position in the stack, oldest first, so you can find it with
`git log --reverse upstream/master..okojo-fast-list`. `Okojo needs it for` is taken from Okojo's
`vendor/README.md` patch table. `Evidence` says how the change was checked, from the commit
messages and from Okojo's notes: **unit** is a headless test in the commit, **live** is a case in
the live self-test, and **app** is a check in the running Okojo application (79 items, 2026-10-10,
against fork commit `660a63bec`, so it does not cover later commits). Nothing here was re-run in a
live window for these pages; see [issues.md](issues.md#d-verification-debt).

Status is measured against upstream `master` at `ffe56ded6`:

| Status | Meaning | Count |
| --- | --- | --- |
| Landed | Upstream has the same behaviour under a different change. Drop it from the fork | 2 |
| Partly landed | Upstream has part of it | 1 |
| Fixed upstream | Upstream fixed the same bug a different way. Drop it, re-verify | 3 |
| Requested | Absent upstream, and the maintainer listed the gap in issue #5087 | 8 |
| New | Absent upstream and not requested. Needs an issue before a pull request | 28 |
| Dependent | Part of the `ListView` row-content rework; cannot be proposed one commit at a time | 7 |
| Hold | Little justification beyond Okojo, or the fix did not do what it claimed | 2 |
| Fixup | Belongs inside another commit; squash before proposing | 7 |

Issue #5087 ("native UI contract and runtime deployment gaps") is the maintainer's own list of
Reactor gaps, opened before the fork's work was posted anywhere. A commit that matches one of its
items has the strongest case available, because the maintainer already agreed the gap exists.

## 1. Already on upstream

Upstream merged these while the fork was being built. Remove them when rebasing.

| # | Commit | Change | Upstream equivalent |
| --- | --- | --- | --- |
| 14 | `71b4837cc` | `CalendarDatePicker::date`, a controlled date that can be set and cleared | #5093 adds `date`, `min_date`, `max_date` |
| 16 | `d3774c884` | `ContentDialog::default_button` | #5103, which addresses #5087 item 5 |
| 3 | `808e8c5e7` | A `MenuBarItem` listed every entry twice after its menu changed | #5079 removes menu-bar items one at a time for the same reason: WinUI does not mirror a vector reset into the flyout |
| 23 | `4b70ba830` | `on_color_scheme` replayed on every render, causing a render loop | #5071 keeps window observers subscribed across renders (fixes #5063) |
| 24 | `12f91d0b9` | A re-render while a menu was open replaced the menu and dropped the pick | #5079 keeps the native popup and swaps in the latest callback |
| 7 | `4e766cc11` | Foreground for `SymbolIcon` and `FontIcon` | `FontIcon` landed in #5106 (#5087 item 2). `SymbolIcon::foreground` has not |

## 2. Requested by the maintainer in #5087

| # | Commit | Change | #5087 item | Okojo needs it for | Evidence |
| --- | --- | --- | --- | --- | --- |
| 10 | `711c70441` | `ThemeBrush::Resource(key)`: any theme brush by resource key | 6 | Coloured icons and styles that follow light and dark | unit; app |
| 11 | `c32b55600` | `register_theme_colors` (light/dark/high-contrast brushes) and `register_resource_doubles` | 6 | Folder and category colours with a dark variant; compact `NavigationView` rows | unit; app |
| 13 | `275f86f65` | `.menu()` on any element becomes its context flyout | 13 | Right-click menus on folders, accounts, rows; Shift+F10 | unit; app |
| 1 | `008a8a725` | More `AcceleratorKey` values (F2-F12, Escape, Delete, Page keys, digits) and Alt modifiers | 14 | Shortcuts for categories, reminders, calendar | unit |
| 22 | `9063b2ec3` | `ListView::item_min_height` for that list only | 8 | Dense message rows without shrinking `AutoSuggestBox` suggestion rows | unit; live; app |
| 29 | `02106d9cd` | `DataItem::container_resource_overrides` | 8 | Date-group headers that must not show row hover | unit; live; app |
| 34 | `a4b5992a9` | `automation_live_setting` (`Off`, `Polite`, `Assertive`) | 4 | A "Saving..." message announced to screen readers | generated live coverage only |
| 36 | `abf0a135c` | `ListView::on_item_click`, keyed by `DataItem` | 12 | Group headers that toggle on Enter, Space or click | unit |

Two of these need reshaping before they are proposed:

- Item 14 asks for combined modifiers "including Control+Shift". The fork's
  `AcceleratorModifiers` has `Menu`, `ControlMenu` and `MenuShift` but no Control+Shift. Its
  draft already proposes the better shape: take the existing `VirtualKey` and `InputModifiers`
  instead of growing an enum. See
  [issues.md](issues.md#f3-accelerator-modifiers-cannot-express-controlshift).
- Item 6 asks for "scoped Light/Default/HighContrast dictionaries" with live re-resolution. The
  fork registers brushes app-wide before `App::run`. The keys are written into XAML without
  escaping; see [issues.md](issues.md#f5-resource-keys-are-inserted-into-xaml-unescaped).

## 3. Missing WinUI properties and events (New)

Each of these exposes a WinUI member Reactor does not. None is in #5087.

| # | Commit | Change | Okojo needs it for | Evidence |
| --- | --- | --- | --- | --- |
| 6 | `9eea877e4` | `is_hit_test_visible` on every element | A "now" line drawn over a calendar grid must not eat clicks | unit; app |
| 25 | `730cc92af` | `cursor`, through `UIElement.ProtectedCursor` | Left-right resize cursor on pane splitters | unit; app |
| 5 | `1b2b2715a` | `PointerEventInfo` gains `width` and `height` | Mapping x to a day column in the calendar | unit |
| 8 | `1949f796d` | `AutoSuggestBox::on_query_submitted` | Run the search on Enter or on the query icon | unit; live; app |
| 9 | `a568d350d` | `AutoSuggestBox` `on_got_focus` and `on_lost_focus` | Unbind single-key shortcuts while the search box has focus | unit; live; app (indirect) |
| 15 | `c12b0ab66` | `ElementRef<ScrollViewer>::request_vertical_offset` | Scroll the calendar to the current hour | unit; live; app |
| 26 | `38ca37af1` | `NavigationViewItem::background` | A favourite's twin drawn as selected | unit; live |
| 28 | `7e5c92895` | `ElementRef<Button>::request_hide_flyout` | An account flyout that closes after a row runs | unit; live |
| 49 | `2a7f1e258` | `WindowPolicy::activate(false)` opens a window without taking focus | A failed send's pop-out returning in the background | unit |
| 51 | `7e9aea584` | `InfoBar::action_button` and `content` | Undo-send bars with a button and progress | unit |
| 35 | `c594a87fe` | `NavigationView::on_item_invoked` | "Add account" and section headers reachable by keyboard | unit; live |
| 48 | `1f9242fe3` | `NavigationView` menu scroll offset, `on_item_expanding`, `on_item_collapsed` | Sticky account names drawn over the folder tree | unit; live |
| 43 | `14f2210a2` | Automation ids on menu items, `DataItem` rows and `ContentDialog` | UI tests that find controls by id | unit; live |
| 21 | `09e15fef5` | `ListView` item containers take the `DataItem` text as their automation name | Screen readers announce rows instead of an unnamed item | unit; live; app |
| 18 | `67df9bdcc` | `ListView::on_selected_keys_changed` for multi-select | Delete or move several selected messages | unit; live; app |
| 17 | `aed0fe619` | `KeyAccelerators::scoped`, which sets `ScopeOwner` | Enter in one text box without firing window-wide | unit; app |

## 4. Menus, icons and colour

| # | Commit | Change | Status | Notes |
| --- | --- | --- | --- | --- |
| 27 | `620752e53` | `MenuItem::icon` for any `Icon` | New | Upstream #4923: the maintainer said icons in menu items need API design. Propose with 12 |
| 12 | `058825bc0` | `MenuItem::swatch`, a coloured dot in the icon slot | New | A narrower version of 27. Fold into it |
| 19 | `3c95a0bd1` | `Icon::foreground` for icon values | New | Extends #5106. The `TitleBar` icon cannot be a themed brush (see [issues.md](issues.md#f6-titlebar-icon-theme-is-unverified)) |
| 2 | `9777d44b1` | Button and `DropDownButton` menus open below the button | New | Still the upstream behaviour: the flyout placement is left at its default |

## 5. Input

| # | Commit | Change | Status | Notes |
| --- | --- | --- | --- | --- |
| 46 | `609612f75` | The remaining twenty letter accelerator keys | Fixup | Belongs with 1 |
| 4 | `5f2d95a1a` | Queue `PointerReleased` before releasing pointer capture | New | Bug, still present upstream: `pointer_event_info` calls `ReleasePointerCapture` while building the event, and WinUI raises `PointerCaptureLost` inside that call. A finished drag looks like a cancelled one |

## 6. Drag and drop

Okojo drags messages out of its own list onto folder rows. It does this with an OLE
`DoDragDrop` of virtual `.eml` files, which XAML presents as storage items. Reactor's drop
policies only knew about files and text.

| # | Commit | Change | Status | Notes |
| --- | --- | --- | --- | --- |
| 32 | `07d4d2ec3` | `DragKind::Other` and `DragDropPolicy::other` accept a drag with no files and no text | New | Superseded in practice by 44: `other` matches any foreign drag too, so its docs now say to set it only during the app's own drag |
| 38 | `0e88c1f5b` | Cancel the `Other` drop callback if it fails to queue | Fixup | Releases a leaked deferral. Belongs with 32 |
| 44 | `86c99deab` | `DragDropPolicy::format` accepts a drag by private clipboard format | New | The reliable form of 32: a private format also tells your own drag from another app's |
| 45 | `01b9d70a6` | Queue an `Other` drop without a deferral | Fixup | Fixes a hang: the deferral's UI callback cannot run while XAML waits in `Drop` for the app's own `DoDragDrop` loop |
| 50 | `acfde14fc` | `DragDropPolicy::hide_image`; accepted drag events are marked handled | New | Stops XAML drawing a large file icon for drags of a private format |

Commits 38 and 45 add no test of their own. Commit 45 fixes a hang that only occurs when the
drag source is the application's own modal `DoDragDrop` loop, which a headless test cannot
reproduce.

## 7. `ListView` selection

| # | Commit | Change | Status | Notes |
| --- | --- | --- | --- | --- |
| 31 | `df9100551` | Apply `SelectedIndex` after the item changes of the same update | New | Bug: it named an item of the old list, so removing a row and selecting the next one across a header selected the header |
| 39 | `9194fe4cc` | The same for a freshly mounted `ListView` | Fixup | Belongs with 31 |
| 40 | `677bc7dcf` | Suppress selection changes during item edits only when an index is set | Fixup | Belongs with 31; lists without an index lost a real change |
| 42 | `981798f05` | Documentation: Ctrl+click and Ctrl+Space also raise `ItemClick` | Fixup | Belongs with 36 |

## 8. `ListView` row content and large lists

These eight commits are one piece of work: make a keyed `ListView` of hundreds of messages show
the right row content, switch quickly, and not build every row. They share state in the adapter
(`ListHosts`, `WaitingContent`) and in the reconciler, and each fixes a failure introduced or
exposed by the one before. They cannot be proposed one at a time.

| # | Commit | Change | Status |
| --- | --- | --- | --- |
| 30 | `660a63bec` | Disconnect a recycled container from its item's content element, and reconnect it when prepared. Fixes stale rows after keyed changes | Dependent |
| 47 | `7804302d7` | Keep content in recycled containers: XAML can hand a recycled container back to its item without preparing it, which left rows with zero height | Dependent |
| 53 | `82c1cb4b9` | One `Mutation::ReplaceItems` (native `ReplaceAll`) when a keyed change drops or adds more than `max(16, len / 2)` items | Dependent |
| 54 | `697e7af4c` | Re-select the kept items after that `ReplaceAll`, which resets the native selection | Dependent |
| 55 | `a0afd95e7` | Mount a `DataItem`'s content only when the list prepares its container | Dependent |
| 56 | `02f414d18` | Release an item's old container while its content is still waiting | Dependent |
| 57 | `a551bf5c5` | Mount the first 40 items' content with the items, so the first frame has correct heights | Dependent |
| 52 | `55159a48f` | `ComponentContext::skip_render()`: an update can say it changed nothing the view shows | New |

Why they exist, from the commit messages. A 500-row list mounted 500 rows of elements for a
screenful (55). Switching folders removed and inserted one row at a time (53). A no-op message
re-ran the whole `view` of a large component (52). Okojo's tracking issue for this is its #594.

Upstream overlaps, and it is the main risk to this group. Issue #5087 item 17 notes that the
adapter already uses `ReplaceAll` at its reorder threshold and asks whether it disrupts scroll
and focus; #5114 changed how keyed reorders are applied. Issue #5085 reports that an
`ItemsRepeater` of 200 keyed rows creates and re-renders every row. These commits touch the same
code paths, so they need a design conversation with the maintainer, not a drive-by pull request.
`ScrollIntoView` after keyed changes can crash `Microsoft.UI.Xaml.dll` with or without these
commits; the repro is in the body of `660a63bec`.

Test coverage is uneven. `7804302d7` (47) changes container handling and adds no test; the
others add unit tests, and four also extend the live self-test. A regression test for 47 should
exist before any of this is proposed.

## 9. Dialogs, windows and navigation

| # | Commit | Change | Status | Notes |
| --- | --- | --- | --- | --- |
| 58 | `dcfb63c07` | Swallow `NavigationView`'s late "nothing selected" event after the reactor deselects | New | Bug: WinUI queues that event at low priority, after the reactor's own-write suppression has ended. It cost a full render per folder pick in Okojo |
| 33 | `12871a8f1` | Hide a shown `ContentDialog` when its element is released | Hold | Its own successor says the premise was wrong: the reconciler already hides the dialog first |
| 41 | `039255210` | Make that hide best effort and add a live test | Hold | Keep only if it is tied to the dialog teardown crash (issues.md U7) |

Okojo uses neither 33 nor 41 directly (its own bug, #212, was fixed in the app).

## 10. Generator and bindgen

| # | Commit | Change | Status | Notes |
| --- | --- | --- | --- | --- |
| 20 | `429496943` | `windows-bindgen`: ignore filtered-out methods when finding event-only delegates | New | Removes dead delegate code from regenerated Reactor bindings. Not checked against upstream's bindgen changes #5075 and #5077 |
| 37 | `bc1990b8e` | Regenerate `public-api.txt` for `AutomationLiveSetting` | Fixup | Belongs with 34 |

Three generator bugs are fixed inside other commits and should be proposed as their own small
changes (they are listed in [issues.md](issues.md#c-generator-bugs-found)): enum probing with a
hard-coded `::None` variant (inside 34), an event-args cast that assumed the default interface
(inside 35), and a `?` emitted in a function that returns `Option` (inside 51).

## Public API impact

Comparing `public-api.txt` at `d5b6a1f68` and at `dcfb63c07` gives 366 added lines and three
changed items. The changed items are source-breaking for code that names them:

| Item | Change | Commit |
| --- | --- | --- |
| `ThemeBrush` | Loses `#[repr(u8)]` because `Resource(&'static str)` carries data | 10 |
| `Icon` | Tuple struct becomes a struct with a foreground | 19 |
| `KeyAccelerators` | Tuple struct becomes a struct with a `scoped` flag | 17 |

Additions that break exhaustive code because these types are not `#[non_exhaustive]`:
new variants on `AcceleratorKey`, `AcceleratorModifiers`, `DragKind` and `DroppedData`; new
fields on `MenuItem::Item`, `MenuItem::Submenu`, `PointerEventInfo` and `DragDropPolicy`. Reactor
calls itself a preview and says releases may refine public APIs, but each proposal should state
this, and [issues.md](issues.md#f4-public-types-are-not-non_exhaustive) recommends fixing it once.
