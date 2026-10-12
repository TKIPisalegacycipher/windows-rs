use super::*;

#[test]
fn selection_observations_preserve_declaration_ownership() {
    let values = Rc::new(RefCell::new(Vec::new()));
    let received = Rc::clone(&values);
    let callback = Callback::new(move |value| received.borrow_mut().push(value));
    let view = |selected| {
        let item = ListBoxItem::new().tag("item");
        ListBox::new()
            .items((match selected {
                Some(selected) => item.is_selected(selected),
                None => item,
            },))
            .on_selected_tag_changed(callback.clone())
    };
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime.update(view(None)).unwrap();
    let owner = runtime.graph().root().unwrap();
    let item = runtime.graph().children(owner, RelationId::Items).unwrap()[0];
    let observation = Observation::SetSelection {
        object: owner,
        selected: Some(item),
    };
    runtime.adapter_mut().queue_native_event(
        Some(observation.clone()),
        Some(EventDispatch::new(
            owner,
            EventId::SelectionChanged,
            EventValue::Selection(callback.clone()),
            EventPayload::Selection(SelectionChange {
                item: Some(item),
                value: Some(Rc::from("item")),
            }),
        )),
    );
    assert_eq!(runtime.dispatch_native_events().unwrap(), 1);
    assert_eq!(*values.borrow(), [Some(Rc::from("item"))]);
    assert!(runtime.update(view(None)).unwrap().is_empty());

    runtime.update(view(Some(false))).unwrap();
    runtime.adapter_mut().observe(observation.clone());
    runtime.dispatch_native_events().unwrap();
    assert!(runtime.update(view(Some(true))).unwrap().is_empty());
    assert!(matches!(
        runtime.update(view(None)).unwrap().as_slice(),
        [Mutation::SetProperties { object, set, clear }]
            if *object == item && set.is_empty() && clear.as_ref() == [PropertyId::IsSelected]
    ));
    runtime.adapter_mut().observe(observation);
    runtime.dispatch_native_events().unwrap();
    assert!(runtime.update(view(None)).unwrap().is_empty());
}

#[test]
fn retained_selection_tracks_keyed_items_across_reorder_and_removal() {
    let selected_values = Rc::new(RefCell::new(Vec::new()));
    let selected_values_for_callback = Rc::clone(&selected_values);
    let callback =
        Callback::new(move |value| selected_values_for_callback.borrow_mut().push(value));
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(
            ListBox::new()
                .keyed_items([
                    keyed("first", ListBoxItem::new().tag("first").is_selected(true)),
                    keyed(
                        "second",
                        ListBoxItem::new().tag("second").is_selected(false),
                    ),
                ])
                .on_selected_tag_changed(callback.clone()),
        )
        .unwrap();
    let owner = runtime.graph().root().unwrap();
    let before = runtime
        .graph()
        .children(owner, RelationId::Items)
        .unwrap()
        .to_vec();
    runtime.adapter_mut().queue_native_event(
        Some(Observation::SetSelection {
            object: owner,
            selected: Some(before[1]),
        }),
        Some(EventDispatch::new(
            owner,
            EventId::SelectionChanged,
            EventValue::Selection(callback.clone()),
            EventPayload::Selection(SelectionChange {
                item: Some(before[1]),
                value: Some(Rc::from("second")),
            }),
        )),
    );

    assert_eq!(runtime.dispatch_native_events().unwrap(), 1);
    assert_eq!(
        runtime.graph().properties(before[0]).unwrap(),
        [
            Property {
                id: PropertyId::IsSelected,
                value: PropertyValue::Bool(false),
            },
            Property {
                id: PropertyId::Tag,
                value: PropertyValue::String(Rc::from("first")),
            },
        ]
    );
    assert_eq!(
        runtime.graph().properties(before[1]).unwrap(),
        [
            Property {
                id: PropertyId::IsSelected,
                value: PropertyValue::Bool(true),
            },
            Property {
                id: PropertyId::Tag,
                value: PropertyValue::String(Rc::from("second")),
            },
        ]
    );
    assert_eq!(&*selected_values.borrow(), &[Some(Rc::from("second"))]);

    let mutations = runtime
        .update(
            ListBox::new()
                .keyed_items([
                    keyed("second", ListBoxItem::new().tag("second").is_selected(true)),
                    keyed("first", ListBoxItem::new().tag("first").is_selected(false)),
                ])
                .on_selected_tag_changed(callback.clone()),
        )
        .unwrap();
    assert_eq!(
        runtime.graph().children(owner, RelationId::Items).unwrap(),
        [before[1], before[0]]
    );
    assert_eq!(
        mutations
            .iter()
            .filter(|mutation| matches!(mutation, Mutation::Reorder { .. }))
            .count(),
        1
    );
    assert!(!mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::SetProperties { object, .. } if before.contains(object)
    )));

    runtime
        .update(
            ListBox::new()
                .keyed_items([keyed(
                    "first",
                    ListBoxItem::new().tag("first").is_selected(true),
                )])
                .on_selected_tag_changed(callback.clone()),
        )
        .unwrap();
    runtime.adapter_mut().queue_native_event(
        Some(Observation::SetSelection {
            object: owner,
            selected: Some(before[1]),
        }),
        Some(EventDispatch::new(
            owner,
            EventId::SelectionChanged,
            EventValue::Selection(callback),
            EventPayload::Selection(SelectionChange {
                item: Some(before[1]),
                value: Some(Rc::from("stale")),
            }),
        )),
    );
    assert_eq!(runtime.dispatch_native_events().unwrap(), 0);
    assert_eq!(&*selected_values.borrow(), &[Some(Rc::from("second"))]);
    assert_eq!(
        runtime.graph().properties(before[0]).unwrap(),
        [
            Property {
                id: PropertyId::IsSelected,
                value: PropertyValue::Bool(true),
            },
            Property {
                id: PropertyId::Tag,
                value: PropertyValue::String(Rc::from("first")),
            },
        ]
    );
}

#[test]
fn selection_contracts_cover_navigation_list_box_and_selector_bar() {
    for (owner, item, relations, payload) in [
        (
            ObjectType::NavigationView,
            ObjectType::NavigationViewItem,
            &[RelationId::MenuItems, RelationId::FooterMenuItems][..],
            PropertyId::Tag,
        ),
        (
            ObjectType::ListBox,
            ObjectType::ListBoxItem,
            &[RelationId::Items][..],
            PropertyId::Tag,
        ),
        (
            ObjectType::SelectorBar,
            ObjectType::SelectorBarItem,
            &[RelationId::Items][..],
            PropertyId::Text,
        ),
    ] {
        let contract = selection_contract(owner).unwrap();
        assert_eq!(contract.item, item);
        assert_eq!(contract.relations, relations);
        assert_eq!(contract.selected_property, PropertyId::IsSelected);
        assert_eq!(contract.event, EventId::SelectionChanged);
        assert_eq!(contract.payload_property, payload);
    }
}

#[test]
fn grid_view_reorder_payload_preserves_item_tag_order() {
    let values = Rc::new(RefCell::new(Vec::new()));
    let callback = {
        let values = Rc::clone(&values);
        Callback::new(move |value| values.borrow_mut().push(value))
    };
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(
            GridView::new()
                .items([
                    GridViewItem::new().tag("first").into(),
                    GridViewItem::new().tag("second").into(),
                ])
                .on_reordered(callback.clone()),
        )
        .unwrap();
    let grid = runtime.graph().root().unwrap();
    runtime.adapter_mut().queue_event(EventDispatch::new(
        grid,
        EventId::DragItemsCompleted,
        EventValue::StringList(callback),
        EventPayload::StringList(vec!["second".to_string(), "first".to_string()]),
    ));

    assert_eq!(runtime.dispatch_native_events().unwrap(), 1);
    assert_eq!(
        &*values.borrow(),
        &[vec!["second".to_string(), "first".to_string()]]
    );
}

#[test]
fn list_view_properties_update_and_clear() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    let mutations = runtime
        .update(
            ListView::new()
                .selected_index(Some(1))
                .selection_mode(ListViewSelectionMode::Extended)
                .can_drag_items(true)
                .can_reorder_items(true)
                .allow_drop(true),
        )
        .unwrap();
    let list = runtime.graph().root().unwrap();
    let set = mutations
        .iter()
        .find_map(|mutation| match mutation {
            Mutation::SetProperties { object, set, .. } if *object == list => Some(set),
            _ => None,
        })
        .unwrap();
    for property in [
        PropertyId::SelectedIndex,
        PropertyId::SelectionMode,
        PropertyId::CanDragItems,
        PropertyId::CanReorderItems,
        PropertyId::AllowDrop,
    ] {
        assert!(set.iter().any(|value| value.id == property));
    }

    let mutations = runtime.update(ListView::new()).unwrap();
    let clear = mutations
        .iter()
        .find_map(|mutation| match mutation {
            Mutation::SetProperties { object, clear, .. } if *object == list => Some(clear),
            _ => None,
        })
        .unwrap();
    assert_eq!(clear.len(), 5);
    for property in [
        PropertyId::SelectedIndex,
        PropertyId::SelectionMode,
        PropertyId::CanDragItems,
        PropertyId::CanReorderItems,
        PropertyId::AllowDrop,
    ] {
        assert!(clear.contains(&property));
    }
}

#[test]
fn data_item_container_resource_overrides_update_and_clear() {
    let resources =
        ResourceOverrides::new().set("ListViewItemBackgroundPointerOver", Color::transparent());
    let list = |item: DataItem| ListView::new().items([item]);
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(list(
            DataItem::new("header", "Today").container_resource_overrides(resources.clone()),
        ))
        .unwrap();
    let list_view = runtime.graph().root().unwrap();
    let item = runtime
        .graph()
        .children(list_view, RelationId::Items)
        .unwrap()[0];
    assert!(
        runtime
            .graph()
            .properties(item)
            .unwrap()
            .contains(&Property {
                id: PropertyId::ContainerResources,
                value: PropertyValue::ResourceOverrides(resources),
            })
    );

    let changed =
        ResourceOverrides::new().set("ListViewItemBackgroundPressed", Color::transparent());
    let mutations = runtime
        .update(list(
            DataItem::new("header", "Today").container_resource_overrides(changed.clone()),
        ))
        .unwrap();
    assert!(mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::SetProperties { object, set, clear }
            if *object == item && clear.is_empty() && set.as_ref() == [Property {
                id: PropertyId::ContainerResources,
                value: PropertyValue::ResourceOverrides(changed.clone()),
            }]
    )));

    let mutations = runtime
        .update(list(DataItem::new("header", "Today")))
        .unwrap();
    assert!(mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::SetProperties { object, set, clear }
            if *object == item && set.is_empty()
                && clear.as_ref() == [PropertyId::ContainerResources]
    )));
}

#[test]
fn list_view_item_min_height_updates_clears_and_rejects_invalid_values() {
    for value in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(std::panic::catch_unwind(|| ListView::new().item_min_height(value)).is_err());
    }

    let mut runtime = Runtime::new(RecordingAdapter::default());
    let mutations = runtime
        .update(ListView::new().item_min_height(28.0))
        .unwrap();
    let list = runtime.graph().root().unwrap();
    let set = mutations
        .iter()
        .find_map(|mutation| match mutation {
            Mutation::SetProperties { object, set, .. } if *object == list => Some(set),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        &**set,
        [Property {
            id: PropertyId::ItemMinHeight,
            value: PropertyValue::F64(28.0),
        }]
    );

    let mutations = runtime.update(ListView::new()).unwrap();
    let clear = mutations
        .iter()
        .find_map(|mutation| match mutation {
            Mutation::SetProperties { object, clear, .. } if *object == list => Some(clear),
            _ => None,
        })
        .unwrap();
    assert_eq!(&**clear, [PropertyId::ItemMinHeight]);
}

#[test]
fn list_view_selection_and_reorder_callbacks_replace_remove_and_reject_stale_events() {
    let first_selections = Rc::new(RefCell::new(Vec::new()));
    let first_selection = {
        let values = Rc::clone(&first_selections);
        Callback::new(move |value| values.borrow_mut().push(value))
    };
    let first_reorders = Rc::new(RefCell::new(Vec::new()));
    let first_reorder = {
        let values = Rc::clone(&first_reorders);
        Callback::new(move |value| values.borrow_mut().push(value))
    };
    let second_selections = Rc::new(RefCell::new(Vec::new()));
    let second_selection = {
        let values = Rc::clone(&second_selections);
        Callback::new(move |value| values.borrow_mut().push(value))
    };
    let second_reorders = Rc::new(RefCell::new(Vec::new()));
    let second_reorder = {
        let values = Rc::clone(&second_reorders);
        Callback::new(move |value| values.borrow_mut().push(value))
    };
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(
            ListView::new()
                .on_selection_changed(first_selection.clone())
                .on_reordered(first_reorder.clone()),
        )
        .unwrap();
    let list = runtime.graph().root().unwrap();
    runtime.adapter_mut().queue_native_event(
        Some(Observation::SetProperty {
            object: list,
            property: Property {
                id: PropertyId::SelectedIndex,
                value: PropertyValue::SelectionIndex(Some(1)),
            },
        }),
        Some(EventDispatch::new(
            list,
            EventId::SelectionChanged,
            EventValue::SelectionIndex(first_selection.clone()),
            EventPayload::SelectionIndex(Some(1)),
        )),
    );
    runtime.adapter_mut().queue_event(EventDispatch::new(
        list,
        EventId::DragItemsCompleted,
        EventValue::StringList(first_reorder.clone()),
        EventPayload::StringList(vec!["second".to_string(), "first".to_string()]),
    ));
    assert_eq!(runtime.dispatch_native_events().unwrap(), 2);
    assert_eq!(&*first_selections.borrow(), &[Some(1)]);
    assert_eq!(
        &*first_reorders.borrow(),
        &[vec!["second".to_string(), "first".to_string()]]
    );

    runtime
        .update(
            ListView::new()
                .selected_index(Some(1))
                .on_selection_changed(second_selection.clone())
                .on_reordered(second_reorder.clone()),
        )
        .unwrap();
    runtime.adapter_mut().queue_event(EventDispatch::new(
        list,
        EventId::SelectionChanged,
        EventValue::SelectionIndex(first_selection),
        EventPayload::SelectionIndex(None),
    ));
    runtime.adapter_mut().queue_event(EventDispatch::new(
        list,
        EventId::DragItemsCompleted,
        EventValue::StringList(first_reorder),
        EventPayload::StringList(vec!["stale".to_string()]),
    ));
    runtime.adapter_mut().queue_event(EventDispatch::new(
        list,
        EventId::SelectionChanged,
        EventValue::SelectionIndex(second_selection.clone()),
        EventPayload::SelectionIndex(None),
    ));
    runtime.adapter_mut().queue_event(EventDispatch::new(
        list,
        EventId::DragItemsCompleted,
        EventValue::StringList(second_reorder.clone()),
        EventPayload::StringList(vec!["current".to_string()]),
    ));
    assert_eq!(runtime.dispatch_native_events().unwrap(), 2);
    assert_eq!(&*second_selections.borrow(), &[None]);
    assert_eq!(&*second_reorders.borrow(), &[vec!["current".to_string()]]);

    runtime.update(ListView::new()).unwrap();
    runtime.adapter_mut().queue_event(EventDispatch::new(
        list,
        EventId::SelectionChanged,
        EventValue::SelectionIndex(second_selection),
        EventPayload::SelectionIndex(Some(2)),
    ));
    runtime.adapter_mut().queue_event(EventDispatch::new(
        list,
        EventId::DragItemsCompleted,
        EventValue::StringList(second_reorder),
        EventPayload::StringList(vec!["removed".to_string()]),
    ));
    assert_eq!(runtime.dispatch_native_events().unwrap(), 0);
}

#[test]
fn list_view_selected_keys_follow_selection_order_and_reject_stale_items() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let selection = {
        let log = Rc::clone(&log);
        Callback::new(move |value: Option<usize>| log.borrow_mut().push(format!("{value:?}")))
    };
    let keys = {
        let log = Rc::clone(&log);
        Callback::new(move |value: Vec<Key>| log.borrow_mut().push(format!("{value:?}")))
    };
    let items = || {
        [
            DataItem::new("first", "First"),
            DataItem::new(2u64, "Second"),
            DataItem::new("third", "Third"),
        ]
    };
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(
            ListView::new()
                .selection_mode(ListViewSelectionMode::Multiple)
                .items(items())
                .on_selection_changed(selection.clone())
                .on_selected_keys_changed(keys.clone()),
        )
        .unwrap();
    let list = runtime.graph().root().unwrap();
    let children = runtime
        .graph()
        .children(list, RelationId::Items)
        .unwrap()
        .to_vec();
    assert!(
        event_contracts(ObjectType::ListView).contains(&EventContract {
            id: EventId::SelectedKeysChanged,
            value: ValueType::Keys,
        })
    );
    runtime.adapter_mut().queue_event(EventDispatch::new(
        list,
        EventId::SelectionChanged,
        EventValue::SelectionIndex(selection),
        EventPayload::SelectionIndex(Some(0)),
    ));
    runtime.adapter_mut().queue_event(EventDispatch::new(
        list,
        EventId::SelectedKeysChanged,
        EventValue::Keys(keys.clone()),
        EventPayload::Items(vec![children[2], children[0], children[1]]),
    ));
    assert_eq!(runtime.dispatch_native_events().unwrap(), 2);
    assert_eq!(
        *log.borrow(),
        [
            format!("{:?}", Some(0)),
            format!(
                "{:?}",
                [Key::from("third"), Key::from("first"), Key::from(2u64)]
            ),
        ]
    );

    runtime
        .update(
            ListView::new()
                .selection_mode(ListViewSelectionMode::Multiple)
                .items([
                    DataItem::new("first", "First"),
                    DataItem::new(2u64, "Second"),
                ])
                .on_selected_keys_changed(keys.clone()),
        )
        .unwrap();
    runtime.adapter_mut().queue_event(EventDispatch::new(
        list,
        EventId::SelectedKeysChanged,
        EventValue::Keys(keys.clone()),
        EventPayload::Items(vec![children[0], children[2]]),
    ));
    runtime.adapter_mut().queue_event(EventDispatch::new(
        list,
        EventId::SelectedKeysChanged,
        EventValue::Keys(keys.clone()),
        EventPayload::Items(Vec::new()),
    ));
    assert_eq!(runtime.dispatch_native_events().unwrap(), 1);
    assert_eq!(log.borrow().last().unwrap(), "[]");

    runtime.update(ListView::new().items(items())).unwrap();
    runtime.adapter_mut().queue_event(EventDispatch::new(
        list,
        EventId::SelectedKeysChanged,
        EventValue::Keys(keys),
        EventPayload::Items(vec![children[0]]),
    ));
    assert_eq!(runtime.dispatch_native_events().unwrap(), 0);
    assert_eq!(log.borrow().len(), 3);
}

#[test]
fn list_view_item_click_delivers_the_clicked_key() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let clicked = {
        let log = Rc::clone(&log);
        Callback::new(move |value: Vec<Key>| log.borrow_mut().push(format!("{value:?}")))
    };
    let items = || {
        [
            DataItem::new("first", "First"),
            DataItem::new(2u64, "Second"),
        ]
    };
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(
            ListView::new()
                .items(items())
                .on_item_click(clicked.clone()),
        )
        .unwrap();
    let list = runtime.graph().root().unwrap();
    let children = runtime
        .graph()
        .children(list, RelationId::Items)
        .unwrap()
        .to_vec();
    assert!(
        event_contracts(ObjectType::ListView).contains(&EventContract {
            id: EventId::ItemClick,
            value: ValueType::Keys,
        })
    );
    runtime.adapter_mut().queue_event(EventDispatch::new(
        list,
        EventId::ItemClick,
        EventValue::Keys(clicked.clone()),
        EventPayload::Items(vec![children[1]]),
    ));
    assert_eq!(runtime.dispatch_native_events().unwrap(), 1);
    assert_eq!(*log.borrow(), [format!("{:?}", [Key::from(2u64)])]);

    runtime.update(ListView::new().items(items())).unwrap();
    runtime.adapter_mut().queue_event(EventDispatch::new(
        list,
        EventId::ItemClick,
        EventValue::Keys(clicked),
        EventPayload::Items(vec![children[0]]),
    ));
    assert_eq!(runtime.dispatch_native_events().unwrap(), 0);
    assert_eq!(log.borrow().len(), 1);
}

#[test]
fn tree_view_selection_mode_and_item_invocation_replace_remove_and_reject_stale_events() {
    let first = Rc::new(RefCell::new(Vec::new()));
    let first_callback = {
        let values = Rc::clone(&first);
        Callback::new(move |value| values.borrow_mut().push(value))
    };
    let second = Rc::new(RefCell::new(Vec::new()));
    let second_callback = {
        let values = Rc::clone(&second);
        Callback::new(move |value| values.borrow_mut().push(value))
    };
    let mut runtime = Runtime::new(RecordingAdapter::default());
    let mutations = runtime
        .update(
            TreeView::new()
                .selection_mode(TreeViewSelectionMode::Multiple)
                .nodes([TreeNode::new("root", "Root")])
                .on_item_invoked(first_callback.clone()),
        )
        .unwrap();
    let tree = runtime.graph().root().unwrap();
    assert!(mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::SetProperties { object, set, .. }
            if *object == tree
                && set.iter().any(|property| property.id == PropertyId::SelectionMode)
    )));
    runtime.adapter_mut().queue_event(EventDispatch::new(
        tree,
        EventId::ItemInvoked,
        EventValue::String(first_callback.clone()),
        EventPayload::String(Rc::from("Root")),
    ));
    assert_eq!(runtime.dispatch_native_events().unwrap(), 1);
    assert_eq!(&*first.borrow(), &[Rc::from("Root")]);

    runtime
        .update(
            TreeView::new()
                .selection_mode(TreeViewSelectionMode::Single)
                .nodes([TreeNode::new("root", "Root")])
                .on_item_invoked(second_callback.clone()),
        )
        .unwrap();
    runtime.adapter_mut().queue_event(EventDispatch::new(
        tree,
        EventId::ItemInvoked,
        EventValue::String(first_callback),
        EventPayload::String(Rc::from("Stale")),
    ));
    runtime.adapter_mut().queue_event(EventDispatch::new(
        tree,
        EventId::ItemInvoked,
        EventValue::String(second_callback.clone()),
        EventPayload::String(Rc::from("Current")),
    ));
    assert_eq!(runtime.dispatch_native_events().unwrap(), 1);
    assert_eq!(&*second.borrow(), &[Rc::from("Current")]);

    let mutations = runtime
        .update(TreeView::new().nodes([TreeNode::new("root", "Root")]))
        .unwrap();
    assert!(mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::SetProperties { object, clear, .. }
            if *object == tree && clear.contains(&PropertyId::SelectionMode)
    )));
    runtime.adapter_mut().queue_event(EventDispatch::new(
        tree,
        EventId::ItemInvoked,
        EventValue::String(second_callback),
        EventPayload::String(Rc::from("Removed")),
    ));
    assert_eq!(runtime.dispatch_native_events().unwrap(), 0);
}

#[test]
fn inspectable_string_item_events_deliver_typed_values() {
    let breadcrumb_values = Rc::new(RefCell::new(Vec::new()));
    let breadcrumb_callback = {
        let values = Rc::clone(&breadcrumb_values);
        Callback::new(move |value| values.borrow_mut().push(value))
    };
    let mut breadcrumb = Runtime::new(RecordingAdapter::default());
    breadcrumb
        .update(
            BreadcrumbBar::new()
                .items_source(["Root", "Current"])
                .on_item_clicked(breadcrumb_callback.clone()),
        )
        .unwrap();
    let object = breadcrumb.graph().root().unwrap();
    breadcrumb.adapter_mut().queue_event(EventDispatch::new(
        object,
        EventId::ItemClicked,
        EventValue::String(breadcrumb_callback),
        EventPayload::String(Rc::from("Current")),
    ));
    assert_eq!(breadcrumb.dispatch_native_events().unwrap(), 1);
    assert_eq!(&*breadcrumb_values.borrow(), &[Rc::from("Current")]);

    let suggestion_values = Rc::new(RefCell::new(Vec::new()));
    let suggestion_callback = {
        let values = Rc::clone(&suggestion_values);
        Callback::new(move |value| values.borrow_mut().push(value))
    };
    let mut suggestions = Runtime::new(RecordingAdapter::default());
    suggestions
        .update(
            AutoSuggestBox::new()
                .items_source(["First", "Second"])
                .on_suggestion_chosen(suggestion_callback.clone()),
        )
        .unwrap();
    let object = suggestions.graph().root().unwrap();
    suggestions.adapter_mut().queue_event(EventDispatch::new(
        object,
        EventId::SuggestionChosen,
        EventValue::String(suggestion_callback),
        EventPayload::String(Rc::from("Second")),
    ));
    assert_eq!(suggestions.dispatch_native_events().unwrap(), 1);
    assert_eq!(&*suggestion_values.borrow(), &[Rc::from("Second")]);
}

#[test]
fn auto_suggest_box_query_submitted_delivers_query_text() {
    let values = Rc::new(RefCell::new(Vec::new()));
    let callback = {
        let values = Rc::clone(&values);
        Callback::new(move |value| values.borrow_mut().push(value))
    };
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(AutoSuggestBox::new().on_query_submitted(callback.clone()))
        .unwrap();
    let object = runtime.graph().root().unwrap();
    assert!(
        event_contracts(ObjectType::AutoSuggestBox).contains(&EventContract {
            id: EventId::QuerySubmitted,
            value: ValueType::String,
        })
    );
    runtime.adapter_mut().queue_event(EventDispatch::new(
        object,
        EventId::QuerySubmitted,
        EventValue::String(callback),
        EventPayload::String(Rc::from("inbox")),
    ));
    assert_eq!(runtime.dispatch_native_events().unwrap(), 1);
    assert_eq!(&*values.borrow(), &[Rc::from("inbox")]);
}

#[test]
fn tab_view_events_deliver_item_tags() {
    let closed_values = Rc::new(RefCell::new(Vec::new()));
    let closed_callback = {
        let values = Rc::clone(&closed_values);
        Callback::new(move |value| values.borrow_mut().push(value))
    };
    let reordered_values = Rc::new(RefCell::new(Vec::new()));
    let reordered_callback = {
        let values = Rc::clone(&reordered_values);
        Callback::new(move |value| values.borrow_mut().push(value))
    };
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(
            TabView::new()
                .tab_items([
                    TabViewItem::new().tag("first").into(),
                    TabViewItem::new().tag("second").into(),
                ])
                .on_close_requested(closed_callback.clone())
                .on_reordered(reordered_callback.clone()),
        )
        .unwrap();
    let object = runtime.graph().root().unwrap();
    runtime.adapter_mut().queue_event(EventDispatch::new(
        object,
        EventId::TabCloseRequested,
        EventValue::String(closed_callback),
        EventPayload::String(Rc::from("second")),
    ));
    runtime.adapter_mut().queue_event(EventDispatch::new(
        object,
        EventId::TabItemsChanged,
        EventValue::StringList(reordered_callback),
        EventPayload::StringList(vec!["second".into(), "first".into()]),
    ));
    assert_eq!(runtime.dispatch_native_events().unwrap(), 2);
    assert_eq!(&*closed_values.borrow(), &[Rc::from("second")]);
    assert_eq!(
        &*reordered_values.borrow(),
        &[vec!["second".to_string(), "first".to_string()]]
    );
}
