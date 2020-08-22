mod event_loops_bridge;
pub mod errors;

pub use woab_macros::{WidgetsFromBuilder, BuilderSignal};

pub use event_loops_bridge::run_actix_inside_gtk_event_loop;

pub trait BuilderSignal: Sized + 'static {
    fn transmit_signal_in_stream_function(signal: &str, tx: tokio::sync::mpsc::Sender<Self>) -> Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>>;

    fn connect_builder_signals<H: actix::StreamHandler<Self>>(ctx: &mut H::Context, builder: &gtk::Builder)
        where <H as actix::Actor>::Context: actix::AsyncContext<H>
    {
        use gtk::prelude::BuilderExtManual;

        let (tx, rx) = tokio::sync::mpsc::channel(16);
        H::add_stream(rx, ctx);
        builder.connect_signals(move |_, signal| {
            Self::transmit_signal_in_stream_function(signal, tx.clone())
        });
    }
}
