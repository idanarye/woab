use actix::prelude::*;
use gtk::prelude::*;

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
    yes_count: gtk::Entry,
    no_count: gtk::Entry,
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;
}

impl actix::Handler<woab::Signal> for WindowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "close" => {
                gtk::main_quit();
                None
            }
            "open_dialog" => {
                self.dialog_factory
                    .instantiate()
                    .with_object("dialog", |dialog: gtk::Dialog| {
                        let self_addr = ctx.address();
                        woab::schedule_outside(move || {
                            let actix_spinner_source = std::rc::Rc::new(std::cell::Cell::new(None));

                            dialog.connect_realize({
                                let actix_spinner_source = actix_spinner_source.clone();
                                move |_| {
                                    let source_id = woab::run_actix_inside_gtk_event_loop().unwrap();
                                    let old_source = actix_spinner_source.replace(Some(source_id));
                                    if old_source.is_some() {
                                        panic!("`realize` called twice without unrealize");
                                    }
                                }
                            });
                            dialog.connect_unrealize(move |_| {
                                let source_id = actix_spinner_source.take().expect("`unrealize` called without `realize`");
                                glib::source_remove(source_id);
                            });
                            dialog.set_modal(false);
                            let response = dialog.run();
                            self_addr.do_send(DialogResponse(response));
                        });
                    })
                    .connect_with(|bld| {
                        DialogActor {
                            widgets: bld.widgets().unwrap(),
                        }
                        .start()
                    });
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

struct DialogResponse(gtk::ResponseType);

impl actix::Message for DialogResponse {
    type Result = ();
}

impl actix::Handler<DialogResponse> for WindowActor {
    type Result = ();

    fn handle(&mut self, msg: DialogResponse, _ctx: &mut Self::Context) -> Self::Result {
        match msg.0 {
            gtk::ResponseType::Yes => {
                self.yes_count += 1;
                self.widgets.yes_count.set_text(&self.yes_count.to_string());
            }
            gtk::ResponseType::No => {
                self.no_count += 1;
                self.widgets.no_count.set_text(&self.no_count.to_string());
            }
            _ => panic!("Cannot handle dialog response {:?}", msg.0),
        }
    }
}

struct DialogActor {
    widgets: DialogWidgets,
}

#[derive(woab::WidgetsFromBuilder)]
struct DialogWidgets {
    dialog: gtk::Dialog,
    alive_time: gtk::Label,
}

impl actix::Actor for DialogActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let alive_time = self.widgets.alive_time.clone();
        ctx.spawn(
            async move {
                let alive_since = std::time::SystemTime::now(); //, gtk::main_level(), caption);
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
                self.widgets.dialog.response(gtk::ResponseType::Yes);
                self.widgets.dialog.close();
                ctx.stop();
                None
            }
            "no" => {
                self.widgets.dialog.response(gtk::ResponseType::No);
                self.widgets.dialog.close();
                ctx.stop();
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

fn main() -> anyhow::Result<()> {
    let factories = Factories::read(std::io::BufReader::new(std::fs::File::open("examples/example_dialog.glade")?))?;

    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop()?;

    woab::block_on(async move {
        factories
            .win_app
            .instantiate()
            .with_object("win_app", |win: gtk::ApplicationWindow| win.show())
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

    gtk::main();

    Ok(())
}
