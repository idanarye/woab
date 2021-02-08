use std::collections::HashMap;

use tokio::sync::mpsc;

/// Type of a gtk signal callback function that operates on uncast glib values
pub type RawSignalCallback = Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>>;

/// Represent a GTK signal that originates from a GTK builder. Refer to [the corresponding derive](derive.BuilderSignal.html).
pub trait BuilderSignal: Sized + 'static {
    /// Generate a signal handler function for GTK.
    ///
    /// The returned function should convert the signals it revceives to the signal type, and
    /// transmit them over `tx`.
    fn bridge_signal(signal: &str, tx: mpsc::Sender<Self>, inhibit_dlg: impl 'static + Fn(&Self) -> Option<gtk::Inhibit>) -> Option<RawSignalCallback>;

    fn list_signals() -> &'static [&'static str];

    fn connector() -> BuilderSingalConnector<Self, (), ()> {
        BuilderSingalConnector {
            transformer: (),
            inhibit_dlg: (),
            _phantom_data: Default::default(),
        }
    }
}

pub trait RegisterSignalHandlers {
    type MessageType;

    fn register_signal_handlers<A>(self, ctx: &mut A::Context, callbacks: &mut HashMap<&'static str, crate::RawSignalCallback>)
    where
        A: actix::Actor<Context = actix::Context<A>>,
        A: actix::StreamHandler<Self::MessageType>;
}

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

pub struct BuilderSingalConnector<S, T, I>
where
    S: BuilderSignal,
    T: Clone,
    I: SignalsInhibit<S>,
{
    transformer: T,
    inhibit_dlg: I,
    _phantom_data: core::marker::PhantomData<S>,
}

impl<S, I> BuilderSingalConnector<S, (), I>
where
    S: BuilderSignal,
    I: SignalsInhibit<S>,
{
    pub fn tag<T: Clone>(self, tag: T) -> BuilderSingalConnector<S, (T,), I> {
        BuilderSingalConnector {
            transformer: (tag,),
            inhibit_dlg: self.inhibit_dlg,
            _phantom_data: Default::default(),
        }
    }
}

impl<S, T> BuilderSingalConnector<S, T, ()>
where
    S: BuilderSignal,
    T: SignalTransformer<S>,
{
    pub fn inhibit<F: Clone + Fn(&S) -> Option<gtk::Inhibit>>(self, dlg: F) -> BuilderSingalConnector<S, T, F> {
        BuilderSingalConnector {
            transformer: self.transformer,
            inhibit_dlg: dlg,
            _phantom_data: Default::default(),
        }
    }
}

impl<S, T, I> RegisterSignalHandlers for BuilderSingalConnector<S, T, I>
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
        let (tx, rx) = mpsc::channel(16);
        for signal in S::list_signals() {
            let inhibit_dlg = self.inhibit_dlg.clone();
            callbacks.insert(signal, S::bridge_signal(signal, tx.clone(), move |signal| inhibit_dlg.inhibit(signal)).unwrap());
        }
        use tokio::stream::StreamExt;
        let rx = rx.map(move|s| self.transformer.transform(s));
        A::add_stream(rx, ctx);
    }
}