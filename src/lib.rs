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
//! as Actix messages, and the `Handler` returns the propagation decision (if the signal requires
//! it)
//!
//! To remove widget-bound actors at runtime, see [`woab::Remove`](Remove).
//!
//! To synchronize the widgets' data with a model (or any old Rust values), see
//! [`woab::PropSync`](crate::PropSync).
//!
//! Do not run the Actix system or GTK manually - use [`woab::main`](crate::main) instead.
//!
//! ```no_run
//! use actix::prelude::*;
//! use gtk4::prelude::*;
//! 
//! struct MyActor {
//!     widgets: MyWidgets,
//! }
//! 
//! impl Actor for MyActor {
//!     type Context = Context<Self>;
//! }
//! 
//! // Use this derive to automatically populate a struct with GTK objects from a builder using their
//! // object IDs.
//! #[derive(woab::WidgetsFromBuilder)]
//! struct MyWidgets {
//!     window: gtk4::ApplicationWindow,
//!     button: gtk4::Button,
//! }
//! 
//! // WoAB converts GTK signals (defined) to Actix messages, which the user defined actors need handle.
//! impl Handler<woab::Signal> for MyActor {
//!     type Result = woab::SignalResult;
//! 
//!     fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
//!         // All the signals get the same message type (`woab::Signal`), and need to be matched by
//!         // the handler name.
//!         Ok(match msg.name() {
//!             "button_clicked" => {
//!                 // Handlers can freely use the GTK widget handles stored inside the actor to
//!                 // interact with the UI.
//!                 self.widgets.button.set_label("Hello World");
//!                 // Some GTK signals require a `glib::Propagation` decision. Others, like
//!                 // `GtkButton::clicked` here, don't. It is up to the signal handler to return the
//!                 // correct type.
//!                 None
//!             }
//!             _ => msg.cant_handle()?,
//!         })
//!     }
//! }
//! 
//! fn main() -> woab::Result<()> {
//!     // Factories can be used to create the GUI and connect the signals.
//!     let factory = woab::BuilderFactory::from(
//!         // Typically the UI XML will be generated with Cambalache and loaded from a file, but for
//!         // the sake of this simple example it is inlined here.
//!         r#"
//!         <interface>
//!           <object class="GtkApplicationWindow" id="window">
//!             <child>
//!               <object class="GtkButton" id="button">
//!                 <property name="label">Click Me!</property>
//!                 <signal name="clicked" handler="button_clicked"/>
//!               </object>
//!             </child>
//!           </object>
//!         </interface>
//!         "#
//!         .to_owned(),
//!     );
//! 
//!     // Setup the application inside `woab::main`. This handles starting/stopping GTK and Actix, and
//!     // making them work together. The actual closure is run inside the application's `startup`
//!     // signal.
//!     woab::main(gtk4::Application::default(), move |app| {
//!         // A useful helper so that when the last window is closed, the application will exit.
//!         woab::shutdown_when_last_window_is_closed(app);
//! 
//!         // We need the actor's address when instantiating the builder (because we need to connect
//!         // the signals) and we need the builder result when we create the actor (because we want to
//!         // provide it with the widgets). Thus, we usually want to use Actix's two-steps actor
//!         // initialization.
//!         let ctx = Context::new();
//! 
//!         // This will create the UI widgets from the XML and route the signals to the actor.
//!         let bld = factory.instantiate_route_to(ctx.address());
//! 
//!         // Automatically assign all the windows inside the builder to the application. Without
//!         // this, `woab::shutdown_when_last_window_is_closed` will be meaningless.
//!         bld.set_application(app);
//! 
//!         // Extract the newly created widgets from the builder.
//!         let widgets: MyWidgets = bld.widgets()?;
//! 
//!         // When the builder loads the window, it starts as hidden. We can use the extracted widgets
//!         // to show it.
//!         widgets.window.show();
//! 
//!         // This is where the actor is actually launched.
//!         ctx.run(MyActor { widgets });
//! 
//!         Ok(())
//!     })
//! }
//! ```
//!
//! # Pitfalls
//!
//! * When starting Actix actors from outside Tokio/Actix, [`woab::block_on`](block_on) must be
//!   used. This is a limitation of Actix that needs to be respected.
//! * If an actor is created inside a `gtk4::Application::connect_activate`, its `started` method
//!   will run **after** the `activate` signal is done. This can be a problem for methods like
//!   `set_application` that can segfault if they are called outside the `activate` signal. A
//!   solution could be to either do the startup inside `connect_activate` or use
//!   [`woab::route_signal`](crate::route_signal) to route the application's `activate` signal
//!   to the actor and do the startup in the actor's signal handler.

mod builder;
mod builder_dissect;
mod error;
mod event_loops_bridge;
mod gtk_app_helpers;
pub mod prop_sync;
mod remove;
mod signal;
mod signal_routing;
mod waking_helpers;

/// Represent a set of GTK widgets created by a GTK builder.
///
/// This needs to be a struct, where each field is a GTK type and its name must match the ID of the
/// widgets in the Glade XML file. This derive implements a `From<&gtk4::Builder>` for the struct.
///
/// ```no_run
/// #[derive(woab::WidgetsFromBuilder)]
/// struct MyAppWidgets {
///     main_window: gtk4::ApplicationWindow,
///     some_button: gtk4::Button,
///     some_label: gtk4::Label,
/// }
/// ```
///
/// The `#[widget(…)]` attribute is supported on the fields of the struct, with the following
/// values:
///
/// - `name = "..."`: Use a different name for matching the ID of the widget.
///
/// - `nested`: Instead of taking a single widget by ID, put another `WidgetsFromBuilder` derived
///   type (or any other type that implements `TryFrom<&gtk4::Builder>`) as the field's type and
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
/// `GtkListStore`) these resources should be listed inside a `#[factory(extra(...))]` attribute
/// (this is leftover from GTK3 and less likely needed in GTK4 where said resources can be placed
/// under the widget in the UI XML)
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
/// The mandatory attribute `removable` must contain the syntax `<widget> in <ParentType>` where:
///
/// * `<widget>` is an expression (typically a path starting with `self`) that resolves to a GTK
///   widget that has a parent.
/// * `<ParentType>` is the GTK type of the parent. That type must have a `remove` method.
///
/// When the `woab::Remove` message is received, this actor will remove that widget
/// from its parent and close itself.
///
/// ```no_run
/// # use actix::prelude::*;
/// # use gtk4::prelude::*;
/// #
/// # #[derive(woab::Factories)]
/// # struct Factories {
/// #     list_box_row: woab::BuilderFactory,
/// # }
/// #
/// # #[derive(woab::WidgetsFromBuilder)]
/// # struct RowWidgets {
/// #     list_box_row: gtk4::ListBoxRow,
/// # }
/// #
/// #[derive(woab::Removable)]
/// #[removable(self.widgets.list_box_row in gtk4::ListBox)]
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
/// fn create_the_row(factories: &Factories, list_box: &gtk4::ListBox) -> actix::Addr<RowActor> {
///     RowActor::create(|ctx| {
///         let bld = factories.list_box_row.instantiate_route_to(ctx.address());
///         let widgets: RowWidgets = bld.widgets().unwrap();
///         list_box.append(&widgets.list_box_row);
///         RowActor {
///             widgets,
///         }
///     })
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

/// Generate methods for setting/getting the widgets' data.
///
/// Useful for syncing the view with a model - use struct literals and struct destructuring on the
/// generated getter and setter structs to make rust-analyzer complete the fields and warn about
/// neglected fields. Usually used together with [`WidgetsFromBuilder`].
///
/// This derive will generate two helper structs:
/// * `StructNamePropSetter` which can be used in
///   [`set_props`](crate::prop_sync::SetProps::set_props) to set the widgets' data.
/// * `StructNamePropGetter` which can be used in
///   [`get_props`](crate::prop_sync::GetProps::get_props) to get the widgets' data.
///
/// The annotated struct will implement [`SetProps`](crate::prop_sync::SetProps) and
/// [`GetProps`](crate::prop_sync::GetProps), but also implement these two methods inherently so
/// the traits will not need to be imported in order to use them.
///
/// Annotate fields with `#[prop_sync(set)]` to include them in the setter and with
/// `#[prop_sync(get)]` to include them in the getter.
///
/// Use `#[prop_sync("property-name" as PropertyType)]` to set the property that will be used for
/// the syncing and its type. If `PropertyType` is a reference (`&PropertyType`), the reference
/// will be used for the setter (the macro will add a lifetime) and its [`ToOwned::Owned`] will be
/// used for the getter.
///
/// There is no need to set a property for some common widgets (like `gtk4::Entry`) - they already
/// implement [`SetProps`](crate::prop_sync::SetProps) and
/// [`GetProps`](crate::prop_sync::GetProps), so the macro will use the traits to set/get the data.
/// Similarly, structs that use this derive implement these two traits so they can be used with
/// [`WidgetsFromBuilder`]'s `#[widget(nested)]`.
///
/// ```no_run
/// #[derive(woab::WidgetsFromBuilder, woab::PropSync)]
/// struct AppWidgets {
///     // Not included in the prop-sync because we are not syncing it.
///     main_window: gtk4::ApplicationWindow,
///
///     #[prop_sync(set, get)]
///     some_text: gtk4::Entry,
///
///     // Combo boxes use the active-id property to select a row in their model.
///     #[prop_sync("active-id" as String, set, get)]
///     some_combo_box: gtk4::ComboBox,
///
///     // We only want to get the value of this checkbox, not set it, so we don't generate a setter.
///     #[prop_sync("active" as bool, get)]
///     some_check_box: gtk4::CheckButton,
/// }
///
/// # let widgets: AppWidgets = panic!();
/// // Set the widgets' data
/// widgets.set_props(&AppWidgetsPropSetter {
///     some_text: "some test",
///     some_combo_box: "1".to_owned(), // the combo box ID column is always a string
///     // No some_check_box - it was not generated for the setter
/// });
///
/// // Get the widgets' data
/// let AppWidgetsPropGetter {
///     some_text,
///     some_combo_box,
///     some_check_box,
/// } = widgets.get_props();
/// ```
pub use woab_macros::PropSync;

pub use builder::*;
pub use builder_dissect::dissect_builder_xml;
pub use error::{Error, Result, WakerPerished};
pub use event_loops_bridge::{
    block_on, close_actix_runtime, is_runtime_running, run_actix_inside_gtk_event_loop, try_block_on, RuntimeStopError,
};
pub use gtk_app_helpers::{main, shutdown_when_last_window_is_closed};
pub use remove::Remove;
pub use signal::{Signal, SignalResult};
pub use signal_routing::{
    route_action, route_signal, GenerateRoutingGtkHandler, IntoGenerateRoutingGtkHandler, NamespacedSignalRouter,
    RawSignalCallback,
};
pub use waking_helpers::{outside, spawn_outside, wake_from, wake_from_signal};
