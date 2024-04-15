use actix::prelude::*;
use gtk4::prelude::*;

#[derive(woab::Factories)]
pub struct Factories {
    win_app: woab::BuilderFactory,
    row_addend: woab::BuilderFactory,
}

#[derive(woab::WidgetsFromBuilder)]
pub struct WindowWidgets {
    win_app: gtk4::ApplicationWindow,
    buf_sum: gtk4::TextBuffer,
    #[allow(unused)]
    lst_addition: gtk4::ListBox,
}

struct WindowActor {
    factories: std::rc::Rc<Factories>,
    widgets: WindowWidgets,
    addends: Vec<actix::Addr<AddendActor>>,
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.widgets.win_app.show();
        ctx.address().do_send(Recalculate);
    }
}

impl actix::Handler<woab::Signal> for WindowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "click_button" => {
                AddendActor::create(|addend_ctx| {
                    let bld = self.factories.row_addend.instantiate_route_to(addend_ctx.address());
                    self.addends.push(addend_ctx.address());
                    let widgets: AddendWidgets = bld.widgets().unwrap();
                    self.widgets.lst_addition.append(&widgets.row_addend);
                    AddendActor {
                        widgets,
                        window: ctx.address(),
                        number: Some(0),
                    }
                });
                ctx.address().do_send(Recalculate);
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

struct CheckForRemovedAddends;

impl actix::Message for CheckForRemovedAddends {
    type Result = ();
}

impl actix::Handler<CheckForRemovedAddends> for WindowActor {
    type Result = ();

    fn handle(&mut self, _msg: CheckForRemovedAddends, ctx: &mut Self::Context) -> Self::Result {
        self.addends.retain(|a| a.connected());
        ctx.address().do_send(Recalculate);
    }
}

struct AddendActor {
    #[allow(unused)]
    widgets: AddendWidgets,
    #[allow(unused)]
    window: actix::Addr<WindowActor>,
    number: Option<isize>,
}

impl actix::Actor for AddendActor {
    type Context = actix::Context<Self>;
}

#[derive(woab::WidgetsFromBuilder)]
struct AddendWidgets {
    #[allow(unused)]
    row_addend: gtk4::ListBoxRow,
}

impl actix::Handler<woab::Signal> for AddendActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "addend_changed" => {
                let woab::params!(buffer: gtk4::TextBuffer) = msg.params()?;
                let new_number = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true).parse().ok();
                if new_number != self.number {
                    self.number = new_number;
                    self.window.do_send(Recalculate);
                }
                None
            }
            "remove_addend" => {
                self.widgets
                    .row_addend
                    .parent()
                    .unwrap()
                    .downcast::<gtk4::ListBox>()
                    .unwrap()
                    .remove(&self.widgets.row_addend);
                ctx.stop();
                self.window.do_send(CheckForRemovedAddends);
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

struct Recalculate;

impl actix::Message for Recalculate {
    type Result = ();
}

impl actix::Handler<Recalculate> for WindowActor {
    type Result = ();

    fn handle(&mut self, _: Recalculate, ctx: &mut Self::Context) -> Self::Result {
        let futures =
            futures_util::future::join_all(self.addends.iter().map(|addend| addend.send(GetNumber)).collect::<Vec<_>>());
        ctx.spawn(futures.into_actor(self).map(|result, actor, _ctx| {
            let mut sum = 0;
            for addend in result {
                if let Some(addend) = addend.unwrap() {
                    sum += addend;
                } else {
                    actor.widgets.buf_sum.set_text("#N/A");
                    return;
                }
            }
            actor.widgets.buf_sum.set_text(&format!("{}", sum));
        }));
    }
}

struct GetNumber;

impl actix::Message for GetNumber {
    type Result = Option<isize>;
}

impl actix::Handler<GetNumber> for AddendActor {
    type Result = Option<isize>;

    fn handle(&mut self, _: GetNumber, _ctx: &mut Self::Context) -> Self::Result {
        self.number
    }
}

fn main() -> woab::Result<()> {
    let factories = std::rc::Rc::new(Factories::read(std::io::BufReader::new(std::fs::File::open(
        "examples/example.ui",
    )?))?);

    woab::main(Default::default(), move |app| {
        woab::shutdown_when_last_window_is_closed(app);
        let factories = factories.clone();
        WindowActor::create(|ctx| {
            let bld = factories.win_app.instantiate_route_to(ctx.address());
            bld.set_application(app);
            WindowActor {
                widgets: bld.widgets().unwrap(),
                factories,
                addends: Vec::new(),
            }
        });
    })
}
