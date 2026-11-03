use super::*;
use std::time::Instant;

fn assert_point(actual: [f32; 2], expected: [f32; 2]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 0.01, "{actual} != {expected}");
    }
}

fn assert_progress(value: f32, from: f32, to: f32, animations: bool) {
    if animations {
        assert!(
            value > from.min(to) + 0.01 && value < from.max(to) - 0.01,
            "{from} -> {value} -> {to}"
        );
    } else {
        assert!((value - to).abs() < 0.001, "{value} != {to}");
    }
}

/// How long an in-flight check waits for the element to move toward its target, measured from the
/// stage's update. 200 ms after a retarget, desktop runners with animations on had seen the element
/// move less than the 0.01 px margin; 1.5 s is still well short of the 3 s animation's end.
const PROGRESS_DEADLINE: Duration = Duration::from_millis(1500);

/// Whether an in-flight check can run now: with animations off at once (the value is the target),
/// otherwise once the value has moved from `from` toward `to` by the margin or the deadline has
/// passed (then `assert_progress` reports the miss). Movement away from `to` doesn't count: where
/// `from` is a reading taken before a retarget, the old animation runs on until the compositor
/// starts the new one, so the element is first seen further along its old direction (a desktop
/// runner read `-39.61442 -> -39.552986 -> -80`).
fn progress_ready(value: f32, from: f32, to: f32, animations: bool, elapsed: Duration) -> bool {
    !animations || (value - from) * (to - from).signum() > 0.01 || elapsed >= PROGRESS_DEADLINE
}

/// Where a retargeted element turned back: `value` if it lies beyond `seen` on the side away from
/// `to` (the old animation still running on), else `seen`. A check after a retarget starts from
/// there, so it needs only the margin of real progress toward the new target, however far the old
/// animation ran on before the retarget took effect. A reading past `to` (an overshoot) is not a
/// turn, so it can't widen the range `assert_progress` accepts.
fn furthest_from(seen: f32, value: f32, to: f32) -> f32 {
    if (value - seen) * (to - seen).signum() < 0.0 {
        value
    } else {
        seen
    }
}

#[test]
fn furthest_from_tracks_where_the_element_turned_back() {
    // Still running on toward 160 after the retarget to -80: the turn moves with it.
    assert_eq!(furthest_from(-39.614_42, -39.552_986, -80.0), -39.552_986);
    // On its way back to -80: the turn stays put.
    assert_eq!(furthest_from(-39.552_986, -39.6, -80.0), -39.552_986);
    assert_eq!(furthest_from(10.0, 5.0, 100.0), 5.0);
    // Past the target is not a turn: the range stays from the turn to the target.
    assert_eq!(furthest_from(-39.5, -130.0, -80.0), -39.5);
}

#[test]
fn progress_waits_for_movement_toward_the_target() {
    let early = Duration::from_millis(200);
    // Animations off: the instant path is checked at once.
    assert!(progress_ready(-40.0, -40.0, -80.0, false, early));
    // The values a desktop runner saw 200 ms into stage 2: not yet moved by the margin.
    assert!(!progress_ready(-39.547_523, -39.537_91, -80.0, true, early));
    assert!(progress_ready(-39.6, -39.537_91, -80.0, true, early));
    // Still moving the old way after a retarget (seen on a desktop runner): keep polling.
    assert!(!progress_ready(-39.552_986, -39.614_42, -80.0, true, early));
    assert!(!progress_ready(-39.4, -39.537_91, -80.0, true, early));
    // Toward a target above the start counts the other way round.
    assert!(progress_ready(-39.9, -40.0, 160.0, true, early));
    assert!(!progress_ready(-40.1, -40.0, 160.0, true, early));
    // At the deadline the check runs whether or not it moved, so an animation that is stuck or
    // ignores a retarget fails in assert_progress.
    assert!(!progress_ready(
        -40.0,
        -40.0,
        -80.0,
        true,
        PROGRESS_DEADLINE - Duration::from_millis(1)
    ));
    assert!(progress_ready(-40.0, -40.0, -80.0, true, PROGRESS_DEADLINE));
    assert!(progress_ready(
        -39.4,
        -39.537_91,
        -80.0,
        true,
        PROGRESS_DEADLINE
    ));
    assert!(std::panic::catch_unwind(|| assert_progress(-39.4, -39.537_91, -80.0, true)).is_err());
    // Stage 2 with the retarget ignored: the turn follows the element, so at the deadline the value
    // is its own start and fails.
    let turn = furthest_from(-39.537_91, -39.4, -80.0);
    assert!(std::panic::catch_unwind(|| assert_progress(-39.4, turn, -80.0, true)).is_err());
    // A jump straight to the target fails at once.
    assert!(progress_ready(-80.0, -39.5, -80.0, true, early));
    assert!(std::panic::catch_unwind(|| assert_progress(-80.0, -39.5, -80.0, true)).is_err());
}

#[test]
#[ignore = "requires an interactive WinUI desktop"]
fn transitions_use_current_batch_policy_and_retarget_in_flight() {
    if std::env::var_os("REACTOR_TRANSITION_TEST_CHILD").is_none() {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native::winui::tests::transitions::transitions_use_current_batch_policy_and_retarget_in_flight",
                "--ignored",
                "--nocapture",
            ])
            .env("REACTOR_TRANSITION_TEST_CHILD", "1")
            .status()
            .unwrap();
        assert!(status.success(), "{status}");
        return;
    }
    let completed = Rc::new(Cell::new(false));
    let finished = completed.clone();
    App::run_with(move |app| {
        let target = |border: Border| {
            border
                .width(100.0)
                .height(100.0)
                .horizontal_alignment(HorizontalAlignment::Left)
                .vertical_alignment(VerticalAlignment::Top)
                .background(Color::rgb(70, 130, 200))
        };
        let view = move |border| Border::new().content(target(border));
        let mut runtime = Runtime::new(WinUiAdapter::default());
        let animations = runtime.adapter().animations_enabled().unwrap();
        eprintln!("Native animations enabled: {animations}");
        runtime
            .update(view(
                Border::new()
                    .translation(-40.0, -20.0)
                    .translation_transition(Duration::from_secs(3))
                    .scale(1.5)
                    .scale_transition(Duration::from_secs(3))
                    .opacity(0.6)
                    .opacity_transition(Duration::from_secs(3)),
            ))
            .unwrap();
        let root = runtime.graph().root().unwrap();
        let window = runtime
            .adapter_mut()
            .create_window_with_policy(root, &WindowPolicy::new().client_size(600.0, 400.0))
            .unwrap();
        window.activate().unwrap();
        let window = RefCell::new(Some(window));
        let runtime = RefCell::new(Some(runtime));
        let stage = Cell::new(0);
        let started = Cell::new(Instant::now());
        let origin = Cell::new([0.0, 0.0]);
        let mid = Cell::new(0.0);
        let timer = native::DispatcherQueue::GetForCurrentThread()?.CreateTimer()?;
        timer.SetInterval(TimeSpan::try_from(Duration::from_millis(20)).unwrap())?;
        let app = app.clone();
        let tick = timer.Tick(move |sender, _| {
            let wait = if stage.get() == 8 { 3500 } else { 200 };
            if started.get().elapsed() < Duration::from_millis(wait) {
                return;
            }
            // An in-flight check: polls on later ticks until the element has moved from `from`
            // toward `to` (or the deadline passes), then asserts it is between `from` and `to`.
            macro_rules! progress {
                ($value:expr, $from:expr, $to:expr) => {{
                    let (value, from, to) = ($value, $from, $to);
                    if !progress_ready(value, from, to, animations, started.get().elapsed()) {
                        return;
                    }
                    assert_progress(value, from, to, animations);
                }};
            }
            let mut holder = runtime.borrow_mut();
            let Some(runtime) = holder.as_mut() else {
                sender.ok().unwrap().Stop().unwrap();
                window.borrow_mut().take().unwrap().close().unwrap();
                finished.set(true);
                app.exit().unwrap();
                return;
            };
            let object = runtime.graph().child(root, RelationId::Content).unwrap();
            let point = runtime
                .adapter()
                .transformed_point(object, 0.0, 0.0)
                .unwrap();
            let offset = [point[0] - origin.get()[0], point[1] - origin.get()[1]];
            let slow = Duration::from_secs(3);
            let next = match stage.get() {
                0 => {
                    assert_eq!(
                        runtime.adapter().translation(object).unwrap(),
                        [-40.0, -20.0]
                    );
                    let right = runtime
                        .adapter()
                        .transformed_point(object, 100.0, 0.0)
                        .unwrap();
                    assert!((right[0] - point[0] - 150.0).abs() < 0.01);
                    origin.set([point[0] + 40.0, point[1] + 20.0]);
                    Border::new()
                        .translation(160.0, 80.0)
                        .translation_transition(slow)
                }
                1 => {
                    progress!(offset[0], -40.0, 160.0);
                    progress!(offset[1], -20.0, 80.0);
                    mid.set(offset[0]);
                    Border::new()
                        .translation(-80.0, -40.0)
                        .translation_transition(slow)
                }
                2 => {
                    // `mid` was read before the retarget; the turn is where the element is seen
                    // furthest along its old direction.
                    mid.set(furthest_from(mid.get(), offset[0], -80.0));
                    progress!(offset[0], mid.get(), -80.0);
                    Border::new()
                        .translation(40.0, 20.0)
                        .translation_transition(Duration::ZERO)
                }
                3 => {
                    assert_point(offset, [40.0, 20.0]);
                    Border::new()
                        .translation(-160.0, -80.0)
                        .translation_transition(slow)
                }
                4 => {
                    progress!(offset[0], 40.0, -160.0);
                    Border::new().translation(20.0, 10.0)
                }
                5 => {
                    assert_point(offset, [20.0, 10.0]);
                    // Add a transition while clearing its target: resetting to zero must animate.
                    Border::new().translation_transition(slow)
                }
                6 => {
                    progress!(offset[0], 20.0, 0.0);
                    mid.set(offset[0]);
                    Border::new()
                }
                7 => {
                    progress!(offset[0], mid.get(), 0.0);
                    Border::new()
                }
                8 => {
                    assert_point(offset, [0.0, 0.0]);
                    assert_eq!(runtime.adapter().translation(object).unwrap(), [0.0, 0.0]);
                    Border::new().scale(1.5).scale_transition(slow)
                }
                9 => {
                    let right = runtime
                        .adapter()
                        .transformed_point(object, 100.0, 0.0)
                        .unwrap();
                    let width = right[0] - point[0];
                    progress!(width, 100.0, 150.0);
                    Border::new()
                        .scale(1.0)
                        .scale_transition(Duration::ZERO)
                        .opacity(0.2)
                        .opacity_transition(slow)
                }
                10 => {
                    let right = runtime
                        .adapter()
                        .transformed_point(object, 100.0, 0.0)
                        .unwrap();
                    assert!((right[0] - point[0] - 100.0).abs() < 0.01);
                    let opacity = runtime.adapter().opacity(object).unwrap();
                    // Opacity readback is the target, unlike transformed points during motion.
                    assert!((opacity - 0.2).abs() < 0.001);
                    Border::new()
                        .opacity(0.8)
                        .opacity_transition(Duration::ZERO)
                }
                11 => {
                    assert!((runtime.adapter().opacity(object).unwrap() - 0.8).abs() < 0.001);
                    Border::new()
                        .translation(100.0, 50.0)
                        .translation_transition(slow)
                }
                12 => {
                    progress!(offset[0], 0.0, 100.0);
                    Border::new()
                }
                13 => {
                    assert_point(offset, [0.0, 0.0]);
                    runtime
                        .update(Border::new().content(TextBlock::new().text("replacement")))
                        .unwrap();
                    Border::new()
                        .translation(80.0, 40.0)
                        .translation_transition(slow)
                }
                14 => {
                    assert_point(offset, [80.0, 40.0]);
                    runtime
                        .update_subtree_before_apply(
                            object,
                            TextBlock::new().text("replacement"),
                            || {},
                        )
                        .unwrap();
                    let mutations = runtime
                        .update_subtree_before_apply(
                            object,
                            target(
                                Border::new()
                                    .translation(-40.0, -20.0)
                                    .translation_transition(slow)
                                    .scale(1.5)
                                    .scale_transition(slow),
                            ),
                            || {},
                        )
                        .unwrap();
                    assert!(mutations.iter().any(
                        |mutation| matches!(mutation, Mutation::Replace { object: replaced, .. }
                            if *replaced == object)
                    ));
                    assert_eq!(
                        runtime.graph().child(root, RelationId::Content),
                        Some(object)
                    );
                    stage.set(15);
                    started.set(Instant::now());
                    return;
                }
                15 => {
                    assert_point(offset, [-40.0, -20.0]);
                    let right = runtime
                        .adapter()
                        .transformed_point(object, 100.0, 0.0)
                        .unwrap();
                    assert!((right[0] - point[0] - 150.0).abs() < 0.01);
                    runtime.update(Border::new()).unwrap();
                    holder.take();
                    stage.set(16);
                    started.set(Instant::now());
                    return;
                }
                _ => unreachable!(),
            };
            runtime.update(view(next)).unwrap();
            if stage.get() == 13 {
                assert_ne!(
                    runtime.graph().child(root, RelationId::Content),
                    Some(object)
                );
            } else {
                assert_eq!(
                    runtime.graph().child(root, RelationId::Content),
                    Some(object)
                );
            }
            stage.set(stage.get() + 1);
            started.set(Instant::now());
        })?;
        timer.Start()?;
        Ok((timer, tick))
    })
    .unwrap();
    assert!(completed.get());
}
