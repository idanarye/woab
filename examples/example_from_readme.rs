use actix::prelude::*;
use gtk4::prelude::*;

struct MyActor {
    widgets: MyWidgets,
}

impl Actor for MyActor {
    type Context = Context<Self>;
}

// Use this derive to automatically populate a struct with GTK objects from a builder using their
// object IDs.
#[derive(woab::WidgetsFromBuilder)]
struct MyWidgets {
    window: gtk4::ApplicationWindow,
    button: gtk4::Button,
}

// WoAB converts GTK signals (defined) to Actix messages, which the user defined actors need handle.
impl Handler<woab::Signal> for MyActor {
    type Result = woab::SignalResult;

    fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
        // All the signals get the same message type (`woab::Signal`), and need to be matched by
        // the handler name.
        Ok(match msg.name() {
            "button_clicked" => {
                // Handlers can freely use the GTK widget handles stored inside the actor to
                // interact with the UI.
                self.widgets.button.set_label("Hello World");
                // Some GTK signals require a `glib::Propagation` decision. Others, like
                // `GtkButton::clicked` here, don't. It is up to the signal handler to return the
                // correct type.
                None
            }
            _ => msg.cant_handle()?,
        })
    }
}

fn main() -> woab::Result<()> {
    // Factories can be used to create the GUI and connect the signals.
    let factory = woab::BuilderFactory::from(
        // Typically the UI XML will be generated with Cambalache and loaded from a file, but for
        // the sake of this simple example it is inlined here.
        r#"
        <interface>
          <object class="GtkApplicationWindow" id="window">
            <child>
              <object class="GtkButton" id="button">
                <property name="label">Click Me!</property>
                <signal name="clicked" handler="button_clicked"/>
              </object>
            </child>
          </object>
        </interface>
        "#
        .to_owned(),
    );

    // Setup the application inside `woab::main`. This handles starting/stopping GTK and Actix, and
    // making them work together. The actual closure is run inside the application's `startup`
    // signal.
    woab::main(gtk4::Application::default(), move |app| {
        // A useful helper so that when the last window is closed, the application will exit.
        woab::shutdown_when_last_window_is_closed(app);

        // We need the actor's address when instantiating the builder (because we need to connect
        // the signals) and we need the builder result when we create the actor (because we want to
        // provide it with the widgets). Thus, we usually want to use Actix's two-steps actor
        // initialization.
        let ctx = Context::new();

        // This will create the UI widgets from the XML and route the signals to the actor.
        let bld = factory.instantiate_route_to(ctx.address());

        // Automatically assign all the windows inside the builder to the application. Without
        // this, `woab::shutdown_when_last_window_is_closed` will be meaningless.
        bld.set_application(app);

        // Extract the newly created widgets from the builder.
        let widgets: MyWidgets = bld.widgets()?;

        // When the builder loads the window, it starts as hidden. We can use the extracted widgets
        // to show it.
        widgets.window.show();

        // This is where the actor is actually launched.
        ctx.run(MyActor { widgets });

        Ok(())
    })
}
