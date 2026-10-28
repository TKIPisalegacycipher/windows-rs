use super::*;

/// A plain button, one styled `Default`, one with a theme background, and one whose style is
/// `last` (Accent first, then Default).
fn view(last: ButtonStyle) -> StackPanel {
    StackPanel::new().children((
        Button::new().content("Plain"),
        Button::new().style(ButtonStyle::Default).content("Default"),
        Button::new()
            .background(ThemeBrush::CardBackground)
            .content("Themed"),
        Button::new().style(last).content("Changed"),
    ))
}

#[test]
#[ignore = "requires an interactive WinUI desktop"]
fn default_and_themed_buttons_keep_the_fluent_style() {
    // WinUI supports one Application lifetime per process.
    if std::env::var_os("REACTOR_BUTTON_STYLE_TEST_CHILD").is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native::winui::tests::button_style::default_and_themed_buttons_keep_the_fluent_style",
                "--ignored",
                "--nocapture",
            ])
            .env("REACTOR_BUTTON_STYLE_TEST_CHILD", "1")
            .status()
            .unwrap();
        assert!(status.success(), "{status}");
        return;
    }
    register_text_block_font("Segoe UI");
    let completed = Rc::new(Cell::new(false));
    let finished = Rc::clone(&completed);
    App::run_with(move |app| {
        let mut runtime = Runtime::new(WinUiAdapter::default());
        runtime.update(view(ButtonStyle::Accent)).unwrap();
        let root = runtime.graph().root().unwrap();
        let window = runtime.adapter_mut().create_window(root).unwrap();
        window.activate().unwrap();
        let window = RefCell::new(Some(window));
        let state = RefCell::new(Some(runtime));
        let timer = native::DispatcherQueue::GetForCurrentThread()?.CreateTimer()?;
        timer.SetInterval(TimeSpan::try_from(Duration::from_millis(150)).unwrap())?;
        let app = app.clone();
        let tick = timer.Tick(move |sender, _| {
            sender.ok().unwrap().Stop().unwrap();
            let mut runtime = state.borrow_mut().take().unwrap();
            let adapter = runtime.adapter();
            assert!(adapter.lookup_style("DefaultButtonStyle").is_some());
            assert!(adapter.lookup_style("DefaultTextBlockStyle").is_some());
            let buttons = runtime
                .graph()
                .children(root, RelationId::Children)
                .unwrap()
                .to_vec();
            let radius = |runtime: &Runtime<WinUiAdapter>, index: usize| {
                runtime
                    .adapter()
                    .control_corner_radius(buttons[index])
                    .unwrap()
            };
            // Fluent buttons are rounded (ControlCornerRadius); the framework's own template
            // isn't.
            let fluent = radius(&runtime, 0);
            assert!(fluent > 0.0, "{fluent}");
            assert_eq!(radius(&runtime, 1), fluent, "ButtonStyle::Default");
            assert_eq!(radius(&runtime, 2), fluent, "theme background");
            assert_eq!(radius(&runtime, 3), fluent, "Accent");
            runtime.update(view(ButtonStyle::Default)).unwrap();
            assert_eq!(radius(&runtime, 3), fluent, "Accent, then Default");
            runtime.update(StackPanel::new()).unwrap();
            window.borrow_mut().take().unwrap().close().unwrap();
            finished.set(true);
            app.exit().unwrap();
        })?;
        timer.Start()?;
        Ok((timer, tick))
    })
    .unwrap();
    assert!(completed.get());
}
