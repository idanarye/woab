use std::collections::HashMap;
use core::convert::TryInto;
use std::cell::RefCell;

use gtk::Builder;
use tokio::sync::mpsc;

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

/// Holds instructions for generating GTK widgets and connecing them to Actix actors.
///
/// 1. The first generic parameter, `A`, is the actor type.
/// 2. The second generic parameter, `W`, is the widgets type. Typically created with
///    [`#[derive(woab::WidgetsFromBuilder)]`](derive.WidgetsFromBuilder.html) on a struct that
///    specifies the widgets of the Glade XML file that the code needs to access.
/// 3. The third generic parameter, `S`, is the signal type. Typically created with
///    [`#[derive(woab::BuilderSignal)]`](derive.BuilderSignal.html) on an enum that lists the
///    signals from the Glade XML file that the code wants to handle.
///
/// `A` can be `()` if the widgets are to be handled by an existing actor - usually the one that
/// handles their parent widget. `S` can also be `()` if it is desired to just generate widgets
/// without connecting a signal.
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

/// Context for utilizing a `gtk::Builder` and connecting it to he Actix world.
/// 
/// It wraps a `gtk::Builder` instance and provides methods to create actors that are
/// connected to the widgets in that builder.
///
/// See [`woab::BuilderFactory`](struct.BuilderFactory.html) for usage example.
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

    /// Create a widgets struct (as defined by the `W` generic parameter of
    /// [`woab::BuilderFactory`](struct.BuilderFactory.html)) and map the builder's widgets to its fields.
    pub fn widgets<W>(&self) -> Result<W, <gtk::Builder as TryInto<W>>::Error>
        where gtk::Builder: TryInto<W>
    {
        self.builder.clone().try_into()
    }

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

    pub fn actor<A: actix::Actor<Context = actix::Context<A>>>(&self) -> ActorBuilder<A> {

        let (_, rx) = actix::dev::channel::channel(16);
        let ctx = actix::Context::with_receiver(rx);
        ActorBuilder {
            builder_connector: &self,
            actor_context: ctx,
        }
    }

    /// Create a stream of all the signals.
    ///
    /// Will return `None` if there are no signals, to allow avoiding closed streams.
    pub fn finish(self) {
        std::mem::drop(self)
    }
}

impl Drop for BuilderConnector {
    fn drop(&mut self) {
        use gtk::prelude::BuilderExtManual;

        let mut callbacks = self.callbacks.borrow_mut();
        self.builder.connect_signals(move |_, signal| {
            callbacks.remove(signal).unwrap_or_else(|| Box::new(|_| None))
        });
    }
}

pub fn make_signal_handler<A, S>(
    handler_name: &str,
    ctx: &mut A::Context,
) -> crate::RawSignalCallback 
where
    A: actix::Actor<Context = actix::Context<A>>,
    A: actix::StreamHandler<S>,
    S: crate::BuilderSignal,
{
    let (tx, rx) = mpsc::channel(16);
    A::add_stream(rx, ctx);
    S::bridge_signal(handler_name, tx, |_| None)
        .ok_or_else(|| format!("Handler '{}' was requested, but only {:?} exist", handler_name, S::list_signals()))
        .unwrap()
}

pub fn connect_signal_handler<A, S, O>(
    object: &O,
    gtk_signal_name: &str,
    handler_name: &str,
    ctx: &mut A::Context,
) -> Result<glib::signal::SignalHandlerId, glib::error::BoolError>
where
    A: actix::Actor<Context = actix::Context<A>>,
    A: actix::StreamHandler<S>,
    S: crate::BuilderSignal,
    O: glib::object::ObjectExt,
{
    let callback = make_signal_handler::<A, S>(handler_name, ctx);
    object.connect_local(gtk_signal_name.as_ref(), false, callback)
}

pub struct ActorBuilder<'a, A: actix::Actor<Context = actix::Context<A>>> {
    builder_connector: &'a BuilderConnector,
    actor_context: A::Context,
}

impl<'a, A: actix::Actor<Context = actix::Context<A>>> ActorBuilder<'a, A> {
    pub fn start(self, actor: A) -> actix::Addr<A> {
        self.actor_context.run(actor)
    }

    pub fn create<'b>(self, dlg: impl FnOnce(&mut ActorBuilderContext<'a, A>) -> A) -> actix::Addr<A> where 'a: 'b {
        let mut actor_builder_context = ActorBuilderContext {
            builder_connector: self.builder_connector,
            actor_context: self.actor_context,
        };
        let actor = dlg(&mut actor_builder_context);
        actor_builder_context.actor_context.run(actor)
    }

    pub fn try_create<'b, E>(self, dlg: impl FnOnce(&mut ActorBuilderContext<'a, A>) -> Result<A, E>) -> Result<actix::Addr<A>, E> where 'a: 'b {
        let mut actor_builder_context = ActorBuilderContext {
            builder_connector: self.builder_connector,
            actor_context: self.actor_context,
        };
        let actor = dlg(&mut actor_builder_context)?;
        Ok(actor_builder_context.actor_context.run(actor))
    }

    pub fn connect_signals<R>(mut self, register_signal_handlers: R) -> Self
    where
        R: crate::RegisterSignalHandlers,
        R::MessageType: 'static,
        A: actix::StreamHandler<R::MessageType>,
    {
        self.builder_connector.connect_signals(&mut self.actor_context, register_signal_handlers);
        self
    }
}

pub struct ActorBuilderContext<'a, A: actix::Actor<Context = actix::Context<A>>> {
    builder_connector: &'a BuilderConnector,
    actor_context: A::Context,
}

impl<A: actix::Actor<Context = actix::Context<A>>> ActorBuilderContext<'_, A> {
    pub fn widgets<W>(&self) -> Result<W, <gtk::Builder as TryInto<W>>::Error> 
        where gtk::Builder: TryInto<W>
    {
        self.builder_connector.widgets()
    }
}

impl <'a, A> std::ops::Deref for ActorBuilderContext<'a, A>
where
    A: actix::Actor<Context = actix::Context<A>>
{
    type Target = actix::Context<A>;

    fn deref(&self) -> &Self::Target {
        &self.actor_context
    }
}

impl <'a, A> std::ops::DerefMut for ActorBuilderContext<'a, A>
where
    A: actix::Actor<Context = actix::Context<A>>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.actor_context
    }
}
