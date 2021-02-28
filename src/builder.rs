use core::convert::TryInto;
use std::cell::RefCell;

use gtk::Builder;
use hashbrown::HashMap;

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
/// # #[derive(woab::BuilderSignal)]
/// # enum WindowSignal {}
///
/// impl actix::StreamHandler<WindowSignal> for WindowActor {
///     fn handle(&mut self, signal: WindowSignal, _ctx: &mut <Self as actix::Actor>::Context) {
///         match signal {
///             // Handle the signals of the main window
///         }
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
/// # #[derive(woab::BuilderSignal)]
/// # enum RowSignal {}
///
/// impl actix::StreamHandler<(usize, RowSignal)> for WindowActor {
///     fn handle(&mut self, (row_number, signal): (usize, RowSignal), _ctx: &mut <Self as actix::Actor>::Context) {
///         match signal {
///             // Handle the signals of row #row_number
///         }
///     }
///
///     // ******************************************************
///     // * VERY IMPORTANT! Otherwise once one row is deleted, *
///     // * its signal stream will be closed and the default   *
///     // * implementation will close the WindowActor.         *
///     // ******************************************************
///     fn finished(&mut self, _ctx: &mut Self::Context) {}
/// }
///
/// fn create_window_with_rows(factory: &Factories) {
///     factory.window.instantiate().actor()
///         .connect_signals(WindowSignal::connector())
///         .create(|ctx| {
///             let widgets: WindowWidgets = ctx.widgets().unwrap();
///             for row_number in 0..10 {
///                 let row_widgets: RowWidgets = factory.row.instantiate()
///                     .connect_signals(ctx, RowSignal::connector().tag(row_number))
///                     .widgets().unwrap();
///                 row_widgets.label.set_text(&format!("Roe number {}", row_number));
///                 widgets.list_box.add(&row_widgets.row);
///             }
///             WindowActor { widgets }
///         });
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
/// # Caveats
///
/// If you connect signals via a builder connector, they will only be connected once the connector
/// is dropped. If you need the signals connected before the connector is naturally dropped (e.g. -
/// if you start `gtk::main()` in the same scope) use [`finish`](BuilderConnector::finish).
pub struct BuilderConnector {
    builder: gtk::Builder,
    callbacks: RefCell<HashMap<&'static str, crate::RawSignalCallback>>,
}

impl From<gtk::Builder> for BuilderConnector {
    fn from(builder: gtk::Builder) -> Self {
        Self {
            builder,
            callbacks: RefCell::new(HashMap::new()),
        }
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

    /// Route signals defined by the builder to an Actix actor.
    ///
    /// This only connects the signals defined by the builder signal connector passed in the second
    /// argument. Such a connector is usually obtained from
    /// [`BuilderSignal::connector`](crate::BuilderSignal::connector).
    ///
    /// # Caveats
    ///
    /// The signals will only be connected when the builder is dropped (either when the scope ends
    /// or when you call [`finish`](BuilderConnector::finish))
    pub fn connect_signals<A, R>(&self, ctx: &mut actix::Context<A>, register_signal_handlers: R) -> &Self
    where
        A: actix::Actor<Context = actix::Context<A>>,
        R: crate::RegisterSignalHandlers,
        R::MessageType: 'static,
        A: actix::StreamHandler<R::MessageType>,
    {
        let mut callbacks = self.callbacks.borrow_mut();
        register_signal_handlers.register_signal_handlers::<A>(ctx, &mut callbacks);
        self
    }

    /// "Entry point" for creating an Actix actor that uses the builder.
    pub fn actor<A: actix::Actor<Context = actix::Context<A>>>(&self) -> ActorBuilder<A> {
        let (_, rx) = actix::dev::channel::channel(16);
        let ctx = actix::Context::with_receiver(rx);
        ActorBuilder {
            builder_connector: &self,
            actor_context: ctx,
        }
    }

    /// Perform the actual signal connection.
    ///
    /// Until this method is called, or until the `BuilderConnector` is dropped, the signals will
    /// not be connected and if GTK runs during that time the signals it emits will not be sent to
    /// the Actix actor(s).
    pub fn finish(self) {
        std::mem::drop(self)
    }
}

impl Drop for BuilderConnector {
    fn drop(&mut self) {
        use gtk::prelude::BuilderExtManual;

        let mut callbacks = self.callbacks.borrow_mut();
        self.builder
            .connect_signals(move |_, signal| callbacks.remove(signal).unwrap_or_else(|| Box::new(|_| None)));
    }
}

/// Fluent interface for launching an Actix actor that works with a GTK builder's instantiation.
pub struct ActorBuilder<'a, A: actix::Actor<Context = actix::Context<A>>> {
    builder_connector: &'a BuilderConnector,
    actor_context: A::Context,
}

impl<'a, A: actix::Actor<Context = actix::Context<A>>> ActorBuilder<'a, A> {
    /// Start a new actor by value.
    ///
    /// The actor will receive builder signals registered by calls to
    /// [`connect_signals`](ActorBuilder::connect_signals) on this `ActorBuilder`.
    pub fn start(self, actor: A) -> actix::Addr<A> {
        self.actor_context.run(actor)
    }

    /// Start a new actor from a closure.
    ///
    /// The closure receives an [`ActorBuilderContext`] which can be used for getting the widgets
    /// and for accessing the Actix actor context.
    ///
    /// The actor will receive builder signals registered by calls to
    /// [`connect_signals`](ActorBuilder::connect_signals) on this `ActorBuilder`.
    pub fn create<'b>(self, dlg: impl FnOnce(&mut ActorBuilderContext<'a, A>) -> A) -> actix::Addr<A>
    where
        'a: 'b,
    {
        let mut actor_builder_context = ActorBuilderContext {
            builder_connector: self.builder_connector,
            actor_context: self.actor_context,
        };
        let actor = dlg(&mut actor_builder_context);
        actor_builder_context.actor_context.run(actor)
    }

    /// Start a new actor from a closure that can return an error.
    ///
    /// The closure receives an [`ActorBuilderContext`] which can be used for getting the widgets
    /// and for accessing the Actix actor context.
    ///
    /// The actor will receive builder signals registered by calls to
    /// [`connect_signals`](ActorBuilder::connect_signals) on this `ActorBuilder`.
    pub fn try_create<'b, E>(self, dlg: impl FnOnce(&mut ActorBuilderContext<'a, A>) -> Result<A, E>) -> Result<actix::Addr<A>, E>
    where
        'a: 'b,
    {
        let mut actor_builder_context = ActorBuilderContext {
            builder_connector: self.builder_connector,
            actor_context: self.actor_context,
        };
        let actor = dlg(&mut actor_builder_context)?;
        Ok(actor_builder_context.actor_context.run(actor))
    }

    /// Connect signals from the GTK builder represented by this `ActorBuilder` to the Actix actor
    /// represented by this `ActorBuilder`.
    ///
    /// The `RegisterSignalHandlers` required for running this method is usually obtained from
    /// [`BuilderSignal::connector`](crate::BuilderSignal::connector).
    pub fn connect_signals<R>(mut self, register_signal_handlers: R) -> Self
    where
        R: crate::RegisterSignalHandlers,
        R::MessageType: 'static,
        A: actix::StreamHandler<R::MessageType>,
    {
        self.builder_connector
            .connect_signals(&mut self.actor_context, register_signal_handlers);
        self
    }

    /// Create a widgets struct who's fields are mapped to the builder's widgets.
    pub fn widgets<W>(&self) -> Result<W, <gtk::Builder as TryInto<W>>::Error>
    where
        gtk::Builder: TryInto<W>,
    {
        self.builder_connector.widgets()
    }

    /// Get a reference to the Actix context.
    pub fn actor_context(&self) -> &A::Context {
        &self.actor_context
    }

    /// Get a mutable reference to the Actix context.
    pub fn actor_context_mut(&mut self) -> &mut A::Context {
        &mut self.actor_context
    }
}

/// Context for creating the actor from the [`ActorBuilder`].
///
/// In addition to its own methods, this context derefs to the Actix context, which can be used for
/// getting the address, connecting streams, spawning futures, and everything else a mutable
/// reference to an Actix context allows.
pub struct ActorBuilderContext<'a, A: actix::Actor<Context = actix::Context<A>>> {
    builder_connector: &'a BuilderConnector,
    actor_context: A::Context,
}

impl<A: actix::Actor<Context = actix::Context<A>>> ActorBuilderContext<'_, A> {
    /// Create a widgets struct who's fields are mapped to the builder's widgets.
    pub fn widgets<W>(&self) -> Result<W, <gtk::Builder as TryInto<W>>::Error>
    where
        gtk::Builder: TryInto<W>,
    {
        self.builder_connector.widgets()
    }
}

impl<'a, A> std::ops::Deref for ActorBuilderContext<'a, A>
where
    A: actix::Actor<Context = actix::Context<A>>,
{
    type Target = actix::Context<A>;

    fn deref(&self) -> &Self::Target {
        &self.actor_context
    }
}

impl<'a, A> std::ops::DerefMut for ActorBuilderContext<'a, A>
where
    A: actix::Actor<Context = actix::Context<A>>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.actor_context
    }
}
