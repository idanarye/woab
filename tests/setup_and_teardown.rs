use std::cell::RefCell;
use std::rc::Rc;

use actix::prelude::*;
use gtk::prelude::*;

#[macro_use]
mod util;

#[derive(woab::Factories)]
struct Factories {
    win_test: woab::BuilderFactory,
}

struct TestActor {
    output: Rc<RefCell<Vec<&'static str>>>,
}

impl actix::Actor for TestActor {
    type Context = actix::Context<Self>;

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        self.output.borrow_mut().push("shutdown");
    }
}

#[derive(Clone, woab::WidgetsFromBuilder)]
pub struct TestWidgets {
    win_test: gtk::ApplicationWindow,
    btn_button: gtk::Button,
}

impl actix::Handler<woab::Signal> for TestActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "button_clicked" => {
                self.output.borrow_mut().push("click");
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

fn prepare() -> anyhow::Result<(TestWidgets, Rc<RefCell<Vec<&'static str>>>)> {
    let factories = Factories::read(include_bytes!("just_a_button.glade") as &[u8])?;
    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop();
    let output = Rc::new(RefCell::new(Vec::new()));
    let widgets = woab::block_on(async {
        let bld = factories.win_test.instantiate();
        let widgets = bld.widgets::<TestWidgets>().unwrap();
        bld.connect_to(TestActor {
            output: output.clone(),
        }.start());
        widgets
    });
    Ok((widgets, output))
}

#[test]
fn test_teardown() -> anyhow::Result<()> {
    let (widgets, output) = prepare()?;
    widgets.btn_button.emit_clicked();
    wait_for!(*output.borrow() == ["click"])?;
    woab::close_actix_runtime()?;
    wait_for!(*output.borrow() == ["click", "shutdown"])?;
    Ok(())
}
