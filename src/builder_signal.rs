use tokio::sync::mpsc;

/// Type of a gtk signal callback function that operates on uncast glib values
pub type RawSignalCallback = Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>>;

pub fn make_signal_handler<A, S>(
    handler_name: &str,
    ctx: &mut A::Context,
) -> RawSignalCallback 
where
    A: actix::Actor<Context = actix::Context<A>>,
    A: actix::StreamHandler<S>,
    S: BuilderSignal,
{
    let (tx, rx) = mpsc::channel(16);
    A::add_stream(rx, ctx);
    S::bridge_signal(handler_name, tx)
        .ok_or_else(|| format!("Handler '{}' was requested, but only {:?} exist", handler_name, S::list_signals()))
        .unwrap()
}

pub fn connect_signal_handler<A, S, O>(
    object: &O,
    gtk_signal_name: &str,
    handler_name: &str,
    ctx: &mut A::Context,
)
where
    A: actix::Actor<Context = actix::Context<A>>,
    A: actix::StreamHandler<S>,
    S: BuilderSignal,
    O: glib::object::ObjectExt,
{
    let callback = make_signal_handler::<A, S>(handler_name, ctx);
    object.connect_local(gtk_signal_name.as_ref(), false, callback).unwrap();
}

/// Represent a GTK signal that originates from a GTK builder. Refer to [the corresponding derive](derive.BuilderSignal.html).
pub trait BuilderSignal: Sized + 'static {

    /// Generate a signal handler function for GTK.
    ///
    /// The returned function should convert the signals it revceives to the signal type, and
    /// transmit them over `tx`.
    fn bridge_signal(signal: &str, tx: mpsc::Sender<Self>) -> Option<RawSignalCallback>;

    fn list_signals() -> &'static [&'static str];
}
