use actix::prelude::*;
use gtk::prelude::*;

#[macro_use]
mod util;

#[derive(woab::Factories)]
struct Factories {
    #[factory(extra(buf_left, buf_right))]
    win_test: woab::BuilderFactory,
}

struct TestActor {
    widgets: TestWidgets,
}

impl actix::Actor for TestActor {
    type Context = actix::Context<Self>;
}

#[derive(Clone, woab::WidgetsFromBuilder)]
pub struct TestWidgets {
    win_test: gtk::ApplicationWindow,
    btn_copy_right_to_left: gtk::Button,
    btn_copy_left_to_right: gtk::Button,
    buf_left: gtk::TextBuffer,
    buf_right: gtk::TextBuffer,
}

fn get_text(buffer: &gtk::TextBuffer) -> String {
    if let Some(text) = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true) {
        text.into()
    } else {
        "".to_owned()
    }
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
    let factories = Factories::read(include_bytes!("basic.glade") as &[u8])?;
    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop()?;
    let mut put_widgets_in = None;
    woab::block_on(async {
        factories.win_test.instantiate().connect_with(|bld| {
            let widgets = bld.widgets::<TestWidgets>().unwrap();
            put_widgets_in = Some(widgets.clone());
            TestActor { widgets }.start()
        });
    });
    let widgets = put_widgets_in.unwrap();
    widgets.buf_left.set_text("test left");
    wait_for!(get_text(&widgets.buf_right).is_empty())?;
    widgets.btn_copy_left_to_right.emit_clicked();
    wait_for!(get_text(&widgets.buf_right) == "test left")?;
    widgets.buf_left.set_text("");
    widgets.buf_right.set_text("test right");
    widgets.btn_copy_right_to_left.emit_clicked();
    wait_for!(get_text(&widgets.buf_left) == "test right")?;
    Ok(())
}
