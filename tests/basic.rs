use actix::prelude::*;
use gtk4::prelude::*;

#[macro_use]
mod util;

struct TestActor {
    widgets: TestWidgets,
}

impl actix::Actor for TestActor {
    type Context = actix::Context<Self>;
}

#[derive(Clone, woab::WidgetsFromBuilder)]
pub struct TestWidgets {
    #[allow(unused)]
    win_test: gtk4::ApplicationWindow,
    btn_copy_right_to_left: gtk4::Button,
    btn_copy_left_to_right: gtk4::Button,
    buf_left: gtk4::TextBuffer,
    buf_right: gtk4::TextBuffer,
}

fn get_text(buffer: &gtk4::TextBuffer) -> String {
    buffer.text(&buffer.start_iter(), &buffer.end_iter(), true).into()
}

impl actix::Handler<woab::Signal> for TestActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        Ok(match msg.name() {
            "copy_right_to_left" => {
                self.widgets.buf_left.set_text(&get_text(&self.widgets.buf_right));
                None
            }
            "copy_left_to_right" => {
                self.widgets.buf_right.set_text(&get_text(&self.widgets.buf_left));
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

#[test]
fn test_basic() -> anyhow::Result<()> {
    util::test_main(async {
        let factory = woab::BuilderFactory::from(std::fs::read_to_string("tests/basic.ui")?);
        let ctx = Context::<TestActor>::new();
        let bld = factory.instantiate_route_to(ctx.address());
        let widgets: TestWidgets = bld.widgets()?;
        ctx.run(TestActor {
            widgets: widgets.clone(),
        });
        widgets.buf_left.set_text("test left");
        wait_for!(get_text(&widgets.buf_right).is_empty())?;
        widgets.btn_copy_left_to_right.emit_clicked();
        wait_for!(get_text(&widgets.buf_right) == "test left")?;
        widgets.buf_left.set_text("");
        widgets.buf_right.set_text("test right");
        widgets.btn_copy_right_to_left.emit_clicked();
        wait_for!(get_text(&widgets.buf_left) == "test right")?;
        Ok(())
    })
}
