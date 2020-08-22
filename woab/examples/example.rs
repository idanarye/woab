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

#[derive(woab::BuilderSignal)]
enum WindowSingals {
    ClickButton(gtk::Button),
}

impl actix::StreamHandler<WindowSingals> for WindowActor {
    fn handle(&mut self, signal: WindowSingals, _ctx: &mut Self::Context) {
        match signal {
            WindowSingals::ClickButton(_button) => {
                self.counter += 1;
                use gtk::TextBufferExt;
                self.widgets.text_buffer.set_text(&format!("{}", self.counter));
            }
        }
    }
}

// These should be created by a macro:

impl WindowActor {
    fn connect_builder_signals(ctx: &mut <WindowActor as actix::Actor>::Context, builder: &gtk::Builder) {
        use actix::StreamHandler;
        use gtk::prelude::BuilderExtManual;

        let (tx, rx) = tokio::sync::mpsc::channel(16);
        WindowActor::add_stream(rx, ctx);
        builder.connect_signals(move |_, signal| {
            WindowSingals::transmit_signal_in_stream_function(signal, tx.clone())
        });
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
