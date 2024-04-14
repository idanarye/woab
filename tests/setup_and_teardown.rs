use std::cell::RefCell;
use std::rc::Rc;

use actix::prelude::*;
use gtk4::prelude::*;

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
    #[allow(unused)]
    win_test: gtk4::ApplicationWindow,
    btn_button: gtk4::Button,
}

impl actix::Handler<woab::Signal> for TestActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "button_clicked" => {
                assert!(matches!(
                    woab::close_actix_runtime(),
                    Err(woab::RuntimeStopError::RuntimeInUse)
                ));
                assert!(woab::is_runtime_running());
                self.output.borrow_mut().push("click");
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

#[test]
fn test_teardown() -> anyhow::Result<()> {
    assert!(matches!(
        woab::close_actix_runtime(),
        Err(woab::RuntimeStopError::RuntimeNotStarted)
    ));
    assert!(!woab::is_runtime_running());

    let output = Rc::new(RefCell::new(Vec::new()));
    util::test_main({
        let output = output.clone();
        async move {
            let factories = Factories::read(include_bytes!("just_a_button.ui") as &[u8])?;
            let bld = factories
                .win_test
                .instantiate_route_to(TestActor { output: output.clone() }.start());
            let widgets = bld.widgets::<TestWidgets>().unwrap();
            widgets.btn_button.emit_clicked();
            wait_for!(*output.borrow() == ["click"])?;
            assert!(woab::is_runtime_running());
            Ok(())
        }
    })?;
    assert!(!woab::is_runtime_running());
    assert_eq!(*output.borrow(), ["click", "shutdown"]);
    Ok(())
}
