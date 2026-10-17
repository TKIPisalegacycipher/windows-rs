use super::*;

#[path = "images.rs"]
mod images;

#[path = "lifecycle.rs"]
mod lifecycle;

#[path = "calendar.rs"]
mod calendar;

#[path = "ownership.rs"]
mod ownership;

mod font_icon;
mod text_block;
#[path = "text_box.rs"]
mod text_box;

#[path = "transitions.rs"]
mod transitions;

#[test]
fn native_queue_preview_uses_current_callback_and_preserves_order() {
    let object = ObjectId::test(1);
    let first = Callback::new(|_: Key| panic!("preview must not invoke callbacks"));
    let second = Callback::new(|_: Key| panic!("preview must not invoke callbacks"));
    let mut adapter = WinUiAdapter::default();
    adapter.menus.insert(
        object,
        NativeMenu {
            menu: Menu::new([], first),
            revision: 1,
            flyout: None,
            context_flyout: None,
            _revokers: Vec::new(),
        },
    );
    let observation = Observation::SetProperty {
        object,
        property: Property {
            id: PropertyId::Text,
            value: PropertyValue::String(Rc::from("Before")),
        },
    };
    adapter.event_queue.queue(
        None,
        Some(QueuedEvent {
            object,
            event: EventId::MenuItemInvoked,
            revision: 0,
            payload: EventPayload::Key("stale".into()),
        }),
    );
    adapter.event_queue.queue(Some(observation.clone()), None);
    adapter.event_queue.queue(
        None,
        Some(QueuedEvent {
            object,
            event: EventId::MenuItemInvoked,
            revision: 1,
            payload: EventPayload::Key("current".into()),
        }),
    );
    adapter.event_queue.queue(Some(observation.clone()), None);

    let mut preview = Vec::new();
    adapter.preview_native_events(&mut preview);
    assert_eq!(preview.len(), 3);
    assert_eq!(adapter.event_queue.events.borrow().len(), 4);

    // Callback replacement preserves the menu presentation revision.
    adapter.menus.get_mut(&object).unwrap().menu.on_click = second.clone();
    preview.clear();
    adapter.preview_native_events(&mut preview);
    assert_eq!(
        preview,
        [
            NativeEvent::new(None, None),
            NativeEvent::new(Some(observation.clone()), None),
            NativeEvent::new(
                None,
                Some(EventDispatch::new(
                    object,
                    EventId::MenuItemInvoked,
                    EventValue::Key(second),
                    EventPayload::Key("current".into()),
                )),
            ),
        ]
    );
    for expected in preview {
        assert_eq!(adapter.pop_native_event(), Some(expected));
    }
    assert_eq!(
        adapter.pop_native_event(),
        Some(NativeEvent::new(Some(observation), None))
    );
    assert_eq!(adapter.pop_native_event(), None);
}

#[test]
fn placement_coordinates_round_trip_negative_monitors_and_taskbar_offsets() {
    let screen = native::RECT {
        left: -1600,
        top: -200,
        right: -800,
        bottom: 400,
    };
    for (x, y) in [(0, 0), (48, 0), (0, 40), (48, 40)] {
        let workspace = window_placement::translate_rect(screen, -x, -y).unwrap();
        let restored = window_placement::translate_rect(workspace, x, y).unwrap();
        assert_eq!(restored, screen);
        assert_eq!(workspace.left, screen.left - x);
        assert_eq!(workspace.top, screen.top - y);
    }
    assert!(
        window_placement::translate_rect(
            native::RECT {
                left: i32::MAX,
                ..Default::default()
            },
            1,
            0,
        )
        .is_err()
    );
}

#[test]
fn initial_placement_rejects_invalid_extents_but_accepts_negative_positions() {
    let placement = WindowPlacement {
        x: -1200,
        y: -600,
        width: 800,
        height: 500,
        maximized: true,
    };
    let visuals = WindowVisuals::new().initial_placement(placement);
    assert_eq!(visuals.initial_placement, Some(placement));
    for invalid in [
        WindowPlacement {
            width: 0,
            ..placement
        },
        WindowPlacement {
            height: -1,
            ..placement
        },
        WindowPlacement {
            x: i32::MAX,
            ..placement
        },
        WindowPlacement {
            y: i32::MAX,
            ..placement
        },
    ] {
        assert!(
            std::panic::catch_unwind(|| WindowVisuals::new().initial_placement(invalid)).is_err()
        );
    }
}

#[test]
fn changed_initial_placement_does_not_become_a_reactive_resize() {
    let initial = WindowVisuals::new()
        .client_size(640.0, 480.0)
        .initial_position(ScreenPoint::new(50, 60));
    let updated = initial.clone().initial_placement(WindowPlacement {
        x: -1200,
        y: -600,
        width: 800,
        height: 500,
        maximized: true,
    });
    let changes = window_visual_changes(&initial, &updated);
    assert!(!changes.client_size);
    assert!(!changes.constraints);
    assert!(!changes.backdrop);
    assert!(!changes.icon);
    assert!(!changes.theme);
}

#[test]
fn content_dialog_schedule_is_fifo_and_cancellable() {
    let first = ObjectId::test(1);
    let second = ObjectId::test(2);
    let third = ObjectId::test(3);
    let mut schedule = ContentDialogSchedule::default();

    schedule.enqueue(first);
    schedule.enqueue(second);
    schedule.enqueue(second);
    schedule.enqueue(third);
    schedule.cancel(second);

    assert_eq!(schedule.next(), Some(first));
    assert_eq!(schedule.next(), Some(third));
    assert_eq!(schedule.next(), None);
}

#[test]
fn content_dialog_schedule_ignores_stale_completion_generation() {
    let dialog = ObjectId::test(1);
    let mut schedule = ContentDialogSchedule::default();
    schedule.begin(dialog, 7);

    assert!(!schedule.complete(dialog, 6));
    assert_eq!(schedule.active, Some((dialog, 7)));
    assert!(schedule.complete(dialog, 7));
    assert_eq!(schedule.active, None);
}

#[test]
fn active_slot_reorder_preserves_interleaved_retiring_indices() {
    let mut current = vec![(1, 1), (99, 99), (2, 2), (3, 3), (4, 4)];
    let active = HashSet::from([1, 2, 3, 4]);

    let swaps = simulate_active_slot_reorder(&mut current, &active, &[4, 3, 2, 1]).unwrap();

    assert_eq!(current, [(4, 4), (99, 99), (3, 3), (2, 2), (1, 1)]);
    assert_eq!(swaps.len(), 2);
    assert!(swaps.iter().all(|(left, right, _, _)| left < right));
}

#[test]
fn active_slot_reorder_rejects_missing_or_duplicate_active_identity() {
    let mut current = vec![(1, ()), (99, ()), (2, ())];
    let active = HashSet::from([1, 2]);

    assert!(simulate_active_slot_reorder(&mut current, &active, &[2, 3]).is_none());
    assert!(simulate_active_slot_reorder(&mut current, &active, &[2, 2]).is_none());
}

#[test]
fn list_items_replacement_keeps_the_selected_items_that_stay_in_selection_order() {
    let selected = vec![(30, "m30"), (5, "m5"), (7, "m7"), (12, "m12")];

    assert_eq!(
        kept_selection(selected.clone(), &HashSet::from([7, 99])),
        ["m30", "m5", "m12"]
    );
    assert_eq!(
        kept_selection(selected, &HashSet::from([30, 5, 7, 12])),
        [""; 0]
    );
    assert_eq!(
        kept_selection(Vec::<(usize, ())>::new(), &HashSet::new()),
        []
    );
}

#[test]
fn controlled_property_feedback_uses_exact_incumbent_clear_values() {
    let cases = [
        (
            ObjectType::CheckBox,
            PropertyId::IsChecked,
            EventId::IsCheckedChanged,
            PropertyValue::OptionalBool(Some(false)),
        ),
        (
            ObjectType::ToggleButton,
            PropertyId::IsChecked,
            EventId::IsCheckedChanged,
            PropertyValue::OptionalBool(Some(false)),
        ),
        (
            ObjectType::Expander,
            PropertyId::IsExpanded,
            EventId::IsExpandedChanged,
            PropertyValue::Bool(false),
        ),
        (
            ObjectType::NavigationView,
            PropertyId::IsPaneOpen,
            EventId::IsPaneOpenChanged,
            PropertyValue::Bool(true),
        ),
    ];

    for (kind, property, event, value) in cases {
        assert_eq!(
            GeneratedHandle::feedback_expectation(kind, property, None),
            Some((
                event,
                FeedbackExpectation::Exact(Property {
                    id: property,
                    value,
                }),
            ))
        );
    }
}

#[test]
fn only_a_navigation_view_losing_its_selection_expects_a_late_deselect_echo() {
    // NavigationView raises "nothing selected" late (ChangeSelection defers it to the idle
    // CompletePendingSelectionChange), so only a write that took its selection away expects one.
    assert!(defers_deselect_echo(
        ObjectType::NavigationView,
        true,
        false
    ));
    // Nothing was selected natively: WinUI raises nothing.
    assert!(!defers_deselect_echo(
        ObjectType::NavigationView,
        false,
        false
    ));
    // The selection moved to an item: WinUI raises that at once, inside the write.
    assert!(!defers_deselect_echo(
        ObjectType::NavigationView,
        true,
        true
    ));
    // Other selectors raise their deselect inside the write.
    for kind in [ObjectType::ListBox, ObjectType::SelectorBar] {
        assert!(!defers_deselect_echo(kind, true, false));
    }
}

fn virtual_index(value: IInspectable) -> i32 {
    value
        .cast::<windows_reference::IReference<i32>>()
        .unwrap()
        .Value()
        .unwrap()
}

#[test]
fn recycled_physical_shell_keeps_old_token_until_acknowledged() {
    let mut shells = ShellPool::<usize>::default();
    let (old, physical) = shells.take(|| Ok::<_, windows_core::Error>(42)).unwrap();

    assert!(shells.retire(old));
    assert!(shells.retired.contains(&old));
    let (new, reused) = shells
        .take(|| panic!("retired physical shell was not reused"))
        .unwrap();

    assert_ne!(new, old);
    assert_eq!(reused, physical);
    assert!(shells.retired.contains(&old));
    assert!(shells.shells.contains_key(&new));

    shells.acknowledge_recycle(old);
    assert!(!shells.retired.contains(&old));
    assert!(shells.shells.contains_key(&new));
}

#[test]
fn virtual_source_generates_indices_on_demand() {
    let source = ComObject::new(NativeVirtualSource::new(3).unwrap());
    let vector: IObservableVector<IInspectable> = source.to_interface();

    assert_eq!(vector.Size().unwrap(), 3);
    assert_eq!(virtual_index(vector.GetAt(0).unwrap()), 0);
    assert_eq!(virtual_index(vector.GetAt(2).unwrap()), 2);
    assert_eq!(vector.GetAt(3).unwrap_err().code(), E_BOUNDS);

    let mut index = u32::MAX;
    assert!(
        vector
            .IndexOf(&virtual_item_value(2).unwrap(), &mut index)
            .unwrap()
    );
    assert_eq!(index, 2);
    let missing: IInspectable = windows_reference::IReference::<i32>::from(9).into();
    assert!(!vector.IndexOf(&missing, &mut index).unwrap());
    assert_eq!(index, 0);
    let negative: IInspectable = windows_reference::IReference::<i32>::from(-1).into();
    assert!(!vector.IndexOf(&negative, &mut index).unwrap());
    let invalid: IInspectable =
        windows_reference::IReference::<HSTRING>::from(HSTRING::from("invalid")).into();
    assert!(vector.IndexOf(&invalid, &mut index).is_err());

    let mut values = vec![None; 4];
    assert_eq!(vector.GetMany(1, &mut values).unwrap(), 2);
    assert_eq!(virtual_index(values[0].take().unwrap()), 1);
    assert_eq!(virtual_index(values[1].take().unwrap()), 2);
    assert!(values[2].is_none());

    let view = vector.GetView().unwrap();
    assert_eq!(view.Size().unwrap(), 3);
    assert_eq!(virtual_index(view.GetAt(1).unwrap()), 1);

    let iterator = vector.First().unwrap();
    assert!(iterator.HasCurrent().unwrap());
    assert_eq!(virtual_index(iterator.Current().unwrap()), 0);
    assert!(iterator.MoveNext().unwrap());
    assert_eq!(virtual_index(iterator.Current().unwrap()), 1);
    let mut tail = vec![None; 3];
    assert_eq!(iterator.GetMany(&mut tail).unwrap(), 2);
    assert_eq!(virtual_index(tail[0].take().unwrap()), 1);
    assert_eq!(virtual_index(tail[1].take().unwrap()), 2);
    assert!(!iterator.HasCurrent().unwrap());
    assert_eq!(iterator.Current().unwrap_err().code(), E_BOUNDS);
}

#[test]
fn virtual_source_is_read_only_and_validates_index_limits() {
    let max_count = i32::MAX as usize + 1;
    let source = ComObject::new(NativeVirtualSource::new(max_count).unwrap());
    let vector: IObservableVector<IInspectable> = source.to_interface();

    assert_eq!(vector.Size().unwrap(), max_count as u32);
    assert_eq!(
        virtual_index(vector.GetAt(i32::MAX as u32).unwrap()),
        i32::MAX
    );
    assert_eq!(
        vector.GetAt(i32::MAX as u32 + 1).unwrap_err().code(),
        E_BOUNDS
    );
    assert!(NativeVirtualSource::new(max_count + 1).is_err());

    let value = virtual_item_value(0).unwrap();
    assert_eq!(
        vector.SetAt(0, &value).unwrap_err().code(),
        E_ILLEGAL_METHOD_CALL
    );
    assert_eq!(
        vector.Append(&value).unwrap_err().code(),
        E_ILLEGAL_METHOD_CALL
    );
    assert_eq!(vector.Clear().unwrap_err().code(), E_ILLEGAL_METHOD_CALL);
    assert_eq!(
        vector.ReplaceAll(&[Some(value)]).unwrap_err().code(),
        E_ILLEGAL_METHOD_CALL
    );
}

#[test]
fn virtual_source_reset_notifies_once_and_honors_event_removal() {
    let source = ComObject::new(NativeVirtualSource::new(0).unwrap());
    let vector: IObservableVector<IInspectable> = source.to_interface();
    let first = Arc::new(AtomicUsize::new(0));
    let second = Arc::new(AtomicUsize::new(0));
    let first_observed = Arc::clone(&first);
    let first_revoker = vector
        .VectorChanged(move |_, args| {
            let args = args.ok().unwrap();
            assert_eq!(args.CollectionChange().unwrap(), CollectionChange::Reset);
            assert_eq!(args.Index().unwrap(), 0);
            first_observed.fetch_add(1, Ordering::Relaxed);
        })
        .unwrap();
    let second_observed = Arc::clone(&second);
    let second_revoker = vector
        .VectorChanged(move |_, _| {
            second_observed.fetch_add(1, Ordering::Relaxed);
        })
        .unwrap();

    source.reset(10_000);
    assert_eq!(vector.Size().unwrap(), 10_000);
    assert_eq!(first.load(Ordering::Relaxed), 1);
    assert_eq!(second.load(Ordering::Relaxed), 1);

    drop(first_revoker);
    source.reset(10_000);
    assert_eq!(first.load(Ordering::Relaxed), 1);
    assert_eq!(second.load(Ordering::Relaxed), 2);

    drop(second_revoker);
    source.reset(0);
    assert_eq!(vector.Size().unwrap(), 0);
    assert_eq!(first.load(Ordering::Relaxed), 1);
    assert_eq!(second.load(Ordering::Relaxed), 2);
}

#[test]
fn policy_preserves_requested_window_state() {
    let policy = WindowPolicy::new()
        .title("Solitaire")
        .theme(WindowTheme::Dark)
        .client_size(800.0, 600.0)
        .minimum_client_size(800.0, 600.0);

    assert_eq!(policy.title.as_deref(), Some("Solitaire"));
    assert_eq!(policy.theme, WindowTheme::Dark);
    assert_eq!(policy.client_size, Some((800.0, 600.0)));
    assert_eq!(policy.minimum_client_size, Some((800.0, 600.0)));
}

#[test]
fn policy_rejects_invalid_window_sizes() {
    for (width, height) in [
        (0.0, 1.0),
        (1.0, -1.0),
        (f64::NAN, 1.0),
        (1.0, f64::INFINITY),
    ] {
        assert!(
            std::panic::catch_unwind(|| WindowPolicy::new().client_size(width, height)).is_err()
        );
        assert!(
            std::panic::catch_unwind(|| { WindowPolicy::new().minimum_client_size(width, height) })
                .is_err()
        );
    }
}

#[test]
fn window_visual_changes_only_apply_changed_contracts() {
    let previous = WindowVisuals::new()
        .theme(WindowTheme::System)
        .backdrop(WindowBackdrop::Mica)
        .client_size(800.0, 600.0);
    let next = WindowVisuals::new()
        .theme(WindowTheme::Dark)
        .backdrop(WindowBackdrop::Mica)
        .client_size(800.0, 600.0);

    let changes = window_visual_changes(&previous, &next);
    assert!(!changes.backdrop);
    assert!(!changes.client_size);
    assert!(!changes.constraints);
    assert!(!changes.icon);
    assert!(changes.theme);
}

#[test]
fn window_visual_changes_detect_icon_removal() {
    let previous = WindowVisuals::new().icon("test.ico");
    let changes = window_visual_changes(&previous, &WindowVisuals::new());
    assert!(changes.icon);
}

#[test]
fn pointer_event_queue_preserves_payload_and_revision() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime.update(Border::new()).unwrap();
    let object = runtime.graph().root().unwrap();
    let event = Rc::new(RefCell::new(NativePointerEventInfoEvent {
        revision: 7,
        callback: Some(Callback::new(|_| {})),
    }));
    let event_queue = Rc::new(NativeEventQueue::default());
    let payload = PointerEventInfo {
        pointer_id: 42,
        modifiers: InputModifiers::CONTROL,
        is_captured: true,
        is_right_button_pressed: true,
        ..Default::default()
    };

    assert!(WinUiAdapter::queue_event(
        &event,
        &event_queue,
        object,
        EventId::PointerReleased,
        None,
        payload,
    ));
    let queued = event_queue.events.borrow();
    assert_eq!(queued.len(), 1);
    assert!(queued[0].observation.is_none());
    let event = queued[0].event.as_ref().unwrap();
    assert_eq!(event.object, object);
    assert_eq!(event.event, EventId::PointerReleased);
    assert_eq!(event.revision, 7);
    assert_eq!(event.payload, EventPayload::PointerEventInfo(payload));
}

fn pointer_release_order(release: windows_core::Result<()>) -> (bool, Vec<EventId>) {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime.update(Border::new()).unwrap();
    let object = runtime.graph().root().unwrap();
    let released = Rc::new(RefCell::new(NativePointerEventInfoEvent {
        revision: 1,
        callback: Some(Callback::new(|_| {})),
    }));
    let capture_lost = Rc::new(RefCell::new(NativeUnitEvent {
        revision: 1,
        callback: Some(Callback::new(|_| {})),
    }));
    let event_queue = Rc::new(NativeEventQueue::default());

    let result = WinUiAdapter::release_then_uncapture(
        || {
            WinUiAdapter::dispatch(
                &released,
                &event_queue,
                object,
                EventId::PointerReleased,
                None,
                PointerEventInfo::default(),
            );
        },
        // `ReleasePointerCapture` raises `PointerCaptureLost` synchronously.
        || {
            release?;
            WinUiAdapter::dispatch(
                &capture_lost,
                &event_queue,
                object,
                EventId::PointerCaptureLost,
                None,
                (),
            );
            Ok(())
        },
    );
    let order = event_queue
        .events
        .borrow()
        .iter()
        .map(|queued| queued.event.as_ref().unwrap().event)
        .collect();
    (result.is_ok(), order)
}

#[test]
fn pointer_released_is_queued_before_capture_lost() {
    assert_eq!(
        pointer_release_order(Ok(())),
        (
            true,
            vec![EventId::PointerReleased, EventId::PointerCaptureLost]
        )
    );
}

#[test]
fn failed_pointer_release_still_queues_pointer_released() {
    assert_eq!(
        pointer_release_order(Err(native::E_FAIL.into())),
        (false, vec![EventId::PointerReleased])
    );
}

#[test]
#[ignore = "requires an interactive WinUI desktop"]
fn event_dispatch_wakes_only_when_queued() {
    // WinUI supports one Application lifetime per process.
    if std::env::var_os("REACTOR_EVENT_WAKE_TEST_CHILD").is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native::winui::tests::event_dispatch_wakes_only_when_queued",
                "--ignored",
                "--nocapture",
            ])
            .env("REACTOR_EVENT_WAKE_TEST_CHILD", "1")
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    let completed = Rc::new(Cell::new(false));
    let finished = Rc::clone(&completed);
    let result = App::run_with(move |app| {
        let mut runtime = Runtime::new(RecordingAdapter::default());
        runtime.update(Border::new()).unwrap();
        let object = runtime.graph().root().unwrap();
        let wakes = Rc::new(Cell::new(0));
        let event_queue = Rc::new(NativeEventQueue::default());
        *event_queue.waker.borrow_mut() = Some(Rc::new({
            let wakes = Rc::clone(&wakes);
            move || wakes.set(wakes.get() + 1)
        }));

        // An unsupported drop with no subscriber or observation must not wake.
        let unsubscribed = Rc::new(RefCell::new(NativeDroppedDataEvent::default()));
        WinUiAdapter::dispatch(
            &unsubscribed,
            &event_queue,
            object,
            EventId::Drop,
            None,
            DroppedData::Unsupported,
        );
        assert!(event_queue.events.borrow().is_empty());
        assert!(!event_queue.wake_pending.get());

        let subscribed = Rc::new(RefCell::new(NativeDroppedDataEvent {
            revision: 1,
            callback: Some(Callback::new(|_| {})),
        }));
        WinUiAdapter::dispatch(
            &subscribed,
            &event_queue,
            object,
            EventId::Drop,
            None,
            DroppedData::Unsupported,
        );
        assert_eq!(event_queue.events.borrow().len(), 1);
        assert!(event_queue.wake_pending.get());

        let timer = native::DispatcherQueue::GetForCurrentThread()?.CreateTimer()?;
        timer.SetInterval(TimeSpan::try_from(Duration::from_millis(100)).unwrap())?;
        let app = app.clone();
        let tick = timer.Tick(move |sender, _| {
            assert_eq!(wakes.get(), 1);
            assert!(!event_queue.wake_pending.get());
            sender.ok().unwrap().Stop().unwrap();
            finished.set(true);
            app.exit().unwrap();
        })?;
        timer.Start()?;
        Ok((timer, tick))
    });
    result.unwrap();
    assert!(completed.get());
}

#[test]
fn pointer_event_modifiers_map_native_flags() {
    let mut expected = InputModifiers::SHIFT;
    expected |= InputModifiers::CONTROL;
    expected |= InputModifiers::ALT;
    expected |= InputModifiers::WINDOWS;

    assert_eq!(
        WinUiAdapter::input_modifiers_from_virtual_keys(
            native::VirtualKeyModifiers::Shift
                | native::VirtualKeyModifiers::Control
                | native::VirtualKeyModifiers::Menu
                | native::VirtualKeyModifiers::Windows
        ),
        expected
    );
    assert_eq!(
        WinUiAdapter::input_modifiers_from_virtual_keys(native::VirtualKeyModifiers::None),
        InputModifiers::NONE
    );
}

#[test]
fn optional_numeric_adapters_use_distinct_sentinels() {
    assert!(native_number_box_value(None).is_nan());
    assert_eq!(number_box_value(f64::NAN), None);
    assert_eq!(number_box_value(-1.0), Some(-1.0));

    assert_eq!(native_rating_value(None), -1.0);
    assert_eq!(rating_value(-1.0), None);
    assert!(rating_value(f64::NAN).unwrap().is_nan());
}

#[test]
fn selection_index_conversion_reports_overflow() {
    assert_eq!(native_selection_index(None), Ok(-1));
    assert_eq!(
        native_selection_index(Some(i32::MAX as usize)),
        Ok(i32::MAX)
    );
    let overflow = i32::MAX as usize + 1;
    assert_eq!(
        native_selection_index(Some(overflow)),
        Err(WinUiError::IndexOverflow(overflow))
    );
}

#[test]
fn generated_feedback_contracts_distinguish_exact_normalized_and_clear() {
    let exact = GeneratedHandle::feedback_expectation(
        ObjectType::ToggleSwitch,
        PropertyId::IsOn,
        Some(&PropertyValue::Bool(true)),
    );
    assert_eq!(
        exact,
        Some((
            EventId::Toggled,
            FeedbackExpectation::Exact(Property {
                id: PropertyId::IsOn,
                value: PropertyValue::Bool(true),
            })
        ))
    );
    assert_eq!(
        GeneratedHandle::feedback_expectation(ObjectType::ToggleSwitch, PropertyId::IsOn, None,),
        Some((
            EventId::Toggled,
            FeedbackExpectation::Exact(Property {
                id: PropertyId::IsOn,
                value: PropertyValue::Bool(false),
            })
        ))
    );
    assert_eq!(
        GeneratedHandle::feedback_expectation(
            ObjectType::Slider,
            PropertyId::Value,
            Some(&PropertyValue::F64(0.5)),
        ),
        Some((
            EventId::ValueChanged,
            FeedbackExpectation::Normalized { observation: None }
        ))
    );
    assert_eq!(
        GeneratedHandle::feedback_expectation(
            ObjectType::ProgressBar,
            PropertyId::Value,
            Some(&PropertyValue::F64(0.5)),
        ),
        None
    );
    assert_eq!(
        GeneratedHandle::feedback_expectation(ObjectType::RichEditBox, PropertyId::Document, None,),
        Some((
            EventId::TextChanged,
            FeedbackExpectation::DeferredExact(Property {
                id: PropertyId::Document,
                value: PropertyValue::String(Rc::from("")),
            })
        ))
    );
}

#[test]
fn accelerator_keys_map_to_virtual_keys() {
    use AcceleratorKey as K;
    use native::VirtualKey as V;

    for (key, expected) in [
        (K::F2, V::F2),
        (K::F3, V::F3),
        (K::F4, V::F4),
        (K::F5, V::F5),
        (K::F6, V::F6),
        (K::F7, V::F7),
        (K::F8, V::F8),
        (K::F9, V::F9),
        (K::F10, V::F10),
        (K::F11, V::F11),
        (K::F12, V::F12),
        (K::Escape, V::Escape),
        (K::D, V::D),
        (K::S, V::S),
        (K::Number1, V::Number1),
        (K::Number2, V::Number2),
        (K::Number3, V::Number3),
        (K::Number4, V::Number4),
        (K::PageUp, V::PageUp),
        (K::PageDown, V::PageDown),
        (K::Home, V::Home),
        (K::Delete, V::Delete),
        (K::Y, V::Y),
        (K::A, V::A),
        (K::B, V::B),
        (K::C, V::C),
        (K::E, V::E),
        (K::F, V::F),
        (K::G, V::G),
        (K::H, V::H),
        (K::I, V::I),
        (K::J, V::J),
        (K::K, V::K),
        (K::L, V::L),
        (K::M, V::M),
        (K::O, V::O),
        (K::Q, V::Q),
        (K::T, V::T),
        (K::U, V::U),
        (K::V, V::V),
        (K::W, V::W),
        (K::X, V::X),
        (K::Z, V::Z),
    ] {
        assert_eq!(virtual_key(key), expected);
    }
}

#[test]
fn accelerator_modifiers_map_to_virtual_key_modifiers() {
    use InputModifiers as M;
    use native::VirtualKeyModifiers as V;

    assert_eq!(virtual_modifiers(M::NONE), V::None);
    assert_eq!(
        virtual_modifiers(AcceleratorModifiers::Control.into()),
        V::Control
    );
    assert_eq!(virtual_modifiers(M::ALT), V::Menu);
    assert_eq!(virtual_modifiers(M::CONTROL | M::ALT), V::Control | V::Menu);
    assert_eq!(virtual_modifiers(M::ALT | M::SHIFT), V::Menu | V::Shift);
    assert_eq!(
        virtual_modifiers(M::CONTROL | M::SHIFT),
        V::Control | V::Shift
    );
    assert_eq!(
        virtual_modifiers(M::CONTROL | M::ALT | M::SHIFT | M::WINDOWS),
        V::Control | V::Menu | V::Shift | V::Windows
    );
}

#[test]
fn scoped_accelerators_name_their_element_as_scope_owner() {
    let accelerator = KeyAccelerator::new(AcceleratorKey::Enter, AcceleratorModifiers::None, || {});
    let window = KeyAccelerators::new([accelerator.clone()]);
    let scoped = KeyAccelerators::scoped([accelerator]);

    assert_eq!(scope_owner(&window, &1), None);
    assert_eq!(scope_owner(&scoped, &1), Some(&1));
}

#[test]
fn button_menus_open_below_the_button() {
    for kind in [ObjectType::Button, ObjectType::DropDownButton] {
        assert_eq!(
            menu_placement(kind),
            Some(native::FlyoutPlacementMode::BottomEdgeAlignedLeft)
        );
    }
    assert_eq!(menu_placement(ObjectType::MenuBarItem), None);
}

#[test]
fn menus_on_other_elements_become_context_flyouts() {
    for kind in [ObjectType::Button, ObjectType::DropDownButton] {
        assert_eq!(menu_host(kind), MenuHost::Button);
    }
    assert_eq!(menu_host(ObjectType::MenuBarItem), MenuHost::MenuBarItem);
    for kind in [
        ObjectType::TextBlock,
        ObjectType::TextBox,
        ObjectType::NavigationViewItem,
    ] {
        assert_eq!(menu_host(kind), MenuHost::Context);
        assert_eq!(menu_placement(kind), None);
    }
}

#[test]
fn context_menus_remember_the_element_flyout_once() {
    let unread =
        || -> Result<Option<&str>, ()> { panic!("a replaced menu must not read the element") };
    assert_eq!(
        original_context_flyout(None, || Ok::<_, ()>(Some("text"))),
        Ok(Some("text"))
    );
    assert_eq!(
        original_context_flyout(None, || Ok::<_, ()>(None::<&str>)),
        Ok(None)
    );
    assert_eq!(
        original_context_flyout(None, || Err::<Option<&str>, _>(())),
        Err(())
    );
    assert_eq!(
        original_context_flyout(Some(Some("text")), unread),
        Ok(Some("text"))
    );
    assert_eq!(original_context_flyout(Some(None), unread), Ok(None));
}

#[test]
fn menu_swatch_is_a_centred_circle_in_the_icon_slot() {
    let xaml = menu_swatch_xaml(ThemeBrush::Resource("SwatchBrush"));
    assert!(xaml.starts_with("<PathIcon "));
    assert!(xaml.contains("Data='M3,8 A5,5 0 1 1 13,8 A5,5 0 1 1 3,8 Z'"));
    assert!(xaml.contains("Foreground='{ThemeResource SwatchBrush}'"));
    assert!(
        menu_swatch_xaml(ThemeBrush::Accent)
            .contains("{ThemeResource AccentFillColorDefaultBrush}")
    );
}

// Covers the XAML text only; lib tests do not run live WinUI.
#[test]
fn themed_icons_load_with_a_theme_resource_foreground() {
    let xaml = themed_icon_xaml("FontIconSource", ThemeBrush::Resource("FolderBrush"));
    assert!(xaml.starts_with("<FontIconSource "));
    assert!(xaml.contains("Foreground='{ThemeResource FolderBrush}'"));
    assert!(
        themed_icon_xaml("SymbolIcon", ThemeBrush::Accent)
            .contains("{ThemeResource AccentFillColorDefaultBrush}")
    );
}

#[test]
fn data_item_text_names_list_containers() {
    let key = HSTRING::from("Text");
    let values: IObservableMap<HSTRING, IInspectable> =
        BTreeMap::<HSTRING, Option<IInspectable>>::new().into();
    let item: IInspectable = values.cast().unwrap();
    assert_eq!(data_item_text(&item, &key).unwrap(), "");
    let text: IInspectable =
        windows_reference::IReference::from(HSTRING::from("Unread message")).into();
    values.Insert(&key, &text).unwrap();
    assert_eq!(data_item_text(&item, &key).unwrap(), "Unread message");
}
