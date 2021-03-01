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

#[derive(woab::BuilderSignal)]
enum WindowSignal {
    #[signal(inhibit = false)]
    WindowConfigure(gtk::ApplicationWindow, #[signal(event)] gdk::EventConfigure),
}

impl actix::StreamHandler<WindowSignal> for WindowActor {
    fn handle(&mut self, signal: WindowSignal, _ctx: &mut Self::Context) {
        match signal {
            WindowSignal::WindowConfigure(_, event) => {
                let (left, top) = event.get_position();
                let (width, height) = event.get_size();
                self.widgets
                    .size_descr
                    .set_text(&format!("Left: {}, Top: {}\rWidth: {}, Height: {}", left, top, width, height));
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let factories = std::rc::Rc::new(Factories::read(std::io::BufReader::new(std::fs::File::open(
        "examples/example_continuous_events.glade",
    )?))?);

    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop("example")?;

    factories
        .win_app
        .instantiate()
        .actor()
        .connect_signals(WindowSignal::connector())
        .create(|ctx| WindowActor {
            widgets: ctx.widgets().unwrap(),
        });

    gtk::main();
    Ok(())
}
