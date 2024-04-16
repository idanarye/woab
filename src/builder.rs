use core::convert::TryInto;

use glib::object::IsA;
use gtk4::prelude::*;

use crate::GenerateRoutingGtkHandler;

/// Holds instructions for generating a GTK builder.
///
/// ```no_run
/// # use gtk4::prelude::*;
/// # use woab::BuilderFactory;
/// let builder_xml = r#"
///     <interface>
///       <requires lib="gtk+" version="3.22"/>
///       <object class="GtkButton" id="my_button">
///         ...
///       </object>
///     </interface>
/// "#;
/// let builder_factory: BuilderFactory = builder_xml.to_owned().into();
/// let bld = builder_factory.instantiate();
/// let my_button: gtk4::Button = bld.get_object("my_button").unwrap();
/// ```
///
/// Refer to [`#[derive(woab::Factories)]`](derive.Factories.html) for how to create instances of
/// this struct.
///
/// ```no_run
/// # use actix::prelude::*;
/// # use gtk4::prelude::*;
/// #[derive(woab::Factories)]
/// struct Factories {
///     window: woab::BuilderFactory,
///     row: woab::BuilderFactory,
/// }
///
/// struct WindowActor {
///     widgets: WindowWidgets,
/// }
/// # impl actix::Actor for WindowActor {
/// #     type Context = actix::Context<Self>;
/// # }
///
/// impl actix::Handler<woab::Signal> for WindowActor {
///     type Result = woab::SignalResult;
///
///     fn handle(&mut self, msg: woab::Signal, _ctx: &mut <Self as actix::Actor>::Context) -> Self::Result {
///         Ok(match msg.name() {
///             // Handle the signals of the main window
///             _ => msg.cant_handle()?,
///         })
///     }
/// }
///
/// #[derive(woab::WidgetsFromBuilder)]
/// struct WindowWidgets {
///     window: gtk4::ApplicationWindow,
///     list_box: gtk4::ListBox,
/// }
///
/// #[derive(woab::WidgetsFromBuilder)]
/// struct RowWidgets {
///     row: gtk4::ListBoxRow,
///     label: gtk4::Label,
/// }
///
/// impl actix::Handler<woab::Signal<usize>> for WindowActor {
///     type Result = woab::SignalResult;
///
///     fn handle(&mut self, msg: woab::Signal<usize>, _ctx: &mut <Self as actix::Actor>::Context) -> Self::Result {
///         let row_number = msg.tag();
///         Ok(match msg.name() {
///             // Handle the signals of the row
///             _ => msg.cant_handle()?,
///         })
///     }
/// }
///
/// fn create_window_with_rows(factory: &Factories) {
///     factory.window.instantiate().connect_with(|bld| {
///         let widgets: WindowWidgets = bld.widgets().unwrap();
///         let list_box = widgets.list_box.clone();
///         let window_actor = WindowActor { widgets }.start();
///         for row_number in 0..10 {
///             let row_bld = factory.row.instantiate();
///             let row_widgets: RowWidgets = row_bld.widgets().unwrap();
///             row_widgets.label.set_text(&format!("Row number {}", row_number));
///             list_box.add(&row_widgets.row);
///             row_bld.connect_to((row_number, window_actor.clone()));
///         }
///         window_actor
///     });
/// }
/// ```
#[derive(Clone)]
pub struct BuilderFactory {
    xml: String,
    signals: Vec<String>,
}

fn extract_signals(xml: &str) -> Vec<String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut result = Vec::new();
    loop {
        match reader.read_event_into(&mut buf).unwrap() {
            Event::Eof => {
                break;
            }
            Event::Empty(tag) if tag.name().0 == b"signal" => {
                if let Some(handler) = tag.try_get_attribute("handler").unwrap() {
                    result.push(String::from_utf8(handler.value.to_vec()).unwrap());
                }
            }
            _ => {}
        }
    }
    result
}

impl From<String> for BuilderFactory {
    fn from(xml: String) -> Self {
        let signals = extract_signals(&xml);
        Self { xml, signals }
    }
}

impl BuilderFactory {
    /// Create a `gtk4::Builder` from the instructions inside this factory.
    ///
    /// Note that "creating a builder" means that the GTK widgets are created (but not yet shown)
    ///
    /// This will panic if the builder declares any signals. To connect the signals, use
    /// [`Self::instantiate_route_to`] (or the lower level [`Self::instantiate_with_scope`])
    pub fn instantiate_without_routing_signals(&self) -> BuilderWidgets {
        gtk4::Builder::from_string(&self.xml).into()
    }

    /// Create a `gtk4::Builder` from the instructions inside this factory, routing its signals
    /// using the provided scope.
    ///
    /// Note that "creating a builder" means that the GTK widgets are created (but not yet shown)
    pub fn instantiate_with_scope(&self, scope: &impl IsA<gtk4::BuilderScope>) -> BuilderWidgets {
        let builder = gtk4::Builder::new();
        builder.set_scope(Some(scope));
        builder.add_from_string(&self.xml).unwrap();
        builder.into()
    }

    /// Create a `gtk4::Builder` from the instructions inside this factory, routing its signals
    /// using WoAB's signal routing mechanism.
    ///
    /// Note that "creating a builder" means that the GTK widgets are created (but not yet shown)
    ///
    /// The target can be:
    /// * `Addr` or `Recipient` of an Actix actor that can handle [`woab::Signal`](crate::Signal).
    /// * A [`NamespacedSignalRouter`](crate::NamespacedSignalRouter) (which can be used to route
    ///   to different actors based on the signal's namespace)
    /// * A tuple of a tag object and an `Addr`/`Recipient`/`NamespacedSignalRouter` that can
    ///   handle signals parametrized with the tag's type.
    pub fn instantiate_route_to(&self, target: impl crate::IntoGenerateRoutingGtkHandler) -> BuilderWidgets {
        let scope = gtk4::BuilderRustScope::new();
        let generator = target.into_generate_routing_gtk_handler();
        for signal_name in self.signals.iter() {
            generator.register_into_builder_rust_scope(&scope, signal_name);
        }
        self.instantiate_with_scope(&scope)
    }
}

/// Context for utilizing a `gtk4::Builder`.
///
/// See [`BuilderFactory`] for usage example.
pub struct BuilderWidgets {
    pub builder: gtk4::Builder,
}

impl From<gtk4::Builder> for BuilderWidgets {
    fn from(builder: gtk4::Builder) -> Self {
        Self { builder }
    }
}

impl BuilderWidgets {
    /// Set the [`gtk4::Application`] to all the windows defined in the builder.
    pub fn set_application(&self, app: &impl IsA<gtk4::Application>) {
        for object in self.builder.objects() {
            if let Some(window) = object.downcast_ref::<gtk4::Window>() {
                window.set_application(Some(app));
            }
        }
    }

    /// Get a GTK object from the builder by id.
    pub fn get_object<W>(&self, id: &str) -> Result<W, crate::Error>
    where
        W: IsA<glib::Object>,
    {
        self.builder.object::<W>(id).ok_or_else(|| {
            if let Some(object) = self.builder.object::<glib::Object>(id) {
                crate::Error::IncorrectWidgetTypeInBuilder {
                    widget_id: id.to_owned(),
                    expected_type: <W as glib::types::StaticType>::static_type(),
                    actual_type: object.type_(),
                }
            } else {
                crate::Error::WidgetMissingInBuilder(id.to_owned())
            }
        })
    }

    /// Fluent interface for doing something with a particular object from the builder.
    ///
    /// This is useful for setting up the builder created widgets:
    ///
    /// ```no_run
    /// # use actix::prelude::*;
    /// # use gtk4::prelude::*;
    /// # struct MyActor;
    /// # impl actix::Actor for MyActor { type Context = actix::Context<Self>; }
    /// # impl actix::Handler<woab::Signal> for MyActor {
    /// #     type Result = woab::SignalResult;
    /// #     fn handle(&mut self, _msg: woab::Signal, _ctx: &mut <Self as actix::Actor>::Context) -> Self::Result {
    /// #         Ok(None)
    /// #     }
    /// # }
    /// # let builder_factory: woab::BuilderFactory = panic!();
    /// builder_factory.instantiate()
    ///     .with_object("window", |window: gtk4::ApplicationWindow| {
    ///         window.show();
    ///     })
    ///     .connect_to(MyActor.start());
    /// ```
    pub fn with_object<W>(&self, id: &str, dlg: impl FnOnce(W)) -> &Self
    where
        W: IsA<glib::Object>,
    {
        dlg(self.get_object(id).unwrap());
        self
    }

    /// Create a widgets struct who's fields are mapped to the builder's widgets.
    pub fn widgets<W>(&self) -> Result<W, <gtk4::Builder as TryInto<W>>::Error>
    where
        gtk4::Builder: TryInto<W>,
    {
        self.builder.clone().try_into()
    }
}
