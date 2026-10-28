# Fork patch catalogue

Every commit on `okojo-flow-layout` on top of upstream `ffe56ded6`, grouped by what it is for,
followed by a map from the original 58-commit stack. Commits 1-45 are the rebuilt stack
(`okojo-rebased`, `47c5b855d`); 46-63 were added on top of it since. Read [readme.md](readme.md) first for the
background.

## How to read the tables

`#` is the commit's position in the stack, oldest first (`git log --reverse
upstream/master..okojo-flow-layout`), counting only code commits: the fork's documentation
commits, such as the one that added these pages, are not numbered. The numbers match Okojo's
`vendor/README.md` patch table. `Was` is the number in the original stack
(`okojo-fast-list`, `none` for a commit made after it) and is explained in [the map below](#what-happened-to-the-original-58).
`Okojo needs it for` is taken from Okojo's `vendor/README.md` patch table.

`Evidence` says how the change was checked, from what each commit touches: **unit** is a headless
test in the commit, **live** is a case added to the live self-test or an integration binary, and
**app** is a check in the running Okojo application (79 items, 2026-10-10, against the original
fork commit `660a63bec`, so it does not cover later work); later commits name their own check.
**desktop** is an `#[ignore]`d test that starts a real WinUI `Application`, which Okojo runs on its
desktop runners. Every commit passed the headless gates
described in [readme.md](readme.md#build-and-test) when it was rebuilt. No live test and no
application run was repeated on the rebuilt stack; see
[issues.md](issues.md#d-verification-debt).

Status is measured against upstream `master` at `ffe56ded6`:

| Status | Meaning | Count |
| --- | --- | --- |
| Prep | Groundwork the other commits depend on | 4 |
| Requested | Absent upstream, and the maintainer listed the gap in issue #5087 | 8 |
| New | Absent upstream and not requested. Needs an issue before a pull request | 43 |
| Dependent | Part of the `ListView` row-content rework; cannot be proposed one commit at a time | 8 |

Issue #5087 ("native UI contract and runtime deployment gaps") is the maintainer's own list of
Reactor gaps. A commit that matches one of its items has the strongest case available, because the
maintainer already agreed the gap exists.

## 1. Preparation

These four go first because later commits need them. The three generator commits are output
neutral: regenerating after each leaves no diff.

| # | Commit | Change | Why |
| --- | --- | --- | --- |
| 1 | `35f63f776` | `#[non_exhaustive]` on `ThemeBrush`, `AcceleratorKey`, `MenuItem` (and its `Item` and `Submenu` variants), `PointerEventInfo`, `DragKind`, `DragDropPolicy`, `DroppedData` | Every later commit adds variants or fields to these. Without the attribute each addition breaks downstream matches and struct literals. `ContentDialogResult` already has it |
| 2 | `69678c087` | `tool-reactor` probes an attached enum property with its first variant | The generator proved an enum was in scope with `native::<Enum>::None`, which only some enums have. Needed for `AutomationLiveSetting` |
| 3 | `172b7b0a1` | `tool-reactor` reads item-tag payloads from the interface that declares them | `InvokedItemContainer` lives on the event args' second interface; the generated call could not compile. Needed for `NavigationView` `ItemInvoked` |
| 4 | `415ba5396` | `tool-reactor` casts a single-child relation item without `?` | The generated setter used `?` in a function returning `Option`. Needed for `InfoBar` `ActionButton` |

## 2. Requested by the maintainer in #5087

| # | Commit | Change | Was | #5087 item | Okojo needs it for | Evidence |
| --- | --- | --- | --- | --- | --- | --- |
| 12 | `687994570` | More `AcceleratorKey` values (F2 to F12, Escape, Delete, Home, Page keys, digits 1 to 4, every letter). `KeyAccelerator::new` takes `impl Into<InputModifiers>`, so Control+Shift, Control+Alt and Alt+Shift work | 1, 46 | 14 | Shortcuts for categories, reminders, calendar | unit |
| 16 | `a99d8c30f` | `ThemeBrush::Resource(key)`: any theme brush by resource key | 10 | 6 | Coloured icons and styles that follow light and dark | unit; app |
| 17 | `6da2a2abe` | `register_theme_colors` (light, dark, high-contrast brushes) and `register_resource_doubles` | 11 | 6 | Folder and category colours with a dark variant; compact `NavigationView` rows | unit; app |
| 19 | `f1c780109` | `.menu()` on any element becomes its context flyout | 13 | 13 | Right-click menus on folders, accounts and rows; Shift+F10 | unit; app |
| 25 | `015b7f10c` | `ListView::item_min_height` for that list only | 22 | 8 | Dense message rows without shrinking `AutoSuggestBox` suggestion rows | unit; live; app |
| 28 | `6342d3ab4` | `DataItem::container_resource_overrides` | 29 | 8 | Date-group headers that must not show row hover | unit; live; app |
| 29 | `a831943c0` | `automation_live_setting` (`Off`, `Polite`, `Assertive`) | 34, 37 | 4 | A "Saving..." message announced to screen readers | generated live coverage only |
| 30 | `b820ceb4a` | `ListView::on_item_click`, keyed by `DataItem` | 36, 42 | 12 | Group headers that toggle on Enter, Space or click | unit |

Two of these were reshaped from the original stack:

- Commit 12. #5087 item 14 asks for combined modifiers "including Control+Shift". The original
  `AcceleratorModifiers` enum had no such value and grew a variant per combination. It is back to
  upstream's two values, which convert into `InputModifiers`, and any combination is written with
  `InputModifiers` (which gains `BitOr`). Existing calls compile unchanged.
- Commit 16. Resource keys are restricted to ASCII letters, digits, `_` and `.`, and any other key
  panics, so a key cannot break the generated XAML. See
  [issues.md](issues.md#f5-resource-keys-are-inserted-into-xaml-unescaped).

## 3. Missing WinUI properties and events (New)

Each of these exposes a WinUI member Reactor does not. None is in #5087.

| # | Commit | Change | Was | Okojo needs it for | Evidence |
| --- | --- | --- | --- | --- | --- |
| 5 | `e86ea4e72` | `is_hit_test_visible` on every element | 6 | A "now" line drawn over a calendar grid must not eat clicks | unit; app |
| 7 | `68b2f26d8` | `cursor`, through `UIElement.ProtectedCursor` | 25 | Left-right resize cursor on pane splitters | unit; app |
| 6 | `f6040c560` | `PointerEventInfo` gains `width` and `height` | 5 | Mapping x to a day column in the calendar | unit |
| 8 | `62cee668b` | `AutoSuggestBox::on_query_submitted` | 8 | Run the search on Enter or on the query icon | unit; live; app |
| 9 | `9615287e4` | `AutoSuggestBox` `on_got_focus` and `on_lost_focus` | 9 | Unbind single-key shortcuts while the search box has focus | unit; live; app (indirect) |
| 10 | `e62f5408c` | `ElementRef<ScrollViewer>::request_vertical_offset` | 15 | Scroll the calendar to the current hour | unit; live; app |
| 11 | `0178bb5d3` | `ElementRef<Button>::request_hide_flyout` | 28 | An account flyout that closes after a row runs | unit; live |
| 26 | `5cc3bde45` | `NavigationViewItem::background` | 26 | A favourite's twin drawn as selected | unit; live |
| 31 | `7bdf68919` | `NavigationView::on_item_invoked` | 35 | "Add account" and section headers reachable by keyboard | unit; live |
| 34 | `edf118e4a` | `NavigationView` menu scroll offset, `on_item_expanding`, `on_item_collapsed` | 48 | Sticky account names drawn over the folder tree | unit; live |
| 32 | `a26741a7e` | `InfoBar::action_button` and `content` | 51 | Undo-send bars with a button and progress | unit |
| 33 | `25b1d8b57` | `WindowPolicy::activate(false)` opens a window without taking focus | 49 | A failed send's pop-out returning in the background | unit |
| 20 | `9c2b32449` | `KeyAccelerators::scoped`, which sets `ScopeOwner` | 17 | Enter in one text box without firing window-wide | unit; app |
| 21 | `2c839cb71` | `ListView::on_selected_keys_changed` for multi-select | 18 | Delete or move several selected messages | unit; live; app |
| 24 | `3c5ed057c` | `ListView` item containers take the `DataItem` text as their automation name | 21 | Screen readers announce rows instead of an unnamed item | unit; live; app |
| 37 | `ba0749bf0` | Automation ids on menu items, `DataItem` rows and `ContentDialog` | 43 | UI tests that find controls by id | unit; live |
| 47 | `6348288cf` | `ListView::item_container_transitions`, which sets `ItemsControl.ItemContainerTransitions` (an empty list turns the entrance transitions off) | none | A message list whose rows show at once instead of fading and sliding in | unit; live |
| 62 | `53ae68930` | `ElementRef<WebView2>::request_core_web_view2_with_environment`, which starts the control's core in a given `CoreWebView2Environment` (`IWebView22::EnsureCoreWebView2WithEnvironmentAsync`); a view keeps the environment it started in | none | Compose editors whose environment `Language` picks WebView2's spelling dictionary | unit |
| 48 | `db87b0e02` | `ItemsRepeater::layout(FlowLayout)`: a layout that wraps items onto lines (first on WinUI `LinedFlowLayout`, replaced by 51) | none | Reading-pane recipient chips that wrap | unit |
| 51 | `666cc88c6` | `FlowLayout` backed by an exact `NonVirtualizingLayout` | none | Chips at their measured width; `LinedFlowLayout` estimated unmeasured items and cropped later chips | unit |
| 49 | `2ea8be5bb` | `ListView::resource_overrides` | none | Clearing the item fills once, on the list, instead of per row | unit |
| 52 | `398477921` | `ElementRef<ListView>::observe_visible_range`, from the list's `ScrollViewer` `ViewChanged` and its `ItemsStackPanel`'s `FirstVisibleIndex` / `LastVisibleIndex` | none | Pre-rendering the messages on screen in the reading pane | unit |
| 54 | `c37a40b7e` | `reference = true` on `ListView` in the generator's `schema.toml`, so 52's `ElementRef<ListView>` code is generated and survives regeneration | none | Nothing new; goes upstream folded into 52 | regeneration leaves no diff |
| 56 | `8ad306f3a` | A `Border`'s flyout (`flyout_with`), `ElementRef<Border>::request_show_flyout_then` and its closed callback, `OverlayInputPassThroughElement` | none | A contact card opened from a photo tile, closed when the tile's flyout is dismissed | unit |

## 4. Theming, icons and menus

| # | Commit | Change | Was | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| 15 | `6218cd8ec` | `SymbolIcon::foreground` | 7 | New | The `FontIcon` half of the original landed upstream in #5106, which addresses #5087 item 2 |
| 22 | `c0f2af652` | `Icon::foreground` for icon values | 19 | New | Extends #5106. A title-bar icon is an icon source: its theme brush is resolved once and may not follow theme changes, as `Icon::foreground` documents |
| 27 | `c4ee30434` | `MenuItem::icon` for any non-image `Icon` | 27 | New | Upstream #4923: the maintainer said menu icons need API design. An image icon is refused with `GraphError::InvalidMenuIcon`, because menus own no image source |
| 18 | `88d8519dd` | `MenuItem::swatch`, a coloured dot in the icon slot | 12 | New | A narrower version of 27. Propose together |
| 53 | `1e2dbc6e3` | Context menus build their items as they open (`FlyoutBase::Opening`, `LazyMenu`) | none | New | A menu on every message-list row costs little until one opens. Evidence: unit |
| 55 | `452247a9a` | `Menu::on_opening` | none | New | Bug-adjacent: selecting a row on the right-button release raced WinUI's context request, and a selection that ran first cancelled the menu. Selecting as the menu opens avoids that. Evidence: unit |
| 60 | `93338694e` | `register_resource_fonts`: app-wide `FontFamily` resources by family name (it loads no font files) | none | New | Lets an app set its interface font through `ContentControlThemeFontFamily`. Evidence: unit |
| 61 | `cd6484d6b` | `register_text_block_font`, an opt-in implicit `TextBlock` style that sets `FontFamily`, so plain text uses the app's interface font | none | New | A `TextBlock` does not inherit `ContentControlThemeFontFamily`, so `register_resource_fonts` misses plain text. The style is merged after the Fluent dictionaries. A local or template `FontFamily` still wins, but an inherited one loses, so it is opt-in. Evidence: unit |
| 63 | `9e55c72a6` | `ButtonStyle::Default` and theme brushes keep the app's implicit (Fluent) style | none | New | Bug. Setting a local null `Style` hides the implicit style, and a brush `Style` without `BasedOn` replaces it, so a `Default` or themed `Button` fell back to the framework's square template. Now the Style is cleared, and a brush style is based on `Default<Type>Style` when the app has it; 61's `TextBlock` style is keyed `DefaultTextBlockStyle` for the same reason. Evidence: unit; desktop |
| 13 | `b7e6f5e8a` | Button and `DropDownButton` menus open below the button | 2 | New | Bug. Upstream still leaves the flyout placement at its default |

## 5. Input fixes

| # | Commit | Change | Was | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| 14 | `93ff359f5` | Queue `PointerReleased` before releasing pointer capture | 4 | New | Bug, still present upstream: `pointer_event_info` calls `ReleasePointerCapture` while building the event, and WinUI raises `PointerCaptureLost` inside that call. A finished drag looks like a cancelled one |
| 57 | `6e6f33ef2` | `Border::handles_plain_presses`: a plain left press and its release are taken by the `Border`; Ctrl, Shift and right presses still reach the list row | none | New | A photo tile that opens its flyout without selecting the row it sits in |

## 6. Drag and drop

Okojo drags messages out of its own list onto folder rows with an OLE `DoDragDrop` of virtual
`.eml` files, which XAML presents as storage items. Reactor's drop policies only knew about files
and text.

| # | Commit | Change | Was | Status |
| --- | --- | --- | --- | --- |
| 38 | `795067612` | `DragDropPolicy::format` accepts a drag by a private clipboard format (`DragKind::Other`, `DroppedData::Other`); the drop is queued without a deferral; a rejected drop callback is cancelled; `DragDropPolicy::hide_image` hides the drag picture | 32, 38, 44, 45, 50 | New |

The original stack also had `DragDropPolicy::other`, which accepted any drag with neither files nor
text. It matched other applications' drags as well, carried no payload, and never matched Okojo's
own virtual-file drag. It was removed, and the drop path it shared is reached only through
`format`.

The deferral-free queueing fixes a hang that only occurs when the drag source is the application's
own modal `DoDragDrop` loop, which a headless test cannot reproduce.

## 7. `ListView` selection and navigation fixes

| # | Commit | Change | Was | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| 35 | `d0d2e1195` | Apply `SelectedIndex` after the item changes of the same update, on mount and on update; suppress the selection events WinUI raises meanwhile, only for lists that declare an index | 31, 39, 40 | New | Bug: the index named an item of the old list, so removing a row and selecting the next one across a header selected the header |
| 36 | `963bb1485` | Swallow `NavigationView`'s late "nothing selected" event after the reactor deselects | 58 | New | Bug: WinUI queues that event at low priority after the reactor's own-write suppression has ended. It cost a full render per folder pick in Okojo |
| 58 | `477d35f4d` | `ElementRef<ListView>::request_selected_keys`: the runtime resolves keys to items; the adapter edits `SelectedItems` with `SelectionChanged` held and reports one change | none | New | A row checkbox that ticks the row into the list's own selection |
| 59 | `aa8e6d812` | `ElementRef<ListView>::request_focused_key`: the realized container whose `FocusState` isn't `Unfocused` | none | New | Ctrl+Space ticks the focused row, which Ctrl+arrow keys can move off the selected one |

## 8. `ListView` row content and large lists

These commits are one piece of work (46 and 50 are later fixes to it): make a keyed `ListView` of hundreds of messages show the
right row content, switch quickly, and not build every row. They share state in the adapter
(`ListHosts`, `WaitingContent`) and in the reconciler, and each fixes a failure introduced or
exposed by the one before. They cannot be proposed one at a time.

| # | Commit | Change | Was | Status |
| --- | --- | --- | --- | --- |
| 40 | `a9f45f69f` | Disconnect the container an item was last prepared in when another is prepared, or before the item is removed; leave a recycled container connected, since XAML can hand it back to the same item | 30, 47 | Dependent |
| 41 | `f708a5081` | One `Mutation::ReplaceItems` (native `ReplaceAll`) when a keyed change drops or adds more than `max(16, len / 2)` items | 53 | Dependent |
| 42 | `23a9cebb1` | Re-select the kept items after that `ReplaceAll`, which resets the native selection | 54 | Dependent |
| 43 | `373b522a0` | Mount a `DataItem`'s content only when the list prepares its container | 55 | Dependent |
| 44 | `e1595905a` | Release an item's old container while its content is still waiting | 56 | Dependent |
| 45 | `47c5b855d` | Mount the first 40 items' content with the items, so the first frame has correct heights | 57 | Dependent |
| 46 | `be56fbd7d` | Connect a container again when it keeps another item's content | none | Dependent |
| 50 | `e042b86d5` | Prune `ListHosts` when a list item is released, so a new item reusing its address starts clean | none | Dependent |

A related change stands apart, because it is not specific to `ListView`:

| # | Commit | Change | Was | Status |
| --- | --- | --- | --- | --- |
| 39 | `036fe741f` | `ComponentContext::skip_render()`: an update can say it changed nothing the view shows | 52 | New (design question) |

`skip_render` skips `view` and the reconcile for one message. Effects and window publication
declared from state a skipped update changed run at the next render, and the rustdoc says to call
it only when the message changed nothing that `view`, an effect or the window publication reads.
Two unit tests pin the behaviour.

Why the group exists, from the commit messages. A 500-row list mounted 500 rows of elements for a
screenful. Switching folders removed and inserted one row at a time. A no-op message re-ran the
whole `view` of a large component. Okojo's tracking issue for this is its #594.

Upstream overlaps, and it is the main risk to this group. Issue #5087 item 17 notes that the adapter
already uses `ReplaceAll` at its reorder threshold and asks whether it disrupts scroll and focus;
#5114 changed how keyed reorders are applied. Issue #5085 reports that an `ItemsRepeater` of 200
keyed rows creates and re-renders every row. These commits touch the same code paths, so they need a
design conversation with the maintainer, not a drive-by pull request. `ScrollIntoView` after keyed
changes can crash `Microsoft.UI.Xaml.dll` with or without these commits; the repro is in the body of
commit 40.

Test coverage is uneven. The recycled-container handling in commit 40 is native behaviour that a
headless test cannot reach; it has a live self-test case that was not run here. Commit 45 and the
drag commit likewise rest on live behaviour.

## 9. Bindgen

| # | Commit | Change | Was | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| 23 | `eb7477cfc` | `windows-bindgen`: ignore filtered-out methods when finding event-only delegates | 20 | New | Removes dead delegate code from regenerated Reactor bindings. Rebuilding the stack showed it is required: without it the first `ListView` naming commit fails `-D warnings`. Output-neutral for the other generated crates (`tool-bindings` leaves no diff) |

## What happened to the original 58

| Was | Original commit | Now |
| --- | --- | --- |
| 1 | `008a8a725` | 12, merged with 46 |
| 2 | `9777d44b1` | 13 |
| 3 | `808e8c5e7` | Dropped: upstream #5079 removes menu-bar items one at a time for the same reason |
| 4 | `5f2d95a1a` | 14 |
| 5 | `1b2b2715a` | 6 |
| 6 | `9eea877e4` | 5 |
| 7 | `4e766cc11` | 15, `SymbolIcon` only; the `FontIcon` half landed upstream (#5106) |
| 8 | `1949f796d` | 8 |
| 9 | `a568d350d` | 9 |
| 10 | `711c70441` | 16 |
| 11 | `c32b55600` | 17 |
| 12 | `058825bc0` | 18 |
| 13 | `275f86f65` | 19 |
| 14 | `71b4837cc` | Dropped: upstream #5093 adds `CalendarDatePicker::date`, `min_date`, `max_date` |
| 15 | `c12b0ab66` | 10 |
| 16 | `d3774c884` | Dropped: upstream #5103 adds `ContentDialog::default_button` |
| 17 | `aed0fe619` | 20 |
| 18 | `67df9bdcc` | 21 |
| 19 | `3c95a0bd1` | 22 |
| 20 | `429496943` | 23 |
| 21 | `09e15fef5` | 24 |
| 22 | `9063b2ec3` | 25 |
| 23 | `4b70ba830` | Dropped: upstream #5071 keeps window observers subscribed across renders |
| 24 | `12f91d0b9` | Dropped: upstream #5079 keeps the native popup and swaps in the latest callback |
| 25 | `730cc92af` | 7 |
| 26 | `38ca37af1` | 26 |
| 27 | `620752e53` | 27 |
| 28 | `7e5c92895` | 11 |
| 29 | `02106d9cd` | 28 |
| 30 | `660a63bec` | 40, merged with 47 |
| 31 | `df9100551` | 35, merged with 39 and 40 |
| 32 | `07d4d2ec3` | 38, without `DragDropPolicy::other` |
| 33 | `12871a8f1` | Dropped: its own successor says the reconciler already hides the dialog first |
| 34 | `a4b5992a9` | 29; its generator fix is commit 2 |
| 35 | `c594a87fe` | 31; its generator fix is commit 3 |
| 36 | `abf0a135c` | 30, merged with 42 |
| 37 | `bc1990b8e` | 29 (the regenerated API snapshot) |
| 38 | `0e88c1f5b` | 38 |
| 39 | `9194fe4cc` | 35 |
| 40 | `677bc7dcf` | 35 |
| 41 | `039255210` | Dropped: it only made the dropped commit 33 best effort |
| 42 | `981798f05` | 30 |
| 43 | `14f2210a2` | 37 |
| 44 | `86c99deab` | 38 |
| 45 | `01b9d70a6` | 38 |
| 46 | `609612f75` | 12 |
| 47 | `7804302d7` | 40 |
| 48 | `1f9242fe3` | 34 |
| 49 | `2a7f1e258` | 33 |
| 50 | `acfde14fc` | 38 |
| 51 | `7e9aea584` | 32; its generator fix is commit 4 |
| 52 | `55159a48f` | 39 |
| 53 | `82c1cb4b9` | 41 |
| 54 | `697e7af4c` | 42 |
| 55 | `a0afd95e7` | 43 |
| 56 | `02f414d18` | 44 |
| 57 | `a551bf5c5` | 45 |
| 58 | `dcfb63c07` | 36 |

Seven were dropped (3, 14, 16, 23, 24, 33, 41), ten were merged into a neighbour (37, 38, 39, 40,
42, 44, 45, 46, 47, 50) and four commits are new (1 to 4). The seven dropped commits are not lost:
they remain reachable from the tag `archive/okojo-fast-list` (the branch itself is retired).
Commits 46-63 came after the rebuild and have no original.

## Public API impact

Comparing `public-api.txt` at upstream `ffe56ded6` and at `47c5b855d`: the stack adds public items
and changes a few. Commits 46-63 only add items (`FlowLayout`, `ItemsRepeater::layout`,
`ListView::resource_overrides`, the `ElementRef<ListView>`, `ElementRef<Border>` and `WebView2`
requests, `Menu::on_opening`, `Border::handles_plain_presses`, `register_resource_fonts`,
`register_text_block_font`); none changes an existing item. `register_resource_fonts` is missing
from `public-api.txt`, which commit 60 did not update. These are source-breaking for code that names them:

| Item | Change | Commit |
| --- | --- | --- |
| `ThemeBrush` | Loses `#[repr(u8)]` because `Resource(&'static str)` carries data | 16 |
| `Icon` | Tuple struct becomes a struct with a foreground | 22 |
| `KeyAccelerators` | Tuple struct becomes a struct with a `scoped` flag | 20 |
| `KeyAccelerator::new` | Parameter is `impl Into<InputModifiers>` instead of `AcceleratorModifiers` | 12 |
| `MenuItem` | Items and submenus gain `icon` (and items `swatch`) fields | 18, 27 |

Types the stack grows are `#[non_exhaustive]` since commit 1, so later additions of variants and
fields are not breaking. The attribute itself is breaking for code that matches them exhaustively
or builds them with a struct literal. Reactor calls itself a preview and says releases may refine
public APIs, but each proposal should state this.
