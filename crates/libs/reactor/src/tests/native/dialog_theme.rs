//! A ContentDialog takes its window's theme (Okojo#3035). WinUI shows a dialog in its `XamlRoot`'s
//! popup layer, outside the window's content, so the theme a window requests (its
//! `WindowVisuals` theme, which goes on the window's surface) never reached it, and the dialog drew
//! in the application's theme. The window here is forced to the theme opposite the application's,
//! as an app's chosen theme can be, and switched while the dialog is open and after it is gone.
//!
//! The app runs in a child process (one Application per process). Each step runs in a timer tick,
//! so WinUI raises its theme and dialog events in between; a failed step ends the app and fails the
//! child after `App::run_with` returns, rather than unwinding into WinUI.
use super::*;

const TEST: &str = "native::winui::tests::dialog_theme::a_content_dialog_takes_its_windows_theme";
const CHILD: &str = "REACTOR_DIALOG_THEME_TEST_CHILD";

#[test]
#[ignore = "requires an interactive WinUI desktop"]
fn a_content_dialog_takes_its_windows_theme() {
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, "1")
            .status()
            .unwrap();
        assert!(status.success(), "{status}");
        return;
    }
    child();
}

/// A button in a border; `dialog`: the button has a dialog, open or not.
fn view(dialog: Option<bool>) -> Border {
    let owner = Button::new().content(TextBlock::new().text("Owner"));
    let owner: View = match dialog {
        Some(open) => owner.content_dialog(
            ContentDialog::new()
                .title("Theme")
                .close_button_text("Close")
                .is_open(open),
        ),
        None => owner.into(),
    };
    Border::new().width(240.0).height(180.0).content(owner)
}

fn opposite(theme: native::ElementTheme) -> native::ElementTheme {
    if theme == native::ElementTheme::Dark {
        native::ElementTheme::Light
    } else {
        native::ElementTheme::Dark
    }
}

/// Gives `window` `theme` the way an app does: its published visuals.
fn set_window_theme(window: &mut NativeWindow, theme: native::ElementTheme) {
    let theme = if theme == native::ElementTheme::Dark {
        WindowTheme::Dark
    } else {
        WindowTheme::Light
    };
    window
        .apply_publication(None, Some(&WindowVisuals::new().theme(theme)), false)
        .unwrap();
}

fn theme_of(element: &native::IFrameworkElement) -> native::ElementTheme {
    element.ActualTheme().unwrap()
}

/// Whether the adapter has shown `dialog` and it hasn't closed since.
fn is_showing(runtime: &Runtime<WinUiAdapter>, dialog: ObjectId) -> bool {
    runtime
        .adapter()
        .event_queue
        .content_dialogs
        .borrow()
        .dialogs
        .get(&dialog)
        .is_some_and(|state| state.pending_generation.is_some())
}

fn child() {
    let completed = Rc::new(Cell::new(false));
    let failed = Rc::new(Cell::new(false));
    let finished = Rc::clone(&completed);
    let failure = Rc::clone(&failed);
    App::run_with(move |app| {
        let mut runtime = Runtime::new(WinUiAdapter::default());
        runtime.update(view(None)).unwrap();
        let root = runtime.graph().root().unwrap();
        let owner = runtime.graph().child(root, RelationId::Content).unwrap();
        let root_element = runtime
            .adapter()
            .ui_element(root)
            .unwrap()
            .cast::<native::IFrameworkElement>()?;
        let window = runtime.adapter_mut().create_window(root).unwrap();
        window.activate().unwrap();
        let runtime = RefCell::new(runtime);
        let window = RefCell::new(Some(window));
        let phase = Cell::new(0);
        // The application's theme, and the one the window is forced to.
        let app_theme = Cell::new(native::ElementTheme::Default);
        let forced = Cell::new(native::ElementTheme::Default);
        // The dialog's object and native element, kept after the view drops it.
        let dialog = RefCell::new(None::<(ObjectId, native::IFrameworkElement)>);
        let timer = native::DispatcherQueue::GetForCurrentThread()?.CreateTimer()?;
        timer.SetInterval(TimeSpan::try_from(Duration::from_millis(150)).unwrap())?;
        let app = app.clone();
        let tick = timer.Tick(move |sender, _| {
            let step = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut runtime = runtime.borrow_mut();
                runtime.dispatch_native_events().unwrap();
                let mut window = window.borrow_mut();
                let window = window.as_mut().unwrap();
                let dialog_element = || dialog.borrow().as_ref().unwrap().1.clone();
                let dialog_object = || dialog.borrow().as_ref().unwrap().0;
                match phase.get() {
                    0 => {
                        // A window with the system theme has the application's.
                        app_theme.set(theme_of(&root_element));
                        forced.set(opposite(app_theme.get()));
                        set_window_theme(window, forced.get());
                    }
                    1 => {
                        assert_eq!(theme_of(&root_element), forced.get(), "the window's theme");
                        runtime.update(view(Some(false))).unwrap();
                        let object = runtime.graph().content_dialog(owner).unwrap().0;
                        let Some(Handle::Generated(GeneratedHandle::ContentDialog(handle))) =
                            runtime.adapter().handles.get(&object)
                        else {
                            panic!("no ContentDialog handle")
                        };
                        let element = handle.value.cast::<native::IFrameworkElement>().unwrap();
                        *dialog.borrow_mut() = Some((object, element));
                        assert_eq!(
                            theme_of(&dialog_element()),
                            forced.get(),
                            "taken when its owner is set"
                        );
                    }
                    2 => {
                        // The theme a dialog has when its owner was set before it was in the
                        // tree (which reports the application's): showing it corrects that.
                        let element = dialog_element();
                        element.SetRequestedTheme(app_theme.get()).unwrap();
                        assert_eq!(theme_of(&element), app_theme.get());
                        runtime.update(view(Some(true))).unwrap();
                        assert!(is_showing(&runtime, dialog_object()), "the dialog is shown");
                        assert_eq!(
                            theme_of(&element),
                            forced.get(),
                            "taken again as it is shown"
                        );
                    }
                    3 => {
                        assert!(
                            is_showing(&runtime, dialog_object()),
                            "the dialog stays open"
                        );
                        assert_eq!(theme_of(&dialog_element()), forced.get(), "while open");
                        set_window_theme(window, app_theme.get());
                    }
                    4 => {
                        assert_eq!(
                            theme_of(&root_element),
                            app_theme.get(),
                            "the window's theme"
                        );
                        assert!(
                            is_showing(&runtime, dialog_object()),
                            "the dialog stays open"
                        );
                        assert_eq!(
                            theme_of(&dialog_element()),
                            app_theme.get(),
                            "an open dialog follows its window"
                        );
                        runtime.update(view(Some(false))).unwrap();
                    }
                    5 => {
                        // Declare no dialog: the owner lets it go.
                        runtime.update(view(None)).unwrap();
                        assert_eq!(runtime.graph().content_dialog(owner), None);
                        set_window_theme(window, forced.get());
                    }
                    6 => {
                        assert_eq!(theme_of(&root_element), forced.get(), "the window's theme");
                        assert_eq!(
                            theme_of(&dialog_element()),
                            app_theme.get(),
                            "a dialog its owner let go no longer follows it"
                        );
                        return true;
                    }
                    _ => unreachable!(),
                }
                phase.set(phase.get() + 1);
                false
            }));
            let done = match step {
                Ok(done) => done,
                Err(_) => {
                    eprintln!("dialog theme: step {} failed", phase.get());
                    failure.set(true);
                    true
                }
            };
            if done {
                sender.ok().unwrap().Stop().unwrap();
                if let Some(window) = window.borrow_mut().take() {
                    window.close().unwrap();
                }
                finished.set(true);
                app.exit().unwrap();
            }
        })?;
        timer.Start()?;
        Ok((timer, tick))
    })
    .unwrap();
    assert!(!failed.get(), "a step failed (its panic is above)");
    assert!(completed.get());
}
