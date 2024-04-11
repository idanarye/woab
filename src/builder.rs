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
    pub fn instantiate(&self) -> BuilderConnector {
        gtk4::Builder::from_string(&self.xml).into()
    }

    pub fn instantiate_with_scope(&self, scope: &impl IsA<gtk4::BuilderScope>) -> BuilderConnector {
        let builder = gtk4::Builder::new();
        builder.set_scope(Some(scope));
        builder.add_from_string(&self.xml).unwrap();
        builder.into()
    }

    pub fn instantiate_route_to(&self, target: impl crate::IntoGenerateRoutingGtkHandler) -> BuilderConnector {
        let scope = gtk4::BuilderRustScope::new();
        let mut generator = target.into_generate_routing_gtk_handler();
        for signal_name in self.signals.iter() {
            scope.add_callback(signal_name, generator.generate_routing_gtk_handler(signal_name));
        }
        self.instantiate_with_scope(&scope)
    }
}

/// Context for utilizing a `gtk4::Builder` and connecting it to he Actix world.
///
/// It wraps a `gtk4::Builder` instance and provides methods to create actors that are
/// connected to the widgets in that builder.
///
/// See [`BuilderFactory`] for usage example.
pub struct BuilderConnector(pub BuilderConnectorWidgetsOnly);

impl From<gtk4::Builder> for BuilderConnector {
    fn from(builder: gtk4::Builder) -> Self {
        Self(BuilderConnectorWidgetsOnly { builder })
    }
}

impl BuilderConnector {
    /// Get a GTK object from the builder by id.
    pub fn get_object<W>(&self, id: &str) -> Result<W, crate::Error>
    where
        W: IsA<glib::Object>,
    {
        self.0.get_object(id)
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
    pub fn with_object<W>(self, id: &str, dlg: impl FnOnce(W)) -> Self
    where
        W: IsA<glib::Object>,
    {
        self.0.with_object(id, dlg);
        self
        // dlg(self.get_object(id).unwrap());
        // self
    }

    /// Create a widgets struct who's fields are mapped to the builder's widgets.
    pub fn widgets<W>(&self) -> Result<W, <gtk4::Builder as TryInto<W>>::Error>
    where
        gtk4::Builder: TryInto<W>,
    {
        self.0.widgets()
    }

    /// Route the builder's signals to Actix.
    ///
    /// * `bld.connect_to(target)` will connect all the signals defined in the Glade XML to
    ///   `target`, which can be an `actix::Recipient<woab::Signal>`, an `actix::Addr<A>` for A
    ///   that handles `woab::Signal`, or a
    ///   [`woab::NamespacedSignalRouter`](crate::NamespacedSignalRouter).
    /// * `bld.connect_to((tag, target))` will connect all the signals to `target` and add a tag.
    ///   The tag must be `Clone`, and will be sent along with the signal. The signal type will be
    ///   `woab::Signal<T>` where `T` is the type of the tag.
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
    /// builder_factory.instantiate().connect_to(MyActor.start());
    /// ```
    pub fn connect_to(self, _target: impl crate::IntoGenerateRoutingGtkHandler) -> BuilderConnectorWidgetsOnly {
        // todo!()
        //let mut generator = target.into_generate_routing_gtk_handler();
        //use crate::GenerateRoutingGtkHandler;
        //use gtk4::prelude::BuilderExtManual;
        //self.0
        //.builder
        //.connect_signals(move |_, signal_name| generator.generate_routing_gtk_handler(signal_name));
        self.0
    }

    /// Runs a closure and passes the result to [`connect_to`](BuilderConnector::connect_to).
    ///
    /// This is mostly a convenience method. The closure receives the `&BuilderConnector` and can
    /// be use it to retrieve widgets from the builder instantiation and use them in the creation
    /// of the actor.
    ///
    /// ```no_run
    /// # use actix::prelude::*;
    /// # use gtk4::prelude::*;
    /// #[derive(woab::WidgetsFromBuilder)]
    /// struct MyWidgets {
    ///     my_button: gtk4::Button,
    ///     my_textfield: gtk4::Entry,
    /// }
    ///
    /// struct MyActor {
    ///     widgets: MyWidgets,
    ///     my_window: gtk4::Window, // for whatever reason not in `MyWidgets`
    /// }
    /// # impl actix::Actor for MyActor { type Context = actix::Context<Self>; }
    /// # impl actix::Handler<woab::Signal> for MyActor {
    /// #     type Result = woab::SignalResult;
    /// #     fn handle(&mut self, _msg: woab::Signal, _ctx: &mut <Self as actix::Actor>::Context) -> Self::Result {
    /// #         Ok(None)
    /// #     }
    /// # }
    /// # let builder_factory: woab::BuilderFactory = panic!();
    /// builder_factory.instantiate().connect_with(|bld| {
    ///     MyActor {
    ///         widgets: bld.widgets().unwrap(),
    ///         my_window: bld.get_object("my_window").unwrap(),
    ///     }
    ///     .start()
    /// });
    /// ```
    pub fn connect_with<G: crate::IntoGenerateRoutingGtkHandler>(
        self,
        dlg: impl FnOnce(&Self) -> G,
    ) -> BuilderConnectorWidgetsOnly {
        let target = dlg(&self);
        self.connect_to(target)
    }

    pub fn set_application(&self, app: &impl IsA<gtk4::Application>) {
        for object in self.0.builder.objects() {
            if let Some(window) = object.downcast_ref::<gtk4::Window>() {
                window.set_application(Some(app));
            }
        }
    }
}

/// Degraded version of [`BuilderConnector`] that can only be used to get widgets.
///
/// After the `BuilderConnector` connects its signals, they cannot be connected again - so the
/// `BuilderConnector` is consumed. But the widgets are still accessible with this object.
pub struct BuilderConnectorWidgetsOnly {
    pub builder: gtk4::Builder,
}

impl BuilderConnectorWidgetsOnly {
    /// See [`BuilderConnector::get_object`].
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

    /// See [`BuilderConnector::with_object`].
    pub fn with_object<W>(&self, id: &str, dlg: impl FnOnce(W)) -> &Self
    where
        W: IsA<glib::Object>,
    {
        dlg(self.get_object(id).unwrap());
        self
    }

    /// See [`BuilderConnector::widgets`].
    pub fn widgets<W>(&self) -> Result<W, <gtk4::Builder as TryInto<W>>::Error>
    where
        gtk4::Builder: TryInto<W>,
    {
        self.builder.clone().try_into()
    }
}
