#[derive(woab::Factories)]
pub struct Factories {
    #[factory(extra(buf_sum))]
    win_app: woab::ActorFactory<WindowActor>,
    #[factory(extra(buf_addend))]
    row_addend: woab::ActorFactory<AddendActor>,
}

#[derive(woab::WidgetsFromBuilder)]
pub struct WindowWidgets {
    win_app: gtk::ApplicationWindow,
    buf_sum: gtk::TextBuffer,
    lst_addition: gtk::ListBox,
}

struct WindowActor {
    factories: std::rc::Rc<Factories>,
    widgets: WindowWidgets,
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
    ClickButton(gtk::ListBox),
}

impl actix::StreamHandler<WindowSingal> for WindowActor {
    fn handle(&mut self, signal: WindowSingal, ctx: &mut Self::Context) {
        use actix::prelude::*;
        use gtk::prelude::*;;
        match signal {
            WindowSingal::ClickButton(_button) => {
                let addend = self.factories.row_addend.create(|_, widgets| {
                    self.widgets.lst_addition.add(&widgets.row_addend);
                    AddendActor {
                        window: ctx.address(),
                        number: Some(0),
                    }
                }).unwrap();
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let factories = std::rc::Rc::new(Factories::read(std::io::BufReader::new(std::fs::File::open("woab/examples/example.glade")?))?);

    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop("example")?;

    factories.win_app.create(|_, widgets| WindowActor {
        widgets,
        factories: factories.clone(),
        addends: Vec::new(),
    })?;

    gtk::main();
    Ok(())
}
