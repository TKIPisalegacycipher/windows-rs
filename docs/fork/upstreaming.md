# Proposing the work upstream

How to turn 58 fork commits into changes `microsoft/windows-rs` will accept, and the case for each.
Read [readme.md](readme.md) for background, [patches.md](patches.md) for the commits and
[issues.md](issues.md) for the problems this plan has to fix first.

Nothing has been posted upstream. Okojo's own notes say to ask before posting anything, and that
still applies: this page is a plan, not an authorization.

## Why upstream, and why now

Carrying the patches costs more every week. Upstream `master` moved 32 commits in the six days
after the fork's base, and a trial rebase of the stack now stops on hand-written conflicts in 36
of 58 commits. Every rebase of a vendored patch set is paid again in Okojo. A change that lands
upstream is maintained there.

Three facts shape how to propose:

| Fact | Consequence |
| --- | --- |
| The maintainer wrote 39 of the last 40 commits that touched Reactor and merges most work from issues, in short pull requests that end "Addresses #5087 item N" or "Fixes #N" | Issues with a concrete scenario are the main route. A large unsolicited pull request is unlikely to be merged as is |
| His issue #5087 lists 19 Reactor gaps, and within three days of opening it he landed parts of items 2 and 5 (#5106, #5103) | Eight fork commits implement items he already wants. Some will land upstream before a pull request can. Offer the working code on the issue instead of racing |
| Upstream's [contributing guide](../contributing.md) asks for an issue or a comment before a pull request | Every proposal below starts as an issue or as a comment on an existing one |

## What makes a proposal well-justified

Each proposal should answer these before it is posted. The first four are what the maintainer's
own pull requests answer.

| Question | What to show |
| --- | --- |
| Is the gap real? | The WinUI member exists and Reactor has no way to reach it. Name the member (for example `UIElement.IsHitTestVisible`) |
| Who needs it? | A scenario any application of the kind would hit, not "Okojo needs it". The drafts use a mail or calendar client; calendar overlays, folder trees and search boxes are ordinary |
| Does the maintainer already agree? | A link to #5087, #4923, #4559 or another issue where he named the gap or the design |
| Is the shape consistent? | It reuses an existing pattern: a generated property with `schema.toml`, an `ElementRef` request for something imperative, a typed callback for an event. No new concept where an existing one fits |
| Does it break anyone? | State source-breaking changes (see the API table in [patches.md](patches.md#public-api-impact)) and offer the non-breaking form first |
| Is it tested? | Unit test with the recording adapter, a live self-test case for native behaviour, `public-api.txt` updated, a runnable sample when users will see it |
| Was it checked on current upstream? | Say what was run on `master`, not on the fork's old base |

## Step 0: prepare the fork

None of this is optional; it removes the problems in [issues.md](issues.md#b-problems-in-the-fork).

1. Create a branch from current `upstream/master` and bring commits over by proposal, not in
   stack order. Regenerate with `cargo run -p tool-reactor` rather than resolving generated
   files.
2. Drop what is already upstream: commits 3, 14, 16, 23, 24, and the `FontIcon` part of 7.
3. Squash the seven fixups into their parents and decide the two held commits (33, 41).
4. Propose `#[non_exhaustive]` on the public enums and structs that grow
   ([F4](issues.md#f4-public-types-are-not-non_exhaustive)).
5. Reshape the accelerator modifier API
   ([F3](issues.md#f3-accelerator-modifiers-cannot-express-controlshift)) and escape resource keys
   ([F5](issues.md#f5-resource-keys-are-inserted-into-xaml-unescaped)).
6. Add the missing regression tests (`7804302d7`, `01b9d70a6`).

## The proposals

A proposal is one issue and, once agreed, one pull request. "Case" rates how well the evidence
supports it today: **strong** means the maintainer asked for it or the defect is confirmed in
current upstream source; **good** means a clear scenario and a small, pattern-following change;
**needs design** means the shape is a real question.

### Wave 1: items the maintainer already listed in #5087

Comment on #5087 for each item, link the implementation, and offer a pull request for the item.

| Proposal | Commits | #5087 | Why upstream should want it | Case |
| --- | --- | --- | --- | --- |
| Context menu on any element | 13 | 13 | He wrote "Existing button flyouts are not element context menus." WinUI's `ContextFlyout` gives right-click, Shift+F10 and touch with no extra code. Without it a list row or tree item cannot have a menu | strong |
| More accelerator keys and combined modifiers | 1, 46 | 14 | He asked for D/O/S/M and Control+Shift. The fork covers all letters, function keys, Escape, Delete and paging, but must adopt `InputModifiers` first (F3) | strong, after reshaping |
| Theme brush by resource key and app theme dictionaries | 10, 11 | 6 | He asked for "named theme-brush references and scoped Light/Default/HighContrast dictionaries". Apps that colour things per user cannot use the eight built-in brushes | strong, after F5 |
| `ListView` container minimum height and overrides | 22, 29 | 8 | He asked for container minimum height and styling that applies to containers realized later as well as those already visible. The only alternative is an app-wide resource that also resizes `AutoSuggestBox` rows | strong |
| `ListView::on_item_click` | 36, 42 | 12 | He asked for `IsItemClickEnabled` and `ItemClick` keyed by stable key, separate from selection. Enter and Space on a focused item reach no accelerator | strong |
| `AutomationProperties.LiveSetting` | 34, 37 | 4 | He listed it. Status messages ("Saving...", errors) are invisible to screen readers without it | strong |

### Wave 2: defects

| Proposal | Commits | Why upstream should want it | Case |
| --- | --- | --- | --- |
| Deliver `PointerReleased` before releasing capture | 4 | A finished drag is reported as a cancel. The faulty call is in `pointer_event_info` today, and the fix changes no public API | strong |
| Open button menus below the button | 2 | Menus on a button near the top of a window open upward over the title bar. Notification-icon menus already use the bottom placement | good; offer `Menu::placement` as the alternative |
| Generator: enum probing, secondary-interface event args, `Option` setter | in 34, 35, 51 | Three patterns the schema cannot express today, two confirmed present on upstream. Each unblocks properties and events | strong |
| `NavigationView` late deselect echo | 58 | The reactor's own write comes back as a user event, which in a real app costs a render per pick | good, once reproduced on current upstream |
| `ListView` `SelectedIndex` ordering | 31, 39, 40 | The index names an item of the old list when items change in the same update | good, once reproduced on current upstream |

### Wave 3: small missing WinUI members

Each is a schema entry plus bindings plus regeneration, with a unit test. The case rests on the
scenario in the draft.

| Proposal | Commits | WinUI member | Scenario |
| --- | --- | --- | --- |
| `is_hit_test_visible` | 6 | `UIElement.IsHitTestVisible` | A decorative layer (a "now" line, a drag preview) over interactive content must pass clicks through |
| `cursor` | 25 | `UIElement.ProtectedCursor` | Custom resize handles need a resize cursor. WinUI exposes it only through a protected property |
| Pointer element size | 5 | `FrameworkElement.ActualWidth` and `ActualHeight` | Turning `x` into a fraction of the element for a seek bar or a day column |
| `AutoSuggestBox` query and focus | 8, 9 | `QuerySubmitted`, `GotFocus`, `LostFocus` | Filter on every key but search on Enter; follow-up to #4987 (`QueryIcon`) |
| `ElementRef<ScrollViewer>` offset | 15 | `ChangeView` | Scroll a day view to the current hour. A declared offset would fight user scrolling on each render |
| `ElementRef<Button>::request_hide_flyout` | 28 | `Flyout.Hide` | A flyout of commands stays open after one runs |
| `NavigationView` events and background | 35, 48, 26 | `ItemInvoked`, `Expanding`, `Collapsed`, menu scroll offset, `NavigationViewItem.Background` | Action rows reachable by keyboard; sticky headers over a tree; marking a mirrored item |
| `InfoBar` `action_button` and `content` | 51 | `InfoBar.ActionButton`, `InfoBar.Content` | An undo bar with a button and progress |
| `WindowPolicy::activate(false)` | 49 | `AppWindow.Show(false)` | Reopen a window in the background without stealing focus |
| Scoped accelerators | 17 | `KeyboardAccelerator.ScopeOwner` | Enter in one text box must not fire window-wide |
| `ListView::on_selected_keys_changed` | 18 | `ListViewBase.SelectedItems` | Multi-select returns only the first index today, so a bulk delete cannot know the rest |
| Automation names and ids | 21, 43 | `AutomationProperties.Name`, `AutomationId` on containers, menu items, dialogs | Screen readers and UI tests otherwise see unnamed items |
| Icon foregrounds and menu icons | 7 (rest), 19, 27, 12 | `IconElement.Foreground`, `MenuFlyoutItem.Icon` | Tinted icons in menus and navigation. #4923 says menu icons need API design, so lead with 27 and fold 12 into it |

### Wave 4: design discussions

These should be conversations first.

| Proposal | Commits | The question for the maintainer |
| --- | --- | --- |
| `ListView` row content realisation | 30, 47, 53, 54, 55, 56, 57 | Reactor mounts every row's content when the row is mounted. Should `ListView` mount content only when XAML prepares a container? It touches the same code as #5087 item 17, #5114 and #5085. On 2026-10-16 he parked item 17 "pending a minimal affected application example"; Okojo's folder switches (its #594) are one, and this is the strongest way in. His own trace is relevant: reversing 101 keyed rows takes the `ReplaceAll` path and resets scroll and focus, and forcing the incremental path crashed natively, which is the family of U6 |
| `ComponentContext::skip_render` | 52 | Is it right to let an update opt out of `view`, given that effects and window publication wait for the next render? What would he measure? |
| Drag formats | 44, 50 (32, 38, 45 if wanted) | Apps that drag their own items as virtual files need a private-format policy. Is `format` the shape, and should `other` exist at all (F8)? |
| `bindgen` event-only delegate fix | 20 | A separate crate. Check it against #5075 and #5077 before proposing |

## How to post one

1. Take the draft in Okojo's `docs/upstream-drafts/`, update it for the status in
   [patches.md](patches.md), and add a line that the code was written with AI assistance.
   Four drafts are obsolete: the `CalendarDatePicker` comment, the observed-property question,
   `ContentDialog` default button and the `MenuBarItem` duplicate entries. The `Icon` foreground
   draft needs trimming, because `FontIcon` has landed.
2. Rebase the one proposal onto current `upstream/master` and run the gates in
   [readme.md](readme.md#build-and-test). Run the live self-test for the control on a desktop.
3. Post as a comment on the existing issue where there is one (#5087 for Wave 1, #4923 for menu
   icons), otherwise as a new issue. Put upstream references in backticks, as Okojo's notes
   advise, so a full URL does not add a "mentioned" event on the upstream thread.
4. Open the pull request only after the maintainer agrees. Follow the repository's rules: subject
   `` `windows-reactor` add X ``, regenerate and commit generated files, update
   `public-api.txt`, `docs/crates/windows-reactor.md` and the crate readme when behaviour
   changes, add a small sample for anything visible.
5. Say in the pull request what was and was not tested on a desktop.

## Done when

The fork's stack is empty or reduced to patches nobody upstream wants, and Okojo's
`[patch.crates-io]` entry for the three vendored crates is deleted. Okojo's `TODO.md` item O11
tracks the same goal from the application side.
