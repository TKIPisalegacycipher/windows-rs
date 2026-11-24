# Fork patch catalogue

Every commit on `okojo-flow-layout` on top of upstream `ffe56ded6`, grouped by what it is for,
followed by a map from the original 58-commit stack.
Commits 1-45 are the rebuilt stack (`okojo-rebased`, `47c5b855d`); 46-93 were added on top of it
since. Read [readme.md](readme.md) first for the background.

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
| New | Absent upstream and not requested. Needs an issue before a pull request | 61 |
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
| 64 | `43ba76c08` | `Border::use_system_focus_visuals`, `Border::focus_visual_margin` and the attached `automation_accessibility_view` (`AutomationProperties.AccessibilityView`) | none | A focusable search caption that draws the system focus rectangle and is not read twice | unit |
| 65 | `db0deeedc` | `ScrollViewer::vertical_anchor_ratio` (`VerticalAnchorRatio`, checked to 0..=1) and `can_be_scroll_anchor` on every element (`UIElement.CanBeScrollAnchor`); the generator gains a `unit_interval` validation | none | A Messages thread pinned to its newest message while older rows prepend (ratio 1.0, rows as anchor candidates) | unit |
| 74 | `9d7ff6ee1` | `NavigationViewItem` reports focus (`on_got_focus` / `on_lost_focus`, the `focus` adapter as on `Border` and `AutoSuggestBox`) and takes an `ElementRef` (`focus = true`, so `request_focus`) | none | A folder's tree item is built anew when the folder gains its first subfolder or loses its last (WinUI shows no first child added to an item that had none); the new item gets the keyboard focus the old one held (Okojo#1469) | unit |
| 76 | `481174e04` | `Button::on_double_tapped` (`UIElement.DoubleTapped`, a unit event; binds `IUIElement::{add_DoubleTapped, remove_DoubleTapped}`) | none | A month-view day cell (a button, whose click carries no input time) opens a new all-day event on a double click paired by WinUI's gesture recognizer from the input's own timing, not by when Okojo handled each click (Okojo#1650) | unit |
| 86 | `60b4a204` | `TextBlock::font_family`, which sets `TextBlock.FontFamily` as a local value (above the implicit style `register_text_block_font` installs; a comma-separated fallback list as XAML takes it; undeclared, the value is cleared). tool-reactor gains a `font_family` property adapter; binds `ITextBlock::put_FontFamily`, `ITextBlockStatics::get_FontFamilyProperty` and `FontFamily::CreateInstanceWithName`, set through the generator's `Uri` property arm, which picks the constructor by adapter (`37a479ac`, a review fix on the same branch) | none | A verification code in a monospace face, so it reads digit by digit with every digit the same width, as codes are set elsewhere (Okojo#1227) | unit (`tests/declarations.rs`; reactor tests pass, Okojo worker check 44195767363); app (Okojo GUI runs 44195508128 before / 44195507871 after: codes and sign-in links in light, dark, high contrast and 175 % scale) |
| 89 | `1eef83ce` | `TextBlock::text_highlighters(impl IntoIterator<Item = TextHighlighter>)`, which sets `TextBlock.TextHighlighters`: each `TextHighlighter` is a set of `TextRange`s (`start` and `length` in UTF-16 code units, as WinUI's `TextRange` counts them) with an optional background and foreground colour; the collection is replaced as a whole when the declared highlighters change and cleared when none are declared, each highlighter filled before it is added; declared after `Text`, so a render that changes both sets the ranges on the new text. `AutoSuggestBox::is_suggestion_list_open(bool)` (`IsSuggestionListOpen`, cleared when undeclared; after `ItemsSource` and `Text`, so the list opens over its new items). tool-reactor gains a `text_highlighters` property adapter (a native collection, replaced as `GridRows` is) and the `TextHighlighters` value type; binds `ITextBlock::get_TextHighlighters`, `ITextHighlighter::{get_Ranges, put_Background, put_Foreground}`, `TextHighlighter::CreateInstance`, `TextRange`, `IAutoSuggestBox::put_IsSuggestionListOpen` and `IAutoSuggestBoxStatics::get_IsSuggestionListOpenProperty` | none | A search's hits marked in the message list's subjects and previews and the reading pane's subject, and the search box's drop-down opened on the recent searches when the empty box gets the focus (Okojo#2603) | unit |
| 90 | `282932d5` | `ContentDialog::resource_overrides(ResourceOverrides)` (`FrameworkElement.Resources` on the dialog, the `resource_overrides` adapter as on `Button` and `ListView`; undeclared, the keys it set are removed), and `ResourceValue::Double(f64)` (`From<f64>`; `ResourceOverrides::set` panics on a NaN, infinite or negative one), boxed as an `IReference<f64>` (`PropertyType::Double`). A dialog's properties are written when it is mounted or reconciled, before the `SetContentDialogOpen` that calls `ShowAsync`, so overrides declared with the dialog are in its `Resources` when the template reads them. WinUI resolves a `ThemeResource` such as `ContentDialogMaxWidth` when the template is applied (and again on a theme change), and the reactor reuses one native dialog across its openings, so a value changed after the first declaration reaches the dialog's `Resources` but may never apply to that native dialog: declare the final values with the dialog's first declaration, or give the dialog a new key to get a new native dialog (not verified on a desktop) | none | The add-account dialog 640-720 px wide, past `ContentDialogMaxWidth` (548), by overriding the theme resource on the dialog itself, the supported way (Okojo#2861) | unit (`tests/declarations.rs`, `tests/dialogs.rs`, `tests/native/winui.rs` for the boxing) |

## 4. Theming, icons and menus

| # | Commit | Change | Was | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| 15 | `6218cd8ec` | `SymbolIcon::foreground` | 7 | New | The `FontIcon` half of the original landed upstream in #5106, which addresses #5087 item 2 |
| 22 | `c0f2af652` | `Icon::foreground` for icon values | 19 | New | Extends #5106. A title-bar icon is an icon source: its theme brush is resolved once and may not follow theme changes, as `Icon::foreground` documents |
| 27 | `c4ee30434` | `MenuItem::icon` for any non-image `Icon` | 27 | New | Upstream #4923: the maintainer said menu icons need API design. An image icon is refused with `GraphError::InvalidMenuIcon`, because menus own no image source |
| 18 | `88d8519dd` | `MenuItem::swatch`, a coloured dot in the icon slot | 12 | New | A narrower version of 27. Propose together |
| 53 | `1e2dbc6e3` | Context menus build their items as they open (`FlyoutBase::Opening`, `LazyMenu`) | none | New | A menu on every message-list row costs little until one opens. Evidence: unit |
| 55 | `452247a9a` | `Menu::on_opening` | none | New | Bug-adjacent: selecting a row on the right-button release raced WinUI's context request, and a selection that ran first cancelled the menu. Selecting as the menu opens avoids that. Evidence: unit |
| 75 | `56e50fbea` | `Menu::on_closed` (`FlyoutBase::Closed`, context and button menus; `MenuClosed` joins tool-reactor's hand-listed events) | none | New | With 55's `on_opening` the app knows while a menu is open: Okojo keeps a folder's tree item in place while its menu is open, since rebuilding the item would close the menu (Okojo#1469). Evidence: unit |
| 81 | `af365bf8` | A button's menu (`Button`, `DropDownButton`) builds its items as its flyout opens, as 53's context menus do: `set_menu` keeps the one flyout when the items change instead of hiding it and building a new one, so an open menu keeps the items it opened with and the next opening builds the latest. A click on the items an open flyout still shows, after a newer declaration replaced them, goes to the callback declared with them (`LazyMenu::shown`), as a held dispatch (`EventDispatch::held`) that `validate_event_dispatch` takes while the element still has a menu; context menus get the same routing | none | New | Bug, and a design question like 78: upstream hides an open button menu whenever its items change, so a menu closed under the pointer when a background sync changed it (Okojo#2552), and it would deliver a stale click to the latest callback or drop it. Here the click reaches the callback declared with the items the user saw, since the latest may not know the item or may read its key differently. Evidence: unit (`tests/native/button_menu.rs`, a fake flyout driving `NativeMenu` and `LazyMenu`; `tests/attachments.rs` for the held click); app (Okojo's `button-menu-keeps-items` scenario) |
| 60 | `93338694e` | `register_resource_fonts`: app-wide `FontFamily` resources by family name (it loads no font files) | none | New | Lets an app set its interface font through `ContentControlThemeFontFamily`. Evidence: unit |
| 61 | `cd6484d6b` | `register_text_block_font`, an opt-in implicit `TextBlock` style that sets `FontFamily`, so plain text uses the app's interface font | none | New | A `TextBlock` does not inherit `ContentControlThemeFontFamily`, so `register_resource_fonts` misses plain text. The style is merged after the Fluent dictionaries. A local or template `FontFamily` still wins, but an inherited one loses, so it is opt-in. Evidence: unit |
| 77 | `0b2f4b6b` | `RichText::on_link_click` (`Hyperlink::Click` instead of `NavigateUri`; `LinkClick` joins tool-reactor's hand-listed events), `RichTextBlock::foreground`, and `register_text_block_font` also styles `RichTextBlock` | none | New | A text message's links open where the app chooses (the account's browser), and rich text matches plain text's font and colour (Okojo#2002). Evidence: unit |
| 63 | `9e55c72a6` | `ButtonStyle::Default` and theme brushes keep the app's implicit (Fluent) style | none | New | Bug. Setting a local null `Style` hides the implicit style, and a brush `Style` without `BasedOn` replaces it, so a `Default` or themed `Button` fell back to the framework's square template. Now the Style is cleared, and a brush style is based on `Default<Type>Style` when the app has it; 61's `TextBlock` style is keyed `DefaultTextBlockStyle` for the same reason. Evidence: unit; desktop |
| 13 | `b7e6f5e8a` | Button and `DropDownButton` menus open below the button | 2 | New | Bug. Upstream still leaves the flyout placement at its default |
| 79 | `7b75eb55` | A ComboBox's drop-down is kept inside the window: when the adapter creates a ComboBox it registers `Loaded` and `SizeChanged` (the box held weakly), which set `ShouldConstrainToRootBounds` on the first `Popup` in its template, retrying until the template is there; a failure to set it is ignored; binds `IPopup::put_ShouldConstrainToRootBounds` | none | New | Bug, an issue for WinUI rather than Reactor to fix: once a UI Automation client has read a ComboBox's drop-down in its own window (`PopupWindowSiteBridge`, WinUI's default), the ComboBox leaving the tree leaves a dead element in the window root's automation children and navigation ends there, so screen readers lose every later element (Okojo#2474). Evidence: the desktop test `combo_box_drop_downs_open_inside_the_window`; Okojo's GUI scenarios |
| 82 | `0977bbb9` | A title bar WinAppSDK draws (content extended into it) keeps `AppWindowTitleBar.PreferredTheme` `Legacy`: `title_bar_theme` answers `Legacy` once content is extended into the title bar (set before `SetExtendsContentIntoTitleBar(true)`, and kept), and the window's theme otherwise; every `PreferredTheme` goes through `set_title_bar_theme` | none | New | Bug, a Reactor workaround for a WinAppSDK bug (also an issue for WinAppSDK): with any other value WinAppSDK redraws that title bar (`CTitleBar::UpdateColors`) on the thread-pool thread that raises `UISettings.ColorValuesChanged` (an agile callback), while the UI thread redraws it for the `WM_SYSCOLORCHANGE` a high contrast switch sends; the two race on one composition surface and `CCompositionSurface::EndDraw` faults in dcomp.dll, so turning high contrast on or off crashed the app now and then (Okojo#2649). For the system theme `Legacy` draws the same caption colours as `UseDefaultAppMode`; a forced Light or Dark theme's caption buttons follow the app mode instead. Evidence: unit (`tests/native/winui.rs`, the mapping); app (Okojo's `theme-switch` scenario: main's code crashed at the 15th switch, this commit's passed 96; crash dumps symbolised against Microsoft's public PDBs) |
| 91 | `478de456` | A `ContentDialog` takes its owner's theme: when the owner is set, the dialog's `RequestedTheme` is set to the owner's `ActualTheme`, set again as the dialog is shown, best-effort (an owner outside the live tree reports the application's theme and raises no `ActualThemeChanged`, so this apply is what corrects a dialog whose owner was set before it loaded), and kept in step through the owner's `ActualThemeChanged` while the dialog has that owner (`NativeDialogTheme`, dropped with the owner or the dialog); no new bindings (`84f3d495`, a review fix on the same branch) | none | New | Bug: WinUI shows a dialog in its `XamlRoot`'s popup layer, outside the owner's subtree, so a theme the window's content requests (`WindowTheme::Light` or `Dark`) never reaches it, and every dialog drew in the application's theme, the system's, while the window drew in the other (Okojo#3035). Upstream has the same gap. Under high contrast WinUI ignores `RequestedTheme`, as before. Evidence: desktop test (`tests/native/dialog_theme.rs`, `a_content_dialog_takes_its_windows_theme`: a window forced to the theme opposite the application's; the dialog takes it when its owner is set and again as it is shown, follows the window while open, and stops following once its owner lets it go); app (Okojo GUI runs 44951479414 before, on main: the dialogs in the mismatched cases draw in Windows' theme; 44951097642 after: dialogs take Okojo's theme in both Windows modes, and an open dialog follows a Windows switch when Okojo follows the system) |
| 92 | `4ebbae06` | An AutoSuggestBox's UI Automation bounds stay its text field's while its suggestion list is closed: when the adapter creates an AutoSuggestBox it registers `Loaded` and `SizeChanged` (the box held weakly, as 79's ComboBox), which hook the first `Popup` in its template once it is there: once `Opened` and `Closed` are both registered, the popup's content is collapsed unless the popup is open, and on each of them shown or collapsed to match whether the popup is open (the handlers take the popup from the sender, so they don't hold it); failures are ignored; binds `IPopup::{add_Closed, add_Opened, get_Child, get_IsOpen}` and `IUIElement::put_Visibility`; the hook reads its state and releases it before hooking, since showing the content may raise it again (`3e37436e`, a pre-screen fix, and `c3007c6b`, a review fix, on the same branch) | none | New | Bug, an issue for WinUI rather than Reactor to fix: WinUI takes the box's bounds from its whole template, the popup's content included, and that content keeps its last size after the popup closes, so once the list had shown the box's bounds took in the list's area for the rest of the launch (460x117, then 461x86 around a 460x32 text field), and voice control or a UI Automation click aimed at the box landed below it (Okojo#2851). Constraining the popup to the root's bounds (79's fix) did not change it. Evidence: app (Okojo GUI `search-box-bounds`, runs 45057630462 before, 45058483661 (probe) and 45059254343 with the change; `search-recent` 45059452221) |
| 93 | `2f405160` | `PathIcon::foreground`: `IconElement.Foreground` on a `PathIcon` element, through the generator (`schema.toml`, the `theme_brush` adapter as on `FontIcon` and `SymbolIcon`, so a theme brush follows theme changes; undeclared, it is cleared and the icon takes its parent's again); no new bindings | none | New | Extends 15 to the last icon element without a foreground: a path icon could only take its parent's, while `Icon::path(..).foreground(..)` (22) already tints one in a menu. Okojo's ribbon Unsubscribe glyph is a `PathIcon` and takes the command's rose brush, as More and the right-click menu do (Okojo#3263). On `okojo-flow-layout` (TKIPisalegacycipher/windows-rs#15, merged as `28d71758`). Evidence: unit (`tests/declarations.rs`, `icon_foregrounds_record_the_same_theme_brush_as_text`) |

## 5. Input fixes

| # | Commit | Change | Was | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| 14 | `93ff359f5` | Queue `PointerReleased` before releasing pointer capture | 4 | New | Bug, still present upstream: `pointer_event_info` calls `ReleasePointerCapture` while building the event, and WinUI raises `PointerCaptureLost` inside that call. A finished drag looks like a cancelled one |
| 57 | `6e6f33ef2` | `Border::handles_plain_presses`: a plain left press and its release are taken by the `Border`; Ctrl, Shift and right presses still reach the list row | none | New | Dropped: reverted by commit 85, unused since Okojo#2546 (no list or drawer photo is a card trigger). Was: a photo tile that opens its flyout without selecting the row it sits in. Neither 57 nor 85 goes upstream |
| 85 | `66d8b498` | Reverts commit 57: `Border::handles_plain_presses`, its `plain_presses` adapter in `tool-reactor`, its handling in `pointer_event_info`, its unit tests, and the `IPointerRoutedEventArgs::put_Handled` and `VisualTreeHelper::GetParent` bindings it alone used; `generated.rs` and `generated_coverage.rs` regenerated | none | New (removal) | Removal: Okojo had no user of 57 left, and each patch costs every rebase (Okojo#2547, Okojo#715). The `pointer_event_info` conflict keeps commit 73's `timestamp_us`. Goes upstream as nothing: drop 57 and 85 together at the next rebuild. The commit also carries a rustfmt reflow of two signatures in `native/winui/events.rs` that 57 never touched (`input_modifiers_from_virtual_keys`, `key_event_info`), formatting only. Evidence: unit (the reactor's 424 tests pass without 57's two, Okojo worker check 44190828615); app (Okojo's `contact-card` and `contact-card-conversation` scenarios, runs 44188780641 and 44189421233, plus `list-photo-hover-stale` and `messages-contact-card`, Okojo#2690) |
| 73 | `5dea1b36f` | `PointerEventInfo::timestamp_us`: the input's own time (`PointerPoint.Timestamp`, microseconds since boot), 0 if unavailable; binds `IPointerPoint::get_Timestamp` | none | New | Bug-adjacent: Okojo timed a double click by when each press was handled, so a second press that waited on a busy UI thread (the reading pane's first web view being created) missed it (Okojo#1612). Evidence: unit (the field is carried); app (Okojo's popout-margins scenario) |
| 78 | `97b4b9c9` | A queued pointer event keeps the callback installed when the input happened, and is delivered to it even after a render replaced that callback. The adapter holds the callback for the seven pointer event ids (capture lost and canceled included, so keyed on the event id; `QueuedEvent::held`, resolved by `QueuedEvent::callback`) and marks the dispatch (`EventDispatch::held`), and `validate_event_dispatch` takes a marked pointer dispatch while the object still has a handler for that event. A removed handler or a gone object still drops it; every other event keeps the revision rule | none | New | Bug, and a deliberate departure from upstream: upstream drops every event queued for a replaced callback, and its test `stale_pointer_event_does_not_reach_replacement_callback` pins that; here it is inverted (`stale_held_pointer_event_reaches_the_callback_it_was_raised_under`, with `stale_unheld_event_does_not_reach_replacement_callback` keeping upstream's rule for an unheld dispatch). A callback whose closure captures data is new on every render, and a drag re-renders on each move, so dropping lost the moves or the release that waited in the queue (Okojo#2173). The old callback rather than the new one, as the element may stand for something else by now (a recycled row). Evidence: unit (adapter, runtime, and the dispatch that joins them) |
| 80 | `a4453a4d` | A render that meets a queued native event waits for it instead of failing the host: when the render after a dispatch gets `PendingNativeEvent` from `update_subtree`'s `prepare_update` (nothing has reached the native tree), the host records the render's scopes (`deferred_renders`), and `drain` renders them again from their current state once the native work is done, ahead of later messages. An expansion or a row's realization renders a deferred scope even when its input is unchanged, a commit or retirement clears it, and `has_pending_input` counts it | none | New | Bug, still present upstream: `drain` checks the native queue before it takes a message, but the handler's own code runs before the render reaches the runtime. A handler that pumps on the UI thread (Okojo's synchronous toast show) lets WinUI queue an event in between, and `run_component_operation` poisoned the host on the error, which ended the UI thread (Okojo#2570). Evidence: unit (`tests/component/pending_native.rs`, a backend the handler fills: a pumped event, a pumped realization, a child's effect, a retired child) |
| 88 | `11411d22` | `ElementRef<WebView2>::request_accelerator_keys(on_key, completion)`, requested before the view's core: the keys WebView2 reports as accelerator keys (`CoreWebView2Controller.AcceleratorKeyPressed`: Ctrl or Alt held, Esc, a key that types nothing) go to `on_key` (a `RoutedCallback<KeyEventInfo>`, key-downs only, modifiers from the keyboard state as `on_key_down`'s); `true` marks the key handled, so the page never sees it. WinUI's `WebView2` keeps its `CoreWebView2CompositionController` private and never subscribes that event, so the core starts in a proxy of its environment (`native::winui::webview_keys`): a COM object with the environment's default face and its `ICoreWebView2Environment3` face, forwarding every call, any other interface answered by the environment itself; it wraps the `CreateCoreWebView2CompositionControllerAsync` operation the control awaits (`ObservedOperation`), so the controller reaches the adapter, which subscribes the event, before the control resumes. Without an app environment it wraps the default one, made as WinUI makes it (`CreateWithOptionsAsync`, no folders, `Language` the first of `ApplicationLanguages.Languages`). Anything that can't be set up completes the request with the error and the core starts as before. Binds the WebView2 Core members it needs (`bindings.txt`) (`ebaa3c89`, a review fix on the same branch) | none | New | A message body (a WebView2) keeps the keys pressed in it from the XAML tree, so the mail shortcuts did nothing with the focus there (Okojo#2763). Shaped by WinUI's control rather than by Okojo, but reaching a private controller through a proxied environment is a design question for upstream: a `WebView2` host of the reactor's own, or WinUI exposing the controller, would be cleaner. The proxy bends COM identity for interfaces it doesn't answer itself (they are the environment's). Known limits, for the upstream note: those interfaces resolve to the inner environment, so a WinUI that held its environment weakly, or made the controller through `ICoreWebView2Environment10`'s options overload, would bypass the proxy (the keys then report not connected, or, after a browser-process restart, stay on the old controller). Evidence: unit (`native/winui/webview_keys.rs`: the proxy's faces, forwarding, fall-through, lifetime, the observed operation's success and failure, against a fake environment; `tests/reference.rs`; 438 reactor tests pass, Okojo worker check 44298895800); app (Okojo's `mail-shortcuts` scenario, runs 44289182503, 44291849016 and later runs: Ctrl+R from the body of the reading pane and of a message window opens the reply; Ctrl+A and Ctrl+C stay the page's) |

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
| 87 | `7cb1c7d0` | A selector whose items are its `ItemsSource` (`ComboBox`, `RadioButtons`) gets its declared `SelectedIndex` written after every `ItemsSource` write, changed or not, on update (`select_after_items_source`; on mount property order already puts the items first, and the call there is defensive) | none | New | Bug: WinUI drops a ComboBox's selection when its `ItemsSource` is replaced, and the reconciler wrote `SelectedIndex` only when it changed, so a box whose items changed under an unchanged index showed its placeholder (Okojo#2662, the rule editor's action types). 35 covers `ListView` / `GridView`. A deselect WinUI may raise during the `ItemsSource` write itself has no feedback guard and can leave the retained index `None` while the native one is back; the next render writes the index again, a native no-op, so it heals. Evidence: unit (`tests/selection.rs`); app (Okojo's `rules-actions` scenario, run 44135498949) |
| 36 | `963bb1485` | Swallow `NavigationView`'s late "nothing selected" event after the reactor deselects | 58 | New | Bug: WinUI queues that event at low priority after the reactor's own-write suppression has ended. It cost a full render per folder pick in Okojo |
| 58 | `477d35f4d` | `ElementRef<ListView>::request_selected_keys`: the runtime resolves keys to items; the adapter edits `SelectedItems` with `SelectionChanged` held and reports one change | none | New | A row checkbox that ticks the row into the list's own selection |
| 59 | `aa8e6d812` | `ElementRef<ListView>::request_focused_key`: the realized container whose `FocusState` isn't `Unfocused` | none | New | Ctrl+Space ticks the focused row, which Ctrl+arrow keys can move off the selected one |
| 83 | `f4dc907e` | `ElementRef<ListView>::request_focus_key` (and `_result`): the runtime resolves the key to the list's item, the adapter focuses its realized container programmatically and leaves the selection as it is | none | New | A shortcut that opens a folder puts the focus on its first message rather than on the date-group header WinUI focuses first (Okojo#2436). An unrealized container is not brought into view: the request reports `false`. Evidence: unit (`tests/reference.rs`); app (Okojo's `mail-shortcuts` scenario) |
| 84 | `66f52b16` | `ElementRef<ListView>::request_selected_keys_into_view`: `request_selected_keys`, and then the adapter scrolls the first item into view (`ScrollIntoView`, default alignment) unless its realized container lies fully inside the list's viewport. The scroll is best effort: an error from it is dropped, since the selection is already set and reported (`ec545428`, a review fix on the same branch). `ImperativeRequest::SelectItems` gains `into_view` (public only with the `test` feature); `request_selected_keys` sends `false` and never scrolls | none | New | Ctrl+. and Ctrl+, select the next or previous message, which can be out of view while the reading pane shows it (Okojo#2389). The tick boxes keep `request_selected_keys`: their first key can be a row selected earlier and scrolled away. Evidence: unit (`tests/reference.rs`, the in-view decision in `tests/native/winui.rs`); desktop (`tests/native/list_into_view.rs`, which also judges the next item just below the viewport and one cut by its bottom edge from the items' measured height, not the code under test); app (Okojo's `mail-shortcuts` scenario) |
| 72 | `2f057531f` | A `ListView`'s `SelectedKeysChanged` is latest wins: the runtime drops one it pops while a newer one for the same list is still queued, asking the adapter through a new provided `Adapter::has_queued_event` (default `false`; `Adapter` is public only with the `test` feature). Nothing else moves, other lists never coalesce, and a programmatic write's keys echo still arrives. The newer event can itself be discarded at its own pop (validation, if an item it names is gone, or a bumped revision): then neither is delivered, which is no staler than the dropped one, and the list reports again on its next change | none | New | Bug: keys were never treated as echoes, and the runtime renders after each event, so a `SelectedIndex` write's keys echo queued at the back. Two keys events out of step with the app each named a stale selection, and an app that selects what its keys name chased its own echoes for minutes (Okojo#1613, Okojo#1555). Decided in the runtime rather than the WinUI queue because `preview_prefix` stops at the next dispatch and so cannot see the newer event. Evidence: unit (the ring test flipped 40 times in 40 turns without it); Okojo's GUI test `folder-switch-checks` not yet run on it |

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

## 10. Test fixes

| # | Commit | Change | Was | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| 66 | `c0acca834` | The desktop test `transitions_use_current_batch_policy_and_retarget_in_flight` (from upstream #5098) polls each in-flight check until the element has left its start by the 0.01 px margin, up to 1.5 s into the 3 s animation, instead of asserting at a fixed 200 ms | none | New | With Windows animations on, a retargeted animation had moved under 0.01 px (0.0086-0.0096 px) at 200 ms (Okojo#1078). Test-only. Evidence: unit (`progress_ready`) and desktop |

## 11. Process lifetime

| # | Commit | Change | Was | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| 67 | `f4d096510` | `initialize_ui_thread` takes one `CoIncrementMTAUsage` cookie per process and never gives it back, so COM and the WinUI DLLs it loaded stay loaded until the process exits; the binding is added to `bindings.txt`. Also rewords commit 66's deadline comment, so its `transitions.rs` hunk applies only on top of 66 | none | New | When the UI thread's apartment was the last one, its exit uninitialized COM for the process and unloaded `Microsoft.UI.Xaml.dll` before `Microsoft.UI.Xaml.Controls.dll`, whose static destructors then called into the unloaded code (0xc0000005): [issues.md U13](issues.md#u13-a-ui-thread-that-exits-before-the-process-unloads-winui-under-its-own-destructors). Two dumps from Okojo's desktop runner show the stack (Okojo#1079). Evidence: unit (68) and Okojo's desktop job, which fails on any crash of a test process and saw none |
| 68 | `f3b77e6ac` | 67's hold is best effort (`std::sync::Once`, result ignored), so a failed `CoIncrementMTAUsage` no longer fails every later `App` start; its test checks the effect in a child process: after an STA thread takes the hold and exits, a fresh thread's `CoGetApartmentType` answers the implicit MTA. Adds `CoGetApartmentType`, `APTTYPE_MTA` and `APTTYPEQUALIFIER_IMPLICIT_MTA` to `bindings.txt` | none | New | Goes upstream folded into 67. Evidence: unit (fails with `CO_E_NOTINITIALIZED` without the hold) |
| 69 | `f361072e4` | 68's test also asserts its child process reported one test passed, so a renamed test can't pass by matching nothing | none | New | Test-only; goes upstream folded into 67 |
| 70 | `4254a8545` | 68's test inherits its child's stderr again (69's `output()` captured it), so a failure shows the child's panic message | none | New | Test-only; goes upstream folded into 67 |
| 71 | `8d21fb52b` | The transitions test's in-flight checks count only movement toward the target, and stage 2 measures from where the element turned back: its start is read before the retarget, and the old animation runs on until the new one starts (`-39.61442 -> -39.552986 -> -80` on a desktop runner), so 66's either-way margin asserted early | none | New | Test-only; goes upstream folded into 66 (upstream #5098) |

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
Commits 46-93 came after the rebuild and have no original.

## Public API impact

Comparing `public-api.txt` at upstream `ffe56ded6` and at `47c5b855d`: the stack adds public items
and changes a few. Commits 46-77, 83, 84, 86, 88, 89, 90 and 93 only add items (`FlowLayout`, `ItemsRepeater::layout`,
`ListView::resource_overrides`, the `ElementRef<ListView>`, `ElementRef<Border>` and `WebView2`
requests, `Menu::on_opening`, `register_resource_fonts`, `register_text_block_font`,
`Border::use_system_focus_visuals`, `Border::focus_visual_margin`, `automation_accessibility_view`,
`ScrollViewer::vertical_anchor_ratio`, `can_be_scroll_anchor`, `PointerEventInfo::timestamp_us`,
`NavigationViewItem::on_got_focus`, `on_lost_focus` and `element_ref`, `Menu::on_closed`,
`Button::on_double_tapped`, `RichText::on_link_click`, `RichTextBlock::foreground`,
`ElementRef<ListView>::request_focus_key`, `ElementRef<ListView>::request_selected_keys_into_view`,
`TextBlock::font_family`, `ElementRef<WebView2>::request_accelerator_keys`, `TextBlock::text_highlighters`, `TextHighlighter`, `TextRange`, `AutoSuggestBox::is_suggestion_list_open`, `ContentDialog::resource_overrides`, `ResourceValue::Double` and its `From<f64>`, `PathIcon::foreground`), 78-82, 87 and 91 change behaviour (above, no public item), and 85 removes the one 57 added
(`Border::handles_plain_presses`); none changes an existing item, except that 90 adds a variant to an enum (the table below). `register_resource_fonts` is
missing from `public-api.txt`, which commit 60 did not update. These are source-breaking for code
that names them:

| Item | Change | Commit |
| --- | --- | --- |
| `ThemeBrush` | Loses `#[repr(u8)]` because `Resource(&'static str)` carries data | 16 |
| `Icon` | Tuple struct becomes a struct with a foreground | 22 |
| `KeyAccelerators` | Tuple struct becomes a struct with a `scoped` flag | 20 |
| `KeyAccelerator::new` | Parameter is `impl Into<InputModifiers>` instead of `AcceleratorModifiers` | 12 |
| `MenuItem` | Items and submenus gain `icon` (and items `swatch`) fields | 18, 27 |
| `ResourceValue` | Gains `Double`; not `#[non_exhaustive]`, so an exhaustive match breaks | 90 |

Types the stack grows are `#[non_exhaustive]` since commit 1, so later additions of variants and
fields are not breaking. The attribute itself is breaking for code that matches them exhaustively
or builds them with a struct literal. Reactor calls itself a preview and says releases may refine
public APIs, but each proposal should state this.
