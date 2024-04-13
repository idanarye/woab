use actix::prelude::*;
use gio::prelude::*;
use gtk4::prelude::*;

struct WindowActor {
    widgets: WindowWidgets,
    simple_data: isize,
    parameter_data: Vec<String>,
}

#[derive(woab::WidgetsFromBuilder)]
struct WindowWidgets {
    win_app: gtk4::ApplicationWindow,
    simple: gtk4::Entry,
    parameter: gtk4::Entry,
    alignment: gtk4::Label,
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;
}

impl actix::Handler<woab::Signal> for WindowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "close" => {
                let woab::params!(win_app: gtk4::ApplicationWindow) = msg.params()?;
                win_app.application().unwrap().quit();
                None
            }
            "increment" => {
                self.simple_data += 1;
                self.widgets.simple.set_text(&format!("{}", self.simple_data));
                None
            }
            "decrement" => {
                self.simple_data -= 1;
                self.widgets.simple.set_text(&format!("{}", self.simple_data));
                None
            }
            "parameter" => {
                let param: String = msg.action_param()?;
                self.parameter_data.push(param);
                self.widgets.parameter.set_text(&self.parameter_data.join(""));
                None
            }
            "alignment" => {
                let param: String = msg.action_param()?;
                self.widgets.alignment.set_halign(match param.as_str() {
                    "left" => gtk4::Align::Start,
                    "center" => gtk4::Align::Center,
                    "right" => gtk4::Align::End,
                    _ => panic!("Invalid alignment {:?}", param),
                });
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

fn main() -> woab::Result<()> {
    let factory = woab::BuilderFactory::from(std::fs::read_to_string("examples/example_actions.ui")?);

    woab::main(Default::default(), move |app| {
        woab::shutdown_when_last_window_is_closed(app);
        WindowActor::create(|ctx| {
            let bld = factory.instantiate_route_to(ctx.address());
            bld.set_application(app);

            for action in &[
                gio::SimpleAction::new("increment", None),
                gio::SimpleAction::new("decrement", None),
                gio::SimpleAction::new("parameter", Some(&String::static_variant_type())),
                gio::SimpleAction::new_stateful("alignment", Some(&String::static_variant_type()), &"".to_variant()),
            ] {
                app.add_action(action);
                woab::route_action(action, ctx.address()).unwrap();
            }

            let widgets: WindowWidgets = bld.widgets().unwrap();
            widgets.win_app.show();
            WindowActor {
                widgets,
                simple_data: 0,
                parameter_data: Vec::new(),
            }
        });
    })
}
