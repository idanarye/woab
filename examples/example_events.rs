use std::time::{Duration, Instant};

use actix::prelude::*;
use gtk::prelude::*;

#[derive(woab::Factories)]
pub struct Factories {
    #[factory(extra(buf_count_pressed_time))]
    win_app: woab::BuilderFactory,
}

struct WindowActor;

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;
}

impl actix::Handler<woab::Signal> for WindowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "close" => {
                gtk::main_quit();
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

#[derive(woab::WidgetsFromBuilder)]
pub struct PressCountingWidgets {
    buf_count_pressed_time: gtk::TextBuffer,
}

struct PressCountingActor {
    widgets: PressCountingWidgets,
    press_times: [Option<Instant>; 2],
    total_durations: [Duration; 2],
}

impl actix::Actor for PressCountingActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        self.update_pressed_time_display();
    }
}

impl PressCountingActor {
    fn update_pressed_time_display(&self) {
        self.widgets
            .buf_count_pressed_time
            .set_text(&format!("L: {:?}, R: {:?}", self.total_durations[0], self.total_durations[1],));
    }
}

impl actix::Handler<woab::Signal> for PressCountingActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        fn button_to_idx(event: &gdk::EventButton) -> Option<usize> {
            match event.button() {
                1 => Some(0),
                3 => Some(1),
                _ => None,
            }
        }

        Ok(match msg.name() {
            "press" => {
                let event: gdk::EventButton = msg.event_param()?;
                if let Some(idx) = button_to_idx(&event) {
                    self.press_times[idx] = Some(Instant::now());
                }
                Some(glib::Propagation::Stop)
            }
            "release" => {
                let event: gdk::EventButton = msg.event_param()?;
                if let Some(idx) = button_to_idx(&event) {
                    if let Some(press_time) = self.press_times[idx] {
                        self.press_times[idx] = None;
                        let duration = Instant::now() - press_time;
                        self.total_durations[idx] += duration;
                        self.update_pressed_time_display();
                    }
                }
                Some(glib::Propagation::Stop)
            }
            _ => msg.cant_handle()?,
        })
    }
}

#[derive(woab::WidgetsFromBuilder)]
pub struct CharacterMoverWidgets {
    only_digits: gtk::Entry,
}

struct CharacterMoverActor {
    widgets: CharacterMoverWidgets,
}

impl actix::Actor for CharacterMoverActor {
    type Context = actix::Context<Self>;
}

impl actix::Handler<woab::Signal> for CharacterMoverActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "all_characters_entry_key_pressed" => {
                let event: gdk::EventKey = msg.event_param()?;
                if let Some(character) = event.keyval().to_unicode() {
                    if character.is_digit(10) {
                        let mut text = self.widgets.only_digits.text().as_str().to_owned();
                        text.push(character);
                        self.widgets.only_digits.set_text(&text);
                        return Ok(Some(glib::Propagation::Proceed));
                    }
                }
                Some(glib::Propagation::Stop)
            }
            _ => msg.cant_handle()?,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let factories = std::rc::Rc::new(Factories::read(std::io::BufReader::new(std::fs::File::open(
        "examples/example_events.glade",
    )?))?);

    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop();

    woab::block_on(async {
        factories.win_app.instantiate().connect_with(|bld| {
            bld.get_object::<gtk::ApplicationWindow>("win_app").unwrap().show();
            woab::NamespacedSignalRouter::default()
                .route(WindowActor.start())
                .route(
                    PressCountingActor {
                        widgets: bld.widgets().unwrap(),
                        press_times: Default::default(),
                        total_durations: Default::default(),
                    }
                    .start(),
                )
                .route(
                    CharacterMoverActor {
                        widgets: bld.widgets().unwrap(),
                    }
                    .start(),
                )
        });
    });

    gtk::main();
    woab::close_actix_runtime()??;
    Ok(())
}
