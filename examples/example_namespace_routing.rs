use actix::prelude::*;
use gtk4::prelude::*;

#[derive(woab::WidgetsFromBuilder)]
struct WindowWidgets {
    win_app: gtk4::ApplicationWindow,
    lst_output: gtk4::ListBox,
}

struct WindowActor {
    widgets: WindowWidgets,
}

impl Actor for WindowActor {
    type Context = Context<Self>;
}

struct AddToOutput(&'static str);

impl Message for AddToOutput {
    type Result = ();
}

impl Handler<AddToOutput> for WindowActor {
    type Result = ();

    fn handle(&mut self, msg: AddToOutput, _ctx: &mut Self::Context) -> Self::Result {
        self.widgets.lst_output.append(&gtk4::Label::new(Some(msg.0)));
    }
}

struct NamespacedActor(Recipient<AddToOutput>);

impl Actor for NamespacedActor {
    type Context = Context<Self>;
}

impl Handler<woab::Signal> for NamespacedActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "button_click" => {
                self.0.do_send(AddToOutput("From namespaced actor"));
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

struct ManualNamespaceActor {
    text: &'static str,
    send_to: Recipient<AddToOutput>,
}

impl Actor for ManualNamespaceActor {
    type Context = Context<Self>;
}

impl Handler<woab::Signal> for ManualNamespaceActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "button_click" => {
                self.send_to.do_send(AddToOutput(self.text));
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

fn main() -> woab::Result<()> {
    let factory = woab::BuilderFactory::from(std::fs::read_to_string("examples/example_namespace_routing.ui")?);

    woab::main(Default::default(), move |app| {
        woab::shutdown_when_last_window_is_closed(app);
        WindowActor::create(|ctx| {
            let bld = factory.instantiate_route_to({
                woab::NamespacedSignalRouter::default()
                    .route(NamespacedActor(ctx.address().recipient()).start())
                    .route_strip_ns(
                        "x",
                        ManualNamespaceActor {
                            text: "From manual namespace X",
                            send_to: ctx.address().recipient(),
                        }
                        .start()
                        .recipient(),
                    )
                    .route_strip_ns(
                        "y",
                        ManualNamespaceActor {
                            text: "From manual namespace Y",
                            send_to: ctx.address().recipient(),
                        }
                        .start()
                        .recipient(),
                    )
            });
            bld.set_application(app);
            let widgets: WindowWidgets = bld.widgets().unwrap();
            widgets.win_app.show();
            WindowActor { widgets }
        });
        Ok(())
    })
}
