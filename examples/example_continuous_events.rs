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
    #[allow(unused)]
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
                self.widgets.win_app.application().unwrap().quit();
                Some(glib::Propagation::Stop)
            }
            "window_notify" => {
                let event: glib::ParamSpec = msg.param(1)?;
                match event.name() {
                    "default-width" | "default-height" => {
                        let width = self.widgets.win_app.default_width();
                        let height = self.widgets.win_app.default_height();
                        self.widgets
                            .size_descr
                            .set_text(&format!("Width: {}, Height: {}", width, height));
                    }
                    _ => {}
                }
                None
            }
            "decrease_width" => {
                let width = self.widgets.win_app.default_width();
                self.widgets.win_app.set_default_width(width - 10);
                None
            }
            "increase_width" => {
                let width = self.widgets.win_app.default_width();
                self.widgets.win_app.set_default_width(width + 10);
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

fn main() -> woab::Result<()> {
    let factory = woab::BuilderFactory::from(std::fs::read_to_string("examples/example_continuous_events.ui")?);

    woab::main(Default::default(), move |app| {
        WindowActor::create(|addr| {
            let bld = factory.instantiate_route_to(addr.address());
            bld.set_application(app);
            WindowActor {
                widgets: bld.widgets().unwrap(),
            }
        });
    })
}
