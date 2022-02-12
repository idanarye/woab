use actix::prelude::*;
use gtk4::prelude::*;

#[derive(woab::Factories)]
struct Factories {
    #[factory(extra(size_descr))]
    win_app: woab::BuilderFactory,
}

#[derive(woab::WidgetsFromBuilder)]
struct WindowWidgets {
    win_app: gtk4::ApplicationWindow,
    size_descr: gtk4::TextBuffer,
}

struct WindowActor {
    widgets: WindowWidgets,
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        self.widgets.win_app.show();
    }
}

impl actix::Handler<woab::Signal> for WindowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "close" => {
                gtk4::main_quit();
                None
            }
            "window_configure" => {
                let event: gdk4::EventConfigure = msg.event_param()?;
                let (left, top) = event.position();
                let (width, height) = event.size();
                self.widgets
                    .size_descr
                    .set_text(&format!("Left: {}, Top: {}\rWidth: {}, Height: {}", left, top, width, height));
                Some(gtk4::Inhibit(false))
            }
            "decrease_width" => {
                let (width, height) = self.widgets.win_app.size();
                self.widgets.win_app.resize(width - 10, height);
                None
            }
            "increase_width" => {
                let (width, height) = self.widgets.win_app.size();
                self.widgets.win_app.resize(width + 10, height);
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let factories = std::rc::Rc::new(Factories::read(std::io::BufReader::new(std::fs::File::open(
        "examples/example_continuous_events.glade",
    )?))?);

    gtk4::init()?;
    woab::run_actix_inside_gtk_event_loop();

    woab::block_on(async {
        factories.win_app.instantiate().connect_with(|bld| {
            WindowActor {
                widgets: bld.widgets().unwrap(),
            }
            .start()
        });
    });

    gtk4::main();
    Ok(())
}
