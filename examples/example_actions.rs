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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let factory = woab::BuilderFactory::from(std::fs::read_to_string("examples/example_actions.glade")?);

    gtk4::init()?;
    woab::run_actix_inside_gtk_event_loop();
    let app = gtk4::Application::new(None, Default::default());

    app.connect_activate(move |app| {
        woab::block_on(async {
            factory.instantiate().connect_with(|bld| {
                let win_app: gtk4::ApplicationWindow = bld.get_object("win_app").unwrap();
                app.add_window(&win_app);

                win_app.show();
                let addr = WindowActor {
                    widgets: bld.widgets().unwrap(),
                    simple_data: 0,
                    parameter_data: Vec::new(),
                }
                .start();

                for action in &[
                    gio::SimpleAction::new("increment", None),
                    gio::SimpleAction::new("decrement", None),
                    gio::SimpleAction::new("parameter", Some(&String::static_variant_type())),
                    gio::SimpleAction::new_stateful("alignment", Some(&String::static_variant_type()), &"".to_variant()),
                ] {
                    app.add_action(action);
                    woab::route_action(action, addr.clone()).unwrap();
                }

                addr
            });
        });
    });

    app.run();
    Ok(())
}
