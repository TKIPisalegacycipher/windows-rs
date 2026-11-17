//! A native event queued while the app's own code runs (Okojo#2570).
//!
//! `drain` checks the native queue before it takes a message, but the message's handler runs
//! app code before the render reaches the runtime. On WinUI that code can pump: Okojo's
//! new-mail notice showed a toast through a synchronous cross-process call on the UI thread, and
//! WinUI queued an event meanwhile. The render then met `PendingNativeEvent`, which ended the
//! UI thread. The render waits for the event instead, and is applied after it.

use super::*;

type PumpQueue = Rc<RefCell<VecDeque<NativeEvent>>>;

/// A backend whose native queue the app's code can fill while it runs, the way WinUI queues an
/// event when a handler pumps messages. Its own events come after the recording backend's.
struct PumpingAdapter {
    inner: RecordingAdapter,
    pumped: PumpQueue,
}

impl PumpingAdapter {
    fn new(pumped: &PumpQueue) -> Self {
        Self {
            inner: RecordingAdapter::new(),
            pumped: Rc::clone(pumped),
        }
    }
}

impl Adapter for PumpingAdapter {
    type Error = AdapterError;

    fn preview_native_events(&self, events: &mut Vec<NativeEvent>) {
        if self.inner.queued_native_event_count() == 0 {
            NativeEvent::preview_prefix(events, self.pumped.borrow().iter().cloned());
        } else {
            self.inner.preview_native_events(events);
        }
    }

    fn pop_native_event(&mut self) -> Option<NativeEvent> {
        self.inner
            .pop_native_event()
            .or_else(|| self.pumped.borrow_mut().pop_front())
    }

    fn validate(&self, mutations: &[Mutation]) -> Result<(), Self::Error> {
        self.inner.validate(mutations)
    }

    fn apply(&mut self, mutations: &[Mutation]) -> Result<(), Self::Error> {
        self.inner.apply(mutations)
    }

    fn focus(&mut self, object: ObjectId) -> Result<bool, Self::Error> {
        self.inner.focus(object)
    }
}

/// What a notice queues while its handler runs, and what the components saw.
#[derive(Clone, Default)]
struct Pump {
    log: Rc<RefCell<Vec<String>>>,
    on_notice: Rc<RefCell<Option<NativeEvent>>>,
    pumped: PumpQueue,
}

impl PartialEq for Pump {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.log, &other.log)
            && Rc::ptr_eq(&self.on_notice, &other.on_notice)
            && Rc::ptr_eq(&self.pumped, &other.pumped)
    }
}

impl Pump {
    /// Queues the native event the next notice "pumps", as WinUI would.
    fn pump(&self) {
        if let Some(event) = self.on_notice.borrow().clone() {
            self.pumped.borrow_mut().push_back(event);
        }
    }

    fn arm(&self, object: ObjectId, callback: EventValue, text: &str) {
        let event = EventDispatch::new(
            object,
            EventId::TextChanged,
            callback,
            EventPayload::String(Rc::from(text)),
        );
        self.on_notice
            .replace(Some(NativeEvent::new(None, Some(event))));
    }

    fn arm_realization(&self, request: RealizationRequest) {
        self.on_notice
            .replace(Some(NativeEvent::realization(request)));
    }
}

fn text_changed_callback<A: Adapter>(host: &ComponentHost<A>, object: ObjectId) -> EventValue {
    let events = host.runtime().graph().events(object).unwrap();
    assert_eq!(events[0].id, EventId::TextChanged);
    events[0].value.clone()
}

fn text<A: Adapter>(host: &ComponentHost<A>, object: ObjectId) -> PropertyValue {
    host.runtime().graph().properties(object).unwrap()[0]
        .value
        .clone()
}

enum NoticeMessage {
    Notice,
    Typed(String),
}

/// Counts notices and shows what was typed; its own text box raises the pumped event.
struct NoticeBox {
    callback: Callback<Rc<str>>,
    notices: usize,
    pump: Pump,
    typed: Vec<String>,
}

impl Component for NoticeBox {
    type Input = Pump;
    type Message = NoticeMessage;

    fn create(pump: &Self::Input, context: &ComponentContext<Self>) -> Self {
        let sender = context.sender();
        Self {
            callback: Callback::new(move |value: Rc<str>| {
                assert!(sender.send(NoticeMessage::Typed(value.to_string())));
            }),
            notices: 0,
            pump: pump.clone(),
            typed: Vec::new(),
        }
    }

    fn update(&mut self, message: NoticeMessage, _: &ComponentContext<Self>) {
        match message {
            NoticeMessage::Notice => {
                self.notices += 1;
                self.pump
                    .log
                    .borrow_mut()
                    .push(format!("notice {}", self.notices));
                self.pump.pump();
            }
            NoticeMessage::Typed(value) => {
                self.pump.log.borrow_mut().push(format!("typed {value}"));
                self.typed.push(value);
            }
        }
    }

    fn view(&self, _: &Self::Input, _: &mut ViewContext<Self>) -> View {
        TextBox::new()
            .text(format!(
                "{} notices, typed {}",
                self.notices,
                self.typed.join("+")
            ))
            .on_text_changed(self.callback.clone())
            .into()
    }
}

#[test]
fn render_waits_for_a_native_event_queued_while_its_message_was_handled() {
    let pump = Pump::default();
    let mut host = ComponentHost::mount(
        PumpingAdapter::new(&pump.pumped),
        [component::<NoticeBox>("notice", pump.clone())],
    )
    .unwrap();
    let key = Key::from("notice");
    let object = host.reference(&key).unwrap().get().unwrap();
    pump.arm(object, text_changed_callback(&host, object), "x");
    let sender = host.sender::<NoticeBox>(&key).unwrap();
    assert!(sender.send(NoticeMessage::Notice));

    // Before the fix: Err(Runtime(PendingNativeEvent)), and the host was poisoned.
    let report = host.drain(usize::MAX).unwrap();

    assert_eq!(report.dispatched, 2);
    assert!(pump.pumped.borrow().is_empty());
    assert_eq!(*pump.log.borrow(), ["notice 1", "typed x"]);
    assert_eq!(
        text(&host, object),
        PropertyValue::String(Rc::from("1 notices, typed x"))
    );

    // The host keeps working.
    pump.on_notice.replace(None);
    assert!(sender.send(NoticeMessage::Notice));
    host.drain(usize::MAX).unwrap();
    assert_eq!(
        text(&host, object),
        PropertyValue::String(Rc::from("2 notices, typed x"))
    );
}

#[derive(Clone)]
struct CountInput {
    count: usize,
    log: Rc<RefCell<Vec<String>>>,
}

impl PartialEq for CountInput {
    fn eq(&self, other: &Self) -> bool {
        self.count == other.count && Rc::ptr_eq(&self.log, &other.log)
    }
}

/// A child whose effect follows its input.
struct CountChild;

impl Component for CountChild {
    type Input = CountInput;
    type Message = ();

    fn create(_: &Self::Input, _: &ComponentContext<Self>) -> Self {
        Self
    }

    fn view(&self, input: &Self::Input, context: &mut ViewContext<Self>) -> View {
        let log = Rc::clone(&input.log);
        let count = input.count;
        context.use_effect("count", count, move || {
            log.borrow_mut().push(format!("effect {count}"));
            None
        });
        TextBlock::new().text(count.to_string()).into()
    }
}

/// Passes its notice count to a child; the pumped event's callback changes nothing, so only the
/// deferred render can apply the notice.
struct NoticeParent {
    ignore: Callback<Rc<str>>,
    notices: usize,
    pump: Pump,
}

impl Component for NoticeParent {
    type Input = Pump;
    type Message = ();

    fn create(pump: &Self::Input, _: &ComponentContext<Self>) -> Self {
        Self {
            ignore: Callback::new(|_: Rc<str>| {}),
            notices: 0,
            pump: pump.clone(),
        }
    }

    fn update(&mut self, (): (), _: &ComponentContext<Self>) {
        self.notices += 1;
        self.pump.pump();
    }

    fn view(&self, _: &Self::Input, _: &mut ViewContext<Self>) -> View {
        let children: Vec<View> = vec![
            TextBox::new()
                .text(self.notices.to_string())
                .on_text_changed(self.ignore.clone())
                .into(),
            component::<CountChild>(
                "child",
                CountInput {
                    count: self.notices,
                    log: Rc::clone(&self.pump.log),
                },
            )
            .into(),
        ];
        StackPanel::new().children(children).into()
    }
}

#[test]
fn deferred_render_reapplies_child_renders_it_had_computed() {
    let pump = Pump::default();
    let mut host = ComponentHost::mount(
        PumpingAdapter::new(&pump.pumped),
        [component::<NoticeParent>("parent", pump.clone())],
    )
    .unwrap();
    let parent = Key::from("parent");
    let root = host.reference(&parent).unwrap().get().unwrap();
    let text_box = host
        .runtime()
        .graph()
        .children(root, RelationId::Children)
        .unwrap()[0];
    let child = host
        .reference_at(&[parent.clone(), Key::from("child")])
        .unwrap()
        .get()
        .unwrap();
    assert_eq!(*pump.log.borrow(), ["effect 0"]);
    pump.arm(text_box, text_changed_callback(&host, text_box), "ignored");
    assert!(host.sender::<NoticeParent>(&parent).unwrap().send(()));

    // Stop right after the parent's retry: the notice, the event and the deferred parent count
    // 3. Left unbounded, a later retry of the child alone would hide a parent retry that skipped
    // it; the child's render must come with its parent's.
    let report = host.drain(3).unwrap();

    assert_eq!(report.dispatched, 1);
    assert!(pump.pumped.borrow().is_empty());
    assert_eq!(text(&host, text_box), PropertyValue::String(Rc::from("1")));
    assert_eq!(text(&host, child), PropertyValue::String(Rc::from("1")));
    // The child's input changed in the render that met the event; its effect must still run.
    assert_eq!(*pump.log.borrow(), ["effect 0", "effect 1"]);
    assert!(!host.has_pending_input());
}

#[test]
fn deferred_render_counts_as_pending_input() {
    let pump = Pump::default();
    let mut host = ComponentHost::mount(
        PumpingAdapter::new(&pump.pumped),
        [component::<NoticeBox>("notice", pump.clone())],
    )
    .unwrap();
    let key = Key::from("notice");
    let object = host.reference(&key).unwrap().get().unwrap();
    pump.arm(object, text_changed_callback(&host, object), "x");
    let sender = host.sender::<NoticeBox>(&key).unwrap();
    assert!(sender.send(NoticeMessage::Notice));
    assert!(!host.has_pending_input());

    // The notice alone: its render meets the pumped event and waits.
    let report = host.drain(1).unwrap();

    assert_eq!(report.dispatched, 1);
    assert_eq!(pump.pumped.borrow().len(), 1);
    assert_eq!(
        text(&host, object),
        PropertyValue::String(Rc::from("0 notices, typed "))
    );
    assert!(host.has_pending_input());

    host.drain(usize::MAX).unwrap();
    assert_eq!(
        text(&host, object),
        PropertyValue::String(Rc::from("1 notices, typed x"))
    );
    assert!(!host.has_pending_input());
}

enum HidingMessage {
    Notice,
    Hide,
}

/// Like `NoticeParent`, but the pumped event hides the child whose render was deferred.
struct HidingParent {
    hide: Callback<Rc<str>>,
    notices: usize,
    pump: Pump,
    shown: bool,
}

impl Component for HidingParent {
    type Input = Pump;
    type Message = HidingMessage;

    fn create(pump: &Self::Input, context: &ComponentContext<Self>) -> Self {
        Self {
            hide: context.callback(|_: Rc<str>| HidingMessage::Hide),
            notices: 0,
            pump: pump.clone(),
            shown: true,
        }
    }

    fn update(&mut self, message: HidingMessage, _: &ComponentContext<Self>) {
        match message {
            HidingMessage::Notice => {
                self.notices += 1;
                self.pump.pump();
            }
            HidingMessage::Hide => self.shown = false,
        }
    }

    fn view(&self, _: &Self::Input, _: &mut ViewContext<Self>) -> View {
        let mut children: Vec<View> = vec![
            TextBox::new()
                .text(self.notices.to_string())
                .on_text_changed(self.hide.clone())
                .into(),
        ];
        if self.shown {
            children.push(
                component::<CountChild>(
                    "child",
                    CountInput {
                        count: self.notices,
                        log: Rc::clone(&self.pump.log),
                    },
                )
                .into(),
            );
        }
        StackPanel::new().children(children).into()
    }
}

#[test]
fn deferred_child_retired_before_its_retry_leaves_nothing_behind() {
    let pump = Pump::default();
    let mut host = ComponentHost::mount(
        PumpingAdapter::new(&pump.pumped),
        [component::<HidingParent>("parent", pump.clone())],
    )
    .unwrap();
    let parent = Key::from("parent");
    let root = host.reference(&parent).unwrap().get().unwrap();
    let text_box = host
        .runtime()
        .graph()
        .children(root, RelationId::Children)
        .unwrap()[0];
    let child = host
        .reference_at(&[parent.clone(), Key::from("child")])
        .unwrap();
    assert!(child.get().is_some());
    pump.arm(text_box, text_changed_callback(&host, text_box), "hide");
    let sender = host.sender::<HidingParent>(&parent).unwrap();
    assert!(sender.send(HidingMessage::Notice));

    // Stop before any retry: the notice and the event count 2, and `Hide`, dispatched during the
    // event, is not counted. Left unbounded, a retry would drop the retired child's entry and
    // hide a retirement that left it behind.
    let report = host.drain(2).unwrap();

    assert!(!host.has_pending_input());
    assert_eq!(report.dispatched, 2);
    assert!(pump.pumped.borrow().is_empty());
    assert_eq!(text(&host, text_box), PropertyValue::String(Rc::from("1")));
    assert_eq!(child.get(), None);
    // The child's deferred render was never committed, so its effect never ran.
    assert_eq!(*pump.log.borrow(), ["effect 0"]);
}

/// Shows its notice count beside a list; a notice "pumps" a request to realize the list's row.
struct ListParent {
    notices: usize,
    pump: Pump,
}

impl Component for ListParent {
    type Input = Pump;
    type Message = ();

    fn create(pump: &Self::Input, _: &ComponentContext<Self>) -> Self {
        Self {
            notices: 0,
            pump: pump.clone(),
        }
    }

    fn update(&mut self, (): (), _: &ComponentContext<Self>) {
        self.notices += 1;
        self.pump.pump();
    }

    fn view(&self, _: &Self::Input, _: &mut ViewContext<Self>) -> View {
        let children: Vec<View> = vec![
            TextBlock::new().text(self.notices.to_string()).into(),
            ItemsRepeater::new()
                .item("row", TextBlock::new().text("row"))
                .into(),
        ];
        StackPanel::new().children(children).into()
    }
}

#[test]
fn render_waits_for_a_realization_queued_while_its_message_was_handled() {
    let pump = Pump::default();
    let mut host = ComponentHost::mount(
        PumpingAdapter::new(&pump.pumped),
        [component::<ListParent>("list", pump.clone())],
    )
    .unwrap();
    let key = Key::from("list");
    let root = host.reference(&key).unwrap().get().unwrap();
    let children = host
        .runtime()
        .graph()
        .children(root, RelationId::Children)
        .unwrap();
    let (label, collection) = (children[0], children[1]);
    pump.arm_realization(RealizationRequest::Realize {
        collection,
        container: RealizedContainer(1),
        index: 0,
        source_revision: 0,
    });
    assert!(host.sender::<ListParent>(&key).unwrap().send(()));

    let report = host.drain(usize::MAX).unwrap();

    assert_eq!(report.dispatched, 1);
    assert!(pump.pumped.borrow().is_empty());
    assert_eq!(host.test_state().virtual_rows, 1);
    assert_eq!(text(&host, label), PropertyValue::String(Rc::from("1")));
    assert!(!host.has_pending_input());
}
