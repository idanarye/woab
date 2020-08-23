#[derive(woab::WidgetsFromBuilder)]
pub struct WindowWidgets {
    win_app: gtk::ApplicationWindow,
    buf_sum: gtk::TextBuffer,
    lst_addition: gtk::ListBox,
}

struct WindowActor {
    widgets: WindowWidgets,
    addend_factory: woab::ActorFactory<AddendActor>,
    addends: Vec<actix::Addr<AddendActor>>,
}

impl woab::WoabActor for WindowActor {
    type Widgets = WindowWidgets;
    type Signal = WindowSingal;
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        use actix::prelude::*;
        use gtk::WidgetExt;
        self.widgets.win_app.show();
        ctx.address().do_send(Recalculate);
    }
}

#[derive(woab::BuilderSignal)]
enum WindowSingal {
    ClickButton(gtk::Button),
}

impl actix::StreamHandler<WindowSingal> for WindowActor {
    fn handle(&mut self, signal: WindowSingal, ctx: &mut Self::Context) {
        use actix::prelude::*;
        use gtk::prelude::*;;
        match signal {
            WindowSingal::ClickButton(_button) => {
                let addend = self.addend_factory.create(|_, widgets| {
                    self.widgets.lst_addition.add(&widgets.row_addend);
                    AddendActor {
                        window: ctx.address(),
                        number: Some(0),
                    }
                });
                self.addends.push(addend);
                ctx.address().do_send(Recalculate);
            }
        }
    }
}

struct AddendActor {
    window: actix::Addr<WindowActor>,
    number: Option<isize>,
}

impl actix::Actor for AddendActor {
    type Context = actix::Context<Self>;
}

impl woab::WoabActor for AddendActor {
    type Widgets = AddendWidgets;
    type Signal = AddendSingal;
}

#[derive(woab::WidgetsFromBuilder)]
struct AddendWidgets {
    row_addend: gtk::ListBoxRow,
}

#[derive(woab::BuilderSignal)]
enum AddendSingal {
    AddendChanged(gtk::TextBuffer),
}

impl actix::StreamHandler<AddendSingal> for AddendActor {
    fn handle(&mut self, signal: AddendSingal, _ctx: &mut Self::Context) {
        match signal {
            AddendSingal::AddendChanged(buffer) => {
                use gtk::TextBufferExt;
                let new_number = buffer
                    .get_text(&buffer.get_start_iter(), &buffer.get_end_iter(), true)
                    .and_then(|text| text.parse().ok());
                if new_number != self.number {
                    self.number = new_number;
                    self.window.do_send(Recalculate);
                }
            }
        }
    }
}

struct Recalculate;

impl actix::Message for Recalculate {
    type Result = ();
}

impl actix::Handler<Recalculate> for WindowActor {
    type Result = ();

    fn handle(&mut self, _: Recalculate, ctx: &mut Self::Context) -> Self::Result {
        use actix::prelude::*;
        use gtk::prelude::*;

        let futures = futures_util::future::join_all(self.addends.iter().map(|addend| addend.send(GetNumber)).collect::<Vec<_>>());
        ctx.spawn(
            futures
            .into_actor(self)
            .map(|result, actor, _ctx| {
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
            })
        );
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

fn main() {
    let f = std::fs::File::open("woab/examples/example.glade").unwrap();
    let b = std::io::BufReader::new(f);
    let mut builders_bytes = [
        Vec::new(),
        Vec::new(),
    ];
    woab::dissect_builder_xml(b, &mut builders_bytes, |id| match id {
        "win_app" | "buf_sum" => Some(0),
        "row_addend" | "buf_addend" => Some(1),
        _ => None,
    });

    gtk::init().unwrap();
    woab::run_actix_inside_gtk_event_loop("example").unwrap();

    let app_factory: woab::ActorFactory<WindowActor> = std::str::from_utf8(&builders_bytes[0]).unwrap().to_owned().into();
    let addend_factory: woab::ActorFactory<AddendActor> = std::str::from_utf8(&builders_bytes[1]).unwrap().to_owned().into();

    app_factory.create(|_, widgets| WindowActor {
        widgets,
        addend_factory,
        addends: Vec::new(),
    });
    gtk::main();
}
