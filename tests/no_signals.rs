use std::cell::RefCell;
use std::rc::Rc;

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
        ctx.spawn(
            async move {
                output.borrow_mut().push("inside spawned future");
            }
            .into_actor(self),
        );
    }
}

impl actix::Handler<woab::Signal> for TestActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        msg.cant_handle()
    }
}

#[derive(Clone, woab::WidgetsFromBuilder)]
pub struct TestWidgets {}

#[test]
fn test_no_signals() -> anyhow::Result<()> {
    util::test_main(async {
        let factories = Factories::read(include_bytes!("no_signals.ui") as &[u8])?;
        let output = Rc::new(RefCell::new(Vec::new()));
        factories
            .win_test
            .instantiate_route_to(TestActor { output: output.clone() }.start());
        wait_for!(*output.borrow() == ["before spawned future", "inside spawned future",])?;
        Ok(())
    })
}
