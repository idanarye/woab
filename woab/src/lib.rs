//! WoAB (Widgets on Actors Bridge) is a library for combining the widgets toolkit
//! [GTK](https://gtk-rs.org/) with the actors framework [Actix](https://actix.rs/). It helps with:
//!
//! * Running the actors inside the GTK thread, allowing message handlers to interact with the
//!   widgets directly.
//! * Routing GTK signals through the asynchronous runtime, so that the code handling them can
//!   proceed naturally to interact with the actors.
//! * Mapping widgets and signals from [Glade](https://glade.gnome.org/) XML files to user types.
//!
//! To use WoAB you need to implement `WoabActor` on types that will be in charge of parts of the
//! GUI. A `WoabActor` needs:
//!
//! * To be an Actix `Actor`.
//! * `Widgets` - a `struct` that contains the widgets of that actor and derives `WidgetsFromBuilder`.
//! * `Signal` - an `enum` that represents the signals defined in the `.glade` file and derives
//!   `BuilderSignal`.
//! * To be a `StreamHandler` of its `Signal`. This is where it handles the signals GTK emits.
//!
//! To create an instance of your `WoabActor` instance you can use an `ActorFactory`, which helps
//! in creating the widgets and connecting the signals to the actor. You can create one directly
//! from the XML string, or you can derive `Factories` to dissect the XML into multiple factories.
//! This is useful if you have some internal container widgets (e.g. `ListBoxRow`) that you want to
//! create multiple instances of while the program runs, but it's more convenient to have a single
//! instance inside their parent when editing them in the Glade designer.
//!
//! If you want to remove widget-bound actors at runtime, see [`Remove`](struct.Remove.html).
//!
//! After initializing GTK and before starting the main loop, you **must** call
//! `run_actix_inside_gtk_event_loop`. Do not run the Actix system manually!
//!
//! ```no_run
//! use gtk::prelude::*;
//!
//! #[derive(woab::Factories)]
//! struct Factories {
//!     // The field name must be the ID from the builder XML file:
//!     main_window: woab::Factory<AppActor, AppWidgets, AppSignal>,
//!     // Possibly other things from the builder XML file you want to create during the program.
//! }
//!
//! struct AppActor {
//!     widgets: AppWidgets,
//!     factories: std::rc::Rc<Factories>, // in case you want to create more things from inside the actor.
//!     // More actor data
//! }
//!
//! impl actix::Actor for AppActor {
//!     type Context = actix::Context<Self>;
//! }
//!
//! #[derive(woab::WidgetsFromBuilder)]
//! struct AppWidgets {
//!     main_window: gtk::ApplicationWindow, // this one is a must - you need it to show the window.
//!     // Other widgets inside the window that you want to interact with.
//! }
//!
//! #[derive(woab::BuilderSignal)]
//! enum AppSignal {
//!     // These are custom signals you define in Glade's "Signals" tab.
//!     Sig1, // Use unit variants when you don't care about the signal parameters.
//!     Sig2(gtk::TextBuffer), // Use tuple variants when you need the signal parameters.
//! }
//!
//! impl actix::StreamHandler<AppSignal> for AppActor {
//!     fn handle(&mut self, signal: AppSignal, ctx: &mut Self::Context) {
//!         match signal {
//!             AppSignal::Sig1 => {
//!                 // Behavior for Sig1.
//!             },
//!             AppSignal::Sig2(text_buffer) => {
//!                 // Behavior for Sig2 that uses the signal parameter.
//!             },
//!         }
//!     }
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//! #    fn read_builder_xml() -> std::io::BufReader<std::fs::File> {
//! #        unreachable!()
//! #    }
//!     let factories = std::rc::Rc::new(Factories::read(read_builder_xml())?);
//!     gtk::init()?;
//!     woab::run_actix_inside_gtk_event_loop("my-WoAB-app")?; // <===== IMPORTANT!!!
//!
//!     factories.main_window.build().actor(|_ctx, widgets| {
//!         widgets.main_window.show_all(); // Or you could do that inside the actor
//!         AppActor {
//!             widgets,
//!             factories: factories,
//!         }
//!     })?;
//!
//!     gtk::main();
//!     Ok(())
//! }
//! ```

mod event_loops_bridge;
mod builder_signal;
mod builder_dissect;
mod factories;

pub use woab_macros::{WidgetsFromBuilder, BuilderSignal, Factories, Removable};

pub use event_loops_bridge::run_actix_inside_gtk_event_loop;
pub use builder_signal::BuilderSignal;
pub use builder_dissect::dissect_builder_xml;
pub use factories::{BuilderFactory, Factory, BuilderUtilizer};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    XmlError(#[from] quick_xml::Error),

    #[error("Builder is missing widget with ID {0:?}")]
    WidgetMissingInBuilder(&'static str),

    #[error("Expected widget {widget_id:?} to be {expected_type} - not {actual_type}")]
    IncorrectWidgetTypeInBuilder {
        widget_id: &'static str,
        expected_type: glib::types::Type,
        actual_type: glib::types::Type,
    },
}

/// A message for removing actors along with their GUI
///
/// Refer to `#[derive(woab::Removable)]` docs for usage instructions.
/// ```no_run
/// #[derive(woab::Removable)]
/// #[removable(self.widgets.main_window)]
/// struct MyActor {
///     widgets: MyWidgets,
/// }
///
/// # impl actix::Actor for MyActor { type Context = actix::Context<Self>; }
///
/// #[derive(woab::WidgetsFromBuilder)]
/// struct MyWidgets {
///     main_window: gtk::ApplicationWindow,
/// }
///
/// let my_actor: actix::Addr<MyActor>;
/// # let mut my_actor: actix::Addr<MyActor> = panic!();
/// my_actor.do_send(woab::Remove);
/// ```
pub struct Remove;

impl actix::Message for Remove {
    type Result = ();
}
