use std::cell::RefCell;
use std::rc::Rc;

use actix::prelude::*;
use gtk4::prelude::*;

#[macro_use]
mod util;

struct TestActor {
    output: Rc<RefCell<Vec<&'static str>>>,
}

impl actix::Actor for TestActor {
    type Context = actix::Context<Self>;
}

#[derive(Clone, woab::WidgetsFromBuilder)]
pub struct TestWidgets {
    rows: gtk4::ListBox,
    row: gtk4::ListBoxRow,
}

impl actix::Handler<woab::Signal> for TestActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "row_removed" => {
                self.output.borrow_mut().push("inside remove");
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

#[test]
fn test_future() -> anyhow::Result<()> {
    let factory = woab::BuilderFactory::from(std::fs::read_to_string("tests/future.glade")?);
    gtk4::init()?;
    woab::run_actix_inside_gtk_event_loop();

    let output = Rc::new(RefCell::new(Vec::<&'static str>::new()));

    woab::block_on({
        let output = output.clone();
        async {
            let widgets: TestWidgets = factory
                .instantiate()
                .connect_to(TestActor { output: output.clone() }.start())
                .widgets()
                .unwrap();
            actix::spawn(async move {
                output.borrow_mut().push("before remove");
                let outside_result = woab::outside(async move {
                    widgets.rows.remove(&widgets.row);
                    "outside result"
                })
                .await
                .unwrap();
                output.borrow_mut().push(outside_result);
                output.borrow_mut().push("after remove");
            });
        }
    });

    wait_for!(*output.borrow() == ["before remove", "inside remove", "outside result", "after remove",]).map_err(|e| {
        println!("is {:?}", output);
        e
    })?;

    woab::close_actix_runtime()??;
    Ok(())
}
