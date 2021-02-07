//! WoAB (Widgets on Actors Bridge) is a library for combining the widgets toolkit
//! [GTK](https://gtk-rs.org/) with the actors framework [Actix](https://actix.rs/). It helps with:
//!
//! * Running the actors inside the GTK thread, allowing message handlers to interact with the
//!   widgets directly.
//! * Routing GTK signals through the asynchronous runtime, so that the code handling them can
//!   proceed naturally to interact with the actors.
//! * Mapping widgets and signals from [Glade](https://glade.gnome.org/) XML files to user types.
//!
//! To use WoAB one would typically create a factories struct using
//! [`woab::Factories`](derive.Factories.html) and use it dissect the Glade XML file(s). Each field
//! of the factories struct will be a [`woab::BuilderFactory`](struct.BuilderFactory.html) that can create:

//! * An Actor (optional)
//! * A widgets struct using [`woab::WidgetsFromBuilder`](derive.WidgetsFromBuilder.html)
//! * A signal enum (optional) using [`woab::BuilderSignal`](derive.BuilderSignal.html)
//!
//! The factories can then be used to generate the GTK widgets and either connect them to a new
//! actor which will receive the signals defined in the Glade GTK or connect them to an existing
//! actor and tag the signals (so that multiple instances can be added - e.g. with `GtkListBox` -
//! and the signal handler can know from which one the event came). The actors receive the signals
//! as Actix streams, and use `StreamHandler` to handle them.
//!
//! **If multiple tagged signals are streamed to the same actor - which is the typical use case for
//! tagged signals - `StreamHandler::finished` should be overridden to avoid stopping the actor
//! when one instance of the widgets is removed!!!**
//!
//! To remove widget-bound actors at runtime, see [`woab::Remove`](struct.Remove.html).
//!
//! After initializing GTK and before starting the main loop,
//! [`woab::run_actix_inside_gtk_event_loop`](fn.run_actix_inside_gtk_event_loop.html) **must** be
//! called. **Do not run the Actix system manually!**
//!
//! ```no_run
//! use gtk::prelude::*;
//!
//! #[derive(woab::Factories)]
//! struct Factories {
//!     // The field name must be the ID from the builder XML file:
//!     main_window: woab::BuilderFactory,
//!     // Possibly other things from the builder XML file that need to be created during the program.
//! }
//!
//! struct AppActor {
//!     widgets: AppWidgets,
//!     factories: std::rc::Rc<Factories>, // for creating more things from inside the actor.
//!     // More actor data
//! }
//!
//! impl actix::Actor for AppActor {
//!     type Context = actix::Context<Self>;
//! }
//!
//! #[derive(woab::WidgetsFromBuilder)]
//! struct AppWidgets {
//!     main_window: gtk::ApplicationWindow, // needed for making the window visible
//!     // Other widgets inside the window to interact with.
//! }
//!
//! #[derive(woab::BuilderSignal)]
//! enum AppSignal {
//!     // These are custom signals defined in Glade's "Signals" tab.
//!     Sig1, // Use unit variants when the signal parameters can be ignored.
//!     Sig2(gtk::TextBuffer), // Use tuple variants when the signal parameters are needed.
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
//!     factories.main_window.instantiate().actor()
//!         .create(|ctx| {
//!             let widgets: AppWidgets = ctx.widgets().unwrap();
//!             widgets.main_window.show_all(); // Could also be done inside the actor
//!             AppActor {
//!                 widgets,
//!                 factories: factories,
//!             }
//!         });
//!
//!     gtk::main();
//!     Ok(())
//! }
//! ```

mod event_loops_bridge;
mod builder;
mod builder_dissect;
mod builder_signal;

/// Represent a set of GTK widgets created by a GTK builder.
///
/// This needs to be a struct, where each field is a GTK type and its name must match the id of the
/// widgets in the Glade XML file. This derive implements a `From<&gtk::Builder>` for the struct.
///
/// ```no_run
/// #[derive(woab::WidgetsFromBuilder)]
/// struct MyAppWidgets {
///     main_window: gtk::ApplicationWindow,
///     some_button: gtk::Button,
///     some_label: gtk::Label,
/// }
/// ```
pub use woab_macros::WidgetsFromBuilder;

/// Represent a GTK signal that originates from a GTK builder. See [the corresponding trait](trait.BuilderSignal.html).
///
/// Must be used to decorate an enum. Each signal one wants to handle should be a variant of the
/// enum. Unit variants ignore the signal parameters, and tuple variants convert each parameter to
/// its proper GTK type.
///
/// ```no_run
/// #[derive(woab::BuilderSignal)]
/// enum MyAppSignal {
///     SomeButtonClicked, // We don't need the parameter because it's just the button.
///     OtherButtonClicked(gtk::Button), // Still just the button but we want it for some reason.
/// }
/// ```
pub use woab_macros::BuilderSignal;

/// Dissect a single Glade XML file to multiple builder factories.
///
/// The motivation is to design nested repeated hierarchies (like `GtkListBoxRow`s inside
/// `GtkListBox`) and see how they look together inside Glade, and then split the XML to multiple
/// factories that create them separately during runtime.
///
/// Typically the fields of the struct will be of type
/// [`woab::BuilderFactory`](struct.BuilderFactory.html), but
/// anything `From<String>` is allowed so [`woab::BuilderFactory`](struct.BuilderFactory.html) or
/// even just `String`s are also okay, if they are needed.
///
/// If a widget needs to be accompanied by some root level resource (like `GtkTextBuffer` or
/// `GtkListStore`) these resources should be listed inside a `#[factory(extra(...))]` attribute.
///
/// ```no_run
/// # type MainWindowActor = ();
/// # type MainWindowWidgets = ();
/// # type MainWindowSignal = ();
/// # type SubWindowActor = ();
/// # type SubWindowWidgets = ();
/// # type SubWindowSignal = ();
/// # type SomeListBoxRowWidgets = ();
/// # type SomeListBoxRowSignal = ();
/// #[derive(woab::Factories)]
/// struct Factories {
///     main_window: woab::BuilderFactory,
///     #[factory(extra(some_text_buffer_used_by_a_text_box_in_sub_window))]
///     sub_window: woab::BuilderFactory,
///     some_list_box_row: woab::BuilderFactory, // doesn't have its own actor
/// }
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     # fn read_builder_xml() -> std::io::BufReader<std::fs::File> {
///     #     unreachable!()
///     # }
///     let factories = Factories::read(read_builder_xml())?;
///     # Ok(())
/// }
/// ```
pub use woab_macros::Factories;

/// Make the actor remove itself and its widgets when it gets the [`woab::Remove`](struct.Remove.html) message.
///
/// The mandatory attribute `removable` must be an expression that resolves to a GTK widget that
/// has a parent. When the `woab::Remove` message is received, this actor will remove that widget
/// from its parent and close itself.
///
/// ```no_run
/// # use gtk::prelude::*;
/// #
/// # #[derive(woab::Factories)]
/// # struct Factories {
/// #     list_box_row: woab::BuilderFactory,
/// # }
/// #
/// # #[derive(woab::WidgetsFromBuilder)]
/// # struct RowWidgets {
/// #     list_box_row: gtk::ListBoxRow,
/// # }
/// #
/// # #[derive(woab::BuilderSignal)]
/// # enum RowSignal {}
/// #
/// #[derive(woab::Removable)]
/// #[removable(self.widgets.list_box_row)]
/// struct RowActor {
///     widgets: RowWidgets,
/// }
/// #
/// # impl actix::Actor for RowActor {
/// #     type Context = actix::Context<Self>;
/// # }
/// #
/// # impl actix::StreamHandler<RowSignal> for RowActor {
/// #     fn handle(&mut self, _: RowSignal, _: &mut <Self as actix::Actor>::Context) {}
/// # }
///
/// fn create_the_row(factories: &Factories, list_box: &gtk::ListBox) -> actix::Addr<RowActor> {
///     factories.list_box_row.instantiate().actor()
///         .connect_signals(RowSignal::connector())
///         .create(|ctx| {
///             let widgets: RowWidgets = ctx.widgets().unwrap();
///             list_box.add(&widgets.list_box_row);
///             RowActor {
///                 widgets,
///             }
///         })
/// }
///
/// fn remove_the_row(row: &actix::Addr<RowActor>) {
///     row.do_send(woab::Remove);
/// }
/// ```
pub use woab_macros::Removable;

pub use event_loops_bridge::run_actix_inside_gtk_event_loop;
pub use builder_dissect::dissect_builder_xml;
pub use builder_signal::{RawSignalCallback, BuilderSignal, RegisterSignalHandlers, BuilderSingalConnector};
// pub use factories::{BuilderFactory, Factory, BuilderUtilizer, BuilderConnector, ActorBuilder, ActorWidgetsBuilder};
pub use builder::*;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    XmlError(#[from] quick_xml::Error),

    #[error("Builder is missing widget with ID {0:?}")]
    WidgetMissingInBuilder(String),

    #[error("Expected widget {widget_id:?} to be {expected_type} - not {actual_type}")]
    IncorrectWidgetTypeInBuilder {
        widget_id: String,
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
