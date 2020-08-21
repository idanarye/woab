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
struct WindowActor_click_button(send_wrapper::SendWrapper<gtk::Button>);

impl actix::Message for WindowActor_click_button {
    type Result = ();
}

impl actix::Handler<WindowActor_click_button> for WindowActor {
    type Result = ();

    fn handle(&mut self, msg: WindowActor_click_button, _ctx: &mut Self::Context) -> Self::Result {
        let WindowActor_click_button(arg0) = msg;
        self.click_button(arg0.take());
    }
}

fn main() {
    use actix::prelude::*;
    use gtk::prelude::*;

    gtk::init().unwrap();
    woab::run_actix_inside_gtk_event_loop("example").unwrap();
    let builder = gtk::Builder::from_file("woab/examples/example.glade");
    let window_addr = WindowActor {
        widgets: WindowWidgets::new_from_builder(&builder),
    }.start();
    builder.connect_signals(|_, signal| {
        let window_addr = window_addr.clone();
        match signal {
            "click_button" => Box::new(move |args| {
                let msg = WindowActor_click_button(send_wrapper::SendWrapper::new(args[0].get().unwrap().unwrap()));
                window_addr.do_send(msg);
                None
            }),
            _ => Box::new(|_| {
                None
            }),
        }
    });
    gtk::main();
}
