use actix::prelude::*;
use gtk4::prelude::*;

#[derive(woab::Factories)]
pub struct Factories {
    #[factory(extra(buf_sum))]
    win_app: woab::BuilderFactory,
    #[factory(extra(buf_addend))]
    row_addend: woab::BuilderFactory,
}

#[derive(woab::WidgetsFromBuilder)]
pub struct WindowWidgets {
    win_app: gtk4::ApplicationWindow,
    buf_sum: gtk4::TextBuffer,
    lst_addition: gtk4::ListBox,
}

struct WindowActor {
    factories: Factories,
    widgets: WindowWidgets,
    next_addend_id: usize,
    addends: std::collections::HashMap<usize, (gtk4::ListBoxRow, Option<isize>)>,
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        self.widgets.win_app.show();
        self.recalculate();
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        gtk4::main_quit();
    }
}

impl actix::Handler<woab::Signal> for WindowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "close" => {
                gtk4::main_quit();
                None
            }
            "click_button" => {
                let addend_id = self.next_addend_id;
                self.next_addend_id += 1;
                self.factories
                    .row_addend
                    .instantiate()
                    .with_object("row_addend", |row_addend| {
                        self.widgets.lst_addition.add(&row_addend);
                        self.addends.insert(addend_id, (row_addend, Some(0)));
                    })
                    .connect_to((addend_id, ctx.address()));
                self.recalculate();
                None
            }
            "addend_removed" => {
                self.recalculate();
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

impl actix::Handler<woab::Signal<usize>> for WindowActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal<usize>, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "addend_changed" => {
                let woab::params!(buffer: gtk4::TextBuffer) = msg.params()?;
                let new_number: Option<isize> = buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), true)
                    .and_then(|text| text.parse().ok());
                if let Some((_, number)) = self.addends.get_mut(msg.tag()) {
                    if new_number != *number {
                        *number = new_number;
                        self.recalculate();
                    }
                }
                None
            }
            "remove_addend" => {
                if let Some((addend, _)) = self.addends.remove(msg.tag()) {
                    let lst_addition = self.widgets.lst_addition.clone();
                    lst_addition.remove(&addend);
                }
                None
            }
            _ => msg.cant_handle()?,
        })
    }
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

    gtk4::init()?;
    woab::run_actix_inside_gtk_event_loop();

    woab::block_on(async {
        factories.win_app.instantiate().connect_with(|bld| {
            WindowActor {
                widgets: bld.widgets().unwrap(),
                factories,
                next_addend_id: 0,
                addends: Default::default(),
            }
            .start()
        });
    });

    gtk4::main();
    Ok(())
}
