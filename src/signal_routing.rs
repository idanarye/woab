use std::rc::Rc;

/// Type of a gtk signal callback function that operates on uncast glib values.
pub type RawSignalCallback = Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>>;

/// Route a GTK signal to an Actix actor that can handle [`woab::Signal`](crate::Signal).
///
/// ```no_run
/// let widget: gtk4::Button;
/// let target: actix::Recipient<woab::Signal>; // `actix::Addr` is also supported
/// # widget = panic!();
/// # target = panic!();
/// woab::route_signal(&widget, "clicked", "button_clicked", target).unwrap();
/// ```
///
/// * The `actix_signal` argument is the signal name used for identifying the signal inside the actor.
pub fn route_signal(
    obj: &impl glib::object::ObjectExt,
    gtk_signal: &str,
    actix_signal: &str,
    target: impl IntoGenerateRoutingGtkHandler,
) -> Result<glib::SignalHandlerId, crate::Error> {
    let handler = target
        .into_generate_routing_gtk_handler()
        .generate_routing_gtk_handler(actix_signal);
    let handler_id = obj.connect_local(gtk_signal, false, handler);
    Ok(handler_id)
}

/// Route a GIO action to an Actix actor that can handle [`woab::Signal`](crate::Signal).
/// ```no_run
/// let action = gio::SimpleAction::new("action_name", None);
/// let target: actix::Recipient<woab::Signal>; // `actix::Addr` is also supported
/// # target = panic!();
/// woab::route_action(&action, target).unwrap();
/// ```
///
/// * The action name will be used for identifying the signal inside the actor.
/// * Both stateless and stateful actions are supported - the correct signal will be chosen
///   automatically.
/// * To get the action parameter/state inside the handler, use the
///   [`action_param`](crate::Signal::action_param) method.
pub fn route_action(
    action: &(impl glib::object::ObjectExt + gio::prelude::ActionExt),
    target: impl IntoGenerateRoutingGtkHandler,
) -> Result<glib::SignalHandlerId, crate::Error> {
    let signal = if action.state().is_some() {
        "change-state"
    } else {
        "activate"
    };
    route_signal(action, signal, action.name().as_str(), target)
}

fn panic_if_signal_cannot_be_queued(signal_name: &str, parameters: &[glib::Value]) {
    for (i, param) in parameters.iter().enumerate() {
        let param_type = param.type_();
        if param_type.name().ends_with("Context") {
            panic!(
                concat!(
                    "Signal {:?}'s param at position {} is a {}. ",
                    "Signals with context params cannot be invoked from inside the Actix runtime. ",
                    "Try running whatever triggered it with `woab::outside()` or `woab::spawn_outside()",
                ),
                signal_name, i, param_type,
            );
        }
    }
}

fn run_signal_routing_future(
    future: impl core::future::Future<Output = Result<Result<Option<gtk4::Inhibit>, crate::Error>, actix::MailboxError>> + 'static,
    signal_name: &Rc<String>,
    parameters: &[glib::Value],
) -> Option<glib::Value> {
    match crate::try_block_on(future) {
        Ok(result) => {
            let result = result.unwrap().unwrap();
            if let Some(gtk4::Inhibit(inhibit)) = result {
                use glib::value::ToValue;
                Some(inhibit.to_value())
            } else {
                None
            }
        }
        Err(future) => {
            panic_if_signal_cannot_be_queued(signal_name, parameters);
            let signal_name = signal_name.clone();
            actix::spawn(async move {
                let result = future.await.unwrap().unwrap();
                if let Some(result) = result {
                    panic!(
                        concat!(
                            "Signal {:?}, was invoked inside the Actix runtime and had to be queued, ",
                            "but it returned {:?} - which is not supported for queued signals. ",
                            "Try running whatever triggered it with `woab::outside()` or `woab::spawn_outside()",
                        ),
                        signal_name.as_str(),
                        result,
                    );
                }
            });
            None
        }
    }
}

#[doc(hidden)]
pub trait GenerateRoutingGtkHandler {
    fn generate_routing_gtk_handler(&mut self, signal_name: &str) -> RawSignalCallback;
}

impl<T: Clone + 'static> GenerateRoutingGtkHandler for (T, actix::Recipient<crate::Signal<T>>) {
    fn generate_routing_gtk_handler(&mut self, signal_name: &str) -> RawSignalCallback {
        let signal_name = Rc::new(signal_name.to_owned());
        let (tag, recipient) = self.clone();
        Box::new(move |parameters| {
            let signal = crate::Signal::new(signal_name.clone(), parameters.to_owned(), tag.clone());
            run_signal_routing_future(recipient.send(signal), &signal_name, parameters)
        })
    }
}

#[doc(hidden)]
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

/// Signal
#[derive(Default)]
pub struct NamespacedSignalRouter<T> {
    targets: hashbrown::HashMap<String, NamespacedSignalRouterTarget<T>>,
}

#[derive(Clone)]
struct NamespacedSignalRouterTarget<T> {
    recipient: actix::Recipient<crate::Signal<T>>,
    strip_namespace: bool,
}

/// Split signals from the same builder to multiple actors, based on namespaces.
///
/// To be passed to [`connect_to`](crate::BuilderConnector::connect_to) instead of an
/// `Addr`/`Recipient`. The namespace format is `"namespace::signal"`. The
/// [`route`](NamespacedSignalRouter::route) method will automatically detect the namespace based
/// on the actor type, and will strip it from the signals passed to that actor.
///
/// ```no_run
/// # use actix::prelude::*;
/// struct Actor1;
/// # impl actix::Actor for Actor1 { type Context = actix::Context<Self>; }
///
/// impl actix::Handler<woab::Signal> for Actor1 {
///     type Result = woab::SignalResult;
///
///     fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
///         Ok(match msg.name() {
///             "signal1" => {
///                 // Handles "Actor1::signal1"
///                 None
///             }
///             "signal2" => {
///                 // Handles "Actor1::signal2"
///                 None
///             }
///             _ => msg.cant_handle()?,
///         })
///     }
/// }
///
/// struct Actor2;
/// # impl actix::Actor for Actor2 { type Context = actix::Context<Self>; }
///
/// impl actix::Handler<woab::Signal> for Actor2 {
///     type Result = woab::SignalResult;
///
///     fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
///         Ok(match msg.name() {
///             "signal3" => {
///                 // Handles "Actor2::signal3"
///                 None
///             }
///             "signal4" => {
///                 // Handles "Actor2::signal4"
///                 None
///             }
///             _ => msg.cant_handle()?,
///         })
///     }
/// }
///
/// # let factory: woab::BuilderFactory = panic!();
/// factory.instantiate().connect_to(
///     woab::NamespacedSignalRouter::default()
///     .route(Actor1.start())
///     .route(Actor2.start())
/// );
/// ```
impl<T> NamespacedSignalRouter<T> {
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

    /// Route signals of the specified namespace, keeping the namespace.
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

    /// Route signals of the specified namespace, stripping the namespace.
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

    /// Route signals of automatically detected namespace, stripping the namespace.
    ///
    /// The namespace is the actor's namespace, without any qualifications, and generics, and
    /// without the name of the module it is in.
    pub fn route<A>(mut self, actor: actix::Addr<A>) -> Self
    where
        T: 'static,
        A: actix::Actor,
        A: actix::Handler<crate::Signal<T>>,
        <A as actix::Actor>::Context: actix::dev::ToEnvelope<A, crate::Signal<T>>,
    {
        let namespace = core::any::type_name::<A>();
        let namespace = namespace.split('<').next().unwrap(); // strip generics
        let namespace = namespace.split("::").last().unwrap(); // strip package prefix (unreliable)
        let namespace = namespace.split("; ").last().unwrap(); // strip qualifies
        let namespace = namespace.split('&').last().unwrap(); // strip reference
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

        let signal_name = Rc::new(
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
            let signal = crate::Signal::new(signal_name.clone(), parameters.to_owned(), tag.clone());
            run_signal_routing_future(target.recipient.send(signal), &signal_name, parameters)
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
