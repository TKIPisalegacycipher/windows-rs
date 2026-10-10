use super::*;

#[test]
fn rich_edit_text_is_canonical_and_uses_deferred_exact_feedback() {
    let values = Rc::new(RefCell::new(Vec::new()));
    let callback = {
        let values = Rc::clone(&values);
        Callback::new(move |value: Rc<str>| values.borrow_mut().push(value))
    };
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(
            RichEditBox::new()
                .text("one\r\ntwo\rthree")
                .on_text_changed(callback.clone()),
        )
        .unwrap();
    let object = runtime.graph().root().unwrap();
    assert!(
        runtime
            .graph()
            .properties(object)
            .unwrap()
            .contains(&Property {
                id: PropertyId::Document,
                value: PropertyValue::String(Rc::from("one\ntwo\nthree")),
            })
    );
    assert!(
        runtime
            .update(
                RichEditBox::new()
                    .text("one\ntwo\nthree")
                    .on_text_changed(callback.clone()),
            )
            .unwrap()
            .is_empty()
    );

    runtime.adapter_mut().queue_native_event(
        Some(Observation::SetProperty {
            object,
            property: Property {
                id: PropertyId::Document,
                value: PropertyValue::String(Rc::from("native\ntext")),
            },
        }),
        Some(EventDispatch::new(
            object,
            EventId::TextChanged,
            EventValue::String(callback),
            EventPayload::String(Rc::from("native\ntext")),
        )),
    );
    assert_eq!(runtime.dispatch_native_events().unwrap(), 1);
    assert_eq!(&*values.borrow(), &[Rc::from("native\ntext")]);
}

#[test]
fn rich_edit_document_removal_clears_once_and_stays_omitted() {
    let callback = Callback::new(|_: Rc<str>| {});
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(
            RichEditBox::new()
                .text("Before")
                .on_text_changed(callback.clone()),
        )
        .unwrap();
    let object = runtime.graph().root().unwrap();

    let mutations = runtime
        .update(RichEditBox::new().on_text_changed(callback.clone()))
        .unwrap();
    assert!(mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::SetProperties { object: changed, clear, .. }
            if *changed == object && clear.as_ref() == [PropertyId::Document]
    )));
    assert!(
        runtime
            .graph()
            .properties(object)
            .unwrap()
            .iter()
            .all(|property| property.id != PropertyId::Document)
    );
    assert!(
        runtime
            .update(RichEditBox::new().on_text_changed(callback))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn calendar_date_is_a_controlled_property_that_clears_to_none() {
    let date = DateTime::from_unix_secs(1_700_000_000);
    let callback = Callback::new(|_: Option<DateTime>| {});
    let date_property = |runtime: &Runtime<RecordingAdapter>| {
        let object = runtime.graph().root().unwrap();
        runtime
            .graph()
            .properties(object)
            .unwrap()
            .iter()
            .find(|property| property.id == PropertyId::Date)
            .map(|property| property.value.clone())
    };
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(
            CalendarDatePicker::new()
                .date(date)
                .on_date_changed(callback.clone()),
        )
        .unwrap();
    assert_eq!(
        date_property(&runtime),
        Some(PropertyValue::OptionalDateTime(Some(date)))
    );

    runtime
        .update(
            CalendarDatePicker::new()
                .date(None)
                .on_date_changed(callback.clone()),
        )
        .unwrap();
    assert_eq!(
        date_property(&runtime),
        Some(PropertyValue::OptionalDateTime(None))
    );

    let object = runtime.graph().root().unwrap();
    let mutations = runtime
        .update(
            CalendarDatePicker::new()
                .date(date)
                .on_date_changed(callback.clone()),
        )
        .unwrap();
    assert!(mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::SetProperties { object: changed, set, .. }
            if *changed == object && set.as_ref() == [Property {
                id: PropertyId::Date,
                value: PropertyValue::OptionalDateTime(Some(date)),
            }]
    )));
    let mutations = runtime
        .update(CalendarDatePicker::new().on_date_changed(callback))
        .unwrap();
    assert!(mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::SetProperties { object: changed, clear, .. }
            if *changed == object && clear.as_ref() == [PropertyId::Date]
    )));
    assert_eq!(date_property(&runtime), None);
}

#[test]
fn calendar_date_pick_is_cleared_when_the_next_declaration_omits_date() {
    let date = DateTime::from_unix_secs(1_700_000_000);
    let callback = Callback::new(|_: Option<DateTime>| {});
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(CalendarDatePicker::new().on_date_changed(callback.clone()))
        .unwrap();
    let object = runtime.graph().root().unwrap();
    runtime.adapter_mut().queue_native_event(
        Some(Observation::SetProperty {
            object,
            property: Property {
                id: PropertyId::Date,
                value: PropertyValue::OptionalDateTime(Some(date)),
            },
        }),
        None,
    );
    runtime.dispatch_native_events().unwrap();
    assert!(
        runtime
            .graph()
            .properties(object)
            .unwrap()
            .contains(&Property {
                id: PropertyId::Date,
                value: PropertyValue::OptionalDateTime(Some(date)),
            })
    );

    let mutations = runtime
        .update(CalendarDatePicker::new().on_date_changed(callback))
        .unwrap();
    assert!(mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::SetProperties { object: changed, set, clear }
            if *changed == object && set.is_empty() && clear.as_ref() == [PropertyId::Date]
    )));
}

#[test]
fn deferred_exact_feedback_suppresses_matching_observations_until_a_difference() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime.update(RichEditBox::new().text("Before")).unwrap();
    let object = runtime.graph().root().unwrap();
    let property = Property {
        id: PropertyId::Document,
        value: PropertyValue::String(Rc::from("After")),
    };
    let mut feedback = FeedbackState::default();
    feedback.begin(
        object,
        EventId::TextChanged,
        FeedbackExpectation::DeferredExact(property.clone()),
    );
    assert_eq!(feedback.finish(object, EventId::TextChanged), None);
    for _ in 0..3 {
        assert!(!feedback.observe(
            object,
            EventId::TextChanged,
            Observation::SetProperty {
                object,
                property: property.clone(),
            }
        ));
    }
    let different = Observation::SetProperty {
        object,
        property: Property {
            id: PropertyId::Document,
            value: PropertyValue::String(Rc::from("Different")),
        },
    };
    assert!(feedback.observe(object, EventId::TextChanged, different.clone()));
    assert!(feedback.observe(object, EventId::TextChanged, different));
}

#[test]
fn newer_deferred_exact_write_replaces_the_previous_expectation() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime.update(RichEditBox::new()).unwrap();
    let object = runtime.graph().root().unwrap();
    let mut feedback = FeedbackState::default();
    let property = |value| Property {
        id: PropertyId::Document,
        value: PropertyValue::String(Rc::from(value)),
    };
    feedback.begin(
        object,
        EventId::TextChanged,
        FeedbackExpectation::DeferredExact(property("Old")),
    );
    assert_eq!(feedback.finish(object, EventId::TextChanged), None);
    feedback.begin(
        object,
        EventId::TextChanged,
        FeedbackExpectation::DeferredExact(property("New")),
    );
    assert_eq!(feedback.finish(object, EventId::TextChanged), None);

    assert!(!feedback.observe(
        object,
        EventId::TextChanged,
        Observation::SetProperty {
            object,
            property: property("New"),
        }
    ));
    assert!(feedback.observe(
        object,
        EventId::TextChanged,
        Observation::SetProperty {
            object,
            property: property("Old"),
        }
    ));
}

#[test]
fn deferred_exact_clear_suppresses_repeated_empty_observations() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime.update(RichEditBox::new().text("Before")).unwrap();
    let object = runtime.graph().root().unwrap();
    let empty = Property {
        id: PropertyId::Document,
        value: PropertyValue::String(Rc::from("")),
    };
    let mut feedback = FeedbackState::default();
    feedback.begin(
        object,
        EventId::TextChanged,
        FeedbackExpectation::DeferredExact(empty.clone()),
    );
    assert_eq!(feedback.finish(object, EventId::TextChanged), None);
    for _ in 0..2 {
        assert!(!feedback.observe(
            object,
            EventId::TextChanged,
            Observation::SetProperty {
                object,
                property: empty.clone(),
            }
        ));
    }
    feedback.remove_object(object);
    assert!(feedback.observe(
        object,
        EventId::TextChanged,
        Observation::SetProperty {
            object,
            property: empty,
        }
    ));
}
