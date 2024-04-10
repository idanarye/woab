use actix::prelude::*;
use gtk4::prelude::*;

#[derive(woab::Factories)]
struct Factories {
    win_app: woab::BuilderFactory,
    dialog: woab::BuilderFactory,
}

struct WindowActor {
    widgets: WindowWidgets,
    dialog_factory: woab::BuilderFactory,
    yes_count: usize,
    no_count: usize,
}

#[derive(woab::WidgetsFromBuilder)]
struct WindowWidgets {
    yes_count: gtk4::Entry,
    no_count: gtk4::Entry,
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;
}

impl actix::Handler<woab::Signal> for WindowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "close" => {
                // gtk4::main_quit();
                None
            }
            "open_dialog" => {
                let bld = self.dialog_factory.instantiate();
                ctx.spawn(
                    async move {
                        let dialog: gtk4::Dialog = bld
                            .connect_with(|bld| {
                                DialogActor {
                                    widgets: bld.widgets().unwrap(),
                                }
                                .start()
                            })
                            .get_object("dialog")
                            .unwrap();
                        woab::run_dialog(&dialog, false).await
                    }
                    .into_actor(self)
                    .map(|response, actor, _ctx| match response {
                        gtk4::ResponseType::Yes => {
                            actor.yes_count += 1;
                            actor.widgets.yes_count.set_text(&actor.yes_count.to_string());
                        }
                        gtk4::ResponseType::No => {
                            actor.no_count += 1;
                            actor.widgets.no_count.set_text(&actor.no_count.to_string());
                        }
                        gtk4::ResponseType::DeleteEvent => {}
                        gtk4::ResponseType::None => {}
                        _ => panic!("Cannot handle dialog response {:?}", response),
                    }),
                );
                None
            }
            "reset" => {
                // ctx.spawn(
                // async move {
                // woab::run_dialog(
                // &gtk4::MessageDialog::new::<gtk4::ApplicationWindow>(
                // None,
                // gtk4::DialogFlags::all(),
                // gtk4::MessageType::Question,
                // gtk4::ButtonsType::YesNo,
                // "Reset the counters?",
                // ),
                // true,
                // )
                // .await
                // }
                // .into_actor(self)
                // .map(|response, actor, _ctx| {
                // if response == gtk4::ResponseType::Yes {
                // actor.yes_count = 0;
                // actor.no_count = 0;
                // actor.widgets.yes_count.set_text("0");
                // actor.widgets.no_count.set_text("0");
                // }
                // }),
                // );
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

struct DialogActor {
    widgets: DialogWidgets,
}

#[derive(woab::WidgetsFromBuilder)]
struct DialogWidgets {
    dialog: gtk4::Dialog,
    alive_time: gtk4::Label,
}

impl actix::Actor for DialogActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let alive_time = self.widgets.alive_time.clone();
        ctx.spawn(
            async move {
                let alive_since = std::time::SystemTime::now(); //, gtk4::main_level(), caption);
                loop {
                    alive_time.set_text(&format!("Alive for {} seconds", alive_since.elapsed().unwrap().as_secs()));
                    actix::clock::sleep(core::time::Duration::new(1, 0)).await;
                }
            }
            .into_actor(self),
        );
    }
}

impl actix::Handler<woab::Signal> for DialogActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "yes" => {
                self.widgets.dialog.response(gtk4::ResponseType::Yes);
                self.widgets.dialog.close();
                ctx.stop();
                None
            }
            "no" => {
                self.widgets.dialog.response(gtk4::ResponseType::No);
                self.widgets.dialog.close();
                ctx.stop();
                None
            }
            // These are here to ensure that `woab::run_dialog` does not fire them from inside the
            // Actix runtime.
            "dialog_realized" => None,
            "dialog_shown" => None,
            _ => msg.cant_handle()?,
        })
    }
}

fn main() -> anyhow::Result<()> {
    let factories = Factories::read(std::io::BufReader::new(std::fs::File::open("examples/example_dialog.glade")?))?;

    gtk4::init()?;
    woab::run_actix_inside_gtk_event_loop();

    woab::block_on(async move {
        factories
            .win_app
            .instantiate()
            .with_object("win_app", |win: gtk4::ApplicationWindow| win.show())
            .connect_with(|bld| {
                WindowActor {
                    widgets: bld.widgets().unwrap(),
                    dialog_factory: factories.dialog,
                    yes_count: 0,
                    no_count: 0,
                }
                .start()
            })
    });

    // gtk4::main();
    woab::close_actix_runtime()??;
    Ok(())
}
