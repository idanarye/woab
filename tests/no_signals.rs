use std::rc::Rc;
use std::cell::RefCell;

use actix::prelude::*;

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

    fn started(&mut self, ctx: &mut Self::Context) {
        let output = self.output.clone();
        output.borrow_mut().push("before spawned future");
        ctx.spawn(async move {
            output.borrow_mut().push("inside spawned future");
        }.into_actor(self));
    }
}

#[derive(Clone, woab::WidgetsFromBuilder)]
pub struct TestWidgets {
}

#[derive(woab::BuilderSignal)]
enum TestSignal {
}

impl actix::StreamHandler<TestSignal> for TestActor {
    fn handle(&mut self, signal: TestSignal, _ctx: &mut Self::Context) {
        match signal {
        }
    }
}

#[test]
fn test_no_signals() -> anyhow::Result<()> {
    let factories = Factories::read(include_bytes!("no_signals.glade") as &[u8])?;
    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop("test")?;
    let output = Rc::new(RefCell::new(Vec::new()));
    factories.win_test.instantiate()
        .new_actor(|_ctx| {
            TestActor { output: output.clone() }
        });
    wait_for!(*output.borrow() == [
        "before spawned future",
        "inside spawned future",
    ])?;
    Ok(())
}
