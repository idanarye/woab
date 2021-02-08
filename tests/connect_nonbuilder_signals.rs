use std::rc::Rc;
use std::cell::RefCell;

// use actix::prelude::*;
use gtk::prelude::*;

#[macro_use]
mod util;

#[derive(woab::Factories)]
struct Factories {
    win_test: woab::BuilderFactory,
}

#[derive(Clone, woab::WidgetsFromBuilder)]
struct TestWidgets {
    win_test: gtk::ApplicationWindow,
    button1: gtk::Button,
    button2: gtk::Button,
}

#[derive(Debug, PartialEq, woab::BuilderSignal)]
enum TestSignal {
    Click1,
    Click2,
}

struct TestActor {
    name: &'static str,
    output: Rc<RefCell<Vec<(&'static str, TestSignal)>>>,
}

impl actix::Actor for TestActor {
    type Context = actix::Context<Self>;
}

impl actix::StreamHandler<TestSignal> for TestActor {
    fn handle(&mut self, signal: TestSignal, _ctx: &mut Self::Context) {
        self.output.borrow_mut().push((self.name, signal));
    }
}

#[test]
fn test_connect_nonbuilder_signals() -> anyhow::Result<()> {
    let factories = Factories::read(include_bytes!("connect_nonbuilder_signals.glade") as &[u8])?;
    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop("test")?;

    let builder = factories.win_test.instantiate();
    let widgets: TestWidgets = builder.widgets().unwrap();

    let output = Rc::new(RefCell::new(Vec::new()));

    builder.actor().create(|ctx| {
        woab::connect_signal_handler::<TestActor, TestSignal, _>(&widgets.button1, "clicked", "Click1", ctx);
        TestActor {
            name: "actor1",
            output: output.clone(),
        }
    });

    builder.actor().create(|ctx| {
        woab::connect_signal_handler::<TestActor, TestSignal, _>(&widgets.button2, "clicked", "Click2", ctx);
        TestActor {
            name: "actor2",
            output: output.clone(),
        }
    });

    builder.finish();

    widgets.button1.emit_clicked();
    wait_for!(*output.borrow() == [
        ("actor1", TestSignal::Click1),
    ])?;
    widgets.button2.emit_clicked();
    wait_for!(*output.borrow() == [
        ("actor1", TestSignal::Click1),
        ("actor2", TestSignal::Click2),
    ])?;

    Ok(())
}
