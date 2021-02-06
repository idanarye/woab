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

#[derive(woab::BuilderSignal)]
enum TestSignal {
    CopyRightToLeft(gtk::Button),
    CopyLeftToRight,
}

fn get_text(buffer: &gtk::TextBuffer) -> String {
    if let Some(text) = buffer.get_text(&buffer.get_start_iter(), &buffer.get_end_iter(), true) {
        text.into()
    } else {
        "".to_owned()
    }
}

impl actix::StreamHandler<TestSignal> for TestActor {
    fn handle(&mut self, signal: TestSignal, _ctx: &mut Self::Context) {
        match signal {
            TestSignal::CopyRightToLeft(_) => {
                self.widgets.buf_left.set_text(&get_text(&self.widgets.buf_right));
            },
            TestSignal::CopyLeftToRight => {
                self.widgets.buf_right.set_text(&get_text(&self.widgets.buf_left));
            },
        }
    }
}

#[test]
fn test_basic() -> anyhow::Result<()> {
    let factories = Factories::read(include_bytes!("basic.glade") as &[u8])?;
    gtk::init()?;
    woab::run_actix_inside_gtk_event_loop("test")?;
    let mut put_widgets_in = None;
    factories.win_test.instantiate()
        .new_actor(|ctx| {
            let widgets = ctx.connect_widgets::<TestWidgets>().unwrap();
            put_widgets_in = Some(widgets.clone());
            TestActor { widgets }
        });
    let widgets = put_widgets_in.unwrap();
    widgets.buf_left.set_text("test left");
    wait_for!(get_text(&widgets.buf_right) == "")?;
    widgets.btn_copy_left_to_right.emit_clicked();
    wait_for!(get_text(&widgets.buf_right) == "test left")?;
    widgets.buf_left.set_text("");
    widgets.buf_right.set_text("test right");
    widgets.btn_copy_right_to_left.emit_clicked();
    wait_for!(get_text(&widgets.buf_left) == "test right")?;
    Ok(())
}
