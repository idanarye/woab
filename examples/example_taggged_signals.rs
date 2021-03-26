use gtk::prelude::*;

#[derive(woab::Factories)]
pub struct Factories {
    #[factory(extra(buf_sum))]
    win_app: woab::BuilderFactory,
    #[factory(extra(buf_addend))]
    row_addend: woab::BuilderFactory,
}

#[derive(woab::WidgetsFromBuilder)]
pub struct WindowWidgets {
    win_app: gtk::ApplicationWindow,
    buf_sum: gtk::TextBuffer,
    lst_addition: gtk::ListBox,
}

struct WindowActor {
    factories: Factories,
    widgets: WindowWidgets,
    next_addend_id: usize,
    addends: std::collections::HashMap<usize, (AddendWidgets, Option<isize>)>,
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        self.widgets.win_app.show();
        self.recalculate();
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        gtk::main_quit();
    }
}

#[derive(woab::BuilderSignal)]
enum WindowSignal {
    ClickButton,
}

impl actix::StreamHandler<WindowSignal> for WindowActor {
    fn handle(&mut self, signal: WindowSignal, ctx: &mut Self::Context) {
        match signal {
            WindowSignal::ClickButton => {
                let addend_id = self.next_addend_id;
                self.next_addend_id += 1;
                let widgets: AddendWidgets = self
                    .factories
                    .row_addend
                    .instantiate()
                    .connect_signals(ctx, AddendSignal::connector().tag(addend_id))
                    .widgets()
                    .unwrap();
                self.widgets.lst_addition.add(&widgets.row_addend);
                self.addends.insert(addend_id, (widgets, Some(0)));
                self.recalculate();
            }
        }
    }
}

#[derive(woab::WidgetsFromBuilder)]
struct AddendWidgets {
    row_addend: gtk::ListBoxRow,
}

#[derive(Debug, woab::BuilderSignal)]
enum AddendSignal {
    AddendChanged(gtk::TextBuffer),
    RemoveAddend,
}

impl actix::StreamHandler<(usize, AddendSignal)> for WindowActor {
    fn handle(&mut self, (id, signal): (usize, AddendSignal), _ctx: &mut Self::Context) {
        match signal {
            AddendSignal::AddendChanged(buffer) => {
                let new_number: Option<isize> = buffer
                    .get_text(&buffer.get_start_iter(), &buffer.get_end_iter(), true)
                    .and_then(|text| text.parse().ok());
                if let Some((_, number)) = self.addends.get_mut(&id) {
                    if new_number != *number {
                        *number = new_number;
                        self.recalculate();
                    }
                }
            }
            AddendSignal::RemoveAddend => {
                if let Some((widgets, _)) = self.addends.remove(&id) {
                    self.widgets.lst_addition.remove(&widgets.row_addend);
                    self.recalculate();
                }
            }
        }
    }

    fn finished(&mut self, _ctx: &mut Self::Context) {}
}

impl WindowActor {
    fn recalculate(&mut self) {
        let sum: Option<isize> = self.addends.values().map(|(_, n)| *n).sum();
        if let Some(sum) = sum {
            self.widgets.buf_sum.set_text(&format!("{}", sum));
        } else {
            self.widgets.buf_sum.set_text("#N/A");
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let factories = Factories::read(std::io::BufReader::new(std::fs::File::open("examples/example.glade")?))?;

    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop()?;

    woab::block_on(async {
        factories
            .win_app
            .instantiate()
            .actor()
            .connect_signals(WindowSignal::connector())
            .create(|ctx| WindowActor {
                widgets: ctx.widgets().unwrap(),
                factories,
                next_addend_id: 0,
                addends: Default::default(),
            });
    });

    gtk::main();
    Ok(())
}
