//! An open menu whose declaration changes (Okojo#2552), against a fake flyout: the lib tests
//! run no live WinUI. The fake does to its flyout what `set_menu` and `fill_lazy_menu` do to
//! WinUI's, through the same `NativeMenu` and `LazyMenu` decisions: a replaced menu's flyout is
//! hidden and built again, a kept one builds the latest items as it next opens, and a click
//! reaches whatever `NativeMenu::click` dispatches it to.
use super::*;

/// A menu flyout as WinUI shows it: open or not, and the revision and keys of the items it holds.
struct FakeFlyout {
    host: MenuHost,
    menu: NativeMenu,
    open: bool,
    shown: Option<(u64, Vec<Key>)>,
}

impl FakeFlyout {
    /// `set_menu` on an element with no menu yet: a lazy flyout is empty until it opens; any
    /// other holds its items at once.
    fn declare(kind: ObjectType, menu: &Menu, revision: u64) -> Self {
        let host = menu_host(kind);
        let menu = NativeMenu::declared(host, menu, revision);
        let shown = menu
            .lazy
            .is_none()
            .then(|| (revision, keys(&menu.menu.items)));
        Self {
            host,
            menu,
            open: false,
            shown,
        }
    }

    /// `set_menu` with a new declaration. A replaced menu's flyout is hidden (`Hide()`) and a
    /// new one is built.
    fn set(&mut self, menu: &Menu, revision: u64) {
        if self.menu.update(menu, revision) == MenuChange::Replaced {
            let kind = match self.host {
                MenuHost::Button => ObjectType::Button,
                _ => ObjectType::TextBlock,
            };
            *self = Self::declare(kind, menu, revision);
        }
    }

    /// `FlyoutBase::Opening`, then shown: a lazy flyout builds the latest items if it doesn't
    /// hold them yet (`fill_lazy_menu`).
    fn open(&mut self) {
        if let Some(lazy) = &self.menu.lazy {
            let mut lazy = lazy.borrow_mut();
            if lazy.needs_build() {
                self.shown = Some((lazy.revision, keys(&lazy.menu.items)));
                lazy.built(Vec::new());
            }
        }
        self.open = true;
    }

    /// A click on the shown item `key`: the flyout closes, and the click is queued with the
    /// revision its item was built at (`build_menu_items`).
    fn click(&mut self, key: &str) -> Option<EventDispatch> {
        assert!(self.open, "only an open menu's items can be clicked");
        let (revision, keys) = self.shown.as_ref().unwrap();
        assert!(keys.contains(&Key::from(key)), "{key} is not shown");
        self.open = false;
        self.menu.click(
            ObjectId::test(1),
            *revision,
            EventPayload::Key(Key::from(key)),
        )
    }
}

fn keys(items: &[MenuItem]) -> Vec<Key> {
    items
        .iter()
        .filter_map(|item| match item {
            MenuItem::Item { key, .. } | MenuItem::Submenu { key, .. } => Some(key.clone()),
            MenuItem::Separator { .. } => None,
        })
        .collect()
}

/// A menu of `keys` whose clicks land in `log` tagged with `name`, as a render's `on_click`
/// is: each render's callback is a new one, and answers for the items declared with it.
fn menu(name: &'static str, keys: &[&'static str], log: &Log) -> Menu {
    let log = Rc::clone(log);
    Menu::new(
        keys.iter().map(|key| MenuItem::item(*key, *key)),
        move |key: Key| log.borrow_mut().push((name, key)),
    )
}

/// Each click a menu's callback took: the callback's name, and the item's key.
type Log = Rc<RefCell<Vec<(&'static str, Key)>>>;

fn clicked(name: &'static str, key: &str) -> (&'static str, Key) {
    (name, Key::from(key))
}

/// The ribbon's More while a sync refits the row: Rules moves back into the row, so the open
/// menu's declaration loses it. The menu stays open on the items it showed, Rules still acts,
/// through the callback declared with it, and the next opening shows the new items.
fn open_menu_keeps_its_items_and_clicks(kind: ObjectType) {
    let log = Log::default();
    let mut flyout = FakeFlyout::declare(kind, &menu("first", &["flag", "rules"], &log), 1);
    flyout.open();
    assert_eq!(flyout.shown.as_ref().unwrap().0, 1);

    flyout.set(&menu("second", &["flag"], &log), 2);
    assert!(flyout.open, "{kind:?}: the open menu stays open");
    assert_eq!(
        flyout.shown,
        Some((1, vec![Key::from("flag"), Key::from("rules")])),
        "{kind:?}: it keeps the items it opened with"
    );

    let dispatch = flyout
        .click("rules")
        .unwrap_or_else(|| panic!("{kind:?}: a click on a shown item counts"));
    dispatch.invoke();
    assert_eq!(
        &*log.borrow(),
        &[clicked("first", "rules")],
        "{kind:?}: it acts as the menu it was clicked in declared"
    );

    flyout.open();
    assert_eq!(
        flyout.shown,
        Some((2, vec![Key::from("flag")])),
        "{kind:?}: the next opening shows the latest items"
    );
    flyout.click("flag").unwrap().invoke();
    assert_eq!(log.borrow().last(), Some(&clicked("second", "flag")));
}

#[test]
fn an_open_button_menu_keeps_its_items_and_clicks_when_they_change() {
    for kind in [ObjectType::Button, ObjectType::DropDownButton] {
        open_menu_keeps_its_items_and_clicks(kind);
    }
}

#[test]
fn an_open_context_menu_clicks_through_the_callback_declared_with_its_items() {
    open_menu_keeps_its_items_and_clicks(ObjectType::TextBlock);
}

/// A click queued from items older than the ones shown (a showing before the last opening) is
/// dropped, and a menu's new callbacks for the same items answer its clicks.
#[test]
fn a_menu_drops_clicks_from_items_it_no_longer_shows() {
    let log = Log::default();
    for kind in [ObjectType::Button, ObjectType::TextBlock] {
        let mut flyout = FakeFlyout::declare(kind, &menu("first", &["a"], &log), 1);
        flyout.open();
        flyout.set(&menu("again", &["a"], &log), 1);
        flyout.click("a").unwrap().invoke();
        assert_eq!(
            log.borrow().last(),
            Some(&clicked("again", "a")),
            "{kind:?}"
        );

        flyout.set(&menu("second", &["a", "b"], &log), 2);
        flyout.open();
        flyout.set(&menu("third", &["a"], &log), 3);
        assert!(
            flyout
                .menu
                .click(ObjectId::test(1), 1, EventPayload::Key(Key::from("a")))
                .is_none(),
            "{kind:?}: revision 1 is neither the one shown (2) nor the latest (3)"
        );
    }
}
