/// Type of a gtk signal callback function that operates on uncast glib values.
pub type RawSignalCallback = Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>>;

pub fn route_signal(
    obj: impl glib::ObjectExt,
    gtk_signal: &str,
    actix_signal: &str,
    target: impl IntoGenerateRoutingGtkHandler,
) -> Result<glib::SignalHandlerId, crate::Error> {
    let handler = target
        .into_generate_routing_gtk_handler()
        .generate_routing_gtk_handler(actix_signal);
    let handler_id = obj.connect_local(gtk_signal, false, handler)?;
    Ok(handler_id)
}

pub trait GenerateRoutingGtkHandler {
    fn generate_routing_gtk_handler(&mut self, signal_name: &str) -> RawSignalCallback;
}

impl<T: Clone + 'static> GenerateRoutingGtkHandler for (T, actix::Recipient<crate::Signal<T>>) {
    fn generate_routing_gtk_handler(&mut self, signal_name: &str) -> RawSignalCallback {
        let signal_name = std::rc::Rc::new(signal_name.to_owned());
        let (tag, recipient) = self.clone();
        Box::new(move |parameters| {
            if let Some(result) = crate::try_block_on(async {
                let signal = crate::Signal::new(signal_name.clone(), parameters.to_owned(), tag.clone());
                recipient.send(signal).await
            }) {
                let result = result.unwrap().unwrap();
                if let Some(gtk::Inhibit(inhibit)) = result {
                    use glib::value::ToValue;
                    Some(inhibit.to_value())
                } else {
                    None
                }
            } else {
                let signal = crate::Signal::new(signal_name.clone(), parameters.to_owned(), tag.clone());
                recipient.do_send(signal).unwrap();
                None
            }
        })
    }
}

pub trait IntoGenerateRoutingGtkHandler {
    type Generator: GenerateRoutingGtkHandler;

    fn into_generate_routing_gtk_handler(self) -> Self::Generator;
}

impl<T: Clone + 'static> IntoGenerateRoutingGtkHandler for (T, actix::Recipient<crate::Signal<T>>) {
    type Generator = Self;

    fn into_generate_routing_gtk_handler(self) -> Self::Generator {
        self
    }
}

impl IntoGenerateRoutingGtkHandler for actix::Recipient<crate::Signal> {
    type Generator = ((), Self);

    fn into_generate_routing_gtk_handler(self) -> Self::Generator {
        ((), self)
    }
}

impl<T: Clone + 'static, A: actix::Actor> IntoGenerateRoutingGtkHandler for (T, actix::Addr<A>)
where
    A: actix::Actor,
    A: actix::Handler<crate::Signal<T>>,
    <A as actix::Actor>::Context: actix::dev::ToEnvelope<A, crate::Signal<T>>,
{
    type Generator = (T, actix::Recipient<crate::Signal<T>>);

    fn into_generate_routing_gtk_handler(self) -> Self::Generator {
        let (tag, actor) = self;
        (tag, actor.recipient())
    }
}

impl<A: actix::Actor> IntoGenerateRoutingGtkHandler for actix::Addr<A>
where
    A: actix::Actor,
    A: actix::Handler<crate::Signal>,
    <A as actix::Actor>::Context: actix::dev::ToEnvelope<A, crate::Signal>,
{
    type Generator = ((), actix::Recipient<crate::Signal>);

    fn into_generate_routing_gtk_handler(self) -> Self::Generator {
        ((), self.recipient())
    }
}
