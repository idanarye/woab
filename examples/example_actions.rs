use actix::prelude::*;
use gio::prelude::*;
use gtk::prelude::*;

struct WindowActor {
    widgets: WindowWidgets,
    simple_data: isize,
    parameter_data: Vec<String>,
}

#[derive(woab::WidgetsFromBuilder)]
struct WindowWidgets {
    simple: gtk::Entry,
    parameter: gtk::Entry,
    state: gtk::Label,
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;
}

impl actix::Handler<woab::Signal> for WindowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
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
            "state" => {
                let param: String = msg.action_param()?;
                let color = match param.as_str() {
                    "red" => Some(gdk::RGBA::red()),
                    "green" => Some(gdk::RGBA::green()),
                    "blue" => Some(gdk::RGBA::blue()),
                    _ => panic!("Invalid color {:?}", param),
                };
                self.widgets
                    .state
                    .override_background_color(gtk::StateFlags::NORMAL, color.as_ref());
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let factory = woab::BuilderFactory::from(std::fs::read_to_string("examples/example_actions.glade")?);

    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop()?;
    let app = gtk::Application::new(None, Default::default()).unwrap();

    app.connect_activate(move |app| {
        woab::block_on(async {
            factory.instantiate().connect_with(|bld| {
                let win_app: gtk::ApplicationWindow = bld.get_object("win_app").unwrap();
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
                    gio::SimpleAction::new_stateful("state", Some(&String::static_variant_type()), &"".to_variant()),
                ] {
                    app.add_action(action);
                    woab::route_action(action, addr.clone()).unwrap();
                }

                addr
            });
        });
    });

    app.run(&[]);
    Ok(())
}
