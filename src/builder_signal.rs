use hashbrown::HashMap;
use tokio::sync::mpsc;

/// Type of a gtk signal callback function that operates on uncast glib values.
pub type RawSignalCallback = Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>>;

/// Represent a GTK signal that originates from a GTK builder. Refer to [the corresponding derive](derive.BuilderSignal.html).
pub trait BuilderSignal: Sized + 'static {
    /// Generate a signal handler function for GTK.
    ///
    /// The returned function should convert the signals it revceives to the signal type, and
    /// transmit them over `tx`.
    ///
    /// The `signal` must be one of the signals listed in [`SIGNALS`](BuilderSignal::SIGNALS).
    fn bridge_signal(
        signal: &str,
        tx: mpsc::UnboundedSender<Self>,
        inhibit_dlg: impl 'static + Fn(&Self) -> Option<gtk::Inhibit>,
    ) -> Result<RawSignalCallback, crate::Error>;

    /// The list of signals supported by this [`BuilderSignal`]
    const SIGNALS: &'static [&'static str];

    /// Entry point for connecting the signals supported by the `BuilderSignal`.
    ///
    /// The [`BuilderSignalConnector`] returned by this method can be configured fluent-interface
    /// style and then either:
    ///
    /// * Get passed to [`connect_signals`](crate::ActorBuilder::connect_signals) that connects
    /// signals emitted from a freshly created GTK builder to a freshly created Actix actor.
    /// * Get converted to a [`SignalRouter`] via [`route_to`](BuilderSignalConnector::route_to) to
    /// connect GTK signals - from builders or otherwise - to an Actix actor.
    fn connector() -> BuilderSignalConnector<Self, (), ()> {
        BuilderSignalConnector {
            transformer: (),
            inhibit_dlg: (),
            _phantom_data: Default::default(),
        }
    }
}

#[doc(hidden)]
pub trait SignalTransformer<S>: Clone {
    type Output: 'static;

    fn transform(&self, signal: S) -> Self::Output;
}

impl<S: 'static> SignalTransformer<S> for () {
    type Output = S;

    fn transform(&self, signal: S) -> Self::Output {
        signal
    }
}

impl<S: 'static, T: 'static + Clone> SignalTransformer<S> for (T,) {
    type Output = (T, S);

    fn transform(&self, signal: S) -> Self::Output {
        (self.0.clone(), signal)
    }
}

#[doc(hidden)]
pub trait SignalsInhibit<S>: Clone {
    fn inhibit(&self, signal: &S) -> Option<gtk::Inhibit>;
}

impl<S: 'static> SignalsInhibit<S> for () {
    fn inhibit(&self, _signal: &S) -> Option<gtk::Inhibit> {
        None
    }
}

impl<S: 'static, F> SignalsInhibit<S> for F
where
    F: Clone,
    F: Fn(&S) -> Option<gtk::Inhibit>,
{
    fn inhibit(&self, signal: &S) -> Option<gtk::Inhibit> {
        self(signal)
    }
}

/// Fluent interface for configuring the signal routing from GTK to Actix.
///
/// Typically created from [`BuilderSignal::connector`], `BuilderSignalConnector` can be configured
/// via fluent methods like [`tag`](BuilderSignalConnector::tag) and
/// [`inhibit`](BuilderSignalConnector::inhibit) and then either:
///
/// * Get passed to [`connect_signals`](crate::ActorBuilder::connect_signals) that connects signals
/// emitted from a freshly created GTK builder to a freshly created Actix actor.
/// * Get converted to a [`SignalRouter`] via [`route_to`](BuilderSignalConnector::route_to) to
/// connect GTK signals - from builders or otherwise - to an Actix actor.
pub struct BuilderSignalConnector<S, T, I>
where
    S: BuilderSignal,
    T: Clone,
    I: SignalsInhibit<S>,
{
    transformer: T,
    inhibit_dlg: I,
    _phantom_data: core::marker::PhantomData<S>,
}

impl<S, I> BuilderSignalConnector<S, (), I>
where
    S: BuilderSignal,
    I: SignalsInhibit<S>,
{
    /// Add a tag for identifying the builder instance that generated the signal.
    ///
    /// This is useful when you are using [the version of `connect_signals` that does not also
    /// create an actor](crate::BuilderConnector::connect_signals) to connect multiple
    /// instantiations of the same set of widgets to a single actor, and need to tell the actor
    /// which set of widgets generated the signal.
    ///
    /// The big example in [`BuilderConnector`](crate::BuilderConnector) demonstrates how to use this.
    pub fn tag<T: Clone>(self, tag: T) -> BuilderSignalConnector<S, (T,), I> {
        BuilderSignalConnector {
            transformer: (tag,),
            inhibit_dlg: self.inhibit_dlg,
            _phantom_data: Default::default(),
        }
    }
}

impl<S, T> BuilderSignalConnector<S, T, ()>
where
    S: BuilderSignal,
    T: SignalTransformer<S>,
{
    /// Register a function for setting the return value of the signal.
    ///
    /// GTK requires some signals to return a boolean value - `true` to "inhibit" and not let the
    /// signal pass up the inheritance to other handlers, and `false` to let it. This can be
    /// statically configured in [the `BuilderSignal` derive macro](derive.BuilderSignal.html) with
    /// `#[signal(inhibit = true)` or `#[signal(inhibit = false)`, but sometimes you need to decide
    /// based on the signal's parameters. In such cases you can use this function to register a
    /// closure. If the closure returns `Some`, the signal handler will return the value returned
    /// by the closure. If it returns `None` the signal handler will return the value configured in
    /// the macro.
    ///
    /// ```no_run
    /// # use gtk::prelude::*;
    /// # #[derive(woab::Factories)]
    /// # struct Factories {
    /// #     window: woab::BuilderFactory,
    /// # }
    /// #
    /// # struct MyActor;
    /// # impl actix::Actor for MyActor {
    /// #     type Context = actix::Context<Self>;
    /// # }
    /// # #[derive(woab::BuilderSignal)]
    /// enum MySignal {
    ///     ButtonPress(gtk::Button, #[signal(event)] gdk::EventButton), // needs to return a boolean
    ///     ButtonClick, // needs to return nothing
    /// }
    /// #
    /// # impl actix::StreamHandler<MySignal> for MyActor {
    /// #     fn handle(&mut self, _signal: MySignal, _ctx: &mut <Self as actix::Actor>::Context) {
    /// #     }
    /// # }
    ///
    /// fn some_function_that_creates_stuff(factory: &Factories) {
    ///     factory.window.instantiate().actor()
    ///         .connect_signals(MySignal::connector().inhibit(|signal| match signal {
    ///             MySignal::ButtonPress(_, event) => Some(gtk::Inhibit(event.get_button() == 3)), // inhibit on right click
    ///             _ => None, // all other signals should use the default (static) setting
    ///         }))
    ///         .start(MyActor);
    /// }
    /// ```
    pub fn inhibit<F: Clone + Fn(&S) -> Option<gtk::Inhibit>>(self, dlg: F) -> BuilderSignalConnector<S, T, F> {
        BuilderSignalConnector {
            transformer: self.transformer,
            inhibit_dlg: dlg,
            _phantom_data: Default::default(),
        }
    }
}

impl<S, T, I> BuilderSignalConnector<S, T, I>
where
    S: 'static,
    S: BuilderSignal,
    T: 'static,
    T: SignalTransformer<S>,
    I: 'static,
    I: SignalsInhibit<S>,
{
    /// Create a [`SignalRouter`] that routes signals to the actor owning the supplied context.
    ///
    /// This method only sets the target of the signal routing - the signal router object provides
    /// methods for connecting the signals from GTK. If they are not used no actual signals will be
    /// routed.
    pub fn route_to<A>(self, ctx: &mut A::Context) -> SignalRouter<S, I>
    where
        A: actix::Actor<Context = actix::Context<A>>,
        A: actix::StreamHandler<T::Output>,
    {
        let Self {
            inhibit_dlg,
            transformer,
            ..
        } = self;

        let (tx, rx) = mpsc::unbounded_channel();

        use tokio::stream::StreamExt;
        let rx = rx.map(move |s| transformer.transform(s));
        A::add_stream(rx, ctx);

        SignalRouter { tx, inhibit_dlg }
    }
}

/// Fluent interface for connecting the GTK side of individual signals by name.
///
/// A `SignalRouter` object is created with the route to the Actix actor that will receive the
/// signals, and can be used together with GTK objects to connect the signals from them to that
/// actor.
pub struct SignalRouter<S, I>
where
    S: 'static,
    S: BuilderSignal,
    I: 'static,
    I: SignalsInhibit<S>,
{
    tx: mpsc::UnboundedSender<S>,
    inhibit_dlg: I,
}

impl<S, I> SignalRouter<S, I>
where
    S: 'static,
    S: BuilderSignal,
    I: 'static,
    I: SignalsInhibit<S>,
{
    /// Generate a handler function for a given signal, that can be connected to objects using `gtk-rs` methods.
    ///
    /// ```no_run
    /// #[derive(woab::BuilderSignal)]
    /// enum ButtonSignal {
    ///     Clicked,
    /// }
    ///
    /// fn connect_button_signal<A>(button: &gtk::Button, actor_ctx: &mut A::Context)
    /// where
    ///     A: actix::Actor<Context = actix::Context<A>>,
    ///     A: actix::StreamHandler<ButtonSignal>,
    /// {
    ///     use gtk::prelude::*;
    ///     let router = ButtonSignal::connector().route_to::<A>(actor_ctx);
    ///     button.connect_local("clicked", false, router.handler("Clicked").unwrap()).unwrap();
    /// }
    /// ```
    pub fn handler(&self, signal: &str) -> Result<crate::RawSignalCallback, crate::Error> {
        let inhibit_dlg = self.inhibit_dlg.clone();
        S::bridge_signal(signal, self.tx.clone(), move |signal| inhibit_dlg.inhibit(signal))
    }

    /// Connect a GTK object's signal. Returns itself - so it can be used fluently.
    ///
    /// ```no_run
    /// #[derive(woab::BuilderSignal)]
    /// enum ButtonSignal {
    ///     Clicked1,
    ///     Clicked2,
    /// }
    ///
    /// fn connect_buttons_signal<A>(button1: &gtk::Button, button2: &gtk::Button, actor_ctx: &mut A::Context)
    /// where
    ///     A: actix::Actor<Context = actix::Context<A>>,
    ///     A: actix::StreamHandler<ButtonSignal>,
    /// {
    ///     ButtonSignal::connector().route_to::<A>(actor_ctx)
    ///         .connect(button1, "clicked", "Clicked1").unwrap()
    ///         .connect(button2, "clicked", "Clicked2").unwrap();
    /// }
    /// ```
    pub fn connect<O: glib::ObjectExt>(&self, obj: &O, gtk_signal: &str, actix_signal: &str) -> Result<&Self, crate::Error> {
        obj.connect_local(gtk_signal, false, self.handler(actix_signal)?)?;
        Ok(self)
    }
}

#[doc(hidden)] // Mainly used insternally by `connect_signals`
pub trait RegisterSignalHandlers {
    type MessageType;

    fn register_signal_handlers<A>(self, ctx: &mut A::Context, callbacks: &mut HashMap<&'static str, crate::RawSignalCallback>)
    where
        A: actix::Actor<Context = actix::Context<A>>,
        A: actix::StreamHandler<Self::MessageType>;
}

impl<S, T, I> RegisterSignalHandlers for BuilderSignalConnector<S, T, I>
where
    S: 'static,
    S: BuilderSignal,
    T: 'static,
    T: SignalTransformer<S>,
    I: 'static,
    I: SignalsInhibit<S>,
{
    type MessageType = T::Output;

    fn register_signal_handlers<A>(self, ctx: &mut A::Context, callbacks: &mut HashMap<&'static str, crate::RawSignalCallback>)
    where
        A: actix::Actor<Context = actix::Context<A>>,
        A: actix::StreamHandler<Self::MessageType>,
    {
        let router = self.route_to::<A>(ctx);

        for signal in S::SIGNALS {
            callbacks.insert(
                signal,
                router
                    .handler(signal)
                    .expect("No signal handler even though its from the list"),
            );
        }
    }
}
