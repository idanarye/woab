//! WoAB (Widgets on Actors Bridge) is a GUI microframework for combining the widgets toolkit
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
//! of the factories struct will be a [`woab::BuilderFactory`](BuilderFactory) that can:
//!
//! * Create a widgets struct using [`woab::WidgetsFromBuilder`](derive.WidgetsFromBuilder.html).
//! * Route the signals defined in the builder to Actix handlers using [`woab::Signal`](Signal)
//!   messages.
//! * In the Actix handler, match on the signal name (`msg.name()`) and use
//!   [`woab::params!`](crate::params!) to extract the signal parameters.
//!
//! The factories can then be used to generate the GTK widgets and either connect them to a new
//! actor which will receive the signals defined in the Glade GTK or connect them to an existing
//! actor and tag the signals (so that multiple instances can be added - e.g. with `GtkListBox` -
//! and the signal handler can know from which one the event came). The actors receive the signals
//! as Actix messages, and the `Handler` returns the inhibitness decision (if the signal requires
//! it)
//!
//! To remove widget-bound actors at runtime, see [`woab::Remove`](Remove).
//!
//! After initializing GTK and before starting the main loop,
//! [`woab::run_actix_inside_gtk_event_loop`](run_actix_inside_gtk_event_loop) **must** be called.
//! **Do not run the Actix system manually!**
//!
//! ```no_run
//! use actix::prelude::*;
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
//! impl actix::Handler<woab::Signal> for AppActor {
//!     type Result = woab::SignalResult;
//!
//!     fn handle(&mut self, msg: woab::Signal, ctx: &mut <Self as actix::Actor>::Context) -> Self::Result {
//!         Ok(match msg.name() {
//!             // These are custom signals defined in Glade's "Signals" tab.
//!             "sig1" => {
//!                 // Behavior for sig1.
//!                 None // GTK does not expect sig1 to return anything
//!             },
//!             "sig2" => {
//!                 let woab::params!(text_buffer: gtk::TextBuffer, _) = msg.params()?;
//!                 // Behavior for sig2 that uses the signal parameters.
//!                 Some(gtk::Inhibit(false)) // GTK expects sig2 to return its inhibitness decision
//!             },
//!             _ => msg.cant_handle()?,
//!         })
//!     }
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//! #    fn read_builder_xml() -> std::io::BufReader<std::fs::File> {
//! #        unreachable!()
//! #    }
//!     let factories = std::rc::Rc::new(Factories::read(read_builder_xml())?);
//!     gtk::init()?;
//!     woab::run_actix_inside_gtk_event_loop()?; // <===== IMPORTANT!!!
//!
//!     factories.main_window.instantiate().connect_with(|bld| {
//!         let widgets: AppWidgets = bld.widgets().unwrap();
//!         widgets.main_window.show_all(); // Could also be done inside the actor
//!         AppActor {
//!             widgets,
//!             factories: factories,
//!         }.start()
//!     });
//!
//!     gtk::main();
//!     Ok(())
//! }
//! ```
//!
//! # Pitfalls
//!
//! * When you start Actix actors from outside Tokio/Actix, you must use
//!   [`woab::block_on`](block_on). This is a limitation of Actix that we need to respect.
//! * Some GTK actions (like removing a widget) can fire signals synchronously. If these signals
//!   are registered as builder signals, WoAB will not be able to route them and panic because it
//!   will happen while the Actix runtime is occupied. To work around this, use
//!   [`woab::schedule_outside`](schedule_outside).

mod builder;
mod builder_dissect;
mod error;
mod event_loops_bridge;
mod remove;
mod signal;
mod signal_routing;

/// Represent a set of GTK widgets created by a GTK builder.
///
/// This needs to be a struct, where each field is a GTK type and its name must match the ID of the
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
///
/// The `#[widget(…)]` attribute is supported on the fields of the struct, with the following
/// values:
///
/// - `name = "..."`: Use a different name for matching the ID of the widget.
///
/// - `nested`: Instead of taking a single widget by ID, put another `WidgetsFromBuilder` derived
///   type (or any other type that implements `TryFrom<&gtk::Builder>`) as the field's type and
///   have take all its widgets from the same builder. The name of the field is ignored, because
///   the nested type already names all the widgets it needs.
pub use woab_macros::WidgetsFromBuilder;

/// Dissect a single Glade XML file to multiple builder factories.
///
/// The motivation is to design nested repeated hierarchies (like `GtkListBoxRow`s inside
/// `GtkListBox`) and see how they look together inside Glade, and then split the XML to multiple
/// factories that create them separately during runtime.
///
/// Typically the fields of the struct will be of type [`woab::BuilderFactory`](BuilderFactory),
/// but anything `From<String>` is allowed so [`woab::BuilderFactory`](BuilderFactory) or even just
/// `String`s are also okay, if they are needed.
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

/// Make the actor remove itself and its widgets when it gets the [`woab::Remove`](Remove) message.
///
/// The mandatory attribute `removable` must be an expression that resolves to a GTK widget that
/// has a parent. When the `woab::Remove` message is received, this actor will remove that widget
/// from its parent and close itself.
///
/// ```no_run
/// # use actix::prelude::*;
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
/// # impl actix::Handler<woab::Signal> for RowActor {
/// #     type Result = woab::SignalResult;
/// #
/// #     fn handle(&mut self, msg: woab::Signal, _ctx: &mut <Self as actix::Actor>::Context) -> Self::Result {
/// #         Ok(None)
/// #     }
/// # }
///
/// fn create_the_row(factories: &Factories, list_box: &gtk::ListBox) -> actix::Addr<RowActor> {
///     let bld = factories.list_box_row.instantiate();
///     let widgets: RowWidgets = bld.widgets().unwrap();
///     list_box.add(&widgets.list_box_row);
///     let addr = RowActor {
///         widgets,
///     }.start();
///     bld.connect_to(addr.clone());
///     addr
/// }
///
/// fn remove_the_row(row: &actix::Addr<RowActor>) {
///     row.do_send(woab::Remove);
/// }
/// ```
pub use woab_macros::Removable;

/// Helper macro for extracting signal parameters from [`woab::Signal`](crate::Signal).
///
/// ```rust
/// # let _ = |msg: woab::Signal| {
/// let woab::params!(
///     _,
///     param1: String,
///     param2,
/// ) = msg.params()?; // `msg` is the `woab::Signal`
/// # woab::SignalResult::Ok(None)
/// # };
/// ```
///
/// All the signal parameters must be matched against, but `_` can be used for unneeded parameters.
/// Parameters with types will be converted to that type, and untyped parameters will be
/// `&glib::Value`.
pub use woab_macros::params;

pub use builder::*;
pub use builder_dissect::dissect_builder_xml;
pub use error::Error;
pub use event_loops_bridge::{block_on, run_actix_inside_gtk_event_loop, schedule_outside, try_block_on};
pub use remove::Remove;
pub use signal::{Signal, SignalResult};
pub use signal_routing::{
    route_action, route_signal, GenerateRoutingGtkHandler, IntoGenerateRoutingGtkHandler, NamespacedSignalRouter,
    RawSignalCallback,
};
