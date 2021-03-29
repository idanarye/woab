use actix::prelude::*;
use gtk::prelude::*;

#[derive(woab::Factories)]
struct Factories {
    #[factory(extra(size_descr))]
    win_app: woab::BuilderFactory,
}

#[derive(woab::WidgetsFromBuilder)]
struct WindowWidgets {
    win_app: gtk::ApplicationWindow,
    size_descr: gtk::TextBuffer,
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
            "window_configure" => {
                let event: gdk::EventConfigure = msg.param::<gdk::Event>(1)?.downcast().unwrap();
                let (left, top) = event.get_position();
                let (width, height) = event.get_size();
                self.widgets
                    .size_descr
                    .set_text(&format!("Left: {}, Top: {}\rWidth: {}, Height: {}", left, top, width, height));
                Some(gtk::Inhibit(false))
            }
            _ => msg.cant_handle()?,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let factories = std::rc::Rc::new(Factories::read(std::io::BufReader::new(std::fs::File::open(
        "examples/example_continuous_events.glade",
    )?))?);

    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop()?;

    woab::block_on(async {
        factories.win_app.instantiate().connect_with(|bld| {
            WindowActor {
                widgets: bld.widgets().unwrap(),
            }
            .start()
        });
    });

    gtk::main();
    Ok(())
}
