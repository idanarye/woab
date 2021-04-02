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

pub struct NamespacedSignalRouter<T> {
    targets: hashbrown::HashMap<String, NamespacedSignalRouterTarget<T>>,
}

#[derive(Clone)]
struct NamespacedSignalRouterTarget<T> {
    recipient: actix::Recipient<crate::Signal<T>>,
    strip_namespace: bool,
}

impl<T> NamespacedSignalRouter<T> {
    pub fn new() -> Self {
        NamespacedSignalRouter {
            targets: Default::default(),
        }
    }

    fn add_target(&mut self, namespace: &str, target: NamespacedSignalRouterTarget<T>) {
        match self.targets.entry(namespace.to_owned()) {
            hashbrown::hash_map::Entry::Occupied(_) => {
                panic!("Namespace {:?} is already routed", namespace);
            }
            hashbrown::hash_map::Entry::Vacant(entry) => {
                entry.insert(target);
            }
        }
    }

    pub fn route_ns(mut self, namespace: &str, recipient: actix::Recipient<crate::Signal<T>>) -> Self {
        self.add_target(
            namespace,
            NamespacedSignalRouterTarget {
                recipient,
                strip_namespace: false,
            },
        );
        self
    }

    pub fn route_strip_ns(mut self, namespace: &str, recipient: actix::Recipient<crate::Signal<T>>) -> Self {
        self.add_target(
            namespace,
            NamespacedSignalRouterTarget {
                recipient,
                strip_namespace: true,
            },
        );
        self
    }

    pub fn route<A>(mut self, actor: actix::Addr<A>) -> Self
    where
        T: 'static,
        A: actix::Actor,
        A: actix::Handler<crate::Signal<T>>,
        <A as actix::Actor>::Context: actix::dev::ToEnvelope<A, crate::Signal<T>>,
    {
        let namespace = core::any::type_name::<A>();
        let namespace = namespace.split("<").next().unwrap(); // strip generics
        let namespace = namespace.split("::").last().unwrap(); // strip package prefix (unreliable)
        let namespace = namespace.split("; ").last().unwrap(); // strip qualifies
        let namespace = namespace.split("&").last().unwrap(); // strip reference
        self.add_target(
            namespace,
            NamespacedSignalRouterTarget {
                recipient: actor.recipient(),
                strip_namespace: true,
            },
        );
        self
    }
}

impl<T: Clone + 'static> crate::GenerateRoutingGtkHandler for (T, NamespacedSignalRouter<T>) {
    fn generate_routing_gtk_handler(&mut self, signal_name: &str) -> RawSignalCallback {
        let (tag, router) = self;
        let signal_namespace = {
            let mut parts = signal_name.split("::");
            if let Some(signal_namespace) = parts.next() {
                if parts.next().is_none() {
                    panic!("Signal {:?} does not have a namespace", signal_name)
                } else {
                    signal_namespace
                }
            } else {
                panic!("Signal is empty")
            }
        };

        let target = if let Some(target) = router.targets.get(signal_namespace) {
            target.clone()
        } else {
            panic!("Unknown namespace {:?}", signal_namespace)
        };

        let signal_name = std::rc::Rc::new(
            if target.strip_namespace {
                let (_, without_namespace) = signal_name.split_at(signal_namespace.len() + 2);
                without_namespace
            } else {
                signal_name
            }
            .to_owned(),
        );
        let tag = tag.clone();
        Box::new(move |parameters| {
            if let Some(result) = crate::try_block_on(async {
                let signal = crate::Signal::new(signal_name.clone(), parameters.to_owned(), tag.clone());
                target.recipient.send(signal).await
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
                target.recipient.do_send(signal).unwrap();
                None
            }
        })
    }
}

impl<T: Clone + 'static> IntoGenerateRoutingGtkHandler for (T, NamespacedSignalRouter<T>) {
    type Generator = Self;

    fn into_generate_routing_gtk_handler(self) -> Self::Generator {
        self
    }
}

impl IntoGenerateRoutingGtkHandler for NamespacedSignalRouter<()> {
    type Generator = ((), Self);

    fn into_generate_routing_gtk_handler(self) -> Self::Generator {
        ((), self)
    }
}
