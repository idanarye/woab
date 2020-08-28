use tokio::sync::mpsc;

pub trait BuilderSignal: Sized + 'static {
    fn transmit_signal_in_stream_function(signal: &str, tx: mpsc::Sender<Self>) -> Option<Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>>>;

    fn stream_builder_signals(builder: &gtk::Builder) -> Option<mpsc::Receiver<Self>> {
        use gtk::prelude::BuilderExtManual;

        let (tx, rx) = mpsc::channel(16);
        let mut connected_any = false;
        builder.connect_signals(|_, signal| {
            if let Some(handler) = Self::transmit_signal_in_stream_function(signal, tx.clone()) {
                connected_any = true;
                handler
            } else {
                Box::new(|_| None)
            }
        });
        if connected_any {
            Some(rx)
        } else {
            None
        }
    }

    fn connect_builder_signals<H: actix::StreamHandler<Self>>(ctx: &mut H::Context, builder: &gtk::Builder)
        where <H as actix::Actor>::Context: actix::AsyncContext<H>
    {
        if let Some(rx) = Self::stream_builder_signals(builder) {
            H::add_stream(rx, ctx);
        }
    }
}

