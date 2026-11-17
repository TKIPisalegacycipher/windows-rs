//! A selection requested with `request_selected_keys_into_view` scrolls its item into view when it
//! is out of view, and doesn't scroll when it is fully in view; `request_selected_keys` never
//! scrolls (Okojo#2389). A real ListView, 300 DIPs tall, of 200 items, in a window of its own.
use super::*;

const TEST: &str = "native::winui::tests::list_into_view::a_selection_into_view_scrolls_its_item_in_only_when_needed";
const CHILD: &str = "REACTOR_LIST_INTO_VIEW_TEST_CHILD";
const COUNT: usize = 200;

/// What the child saw at each step: the list's vertical offset, its `SelectedIndex`, and whether
/// the step's item lay fully in view.
#[derive(Debug, Default)]
struct Seen {
    start: Option<f64>,
    /// After `request_selected_keys(["item-150"])`, which leaves item 150 out of view.
    plain: Option<(f64, Option<usize>, bool)>,
    /// After `request_selected_keys_into_view(["item-120"])`.
    brought: Option<(f64, Option<usize>, bool)>,
    /// The item fully in view chosen next, and what `request_selected_keys_into_view` on it left.
    in_view: Option<usize>,
    stayed: Option<(f64, Option<usize>, bool)>,
}

#[test]
#[ignore = "requires an interactive WinUI desktop"]
fn a_selection_into_view_scrolls_its_item_in_only_when_needed() {
    // WinUI supports one Application lifetime per process.
    if std::env::var_os(CHILD).is_some() {
        child();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", TEST, "--ignored", "--nocapture"])
        .env(CHILD, "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    println!("{stdout}");
    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "the app failed");
    assert!(stdout.contains("LIST_INTO_VIEW=ok"), "{stdout}");
}

fn key(index: usize) -> String {
    format!("item-{index}")
}

fn list(reference: &ElementRef<ListView>) -> View {
    Border::new()
        .vertical_alignment(VerticalAlignment::Top)
        .content(
            ListView::new()
                .element_ref(reference)
                .height(300.0)
                .selection_mode(ListViewSelectionMode::Extended)
                .items((0..COUNT).map(|i| DataItem::new(key(i), format!("Item {i}")))),
        )
        .into()
}

/// The list's ScrollViewer offset, its `SelectedIndex`, and whether item `index` lies fully in
/// view.
fn state(adapter: &WinUiAdapter, object: ObjectId, index: usize) -> (f64, Option<usize>, bool) {
    let Some(Handle::ListView(list)) = adapter.handles.get(&object) else {
        panic!("no ListView");
    };
    let element = list.value.cast::<native::DependencyObject>().unwrap();
    let offset = find_scroller(&element).unwrap().VerticalOffset().unwrap();
    let item = list
        .value
        .cast::<native::IItemsControl>()
        .unwrap()
        .Items()
        .unwrap()
        .GetAt(index as u32)
        .unwrap();
    let in_view = !needs_scrolling_into_view(list_item_span(&list.value, &item).unwrap());
    let selected = adapter.list_view_state(object).unwrap().0;
    (offset, selected, in_view)
}

/// An item fully in view, not `selected` and not the first one fully in view (an item at the
/// viewport's edge).
fn an_item_in_view(adapter: &WinUiAdapter, object: ObjectId, selected: usize) -> usize {
    (1..COUNT)
        .find(|i| *i != selected && state(adapter, object, *i).2 && state(adapter, object, i - 1).2)
        .expect("an item fully in view")
}

/// The app: steps every 100 ms (layout and the scroll settle between), checks, and exits.
fn child() {
    let seen = Rc::new(RefCell::new(Seen::default()));
    let result = App::run_with({
        let seen = Rc::clone(&seen);
        move |app| {
            let reference = ElementRef::<ListView>::new();
            let mut runtime = Runtime::new(WinUiAdapter::default());
            runtime.update(list(&reference)).unwrap();
            let root = runtime.graph().root().unwrap();
            let window = runtime.adapter_mut().create_window(root).unwrap();
            window.activate().unwrap();
            let timer = native::DispatcherQueue::GetForCurrentThread()?.CreateTimer()?;
            timer.SetInterval(TimeSpan::try_from(Duration::from_millis(100)).unwrap())?;
            let runtime = RefCell::new(runtime);
            let window = RefCell::new(Some(window));
            let ticks = Cell::new(0u32);
            let app = app.clone();
            let tick = timer.Tick(move |_, _| {
                let now = ticks.get() + 1;
                ticks.set(now);
                let mut runtime = runtime.borrow_mut();
                runtime.dispatch_native_events().unwrap();
                let object = reference.get().unwrap();
                let mut seen = seen.borrow_mut();
                match now {
                    10 => {
                        seen.start = Some(state(runtime.adapter(), object, 0).0);
                        assert!(reference.request_selected_keys([key(150)]));
                        runtime.dispatch_native_events().unwrap();
                    }
                    15 => {
                        seen.plain = Some(state(runtime.adapter(), object, 150));
                        assert!(reference.request_selected_keys_into_view([key(120)]));
                        runtime.dispatch_native_events().unwrap();
                    }
                    20 => {
                        seen.brought = Some(state(runtime.adapter(), object, 120));
                        let index = an_item_in_view(runtime.adapter(), object, 120);
                        seen.in_view = Some(index);
                        assert!(reference.request_selected_keys_into_view([key(index)]));
                        runtime.dispatch_native_events().unwrap();
                    }
                    25 => {
                        let index = seen.in_view.unwrap();
                        seen.stayed = Some(state(runtime.adapter(), object, index));
                    }
                    _ => {}
                }
                if now >= 26
                    && let Some(window) = window.borrow_mut().take()
                {
                    window.close().unwrap();
                    app.exit().unwrap();
                }
            })?;
            timer.Start()?;
            Ok((timer, tick))
        }
    });
    result.unwrap();
    let seen = seen.borrow();
    println!("{seen:?}");
    let start = seen.start.expect("the list laid out");
    assert!(start.abs() < 0.5, "the list starts at the top: {seen:?}");
    // Plain: item 150 is selected and left out of view, the list where it was.
    let (offset, selected, in_view) = seen.plain.unwrap();
    assert_eq!(selected, Some(150), "{seen:?}");
    assert!(!in_view && (offset - start).abs() < 0.5, "{seen:?}");
    // Into view: item 120 replaces 150 and is scrolled fully into view.
    let (brought, selected, in_view) = seen.brought.unwrap();
    assert_eq!(selected, Some(120), "{seen:?}");
    assert!(in_view && brought > start + 0.5, "{seen:?}");
    // Already fully in view: selected, and the list doesn't move.
    let (offset, selected, in_view) = seen.stayed.unwrap();
    assert_eq!(selected, seen.in_view, "{seen:?}");
    assert!(in_view && (offset - brought).abs() < 0.5, "{seen:?}");
    println!("LIST_INTO_VIEW=ok");
}
