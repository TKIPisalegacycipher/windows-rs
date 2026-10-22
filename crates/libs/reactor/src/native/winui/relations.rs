/// The ListView items whose realized containers `with_list_items_disconnected` disconnects.
enum ListItems<'a> {
    /// These items.
    These(&'a [IInspectable]),
    /// The items with these COM identities (`com_identity`).
    Matching(&'a HashSet<usize>),
}

/// Whether a reactor write that left a `kind` selector with `has_selection`, after it had
/// `had_selection`, gets a "nothing selected" event only after the write returns.
///
/// NavigationView raises a change to "nothing selected" late: `ChangeSelection` finds no container
/// for the null item and leaves the event to `CompletePendingSelectionChange`, queued at Low
/// priority for when the UI thread falls idle. A change to an item is raised inside the write, and
/// other selectors raise both inside it.
fn defers_deselect_echo(kind: ObjectType, had_selection: bool, has_selection: bool) -> bool {
    kind == ObjectType::NavigationView && had_selection && !has_selection
}

/// The selection feedback of one reactor write to a selector (`begin_selection_feedback`).
#[derive(Clone, Copy)]
struct SelectionFeedback {
    event: EventId,
    /// The selector may raise its deselect late and had a selection before the write.
    had_selection: bool,
}

impl WinUiAdapter {
    fn attach(
        &mut self,
        parent: ObjectId,
        relation: RelationId,
        child: ObjectId,
    ) -> Result<(), WinUiError> {
        if self.owners.contains_key(&child) {
            return Err(WinUiError::StillOwned(child));
        }
        if matches!(
            self.kind(child)?,
            ObjectType::ToolTip | ObjectType::ContentDialog
        ) {
            return Err(WinUiError::InvalidObject(child));
        }
        let child_element = self.ui_element(child)?;
        let data_content_key = self.data_content_key.clone();
        let parent_id = parent;
        match (self.handle(parent)?, relation) {
            (Handle::Generated(value), relation) => {
                value
                    .set_content(relation, Some(&child_element))
                    .ok_or(WinUiError::InvalidRelation(parent, relation))??;
            }
            (Handle::TreeNode(parent), RelationId::Content) => {
                parent.value.SetContent(&child_element)?;
                parent.content = Some(child);
            }
            (Handle::TextBox(parent), RelationId::Header) => {
                parent.value.SetHeader(&child_element)?;
            }
            (Handle::Data(parent), RelationId::Content) => {
                let properties =
                    parent.cast::<IMap<HSTRING, IInspectable>>()?;
                let content: IInspectable = child_element.cast()?;
                properties.Insert(&data_content_key, &content)?;
                self.deferred_content.borrow_mut().end(parent_id);
            }
            _ => return Err(WinUiError::InvalidRelation(parent, relation)),
        }
        self.owners.insert(child, (parent, relation));
        Ok(())
    }

    fn detach(
        &mut self,
        parent: ObjectId,
        relation: RelationId,
        child: ObjectId,
    ) -> Result<(), WinUiError> {
        if self.owners.get(&child) != Some(&(parent, relation)) {
            return Err(WinUiError::ChildNotFound(child));
        }
        let data_content_key = self.data_content_key.clone();
        let data_text_key = self.data_text_key.clone();
        match (self.handle(parent)?, relation) {
            (Handle::Generated(value), relation) => {
                value
                    .set_content(relation, None)
                    .ok_or(WinUiError::InvalidRelation(parent, relation))??;
            }
            (Handle::TreeNode(parent), RelationId::Content) => {
                let content: IInspectable =
                    windows_reference::IReference::from(parent.text.clone()).into();
                parent.value.SetContent(&content)?;
                parent.content = None;
            }
            (Handle::TextBox(parent), RelationId::Header) => {
                parent.value.SetHeader(None::<&IInspectable>)?;
            }
            (Handle::Data(parent), RelationId::Content) => {
                let properties =
                    parent.cast::<IMap<HSTRING, IInspectable>>()?;
                let text = properties.Lookup(&data_text_key)?;
                properties.Insert(&data_content_key, &text)?;
            }
            _ => return Err(WinUiError::InvalidRelation(parent, relation)),
        }
        self.owners.remove(&child);
        Ok(())
    }

    fn has_retirements(&self, parent: ObjectId, relation: RelationId) -> bool {
        self.retirements
            .values()
            .any(|retirement| retirement.parent == parent && retirement.relation == relation)
    }

    fn active_native_index(
        &self,
        parent: ObjectId,
        relation: RelationId,
        active_index: usize,
    ) -> Result<usize, WinUiError> {
        let retired = self
            .retirements
            .iter()
            .filter_map(|(root, retirement)| {
                (retirement.parent == parent && retirement.relation == relation).then_some(*root)
            })
            .collect::<Vec<_>>();
        if retired.is_empty() {
            return Ok(active_index);
        }
        let values = self.owned_collection(parent, relation)?;
        let retired = retired
            .into_iter()
            .map(|root| self.ui_element(root))
            .collect::<Result<Vec<_>, _>>()?;
        let mut active = 0;
        for index in 0..values.size()? {
            if retired.contains(&values.get_at(index)?) {
                continue;
            }
            if active == active_index {
                return Ok(index as usize);
            }
            active += 1;
        }
        if active == active_index {
            Ok(values.size()? as usize)
        } else {
            Err(WinUiError::InvalidMutation(relation))
        }
    }

    fn insert(
        &mut self,
        parent: ObjectId,
        relation: RelationId,
        child: ObjectId,
        index: usize,
    ) -> Result<(), WinUiError> {
        if self.owners.contains_key(&child) {
            return Err(WinUiError::StillOwned(child));
        }
        if matches!(
            self.kind(child)?,
            ObjectType::ToolTip | ObjectType::ContentDialog
        ) {
            return Err(WinUiError::InvalidObject(child));
        }
        let index = self.active_native_index(parent, relation, index)?;
        let selection = selection_for_relation(self.kind(parent)?, relation);
        let selected = selection
            .map(|selection| self.read_selected_item(parent, selection))
            .transpose()?
            .flatten();
        let selection_item =
            selection.filter(|selection| self.kind(child).is_ok_and(|kind| kind == selection.item));
        let child_value = selection_item
            .map(|_| {
                self.ui_element(child)
                    .and_then(|value| value.cast::<IInspectable>().map_err(Into::into))
            })
            .transpose()?;
        let child_selected = selection_item
            .map(|selection| self.selection_item_is_selected(child, selection))
            .transpose()?
            .unwrap_or(false);
        let feedback = self.begin_selection_feedback(parent, relation, selection)?;
        let result = (|| {
            match relation_contract(self.kind(parent)?, relation)?.realization {
                Realization::Owned => {
                    self.owned_collection(parent, relation)?
                        .insert_at(index32(index)?, &self.ui_element(child)?)?;
                }
                Realization::Structural => {
                    self.tree_nodes(parent)?
                        .InsertAt(index32(index)?, &self.tree_node(child)?)?;
                }
                Realization::Container => {
                    self.items(parent)?
                        .InsertAt(index32(index)?, &self.inspectable(child)?)?;
                }
            }
            self.owners.insert(child, (parent, relation));
            if let (Some(selection), Some(value)) = (selection_item, child_value.as_ref()) {
                self.event_queue
                    .selection_items
                    .borrow_mut()
                    .push(NativeSelectionItem {
                        owner: parent,
                        object: child,
                        value: value.clone(),
                    });
                if child_selected {
                    self.write_selected_item(parent, selection, Some(value))?;
                } else if let Some(selected) = selected.as_ref() {
                    self.write_selected_item(parent, selection, Some(selected))?;
                }
            }
            Ok(())
        })();
        self.finish_selection_feedback(parent, feedback);
        result
    }

    fn remove(
        &mut self,
        parent: ObjectId,
        relation: RelationId,
        child: ObjectId,
        index: usize,
    ) -> Result<(), WinUiError> {
        if self.owners.get(&child) != Some(&(parent, relation)) {
            return Err(WinUiError::ChildNotFound(child));
        }
        let index = self.active_native_index(parent, relation, index)?;
        let selection = selection_for_relation(self.kind(parent)?, relation);
        let selected = selection
            .map(|selection| self.read_selected_item(parent, selection))
            .transpose()?
            .flatten();
        let selection_item =
            selection.filter(|selection| self.kind(child).is_ok_and(|kind| kind == selection.item));
        let child_value = selection_item
            .map(|_| {
                self.ui_element(child)
                    .and_then(|value| value.cast::<IInspectable>().map_err(Into::into))
            })
            .transpose()?;
        let feedback = self.begin_selection_feedback(parent, relation, selection)?;
        let result = (|| {
            match relation_contract(self.kind(parent)?, relation)?.realization {
                Realization::Owned => {
                    let values = self.owned_collection(parent, relation)?;
                    if values.get_at(index32(index)?)? != self.ui_element(child)? {
                        return Err(WinUiError::ChildNotFound(child));
                    }
                    values.remove_at(index32(index)?)?;
                }
                Realization::Structural => {
                    let values = self.tree_nodes(parent)?;
                    if values.GetAt(index32(index)?)? != self.tree_node(child)? {
                        return Err(WinUiError::ChildNotFound(child));
                    }
                    values.RemoveAt(index32(index)?)?;
                }
                Realization::Container => {
                    let values = self.items(parent)?;
                    let item = self.inspectable(child)?;
                    if values.GetAt(index32(index)?)? != item {
                        return Err(WinUiError::ChildNotFound(child));
                    }
                    self.with_list_items_disconnected(parent, ListItems::These(&[item]), || {
                        Ok(values.RemoveAt(index32(index)?)?)
                    })?;
                }
            }
            self.owners.remove(&child);
            if let Some(selection) = selection {
                self.event_queue
                    .selection_items
                    .borrow_mut()
                    .retain(|item| item.object != child);
                if let Some(selected) = selected.as_ref()
                    && child_value.as_ref() != Some(selected)
                {
                    self.write_selected_item(parent, selection, Some(selected))?;
                }
            }
            Ok(())
        })();
        self.finish_selection_feedback(parent, feedback);
        result
    }

    fn reorder(
        &mut self,
        parent: ObjectId,
        relation: RelationId,
        moves: &[Move],
        children: &[ObjectId],
    ) -> Result<(), WinUiError> {
        let selection = selection_for_relation(self.kind(parent)?, relation);
        let selected = selection
            .map(|selection| self.read_selected_item(parent, selection))
            .transpose()?
            .flatten();
        let feedback = self.begin_selection_feedback(parent, relation, selection)?;
        let result = (|| {
            match relation_contract(self.kind(parent)?, relation)?.realization {
                Realization::Owned if !self.has_retirements(parent, relation) => {
                    // Without retiring children, native order matches active order, so the
                    // reconciler's moves apply directly and unmoved children stay in place.
                    let values = self.owned_collection(parent, relation)?;
                    for movement in moves {
                        let child = self.ui_element(movement.child)?;
                        let previous = values
                            .index_of(&child)?
                            .ok_or(WinUiError::ChildNotFound(movement.child))?;
                        values.remove_at(previous)?;
                        let index = if let Some(before) = movement.before {
                            values
                                .index_of(&self.ui_element(before)?)?
                                .ok_or(WinUiError::ChildNotFound(before))?
                        } else {
                            values.size()?
                        };
                        values.insert_at(index, &child)?;
                    }
                }
                Realization::Owned => {
                    let values = self.owned_collection(parent, relation)?;
                    let mut current = (0..values.size()?)
                        .map(|index| {
                            let value = values.get_at(index)?;
                            Ok((com_identity(&value)?, value))
                        })
                        .collect::<Result<Vec<_>, WinUiError>>()?;
                    let desired = children
                        .iter()
                        .map(|child| {
                            self.ui_element(*child)
                                .and_then(|value| com_identity(&value))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let active = desired.iter().copied().collect::<HashSet<_>>();
                    let swaps = simulate_active_slot_reorder(&mut current, &active, &desired)
                        .ok_or(WinUiError::InvalidMutation(relation))?;
                    for (left, right, left_value, right_value) in swaps {
                        values.remove_at(index32(right)?)?;
                        values.remove_at(index32(left)?)?;
                        values.insert_at(index32(left)?, &left_value)?;
                        values.insert_at(index32(right)?, &right_value)?;
                    }
                }
                Realization::Structural => {
                    let tree = self.tree_owner(parent)?;
                    let template = self.tree_template()?;
                    let tree2 = tree.cast::<native::ITreeView2>()?;
                    tree2.SetItemTemplate(None::<&native::DataTemplate>)?;
                    tree.cast::<native::IUIElement>()?.UpdateLayout()?;
                    let values = self.tree_nodes(parent)?;
                    for movement in moves {
                        let child = self.tree_node(movement.child)?;
                        let mut previous = 0;
                        if !values.IndexOf(&child, &mut previous)? {
                            return Err(WinUiError::ChildNotFound(movement.child));
                        }
                        values.RemoveAt(previous)?;
                        let index = if let Some(before) = movement.before {
                            let before = self.tree_node(before)?;
                            let mut index = 0;
                            if !values.IndexOf(&before, &mut index)? {
                                return Err(WinUiError::ChildNotFound(movement.child));
                            }
                            index
                        } else {
                            values.Size()?
                        };
                        values.InsertAt(index, &child)?;
                    }
                    tree2.SetItemTemplate(&template)?;
                }
                Realization::Container => {
                    let values = self.items(parent)?;
                    if moves.len().saturating_add(1) >= children.len() {
                        let children = children
                            .iter()
                            .map(|child| self.inspectable(*child).map(Some))
                            .collect::<Result<Vec<_>, _>>()?;
                        values.ReplaceAll(&children)?;
                    } else {
                        // Check every move before disconnecting anything.
                        let mut moved = Vec::with_capacity(moves.len());
                        for movement in moves {
                            let child = self.inspectable(movement.child)?;
                            let before = movement
                                .before
                                .map(|before| self.inspectable(before))
                                .transpose()?;
                            for value in std::iter::once(&child).chain(before.as_ref()) {
                                let mut index = 0;
                                if !values.IndexOf(value, &mut index)? {
                                    return Err(WinUiError::ChildNotFound(movement.child));
                                }
                            }
                            moved.push((movement.child, child, before));
                        }
                        // No container is disconnected first: the one XAML prepares for a moved
                        // item takes its content over (`ContainerContentChanging`), and one XAML
                        // keeps for it would show nothing.
                        for (object, child, before) in &moved {
                            let mut previous = 0;
                            if !values.IndexOf(child, &mut previous)? {
                                return Err(WinUiError::ChildNotFound(*object));
                            }
                            values.RemoveAt(previous)?;
                            let index = if let Some(before) = before {
                                let mut index = 0;
                                if !values.IndexOf(before, &mut index)? {
                                    return Err(WinUiError::ChildNotFound(*object));
                                }
                                index
                            } else {
                                values.Size()?
                            };
                            values.InsertAt(index, child)?;
                        }
                    }
                }
            }
            if let (Some(selection), Some(selected)) = (selection, selected.as_ref()) {
                self.write_selected_item(parent, selection, Some(selected))?;
            }
            Ok(())
        })();
        self.finish_selection_feedback(parent, feedback);
        result?;
        let active = children.iter().copied().collect::<HashSet<_>>();
        debug_assert_eq!(
            children.len(),
            self.owners
                .iter()
                .filter(|(object, owner)| **owner == (parent, relation) && active.contains(object))
                .count()
        );
        Ok(())
    }

    fn selection_feedback(
        &self,
        object: ObjectId,
        property: PropertyId,
    ) -> Result<Option<(ObjectId, EventId)>, WinUiError> {
        let Some((owner, relation)) = self.owners.get(&object).copied() else {
            return Ok(None);
        };
        Ok(
            selection_for_item_property(self.kind(owner)?, relation, self.kind(object)?, property)
                .map(|selection| (owner, selection.event)),
        )
    }

    /// Suppresses the selection changes a mutation of `owner`'s `relation` causes natively: an
    /// item selector's, and an index selector's (ListView, GridView) whose selected item is
    /// removed or moved; the planner sets the declared `SelectedIndex` again after its items.
    fn begin_selection_feedback(
        &self,
        owner: ObjectId,
        relation: RelationId,
        selection: Option<SelectionContract>,
    ) -> Result<Option<SelectionFeedback>, WinUiError> {
        let index_selection = matches!(
            (self.kind(owner)?, relation),
            (ObjectType::ListView | ObjectType::GridView, RelationId::Items)
        ) && self.controlled_index_selectors.contains(&owner);
        let Some(event) = selection
            .map(|selection| selection.event)
            .or(index_selection.then_some(EventId::SelectionChanged))
        else {
            return Ok(None);
        };
        Ok(Some(self.begin_selector_write(owner, event)))
    }

    fn finish_selection_feedback(&self, owner: ObjectId, feedback: Option<SelectionFeedback>) {
        if let Some(feedback) = feedback {
            self.finish_selector_write(owner, feedback);
        }
    }

    /// Suppresses `owner`'s `event` while the reactor writes its selection, and notes whether the
    /// write could take away a selection whose "nothing selected" event comes late.
    fn begin_selector_write(&self, owner: ObjectId, event: EventId) -> SelectionFeedback {
        let had_selection = self
            .kind(owner)
            .is_ok_and(|kind| defers_deselect_echo(kind, true, false))
            && self.native_selection(owner, event) == Some(true);
        self.event_queue
            .feedback
            .borrow_mut()
            .begin(owner, event, FeedbackExpectation::Suppressed);
        SelectionFeedback {
            event,
            had_selection,
        }
    }

    /// Ends `begin_selector_write`. If the write took `owner`'s selection away and the selector
    /// raises that late, its one late "nothing selected" event is expected and swallowed.
    fn finish_selector_write(&self, owner: ObjectId, feedback: SelectionFeedback) {
        let SelectionFeedback {
            event,
            had_selection,
        } = feedback;
        self.event_queue.feedback.borrow_mut().finish(owner, event);
        if had_selection
            && let Ok(kind) = self.kind(owner)
            && let Some(has_selection) = self.native_selection(owner, event)
            && defers_deselect_echo(kind, had_selection, has_selection)
        {
            self.event_queue
                .feedback
                .borrow_mut()
                .expect_deferred_deselect(owner, event);
        }
    }

    /// Whether `owner` has an item selected natively, or `None` if that can't be read. Callers
    /// expect no late echo from a failed read, so a real event is never swallowed.
    fn native_selection(&self, owner: ObjectId, event: EventId) -> Option<bool> {
        let Some(Handle::Generated(handle)) = self.handles.get(&owner) else {
            return None;
        };
        match handle.selected_item(event) {
            Some(Ok(selected)) => Some(selected.is_some()),
            Some(Err(_)) | None => None,
        }
    }

    fn read_selected_item(
        &self,
        object: ObjectId,
        selection: SelectionContract,
    ) -> Result<Option<IInspectable>, WinUiError> {
        let Some(Handle::Generated(handle)) = self.handles.get(&object) else {
            return Err(WinUiError::InvalidObject(object));
        };
        handle
            .selected_item(selection.event)
            .unwrap_or(Err(WinUiError::InvalidObject(object)))
    }

    fn write_selected_item(
        &self,
        object: ObjectId,
        selection: SelectionContract,
        selected: Option<&IInspectable>,
    ) -> Result<(), WinUiError> {
        let Some(Handle::Generated(handle)) = self.handles.get(&object) else {
            return Err(WinUiError::InvalidObject(object));
        };
        handle
            .set_selected_item(selection.event, selected)
            .unwrap_or(Err(WinUiError::InvalidObject(object)))
    }

    fn selection_item_is_selected(
        &self,
        object: ObjectId,
        selection: SelectionContract,
    ) -> Result<bool, WinUiError> {
        let Some(Handle::Generated(handle)) = self.handles.get(&object) else {
            return Err(WinUiError::InvalidObject(object));
        };
        handle
            .selection_item_is_selected(selection.selected_property)
            .unwrap_or(Err(WinUiError::InvalidObject(object)))
    }

    fn handle(&mut self, object: ObjectId) -> Result<&mut Handle, WinUiError> {
        self.handles
            .get_mut(&object)
            .ok_or(WinUiError::MissingObject(object))
    }

    fn kind(&self, object: ObjectId) -> Result<ObjectType, WinUiError> {
        Ok(
            match self
                .handles
                .get(&object)
                .ok_or(WinUiError::MissingObject(object))?
            {
                Handle::Generated(value) => value.kind(),
                Handle::TextBox(_) => ObjectType::TextBox,
                Handle::TreeView(_) => ObjectType::TreeView,
                Handle::TreeNode(_) => ObjectType::TreeNode,
                Handle::ListView(_) => ObjectType::ListView,
                Handle::Data(_) => ObjectType::DataItem,
            },
        )
    }

    fn ui_element(&self, object: ObjectId) -> Result<native::UIElement, WinUiError> {
        match self
            .handles
            .get(&object)
            .ok_or(WinUiError::MissingObject(object))?
        {
            Handle::Generated(value) => value.ui_element(),
            Handle::TextBox(value) => Ok(value.value.cast()?),
            Handle::TreeView(value) => Ok(value.value.cast()?),
            Handle::ListView(value) => Ok(value.value.cast()?),
            Handle::TreeNode(_) | Handle::Data(_) => Err(WinUiError::InvalidObject(object)),
        }
    }

    fn inspectable(&self, object: ObjectId) -> Result<IInspectable, WinUiError> {
        match self
            .handles
            .get(&object)
            .ok_or(WinUiError::MissingObject(object))?
        {
            Handle::Data(value) => Ok(value.cast()?),
            _ => Err(WinUiError::InvalidObject(object)),
        }
    }

    fn tree_node(&self, object: ObjectId) -> Result<native::TreeViewNode, WinUiError> {
        match self
            .handles
            .get(&object)
            .ok_or(WinUiError::MissingObject(object))?
        {
            Handle::TreeNode(value) => Ok(value.value.clone()),
            _ => Err(WinUiError::InvalidObject(object)),
        }
    }

    fn owned_collection(
        &self,
        object: ObjectId,
        relation: RelationId,
    ) -> Result<GeneratedCollection, WinUiError> {
        match self
            .handles
            .get(&object)
            .ok_or(WinUiError::MissingObject(object))?
        {
            Handle::Generated(value) => value
                .owned_collection(relation)
                .unwrap_or(Err(WinUiError::InvalidObject(object))),
            _ => Err(WinUiError::InvalidObject(object)),
        }
    }

    fn tree_nodes(
        &self,
        object: ObjectId,
    ) -> Result<IVector<native::TreeViewNode>, WinUiError> {
        match self
            .handles
            .get(&object)
            .ok_or(WinUiError::MissingObject(object))?
        {
            Handle::TreeView(value) => Ok(value.value.RootNodes()?),
            Handle::TreeNode(value) => Ok(value.value.Children()?),
            _ => Err(WinUiError::InvalidObject(object)),
        }
    }

    fn tree_owner(&self, object: ObjectId) -> Result<native::TreeView, WinUiError> {
        let mut current = object;
        loop {
            match self
                .handles
                .get(&current)
                .ok_or(WinUiError::MissingObject(current))?
            {
                Handle::TreeView(tree) => return Ok(tree.value.clone()),
                Handle::TreeNode(_) => {
                    current = self
                        .owners
                        .get(&current)
                        .map(|(parent, _)| *parent)
                        .ok_or(WinUiError::StillOwned(current))?;
                }
                _ => return Err(WinUiError::InvalidObject(current)),
            }
        }
    }

    fn items(&self, object: ObjectId) -> Result<native::ItemCollection, WinUiError> {
        match self
            .handles
            .get(&object)
            .ok_or(WinUiError::MissingObject(object))?
        {
            Handle::ListView(value) => Ok(value.value.cast::<native::IItemsControl>()?.Items()?),
            _ => Err(WinUiError::InvalidObject(object)),
        }
    }

    fn set_container_resources(
        &mut self,
        object: ObjectId,
        resources: Option<&ResourceOverrides>,
    ) -> Result<(), WinUiError> {
        let Handle::Data(value) = self.handle(object)? else {
            return Err(WinUiError::InvalidObject(object));
        };
        let item = value.cast::<IInspectable>()?;
        {
            let mut container_resources = self.container_resources.borrow_mut();
            match resources {
                Some(resources) => {
                    container_resources
                        .keys
                        .extend(resources.values().map(|(key, _)| key.to_string()));
                    container_resources.items.insert(object, resources.clone());
                }
                None => {
                    container_resources.items.remove(&object);
                }
            }
        }
        if let Some((parent, RelationId::Items)) = self.owners.get(&object).copied()
            && let Some(Handle::ListView(list)) = self.handles.get(&parent)
        {
            // Update a realized container; the others are updated when the list prepares them.
            match list
                .value
                .cast::<native::IItemContainerMapping>()?
                .ContainerFromItem(&item)
            {
                Ok(container) => set_list_container_resources(
                    &container,
                    &item,
                    &self.data_items,
                    &self.container_resources,
                )?,
                Err(error) if error.code().is_ok() => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    /// Replaces all of a ListView's items with `children` in one `ReplaceAll`, for a keyed change
    /// that drops or adds most of them (`Mutation::ReplaceItems`). Only the realized containers
    /// of `removed` items are disconnected first, as `remove` does for one item; the items that
    /// stay keep their content, which the container XAML prepares for each takes over
    /// (`ContainerContentChanging`). The selected items that stay are selected again
    /// (`replace_list_items`), selection feedback is suppressed once for the whole change, and
    /// the planner sets `SelectedIndex` again after it.
    fn replace_items(
        &mut self,
        parent: ObjectId,
        relation: RelationId,
        removed: &[ObjectId],
        children: &[ObjectId],
    ) -> Result<(), WinUiError> {
        if relation_contract(self.kind(parent)?, relation)?.realization != Realization::Container
        {
            return Err(WinUiError::InvalidMutation(relation));
        }
        // Container relations have no item selection contract (`selection_for_relation`): the
        // ListView's own selection is kept by `replace_list_items`.
        debug_assert!(selection_for_relation(self.kind(parent)?, relation).is_none());
        let owner = Some(&(parent, relation));
        if let Some(child) = removed.iter().find(|child| self.owners.get(child) != owner) {
            return Err(WinUiError::ChildNotFound(*child));
        }
        if let Some(child) = children
            .iter()
            .find(|child| self.owners.get(child).is_some_and(|owned| Some(owned) != owner))
        {
            return Err(WinUiError::StillOwned(*child));
        }
        let values = self.items(parent)?;
        let items = children
            .iter()
            .map(|child| self.inspectable(*child).map(Some))
            .collect::<Result<Vec<_>, _>>()?;
        let removed_items = removed
            .iter()
            .map(|child| self.inspectable(*child).and_then(|item| com_identity(&item)))
            .collect::<Result<HashSet<_>, _>>()?;
        let feedback = self.begin_selection_feedback(parent, relation, None)?;
        let result = self.replace_list_items(parent, &values, &items, &removed_items);
        self.finish_selection_feedback(parent, feedback);
        result?;
        for child in removed {
            self.owners.remove(child);
        }
        for child in children {
            self.owners.insert(*child, (parent, relation));
        }
        Ok(())
    }

    /// Replaces a ListView's items with `items`, with the realized containers of the `removed`
    /// items disconnected first, and keeps the selection of the items that stay.
    ///
    /// `ReplaceAll` resets the native selection, which `Remove`s and `Insert`s of other items
    /// would not, so the selected items that stay are selected again in their selection order
    /// (keeping the first one, and so `SelectedIndex`, the same). The `SelectionChanged` events
    /// WinUI raises meanwhile are held back, and one is reported afterwards if the selection lost
    /// any removed items, as removing them one by one would have.
    fn replace_list_items(
        &self,
        parent: ObjectId,
        values: &native::ItemCollection,
        items: &[Option<IInspectable>],
        removed: &HashSet<usize>,
    ) -> Result<(), WinUiError> {
        let replace = || {
            self.with_list_items_disconnected(parent, ListItems::Matching(removed), || {
                Ok(values.ReplaceAll(items)?)
            })
        };
        let Some(Handle::ListView(list)) = self.handles.get(&parent) else {
            return replace();
        };
        let selector = list.value.cast::<native::ISelector>()?;
        let list_base = list.value.cast::<native::IListViewBase>()?;
        let selected = list_base.SelectedItems()?;
        let before = (0..selected.Size()?)
            .map(|index| {
                let item = selected.GetAt(index)?;
                Ok((com_identity(&item)?, item))
            })
            .collect::<Result<Vec<_>, WinUiError>>()?;
        let selected_before = before.len();
        let kept = kept_selection(before, removed);
        list.selection_held.set(true);
        let result = replace().and_then(|()| match kept.as_slice() {
            [] => Ok(()),
            // Also right for `Single` mode, where `SelectedItems` holds at most one item.
            [item] => Ok(selector.SetSelectedItem(item)?),
            kept => {
                let selected = list_base.SelectedItems()?;
                for item in kept {
                    selected.Append(item)?;
                }
                Ok(())
            }
        });
        list.selection_held.set(false);
        result?;
        if kept.len() == selected_before {
            return Ok(());
        }
        self.report_list_selection(parent, list, &selector, &list_base)
    }

    /// Sets ListView `parent`'s selection to `items` (`ElementRef<ListView>::request_selected_keys`):
    /// the selected items not among them are deselected, and the ones missing are appended in
    /// their order, so the items that stay keep their place (and the first, `SelectedIndex`).
    /// WinUI's `SelectionChanged` events meanwhile are held back, and one change is reported
    /// afterwards, as a user's selection would be, if the selection changed.
    pub(super) fn set_list_selection(
        &self,
        parent: ObjectId,
        items: &[ObjectId],
    ) -> Result<(), WinUiError> {
        let Some(Handle::ListView(list)) = self.handles.get(&parent) else {
            return Ok(());
        };
        let wanted = items
            .iter()
            .filter_map(|item| self.inspectable(*item).ok())
            .map(|item| Ok((com_identity(&item)?, item)))
            .collect::<Result<Vec<_>, WinUiError>>()?;
        let selector = list.value.cast::<native::ISelector>()?;
        let list_base = list.value.cast::<native::IListViewBase>()?;
        let selected = list_base.SelectedItems()?;
        let before = (0..selected.Size()?)
            .map(|index| com_identity(&selected.GetAt(index)?))
            .collect::<Result<Vec<_>, WinUiError>>()?;
        let identities = wanted
            .iter()
            .map(|(identity, _)| *identity)
            .collect::<Vec<_>>();
        let (removals, appends) = selection_edits(&before, &identities);
        if removals.is_empty() && appends.is_empty() {
            return Ok(());
        }
        list.selection_held.set(true);
        let result = (|| {
            for index in removals {
                selected.RemoveAt(index as u32)?;
            }
            for (_, item) in wanted
                .iter()
                .filter(|(identity, _)| appends.contains(identity))
            {
                selected.Append(item)?;
            }
            Ok::<(), WinUiError>(())
        })();
        list.selection_held.set(false);
        result?;
        self.report_list_selection(parent, list, &selector, &list_base)
    }

    /// The key of ListView `parent`'s item whose container has the focus
    /// (`ElementRef<ListView>::request_focused_key`), among `items` (its `Items` children and
    /// their keys); `None` when none has. Unrealized containers have no focus, and are cheap to
    /// pass over.
    pub(super) fn focused_list_item(
        &self,
        parent: ObjectId,
        items: &[(ObjectId, Key)],
    ) -> Result<Option<Key>, WinUiError> {
        let Some(Handle::ListView(list)) = self.handles.get(&parent) else {
            return Ok(None);
        };
        let mapping = list.value.cast::<native::IItemContainerMapping>()?;
        let values = list.value.cast::<native::IItemsControl>()?.Items()?;
        for index in 0..values.Size()? {
            let container = match mapping.ContainerFromIndex(index as i32) {
                Ok(container) => container,
                Err(error) if error.code().is_ok() => continue,
                Err(error) => return Err(error.into()),
            };
            let focus = container.cast::<native::IUIElement>()?.FocusState()?;
            if focus == native::FocusState::Unfocused {
                continue;
            }
            let identity = com_identity(&values.GetAt(index)?)?;
            let object = self.data_items.borrow().get(&identity).copied();
            return Ok(object.and_then(|object| {
                items
                    .iter()
                    .find(|(item, _)| *item == object)
                    .map(|(_, key)| key.clone())
            }));
        }
        Ok(None)
    }

    /// Reports ListView `parent`'s selection after a change made while its `SelectionChanged`
    /// events were held back: its `SelectedIndex` (observed, then the app's callback) and its
    /// selected keys.
    fn report_list_selection(
        &self,
        parent: ObjectId,
        list: &NativeListView,
        selector: &native::ISelector,
        list_base: &native::IListViewBase,
    ) -> Result<(), WinUiError> {
        let value = selection_index(selector.SelectedIndex()?)?;
        let observation = Observation::SetProperty {
            object: parent,
            property: Property {
                id: PropertyId::SelectedIndex,
                value: PropertyValue::SelectionIndex(value),
            },
        };
        if self
            .event_queue
            .observe(parent, EventId::SelectionChanged, observation.clone())
        {
            Self::dispatch(
                &list.selection_changed,
                &self.event_queue,
                parent,
                EventId::SelectionChanged,
                Some(observation),
                value,
            );
        }
        Self::dispatch_selected_items(
            &list.selected_keys_changed,
            &self.event_queue,
            parent,
            list_base,
            &self.data_items,
        )
    }

    /// Runs `mutate` on a ListView's items with the realized containers of `items` disconnected
    /// first, so that their elements are free for the containers XAML prepares next. If `mutate`
    /// fails the containers are connected again, since XAML will not prepare them again for
    /// items that did not change.
    fn with_list_items_disconnected(
        &self,
        parent: ObjectId,
        items: ListItems<'_>,
        mutate: impl FnOnce() -> Result<(), WinUiError>,
    ) -> Result<(), WinUiError> {
        let Some(Handle::ListView(list)) = self.handles.get(&parent) else {
            return mutate();
        };
        let mapping = list.value.cast::<native::IItemContainerMapping>()?;
        let realized = |container: windows_core::Result<native::DependencyObject>| match container
        {
            Ok(container) => Ok(Some(container)),
            Err(error) if error.code().is_ok() => Ok(None),
            Err(error) => Err(WinUiError::from(error)),
        };
        let mut containers = Vec::new();
        match items {
            ListItems::These(items) => {
                for item in items {
                    containers.extend(realized(mapping.ContainerFromItem(item))?);
                }
            }
            ListItems::Matching(identities) => {
                // One pass over the indices, which is cheap for unrealized ones, rather than a
                // `ContainerFromItem` search per item.
                let values = list.value.cast::<native::IItemsControl>()?.Items()?;
                for index in 0..values.Size()? {
                    if let Some(container) = realized(mapping.ContainerFromIndex(index as i32))?
                        && identities.contains(&com_identity(&values.GetAt(index)?)?)
                    {
                        containers.push(container);
                    }
                }
            }
        }
        let result = containers
            .iter()
            .try_for_each(|container| set_list_container_connected(container, false))
            .map_err(WinUiError::from)
            .and_then(|()| mutate());
        if result.is_err() {
            // Best effort: the mutation's error is the one to report.
            for container in &containers {
                _ = set_list_container_connected(container, true);
            }
        }
        result
    }

    fn set_data_text(&mut self, object: ObjectId, text: &str) -> Result<(), WinUiError> {
        let content_key = self.data_content_key.clone();
        let text_key = self.data_text_key.clone();
        let Handle::Data(value) = self.handle(object)? else {
            return Err(WinUiError::InvalidObject(object));
        };
        let properties = value.cast::<IMap<HSTRING, IInspectable>>()?;
        let text: IInspectable = windows_reference::IReference::from(HSTRING::from(text)).into();
        properties.Insert(&text_key, &text)?;
        if !properties.HasKey(&content_key)? {
            properties.Insert(&content_key, &text)?;
        }
        let item = value.cast::<IInspectable>()?;
        self.rename_list_container(object, &item, &text_key)
    }

    /// Records that a `DataItem`'s content waits for a container (`Mutation::DeferContent`), or
    /// no longer does. A waiting item shows its text, as one without content does: the text gives
    /// the row a height, so the list realizes about the containers that fill its view, not all of
    /// them as it would for rows of no height.
    fn defer_content(&mut self, item: ObjectId, deferred: bool) -> Result<(), WinUiError> {
        let Handle::Data(_) = self.handle(item)? else {
            return Err(WinUiError::InvalidObject(item));
        };
        let mut waiting = self.deferred_content.borrow_mut();
        if deferred {
            waiting.defer(item);
        } else {
            waiting.end(item);
        }
        Ok(())
    }

    fn set_data_automation_id(&mut self, object: ObjectId, id: &str) -> Result<(), WinUiError> {
        let text_key = self.data_text_key.clone();
        let Handle::Data(value) = self.handle(object)? else {
            return Err(WinUiError::InvalidObject(object));
        };
        let properties = value.cast::<IMap<HSTRING, IInspectable>>()?;
        let id: IInspectable = windows_reference::IReference::from(HSTRING::from(id)).into();
        properties.Insert(&HSTRING::from(DATA_AUTOMATION_ID_KEY), &id)?;
        let item = value.cast::<IInspectable>()?;
        self.rename_list_container(object, &item, &text_key)
    }

    /// Names the realized container showing a list's `DataItem` again; the others are named when
    /// the list prepares them.
    fn rename_list_container(
        &self,
        object: ObjectId,
        item: &IInspectable,
        text_key: &HSTRING,
    ) -> Result<(), WinUiError> {
        if let Some((parent, RelationId::Items)) = self.owners.get(&object).copied()
            && let Some(Handle::ListView(list)) = self.handles.get(&parent)
        {
            match list
                .value
                .cast::<native::IItemContainerMapping>()?
                .ContainerFromItem(item)
            {
                Ok(container) => name_list_container(&container, item, text_key)?,
                Err(error) if error.code().is_ok() => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    fn tree_template(&mut self) -> Result<native::DataTemplate, WinUiError> {
        if let Some(template) = &self.tree_template {
            return Ok(template.clone());
        }
        let template = native::XamlReader::Load(
            "<DataTemplate xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation'>\
             <ContentPresenter Content='{Binding Content}'/>\
             </DataTemplate>",
        )?
        .cast::<native::DataTemplate>()?;
        self.tree_template = Some(template.clone());
        Ok(template)
    }

    fn list_template(&mut self) -> Result<native::DataTemplate, WinUiError> {
        if let Some(template) = &self.list_template {
            return Ok(template.clone());
        }
        let template = native::XamlReader::Load(
            "<DataTemplate xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation'>\
             <ContentPresenter Content='{Binding [Content]}'/>\
             </DataTemplate>",
        )?
        .cast::<native::DataTemplate>()?;
        self.list_template = Some(template.clone());
        Ok(template)
    }

    fn start_retirement(
        &mut self,
        root: ObjectId,
        nodes: Vec<ObjectId>,
        parent: ObjectId,
        relation: RelationId,
        duration: Duration,
    ) -> Result<(), WinUiError> {
        if self.retirements.contains_key(&root)
            || self.owners.get(&root) != Some(&(parent, relation))
            || nodes.iter().any(|node| !self.handles.contains_key(node))
        {
            return Err(WinUiError::MissingObject(root));
        }
        let duration =
            TimeSpan::try_from(duration).map_err(|_| WinUiError::InvalidDuration)?;
        let root_element = self.ui_element(root)?;
        let transition = native::ScalarTransition::new()?;
        transition.SetDuration(duration)?;
        root_element.SetOpacityTransition(&transition)?;
        let timer = native::DispatcherQueue::GetForCurrentThread()?.CreateTimer()?;
        timer.SetInterval(duration)?;
        timer.SetIsRepeating(false)?;
        let event_queue = Rc::clone(&self.event_queue);
        let tick = timer.Tick(move |_, _| {
            event_queue
                .events
                .borrow_mut()
                .push_back(QueuedNativeEvent {
                    observation: None,
                    event: None,
                    retirement: Some(RetirementCompletion { root }),
                    realization: None,
                });
            Self::schedule_event_wake(&event_queue);
        })?;
        self.retirements.insert(
            root,
            NativeRetirement {
                nodes,
                parent,
                relation,
                timer: timer.clone(),
                _tick: tick,
            },
        );
        if let Err(error) = root_element.SetOpacity(0.0) {
            self.retirements.remove(&root);
            return Err(error.into());
        }
        if let Err(error) = timer.Start() {
            self.retirements.remove(&root);
            return Err(error.into());
        }
        Ok(())
    }

    fn complete_retirement(
        &mut self,
        root: ObjectId,
        nodes: &[ObjectId],
    ) -> Result<(), WinUiError> {
        let retirement = self
            .retirements
            .get(&root)
            .ok_or(WinUiError::MissingObject(root))?;
        if retirement.nodes != nodes {
            return Err(WinUiError::InvalidObject(root));
        }
        retirement.timer.Stop()?;
        let parent = retirement.parent;
        let relation = retirement.relation;
        let values = self.owned_collection(parent, relation)?;
        let root_value = self.ui_element(root)?;
        let mut index = 0;
        while index < values.size()? && values.get_at(index)? != root_value {
            index += 1;
        }
        if index == values.size()? {
            return Err(WinUiError::ChildNotFound(root));
        }
        values.remove_at(index)?;
        self.retirements.remove(&root);
        for object in nodes {
            self.owners.remove(object);
            let handle = self
                .handles
                .remove(object)
                .ok_or(WinUiError::MissingObject(*object))?;
            self.release_object(*object, handle)?;
        }
        Ok(())
    }

    fn native_event(&self, queued: &QueuedNativeEvent) -> Option<NativeEvent> {
        if let Some(completion) = queued.retirement {
            return Some(NativeEvent::retirement(completion));
        }
        if let Some(request) = queued.realization {
            return Some(NativeEvent::realization(request));
        }
        let event = queued.event.as_ref().and_then(|queued| {
            if queued.event == EventId::MenuItemInvoked {
                let menu = self.menus.get(&queued.object)?;
                return menu.accepts(queued.revision).then(|| {
                    EventDispatch::new(
                        queued.object,
                        queued.event,
                        EventValue::Key(menu.menu.on_click.clone()),
                        queued.payload.clone(),
                    )
                });
            }
            if queued.event == EventId::MenuOpening {
                let menu = self.menus.get(&queued.object)?;
                return menu.menu.on_opening.clone().map(|callback| {
                    EventDispatch::new(
                        queued.object,
                        queued.event,
                        EventValue::Unit(callback),
                        EventPayload::Unit,
                    )
                });
            }
            if queued.event == EventId::CommandInvoked {
                let flyout = self.command_bar_flyouts.get(&queued.object)?;
                return (flyout.revision == queued.revision).then(|| {
                    EventDispatch::new(
                        queued.object,
                        queued.event,
                        EventValue::Key(flyout.flyout.on_click.clone()),
                        queued.payload.clone(),
                    )
                });
            }
            let callback = match (self.handles.get(&queued.object), queued.event) {
                (Some(Handle::TextBox(text_box)), EventId::TextChanged) => {
                    let event = text_box.event.borrow();
                    (event.revision == queued.revision)
                        .then(|| event.callback.clone().map(EventValue::String))
                        .flatten()
                }
                (Some(Handle::ListView(list)), EventId::SelectionChanged) => {
                    let event = list.selection_changed.borrow();
                    (event.revision == queued.revision)
                        .then(|| event.callback.clone().map(EventValue::SelectionIndex))
                        .flatten()
                }
                (Some(Handle::TreeView(tree)), EventId::ItemInvoked) => {
                    let event = tree.item_invoked.borrow();
                    (event.revision == queued.revision)
                        .then(|| event.callback.clone().map(EventValue::String))
                        .flatten()
                }
                (Some(Handle::ListView(list)), EventId::DragItemsCompleted) => {
                    let event = list.drag_items_completed.borrow();
                    (event.revision == queued.revision)
                        .then(|| event.callback.clone().map(EventValue::StringList))
                        .flatten()
                }
                (Some(Handle::ListView(list)), EventId::SelectedKeysChanged) => {
                    let event = list.selected_keys_changed.borrow();
                    (event.revision == queued.revision)
                        .then(|| event.callback.clone().map(EventValue::Keys))
                        .flatten()
                }
                (Some(Handle::ListView(list)), EventId::ItemClick) => {
                    let event = list.item_click.borrow();
                    (event.revision == queued.revision)
                        .then(|| event.callback.clone().map(EventValue::Keys))
                        .flatten()
                }
                (Some(Handle::Generated(handle)), event) => {
                    handle.event_callback(event, queued.revision)
                }
                _ => None,
            }?;
            Some(EventDispatch::new(
                queued.object,
                queued.event,
                callback,
                queued.payload.clone(),
            ))
        });
        Some(NativeEvent::new(queued.observation.clone(), event))
    }
}
