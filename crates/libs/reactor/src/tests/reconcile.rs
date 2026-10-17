use super::*;

#[test]
fn initial_mount_reserves_the_validated_object_count() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(Grid::new().keyed_children((0usize..512).map(|index| keyed(index, Border::new()))))
        .unwrap();

    let memory = runtime.graph().retained_memory();
    assert_eq!(memory.slot_len, 513);
    assert_eq!(memory.slot_capacity, 513);
}

#[test]
fn disparate_controls_mount_into_the_same_retained_arena() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(
            Grid::new().keyed_children([
                keyed("border", Border::new().content(text("content"))),
                keyed(
                    "tree",
                    TreeView::new().nodes([TreeNode::new("root", "Root")
                        .content(text("label"))
                        .children([TreeNode::new("child", "Child")])]),
                ),
                keyed(
                    "list",
                    ListView::new().items([
                        DataItem::new("first", "First"),
                        DataItem::new("second", "Second"),
                    ]),
                ),
            ]),
        )
        .unwrap();
    let root = runtime.graph().root().unwrap();

    assert_eq!(runtime.graph().object_count(), 10);
    assert_eq!(runtime.graph().kind(root), Some(ObjectType::Grid));
    assert_eq!(
        runtime
            .graph()
            .children(root, RelationId::Children)
            .map(<[ObjectId]>::len),
        Some(3)
    );
}

#[test]
fn stack_panel_keyed_reorder_preserves_objects_and_emits_one_generic_reorder() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(StackPanel::new().keyed_children([
            keyed("first", TextBlock::new().text("First")),
            keyed("second", TextBlock::new().text("Second")),
        ]))
        .unwrap();
    let root = runtime.graph().root().unwrap();
    let before = runtime
        .graph()
        .children(root, RelationId::Children)
        .unwrap()
        .to_vec();

    let mutations = runtime
        .update(StackPanel::new().keyed_children([
            keyed("second", TextBlock::new().text("Second")),
            keyed("first", TextBlock::new().text("Changed")),
        ]))
        .unwrap();
    let after = runtime
        .graph()
        .children(root, RelationId::Children)
        .unwrap();

    assert_eq!(after, [before[1], before[0]]);
    assert_eq!(
        mutations
            .iter()
            .filter(|mutation| matches!(mutation, Mutation::Reorder { .. }))
            .count(),
        1
    );
    assert!(
        !mutations
            .iter()
            .any(|mutation| matches!(mutation, Mutation::Create { .. } | Mutation::Destroy { .. }))
    );
    assert!(mutations
        .iter()
        .any(|mutation| matches!(mutation, Mutation::SetProperties { object, .. } if *object == before[0])));
}

#[test]
fn tab_view_keyed_reorder_preserves_tab_objects() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(TabView::new().keyed_tab_items([
            keyed("first", TabViewItem::new().header("First")),
            keyed("second", TabViewItem::new().header("Second")),
        ]))
        .unwrap();
    let root = runtime.graph().root().unwrap();
    let before = runtime
        .graph()
        .children(root, RelationId::TabItems)
        .unwrap()
        .to_vec();

    let mutations = runtime
        .update(TabView::new().keyed_tab_items([
            keyed("second", TabViewItem::new().header("Second")),
            keyed("first", TabViewItem::new().header("Changed")),
        ]))
        .unwrap();
    let after = runtime
        .graph()
        .children(root, RelationId::TabItems)
        .unwrap();

    assert_eq!(after, [before[1], before[0]]);
    assert_eq!(
        mutations
            .iter()
            .filter(|mutation| matches!(mutation, Mutation::Reorder { .. }))
            .count(),
        1
    );
    assert!(
        !mutations
            .iter()
            .any(|mutation| matches!(mutation, Mutation::Create { .. } | Mutation::Destroy { .. }))
    );
}

#[test]
fn keyed_insert_and_remove_produce_the_declared_order() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(Grid::new().keyed_children([
            keyed("first", TextBlock::new().text("First")),
            keyed("second", TextBlock::new().text("Second")),
        ]))
        .unwrap();
    let root = runtime.graph().root().unwrap();
    let second = runtime
        .graph()
        .children(root, RelationId::Children)
        .unwrap()[1];

    runtime
        .update(Grid::new().keyed_children([
            keyed("second", TextBlock::new().text("Second")),
            keyed("third", TextBlock::new().text("Third")),
        ]))
        .unwrap();

    let children = runtime
        .graph()
        .children(root, RelationId::Children)
        .unwrap();
    assert_eq!(children[0], second);
    assert_eq!(
        runtime.graph().kind(children[1]),
        Some(ObjectType::TextBlock)
    );
    assert_eq!(
        runtime.adapter().children(root, RelationId::Children),
        Some(children)
    );
}

#[test]
fn hierarchical_objects_use_the_same_keyed_relation_reconciler() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(
            TreeView::new().nodes([TreeNode::new("root", "Root")
                .content(TextBlock::new().text("Content"))
                .children([
                    TreeNode::new("first", "First"),
                    TreeNode::new("second", "Second"),
                ])]),
        )
        .unwrap();
    let tree = runtime.graph().root().unwrap();
    let root = runtime.graph().children(tree, RelationId::Roots).unwrap()[0];
    let content = runtime.graph().child(root, RelationId::Content).unwrap();
    let children = runtime
        .graph()
        .children(root, RelationId::Children)
        .unwrap()
        .to_vec();

    let mutations = runtime
        .update(
            TreeView::new().nodes([TreeNode::new("root", "Renamed")
                .content(TextBlock::new().text("Updated"))
                .children([
                    TreeNode::new("second", "Second"),
                    TreeNode::new("first", "First"),
                ])]),
        )
        .unwrap();

    assert_eq!(
        runtime.graph().children(root, RelationId::Children),
        Some([children[1], children[0]].as_slice())
    );
    assert_eq!(
        runtime.graph().child(root, RelationId::Content),
        Some(content)
    );
    assert_eq!(
        mutations
            .iter()
            .filter(|mutation| matches!(mutation, Mutation::Reorder { .. }))
            .count(),
        1
    );
    assert!(
        !mutations
            .iter()
            .any(|mutation| matches!(mutation, Mutation::Create { .. } | Mutation::Destroy { .. }))
    );
}

#[test]
fn container_generated_data_uses_the_same_keyed_relation_reconciler() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(ListView::new().items([
            DataItem::new("first", "First"),
            DataItem::new("second", "Second"),
        ]))
        .unwrap();
    let list = runtime.graph().root().unwrap();
    let before = runtime
        .graph()
        .children(list, RelationId::Items)
        .unwrap()
        .to_vec();

    let mutations = runtime
        .update(ListView::new().items([
            DataItem::new("second", "Second"),
            DataItem::new("first", "Changed"),
        ]))
        .unwrap();

    assert_eq!(
        runtime.graph().children(list, RelationId::Items),
        Some([before[1], before[0]].as_slice())
    );
    assert_eq!(
        mutations
            .iter()
            .filter(|mutation| matches!(mutation, Mutation::Reorder { .. }))
            .count(),
        1
    );
}

#[test]
fn owned_content_replacement_uses_generic_relation_mutations() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(Border::new().content(TextBlock::new().text("Text")))
        .unwrap();
    let border = runtime.graph().root().unwrap();
    let previous = runtime.graph().child(border, RelationId::Content).unwrap();

    let mutations = runtime.update(Border::new().content(Grid::new())).unwrap();
    let next = runtime.graph().child(border, RelationId::Content).unwrap();

    assert_ne!(next, previous);
    assert_eq!(runtime.graph().kind(next), Some(ObjectType::Grid));
    assert!(matches!(
        mutations.as_slice(),
        [
            Mutation::Detach { .. },
            Mutation::Destroy { .. },
            Mutation::Create { .. },
            Mutation::Attach { .. }
        ]
    ));
}

#[test]
fn positional_children_reconcile_by_index_without_keys() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(
            StackPanel::new()
                .children([TextBlock::new().text("First").into(), Border::new().into()]),
        )
        .unwrap();
    let panel = runtime.graph().root().unwrap();
    let before = runtime
        .graph()
        .children(panel, RelationId::Children)
        .unwrap()
        .to_vec();

    runtime
        .update(StackPanel::new().children([
            TextBlock::new().text("Changed").into(),
            Grid::new().into(),
            TextBlock::new().text("Third").into(),
        ]))
        .unwrap();

    let after = runtime
        .graph()
        .children(panel, RelationId::Children)
        .unwrap();
    assert_eq!(after[0], before[0]);
    assert_ne!(after[1], before[1]);
    assert_eq!(runtime.graph().kind(after[1]), Some(ObjectType::Grid));
    assert_eq!(
        runtime.adapter().children(panel, RelationId::Children),
        Some(after)
    );
}

#[test]
fn property_updates_only_emit_changed_values() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(TreeView::new().nodes([TreeNode::new("root", "Before").expanded(true)]))
        .unwrap();

    let mutations = runtime
        .update(TreeView::new().nodes([TreeNode::new("root", "After").expanded(true)]))
        .unwrap();

    let (set, clear) = mutations
        .iter()
        .find_map(|mutation| match mutation {
            Mutation::SetProperties { set, clear, .. } => Some((set, clear)),
            _ => None,
        })
        .unwrap();
    assert_eq!(set.len(), 1);
    assert_eq!(set[0].id, PropertyId::Text);
    assert!(clear.is_empty());
}

#[test]
fn visual_transitions_are_retained_stable_and_removed_when_omitted() {
    let transitions = || [ThemeTransition::Reposition];
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(
            Border::new()
                .margin(Thickness::new(10.0, 20.0, 0.0, 0.0))
                .transitions(transitions()),
        )
        .unwrap();
    let border = runtime.graph().root().unwrap();
    assert!(matches!(
        runtime
            .graph()
            .properties(border)
            .unwrap()
            .iter()
            .find(|property| property.id == PropertyId::Transitions),
        Some(Property {
            value: PropertyValue::ThemeTransitions(value),
            ..
        }) if value.as_ref() == transitions().as_slice()
    ));

    assert!(
        runtime
            .update(
                Border::new()
                    .margin(Thickness::new(10.0, 20.0, 0.0, 0.0))
                    .transitions(transitions()),
            )
            .unwrap()
            .is_empty()
    );

    let mutations = runtime
        .update(Border::new().margin(Thickness::new(30.0, 40.0, 0.0, 0.0)))
        .unwrap();
    assert_eq!(
        mutations,
        [Mutation::SetProperties {
            object: border,
            set: Rc::from([Property {
                id: PropertyId::Margin,
                value: PropertyValue::Thickness(Thickness::new(30.0, 40.0, 0.0, 0.0)),
            }]),
            clear: Rc::from([PropertyId::Transitions]),
        }]
    );
}

#[test]
fn keyed_visual_moves_preserve_identity_and_only_update_changed_state() {
    let first_callback = Callback::new(|_: PointerEventInfo| {});
    let second_callback = Callback::new(|_: PointerEventInfo| {});
    let card = |key: &str, left: f64, background: Color, callback: Callback<PointerEventInfo>| {
        keyed(
            key,
            Border::new()
                .margin(Thickness::new(left, 0.0, 0.0, 0.0))
                .background(background)
                .transitions([ThemeTransition::Reposition])
                .on_pointer_released(callback),
        )
    };
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(Grid::new().keyed_children([
            card("first", 0.0, Color::rgb(255, 255, 255), first_callback),
            card(
                "second",
                100.0,
                Color::rgb(255, 255, 255),
                second_callback.clone(),
            ),
        ]))
        .unwrap();
    let grid = runtime.graph().root().unwrap();
    let before = runtime
        .graph()
        .children(grid, RelationId::Children)
        .unwrap()
        .to_vec();
    let replacement_callback = Callback::new(|_: PointerEventInfo| {});

    let mutations = runtime
        .update(Grid::new().keyed_children([
            card("second", 100.0, Color::rgb(255, 255, 255), second_callback),
            card(
                "first",
                200.0,
                Color::rgb(255, 220, 220),
                replacement_callback,
            ),
        ]))
        .unwrap();
    let after = runtime
        .graph()
        .children(grid, RelationId::Children)
        .unwrap();

    assert_eq!(after, [before[1], before[0]]);
    assert_eq!(
        mutations
            .iter()
            .filter(|mutation| matches!(mutation, Mutation::Reorder { .. }))
            .count(),
        1
    );
    assert!(
        !mutations
            .iter()
            .any(|mutation| matches!(mutation, Mutation::Create { .. } | Mutation::Destroy { .. }))
    );
    assert!(mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::SetProperties { object, set, clear }
            if *object == before[0]
                && clear.is_empty()
                && set.len() == 2
                && set.iter().any(|property| property.id == PropertyId::Margin)
                && set.iter().any(|property| property.id == PropertyId::Background)
    )));
    assert!(mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::SetEvents { object, set, clear }
            if *object == before[0]
                && clear.is_empty()
                && set.len() == 1
                && set[0].id == EventId::PointerReleased
    )));
    assert!(!mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::SetProperties { set, .. }
            if set.iter().any(|property| property.id == PropertyId::Transitions)
    )));
}

#[test]
fn subtree_update_reconciles_only_the_target_object() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(Grid::new().keyed_children([
            keyed("first", TextBlock::new().text("First")),
            keyed("second", TextBlock::new().text("Second")),
        ]))
        .unwrap();
    let root = runtime.graph().root().unwrap();
    let children = runtime
        .graph()
        .children(root, RelationId::Children)
        .unwrap();
    let first = children[0];
    let second = children[1];

    let mutations = runtime
        .update_subtree(first, TextBlock::new().text("Changed"))
        .unwrap();

    assert_eq!(
        mutations,
        vec![Mutation::SetProperties {
            object: first,
            set: Rc::from([Property {
                id: PropertyId::Text,
                value: PropertyValue::String(Rc::from("Changed")),
            }]),
            clear: Rc::from([]),
        }]
    );
    assert_eq!(
        runtime
            .graph()
            .children(root, RelationId::Children)
            .unwrap(),
        [first, second]
    );
}

#[test]
fn subtree_root_type_replacement_preserves_identity() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(Grid::new().keyed_children([keyed("child", TextBlock::new().text("Text"))]))
        .unwrap();
    let root = runtime.graph().root().unwrap();
    let child = runtime
        .graph()
        .children(root, RelationId::Children)
        .unwrap()[0];

    let mutations = runtime.update_subtree(child, Border::new()).unwrap();

    assert_eq!(
        mutations,
        vec![Mutation::Replace {
            object: child,
            kind: ObjectType::Border,
        }]
    );
    assert_eq!(runtime.graph().kind(child), Some(ObjectType::Border));
    assert_eq!(
        runtime
            .graph()
            .children(root, RelationId::Children)
            .unwrap(),
        [child]
    );
}

#[test]
fn subtree_root_type_replacement_rejects_incompatible_relations() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(TreeView::new().nodes([TreeNode::new("node", "Node")]))
        .unwrap();
    let root = runtime.graph().root().unwrap();
    let node = runtime.graph().children(root, RelationId::Roots).unwrap()[0];

    assert_eq!(
        runtime.update_subtree(node, TextBlock::new().text("Invalid")),
        Err(UpdateError::Graph(GraphError::InvalidChildCategory(
            RelationId::Roots
        )))
    );
    assert_eq!(runtime.graph().kind(node), Some(ObjectType::TreeNode));
}

#[test]
fn recording_adapter_rejects_invalid_replacement_batches() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime.update(TextBlock::new().text("Root")).unwrap();
    let root = runtime.graph().root().unwrap();
    assert_eq!(
        runtime.adapter_mut().apply(&[Mutation::Replace {
            object: root,
            kind: ObjectType::Border,
        }]),
        Err(AdapterError::InvalidReplacement(root))
    );

    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(TreeView::new().nodes([TreeNode::new("node", "Node")]))
        .unwrap();
    let tree = runtime.graph().root().unwrap();
    let node = runtime.graph().children(tree, RelationId::Roots).unwrap()[0];
    assert_eq!(
        runtime.adapter_mut().apply(&[Mutation::Replace {
            object: node,
            kind: ObjectType::TextBlock,
        }]),
        Err(AdapterError::InvalidReplacement(node))
    );
}

#[test]
fn replacement_discards_observations_and_events_from_the_old_native_object() {
    let calls = Rc::new(Cell::new(0));
    let callback_calls = Rc::clone(&calls);
    let callback = Callback::new(move |_| callback_calls.set(callback_calls.get() + 1));
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(Grid::new().keyed_children([keyed(
            "child",
            TextBox::new().text("Old").on_text_changed(callback.clone()),
        )]))
        .unwrap();
    let root = runtime.graph().root().unwrap();
    let child = runtime
        .graph()
        .children(root, RelationId::Children)
        .unwrap()[0];
    for value in ["First", "Stale"] {
        runtime.adapter_mut().queue_native_event(
            Some(Observation::SetProperty {
                object: child,
                property: Property {
                    id: PropertyId::Text,
                    value: PropertyValue::String(Rc::from(value)),
                },
            }),
            Some(EventDispatch::new(
                child,
                EventId::TextChanged,
                EventValue::String(callback.clone()),
                EventPayload::String(Rc::from(value)),
            )),
        );
    }
    let mut active = runtime.next_native_event().unwrap().unwrap();
    active.invoke();
    runtime.update_subtree(child, Border::new()).unwrap();
    drop(active);

    assert_eq!(runtime.dispatch_native_events().unwrap(), 0);
    assert_eq!(calls.get(), 1);
    assert_eq!(runtime.graph().kind(child), Some(ObjectType::Border));
    assert!(runtime.graph().properties(child).unwrap().is_empty());
}

#[test]
fn targeted_child_removal_preserves_sibling_identity() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(Grid::new().keyed_children([
            keyed("first", TextBlock::new().text("First")),
            keyed("second", TextBlock::new().text("Second")),
        ]))
        .unwrap();
    let root = runtime.graph().root().unwrap();
    let children = runtime
        .graph()
        .children(root, RelationId::Children)
        .unwrap();
    let first = children[0];
    let second = children[1];

    let mutations = runtime
        .remove_child(root, RelationId::Children, first)
        .unwrap();

    assert_eq!(
        mutations,
        vec![
            Mutation::Remove {
                parent: root,
                relation: RelationId::Children,
                child: first,
                index: 0,
            },
            Mutation::Destroy { object: first },
        ]
    );
    assert_eq!(
        runtime
            .graph()
            .children(root, RelationId::Children)
            .unwrap(),
        [second]
    );
    assert_eq!(runtime.graph().kind(first), None);
}

#[test]
fn no_change_produces_no_mutations() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(Border::new().content(TextBlock::new().text("Text")))
        .unwrap();

    assert!(
        runtime
            .update(Border::new().content(TextBlock::new().text("Text")))
            .unwrap()
            .is_empty()
    );
}

fn keyed_list(keys: &[String], selected: Option<usize>) -> ListView {
    ListView::new().selected_index(selected).items(
        keys.iter()
            .map(|key| DataItem::new(key.clone(), key).content(text(key.as_str()))),
    )
}

fn list_keys(prefix: &str, range: std::ops::Range<usize>) -> Vec<String> {
    range.map(|index| format!("{prefix}{index}")).collect()
}

/// The list's `Insert`, `Remove`, `Reorder` and `ReplaceItems` mutations, counted.
fn list_item_mutations(mutations: &UpdateStats, list: ObjectId) -> [usize; 4] {
    let mut counts = [0; 4];
    for mutation in mutations.iter() {
        match mutation {
            Mutation::Insert { parent, .. } if *parent == list => counts[0] += 1,
            Mutation::Remove { parent, .. } if *parent == list => counts[1] += 1,
            Mutation::Reorder { parent, .. } if *parent == list => counts[2] += 1,
            Mutation::ReplaceItems { parent, .. } if *parent == list => counts[3] += 1,
            _ => {}
        }
    }
    counts
}

#[test]
fn list_view_replaces_its_items_at_once_when_most_keys_change() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(keyed_list(&list_keys("m", 0..500), Some(3)))
        .unwrap();
    let list = runtime.graph().root().unwrap();
    let before = runtime
        .graph()
        .children(list, RelationId::Items)
        .unwrap()
        .to_vec();

    let mutations = runtime
        .update(keyed_list(&list_keys("n", 0..12), Some(0)))
        .unwrap();
    let after = runtime
        .graph()
        .children(list, RelationId::Items)
        .unwrap()
        .to_vec();

    assert_eq!(list_item_mutations(&mutations, list), [0, 0, 0, 1]);
    let (replace_at, removed, children) = mutations
        .iter()
        .enumerate()
        .find_map(|(at, mutation)| match mutation {
            Mutation::ReplaceItems {
                parent,
                relation: RelationId::Items,
                removed,
                children,
            } if *parent == list => Some((at, removed.clone(), children.clone())),
            _ => None,
        })
        .unwrap();
    assert_eq!(children, after);
    let mut removed_sorted = removed;
    removed_sorted.sort_unstable_by_key(|object| (object.index(), object.generation()));
    let mut before_sorted = before.clone();
    before_sorted.sort_unstable_by_key(|object| (object.index(), object.generation()));
    assert_eq!(removed_sorted, before_sorted);
    // New items are created before they are placed; old ones are destroyed after they leave.
    for (at, mutation) in mutations.iter().enumerate() {
        match mutation {
            Mutation::Create { object, .. } if after.contains(object) => {
                assert!(at < replace_at);
            }
            Mutation::Destroy { object } if before.contains(object) => assert!(at > replace_at),
            _ => {}
        }
    }
    // SelectedIndex is set again after the items (df9100551).
    let index_set = mutations
        .iter()
        .position(|mutation| {
            matches!(mutation, Mutation::SetProperties { object, set, .. }
                if *object == list && set.iter().any(|p| p.id == PropertyId::SelectedIndex))
        })
        .unwrap();
    assert!(index_set > replace_at);
}

#[test]
fn list_view_replace_keeps_the_objects_and_content_of_kept_keys() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(keyed_list(&list_keys("m", 0..40), None))
        .unwrap();
    let list = runtime.graph().root().unwrap();
    let before = runtime
        .graph()
        .children(list, RelationId::Items)
        .unwrap()
        .to_vec();
    realize_item_content(&mut runtime, &[before[5], before[30]]);
    let kept_content = [5, 30].map(|index| {
        runtime
            .graph()
            .child(before[index], RelationId::Content)
            .unwrap()
    });

    let mut keys = list_keys("n", 0..38);
    keys.insert(0, "m30".to_string());
    keys.push("m5".to_string());
    let mutations = runtime.update(keyed_list(&keys, None)).unwrap();
    let after = runtime
        .graph()
        .children(list, RelationId::Items)
        .unwrap()
        .to_vec();

    assert_eq!(list_item_mutations(&mutations, list), [0, 0, 0, 1]);
    assert_eq!(after.len(), 40);
    assert_eq!(after[0], before[30]);
    assert_eq!(after[39], before[5]);
    assert_eq!(
        runtime.graph().child(after[0], RelationId::Content),
        Some(kept_content[1])
    );
    assert_eq!(
        runtime.graph().child(after[39], RelationId::Content),
        Some(kept_content[0])
    );
    let kept = [before[5], before[30], kept_content[0], kept_content[1]];
    assert!(!mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::Destroy { object } | Mutation::Detach { child: object, .. }
            if kept.contains(object)
    )));
    let removed = mutations
        .iter()
        .find_map(|mutation| match mutation {
            Mutation::ReplaceItems { removed, .. } => Some(removed.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(removed.len(), 38);
    assert!(!removed.contains(&before[5]) && !removed.contains(&before[30]));
    // The new items' content waits for their containers.
    assert!(!mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::Create {
            kind: ObjectType::TextBlock,
            ..
        }
    )));
}

/// Mounts the content of `items`, as a ListView asks when it prepares their containers.
fn realize_item_content(runtime: &mut Runtime<RecordingAdapter>, items: &[ObjectId]) {
    for item in items {
        runtime.queue_realization(RealizationRequest::Content { item: *item });
    }
    runtime.dispatch_native_events().unwrap();
}

fn list_items(runtime: &Runtime<RecordingAdapter>) -> Vec<ObjectId> {
    let list = runtime.graph().root().unwrap();
    runtime
        .graph()
        .children(list, RelationId::Items)
        .unwrap()
        .to_vec()
}

#[test]
fn list_view_mounts_item_content_only_when_its_container_is_realized() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    let mutations = runtime
        .update(keyed_list(&list_keys("m", 0..500), Some(0)))
        .unwrap();
    let items = list_items(&runtime);

    // The list and its 500 items, but none of the items' content.
    assert_eq!(runtime.adapter().object_count(), 501);
    assert!(!mutations.iter().any(|mutation| matches!(
        mutation,
        Mutation::Create {
            kind: ObjectType::TextBlock,
            ..
        }
    )));
    assert_eq!(
        mutations
            .iter()
            .filter(|mutation| matches!(mutation, Mutation::DeferContent { deferred: true, .. }))
            .count(),
        500
    );
    assert!(
        items
            .iter()
            .all(|item| runtime.graph().child(*item, RelationId::Content).is_none())
    );

    realize_item_content(&mut runtime, &items[..3]);

    assert_eq!(runtime.adapter().object_count(), 504);
    for (index, item) in items.iter().enumerate() {
        assert_eq!(
            runtime.graph().child(*item, RelationId::Content).is_some(),
            index < 3,
            "item {index}"
        );
    }
    // An unchanged list stays unchanged, realized or not.
    assert!(
        runtime
            .update(keyed_list(&list_keys("m", 0..500), Some(0)))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn list_view_item_content_changes_in_place_once_realized_and_is_only_stored_before() {
    let item = |key: &str, text: &str| DataItem::new(key, key).content(TextBlock::new().text(text));
    let list = |text: &str| ListView::new().items([item("a", text), item("b", text)]);
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime.update(list("one")).unwrap();
    let items = list_items(&runtime);
    realize_item_content(&mut runtime, &items[..1]);
    let content = runtime
        .graph()
        .child(items[0], RelationId::Content)
        .unwrap();

    let mutations = runtime.update(list("two")).unwrap();

    // The realized item's text block changes; the other item only keeps the new declaration.
    assert_eq!(mutations.len(), 1);
    assert!(matches!(
        mutations.iter().next(),
        Some(Mutation::SetProperties { object, .. }) if *object == content
    ));
    assert_eq!(
        runtime.graph().child(items[0], RelationId::Content),
        Some(content)
    );
    assert_eq!(runtime.graph().child(items[1], RelationId::Content), None);

    runtime.record_batches(true);
    realize_item_content(&mut runtime, &items[1..]);
    let batch = runtime.adapter().batches().last().unwrap();
    assert!(batch.iter().any(|mutation| matches!(
        mutation,
        Mutation::SetProperties { set, .. }
            if set.iter().any(|property| property.value == PropertyValue::String(Rc::from("two")))
    )));
    assert!(batch.iter().any(|mutation| matches!(
        mutation,
        Mutation::Attach { parent, relation: RelationId::Content, .. } if *parent == items[1]
    )));
}

#[test]
fn list_view_ignores_content_requests_for_realized_or_removed_items() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(keyed_list(&list_keys("m", 0..3), None))
        .unwrap();
    let items = list_items(&runtime);
    realize_item_content(&mut runtime, &items[..1]);
    let content = runtime.graph().child(items[0], RelationId::Content);
    assert_eq!(runtime.adapter().object_count(), 5);

    // Prepared again, as a recycled container is.
    realize_item_content(&mut runtime, &items[..1]);
    assert_eq!(runtime.adapter().object_count(), 5);
    assert_eq!(
        runtime.graph().child(items[0], RelationId::Content),
        content
    );

    // Removing items whose content was never mounted detaches nothing.
    let mutations = runtime
        .update(keyed_list(&list_keys("m", 0..1), None))
        .unwrap();
    assert!(
        !mutations
            .iter()
            .any(|mutation| matches!(mutation, Mutation::Detach { .. }))
    );
    assert_eq!(runtime.adapter().object_count(), 3);
    realize_item_content(&mut runtime, &items[1..]);
    assert_eq!(runtime.adapter().object_count(), 3);
}

#[test]
fn list_view_item_content_dropped_before_realization_restores_the_item_text() {
    let with = || ListView::new().items([DataItem::new("a", "a").content(text("a"))]);
    let without = || ListView::new().items([DataItem::new("a", "a")]);
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime.update(with()).unwrap();
    let item = list_items(&runtime)[0];

    let mutations = runtime.update(without()).unwrap();
    assert_eq!(
        mutations.as_slice(),
        [Mutation::DeferContent {
            item,
            deferred: false
        }]
    );

    // Content added to a mounted item is mounted at once: its container may be showing it.
    runtime.update(with()).unwrap();
    assert!(runtime.graph().child(item, RelationId::Content).is_some());
}

#[test]
fn list_view_mounts_item_content_holding_references_at_once() {
    let reference: ElementRef = ElementRef::default();
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(
            ListView::new()
                .items([DataItem::new("a", "a").content(Button::new().element_ref(&reference))]),
        )
        .unwrap();
    let item = list_items(&runtime)[0];
    let content = runtime.graph().child(item, RelationId::Content);
    assert!(content.is_some());
    assert_eq!(reference.get(), content);
}

#[test]
fn list_view_small_edit_still_inserts_and_removes_single_items() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(keyed_list(&list_keys("m", 0..500), Some(3)))
        .unwrap();
    let list = runtime.graph().root().unwrap();

    let mut keys = list_keys("m", 0..500);
    keys.remove(7);
    keys.insert(0, "new".to_string());
    let mutations = runtime.update(keyed_list(&keys, Some(4))).unwrap();

    assert_eq!(list_item_mutations(&mutations, list), [1, 1, 0, 0]);
}

#[test]
fn list_view_reorder_of_mostly_the_same_keys_is_unchanged() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(keyed_list(&list_keys("m", 0..40), None))
        .unwrap();
    let list = runtime.graph().root().unwrap();
    let before = runtime
        .graph()
        .children(list, RelationId::Items)
        .unwrap()
        .to_vec();

    let mut keys = list_keys("m", 0..40);
    keys.swap(0, 39);
    keys.swap(10, 20);
    keys[25] = "new".to_string();
    let mutations = runtime.update(keyed_list(&keys, None)).unwrap();
    let after = runtime
        .graph()
        .children(list, RelationId::Items)
        .unwrap()
        .to_vec();

    assert_eq!(list_item_mutations(&mutations, list), [1, 1, 1, 0]);
    assert_eq!(after[0], before[39]);
    assert_eq!(after[39], before[0]);
    assert_eq!(after[10], before[20]);
}

#[test]
fn panel_keyed_children_still_change_one_at_a_time_when_most_keys_change() {
    let mut runtime = Runtime::new(RecordingAdapter::default());
    runtime
        .update(StackPanel::new().keyed_children(
            (0..40).map(|index| keyed(format!("m{index}"), TextBlock::new().text("Old"))),
        ))
        .unwrap();
    let root = runtime.graph().root().unwrap();

    let mutations = runtime
        .update(StackPanel::new().keyed_children(
            (0..12).map(|index| keyed(format!("n{index}"), TextBlock::new().text("New"))),
        ))
        .unwrap();

    assert_eq!(list_item_mutations(&mutations, root), [12, 40, 0, 0]);
}
