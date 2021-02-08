use std::time::{Instant, Duration};

use gtk::prelude::*;

#[derive(woab::Factories)]
pub struct Factories {
    #[factory(extra(buf_count_pressed_time))]
    win_app: woab::BuilderFactory,
}

#[derive(woab::WidgetsFromBuilder)]
pub struct WindowWidgets {
    win_app: gtk::ApplicationWindow,
    buf_count_pressed_time: gtk::TextBuffer,
    only_digits: gtk::Entry,
}

struct WindowActor {
    widgets: WindowWidgets,
    press_times: [Option<Instant>; 2],
    total_durations: [Duration; 2],
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        self.update_pressed_time_display();
        self.widgets.win_app.show();
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        gtk::main_quit();
    }
}

impl WindowActor {
    fn update_pressed_time_display(&self) {
        self.widgets.buf_count_pressed_time.set_text(&format!(
            "L: {:?}, R: {:?}",
            self.total_durations[0],
            self.total_durations[1],
        ));
    }
}

#[derive(woab::BuilderSignal)]
enum WindowSignal {
    #[signal(inhibit = false)]
    Press(gtk::Button, #[signal(event)] gdk::EventButton),
    #[signal(inhibit = false)]
    Release(gtk::Button, #[signal(event)] gdk::EventButton),
    AllCharactersEntryKeyPressed(gtk::Entry, #[signal(event)] gdk::EventKey),
}

impl actix::StreamHandler<WindowSignal> for WindowActor {
    fn handle(&mut self, signal: WindowSignal, _ctx: &mut Self::Context) {
        macro_rules! button_to_idx {
            ($event:ident) => {
                match $event.get_button() {
                    1 => 0,
                    3 => 1,
                    _ => {
                        return;
                    }
                }
            }
        }
        match signal {
            WindowSignal::Press(_, event) => {
                let idx = button_to_idx!(event);
                self.press_times[idx] = Some(Instant::now());
            }
            WindowSignal::Release(_, event) => {
                let idx = button_to_idx!(event);
                if let Some(press_time) = self.press_times[idx] {
                    self.press_times[idx] = None;
                    let duration = Instant::now() - press_time;
                    self.total_durations[idx] += duration;
                    self.update_pressed_time_display();
                }
            }
            WindowSignal::AllCharactersEntryKeyPressed(_, event) => {
                if let Some(character) = event.get_keyval().to_unicode() {
                    if character.is_digit(10) {
                        let mut text = self.widgets.only_digits.get_text().as_str().to_owned();
                        text.push(character);
                        self.widgets.only_digits.set_text(&text);
                    }
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let factories = std::rc::Rc::new(Factories::read(std::io::BufReader::new(std::fs::File::open("examples/example_events.glade")?))?);

    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop("example")?;

    factories.win_app.instantiate().actor()
        .connect_signals(WindowSignal::connector()
            .inhibit(|signal| {
                match signal {
                    WindowSignal::AllCharactersEntryKeyPressed(_, event) => {
                        let character = event.get_keyval().to_unicode();
                        let is_digit = character.map(|c| c.is_digit(10)).unwrap_or(false);
                        Some(gtk::Inhibit(is_digit))
                    }
                    _ => None,
                }
            })
        )
        .create(|ctx| WindowActor {
            widgets: ctx.widgets().unwrap(),
            press_times: Default::default(),
            total_durations: Default::default(),
        });

    gtk::main();
    Ok(())
}
