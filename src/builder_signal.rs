use tokio::sync::mpsc;

/// Type of a gtk signal callback function that operates on uncast glib values
pub type RawSignalCallback = Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>>;

/// Represent a GTK signal that originates from a GTK builder. Refer to [the corresponding derive](derive.BuilderSignal.html).
pub trait BuilderSignal: Sized + 'static {

    /// Generate a signal handler function for GTK.
    ///
    /// The returned function should convert the signals it revceives to the signal type, and
    /// transmit them over `tx`.
    fn bridge_signal(signal: &str, tx: mpsc::Sender<Self>) -> Option<RawSignalCallback>;

    fn list_signals() -> &'static [&'static str];
}
