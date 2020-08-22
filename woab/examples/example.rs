// This stuff should be implemented by the user:

#[derive(woab::WidgetsFromBuilder)]
pub struct WindowWidgets {
    pub app_window: gtk::ApplicationWindow,
    pub text_buffer: gtk::TextBuffer,
}

struct WindowActor {
    widgets: WindowWidgets,
    counter: usize,
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        use gtk::WidgetExt;
        self.widgets.app_window.show();
    }
}
impl WindowActor {
    fn click_button(&mut self, _button: gtk::Button) {
        self.counter += 1;
        use gtk::TextBufferExt;
        self.widgets.text_buffer.set_text(&format!("{}", self.counter));
    }
}

// These should be created by a macro:

impl WindowActor {
    fn connect_builder_signals(ctx: &mut <WindowActor as actix::Actor>::Context, builder: &gtk::Builder) {
        use tokio::sync::mpsc::error::TrySendError;
        use actix::StreamHandler;
        use gtk::prelude::BuilderExtManual;

        let (tx, rx) = tokio::sync::mpsc::channel(16);
        WindowActor::add_stream(rx, ctx);
        builder.connect_signals(move |_, signal| {
            let tx = tx.clone();
            // TODO: Better error handling?
            match signal {
                "click_button" => Box::new(move |args| {
                    let mut tx = tx.clone();
                    match tx.try_send(WindowActorSignal::click_button(args[0].get().unwrap().unwrap())) {
                        Ok(_) => None,
                        Err(TrySendError::Closed(_)) => None,
                        Err(TrySendError::Full(_)) => {
                            panic!("Unable to send click_button signal - channel is full");
                        },
                    }
                }),
                _ => Box::new(|_| {
                    None
                }),
            }
        });
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug)]
enum WindowActorSignal {
    click_button(gtk::Button),
}

impl actix::StreamHandler<WindowActorSignal> for WindowActor {
    fn handle(&mut self, signal: WindowActorSignal, _ctx: &mut Self::Context) {
        match signal {
            WindowActorSignal::click_button(button) => {
                self.click_button(button);
            }
        }
    }
}

fn main() {
    use actix::prelude::*;

    gtk::init().unwrap();
    woab::run_actix_inside_gtk_event_loop("example").unwrap();
    let builder = gtk::Builder::from_file("woab/examples/example.glade");
    WindowActor::create(|ctx| {
        WindowActor::connect_builder_signals(ctx, &builder);
        use std::convert::TryInto;
        WindowActor {
            widgets: (&builder).try_into().unwrap(),
            counter: 0,
        }
    });
    gtk::main();
}
