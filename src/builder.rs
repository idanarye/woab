use core::convert::TryInto;

use gtk::Builder;

/// Holds instructions for generating a GTK builder.
///
/// ```no_run
/// # use gtk::prelude::*;
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
/// let builder = builder_factory.instantiate();
/// let my_button: gtk::Button = builder.get_object("my_button").unwrap();
/// ```
///
/// Refer to [`#[derive(woab::Factories)]`](derive.Factories.html) for how to create instances of
/// this struct.
///
/// ```no_run
/// # use actix::prelude::*;
/// # use gtk::prelude::*;
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
///     window: gtk::ApplicationWindow,
///     list_box: gtk::ListBox,
/// }
///
/// #[derive(woab::WidgetsFromBuilder)]
/// struct RowWidgets {
///     row: gtk::ListBoxRow,
///     label: gtk::Label,
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
pub struct BuilderFactory(String);

impl From<String> for BuilderFactory {
    fn from(xml: String) -> Self {
        Self(xml)
    }
}

impl BuilderFactory {
    /// Create a `gtk::Builder` from the instructions inside this factory.
    ///
    /// Note that "creating a builder" means that the GTK widgets are created (but not yet shown)
    pub fn instantiate(&self) -> BuilderConnector {
        Builder::from_string(&self.0).into()
    }
}

/// Context for utilizing a `gtk::Builder` and connecting it to he Actix world.
///
/// It wraps a `gtk::Builder` instance and provides methods to create actors that are
/// connected to the widgets in that builder.
///
/// See [`BuilderFactory`] for usage example.
///
/// # Pitfalls
///
/// If you connect signals via a builder connector, they will only be connected once the connector
/// is dropped. If you need the signals connected before the connector is naturally dropped (e.g. -
/// if you start `gtk::main()` in the same scope) use [`finish`](BuilderConnector::finish).
pub struct BuilderConnector {
    builder: gtk::Builder,
}

impl From<gtk::Builder> for BuilderConnector {
    fn from(builder: gtk::Builder) -> Self {
        Self { builder }
    }
}

impl BuilderConnector {
    /// Get a GTK object from the builder by id.
    pub fn get_object<W>(&self, id: &str) -> Result<W, crate::Error>
    where
        W: glib::IsA<glib::Object>,
    {
        use gtk::prelude::BuilderExtManual;
        self.builder.get_object::<W>(id).ok_or_else(|| {
            if let Some(object) = self.builder.get_object::<glib::Object>(id) {
                use glib::object::ObjectExt;
                crate::Error::IncorrectWidgetTypeInBuilder {
                    widget_id: id.to_owned(),
                    expected_type: <W as glib::types::StaticType>::static_type(),
                    actual_type: object.get_type(),
                }
            } else {
                crate::Error::WidgetMissingInBuilder(id.to_owned())
            }
        })
    }

    /// Create a widgets struct who's fields are mapped to the builder's widgets.
    pub fn widgets<W>(&self) -> Result<W, <gtk::Builder as TryInto<W>>::Error>
    where
        gtk::Builder: TryInto<W>,
    {
        self.builder.clone().try_into()
    }

    pub fn connect_to(&self, target: impl crate::IntoGenerateRoutingGtkHandler) -> &Self {
        let mut generator = target.into_generate_routing_gtk_handler();
        use crate::GenerateRoutingGtkHandler;
        use gtk::prelude::BuilderExtManual;
        self.builder
            .connect_signals(move |_, signal_name| generator.generate_routing_gtk_handler(signal_name));
        self
    }

    pub fn connect_with<G: crate::IntoGenerateRoutingGtkHandler>(&self, dlg: impl FnOnce(&Self) -> G) -> &Self {
        self.connect_to(dlg(self))
    }
}
