use std::time::Duration;
use windows_reactor::*;

struct Fixture {
    phase: u8,
    _timeout: ComponentTimer,
    settle: Option<ComponentTimer>,
}

enum Message {
    Scheme(ColorScheme),
    Resubscribed(ColorScheme),
    Settled,
    Timeout,
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}

impl Component for Fixture {
    type Input = ();
    type Message = Message;

    fn create(_: &(), context: &ComponentContext<Self>) -> Self {
        Self {
            phase: 0,
            _timeout: context.set_timeout(Duration::from_secs(10), Message::Timeout),
            settle: None,
        }
    }

    fn update(&mut self, message: Message, context: &ComponentContext<Self>) {
        match message {
            Message::Scheme(ColorScheme::Dark) if self.phase == 0 => {
                self.phase = 1;
                self.settle =
                    Some(context.set_timeout(Duration::from_millis(300), Message::Settled));
            }
            Message::Scheme(ColorScheme::Light) if self.phase == 2 => {
                self.phase = 3;
                self.settle =
                    Some(context.set_timeout(Duration::from_millis(300), Message::Settled));
            }
            Message::Scheme(scheme) => fail(&format!(
                "unexpected color scheme notification in phase {}: {scheme:?}",
                self.phase
            )),
            Message::Settled if matches!(self.phase, 1 | 3) => self.phase += 1,
            Message::Resubscribed(ColorScheme::Light) if self.phase == 4 => {
                self.phase = 5;
                assert!(context.close_window());
            }
            Message::Resubscribed(scheme) => fail(&format!(
                "unexpected resubscribed color scheme in phase {}: {scheme:?}",
                self.phase
            )),
            Message::Settled => fail("unexpected settle timer"),
            Message::Timeout => fail(&format!(
                "color scheme fixture stalled in phase {}",
                self.phase
            )),
        }
    }

    fn view(&self, _: &(), context: &mut ViewContext<Self>) -> View {
        context.window_title("Color scheme fixture");
        if self.phase < 4 {
            context.on_color_scheme(context.callback(Message::Scheme));
        } else {
            context.on_color_scheme(context.callback(Message::Resubscribed));
        }
        context.window_visuals(WindowVisuals::new().theme(if self.phase < 2 {
            WindowTheme::Dark
        } else {
            WindowTheme::Light
        }));
        TextBlock::new()
            .text(format!("Color scheme fixture phase {}", self.phase))
            .into()
    }
}

fn main() -> windows_core::Result<()> {
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(20));
        eprintln!("color scheme fixture timed out");
        std::process::exit(1);
    });
    App::run_component::<Fixture>(())
}
