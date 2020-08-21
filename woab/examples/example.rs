struct WindowWidgets {
    app_window: gtk::ApplicationWindow,
}

impl WindowWidgets {
    pub fn new_from_builder(builder: &gtk::Builder) -> Self {
        use gtk::prelude::BuilderExtManual;
        Self {
            app_window: builder.get_object::<gtk::ApplicationWindow>("app_window").expect("Missing widget"),
        }
    }
}

struct WindowActor {
    widgets: WindowWidgets,
}

impl actix::Actor for WindowActor {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        use gtk::WidgetExt;
        self.widgets.app_window.show();
    }
}

impl WindowActor {
    fn click_button(&self, _button: gtk::Button) {
        println!("Button clicked");
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
    use gtk::prelude::*;

    gtk::init().unwrap();
    woab::run_actix_inside_gtk_event_loop("example").unwrap();
    let builder = gtk::Builder::from_file("woab/examples/example.glade");
    WindowActor::create(|ctx| {
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        WindowActor::add_stream(rx, ctx);
        builder.connect_signals(|_, signal| {
            let tx = tx.clone();
            match signal {
                "click_button" => Box::new(move |args| {
                    let mut tx = tx.clone();
                    use tokio::sync::mpsc::error::TrySendError;
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
        WindowActor {
            widgets: WindowWidgets::new_from_builder(&builder),
        }
    });
    gtk::main();
}
