use actix::prelude::*;
use gtk4::prelude::*;

struct WindowActor {
    widgets: WindowWidgets,
}

impl Actor for WindowActor {
    type Context = Context<Self>;
}

#[derive(woab::WidgetsFromBuilder)]
struct WindowWidgets {
    win_app: gtk4::ApplicationWindow,
    lst_output: gtk4::ListBox,
}

impl Handler<woab::Signal> for WindowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            _ => msg.cant_handle()?,
        })
    }
}

struct WriteRow(String);

impl Message for WriteRow {
    type Result = ();
}

impl Handler<WriteRow> for WindowActor {
    type Result = ();

    fn handle(&mut self, msg: WriteRow, _ctx: &mut Self::Context) -> Self::Result {
        self.widgets.lst_output.append(&gtk4::Label::new(Some(&msg.0)));
    }
}

struct MyActor;
impl Actor for MyActor {
    type Context = Context<Self>;
}

fn main() -> woab::Result<()> {
    let factory = woab::BuilderFactory::from(std::fs::read_to_string("examples/example_shared_process.ui")?);
    let app = gtk4::Application::builder()
        .application_id("woab.example_shared_process")
        .build();
    woab::main_shared(app, move |app| {
        woab::shutdown_when_last_window_is_closed(app);
        let addr = WindowActor::create(|ctx| {
            let bld = factory.instantiate_route_to(ctx.address());
            bld.set_application(app);
            let widgets: WindowWidgets = bld.widgets().unwrap();
            widgets.win_app.show();
            WindowActor { widgets }
        });
        addr.do_send(WriteRow("Initial activation".to_owned()));
        Ok(addr)
    }, |addr, _| {
        addr.do_send(WriteRow("Another activation".to_owned()));
    })
}
