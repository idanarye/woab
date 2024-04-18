use std::rc::Rc;

use send_wrapper::SendWrapper;

/// The generic signal WoAB uses.
///
/// The signal contains a name, list of parameters, and an optional tag. Route the signals from GTK
/// to Actix actors using
/// [`BuilderFactory::instantiate_route_to`](crate::BuilderFactory::instantiate_route_to),
/// [`woab::route_signal`](crate::route_signal) or [`woab::route_action`](crate::route_action) and
/// handle them as actix messages, matching on their [`name`](Signal::name) and using
/// [`woab::params!`](crate::params!) to get their parameters.
pub struct Signal<T = ()>(SendWrapper<SignalData<T>>);

/// Result type for Actix handlers that handle [`woab::Signal`](Signal).
pub type SignalResult = Result<Option<glib::Propagation>, crate::Error>;

impl<T> actix::Message for Signal<T> {
    type Result = SignalResult;
}

#[doc(hidden)]
pub struct SignalData<T> {
    name: Rc<String>,
    parameters: Vec<glib::Value>,
    tag: T,
}

impl<T: Clone> Signal<T> {
    pub fn creator(name: &str, tag: T) -> impl Fn(Vec<glib::Value>) -> Self {
        let name = Rc::new(name.to_owned());
        move |parameters| {
            Signal(SendWrapper::new(SignalData {
                name: name.clone(),
                parameters,
                tag: tag.clone(),
            }))
        }
    }
}

impl<T> SignalData<T> {
    fn raw_param(&self, index: usize) -> Result<&glib::Value, crate::Error> {
        self.parameters
            .get(index)
            .ok_or_else(|| crate::Error::SignalParameterIndexOutOfBound {
                signal: self.name.as_str().to_owned(),
                index,
                num_parameters: self.parameters.len(),
            })
    }

    fn param<'a, P>(&'a self, index: usize) -> Result<P, crate::Error>
    where
        P: glib::value::FromValue<'a>,
        P: glib::types::StaticType,
    {
        let value = self.raw_param(index)?;
        if let Ok(value) = value.get() {
            Ok(value)
        } else {
            Err(crate::Error::IncorrectSignalParameterType {
                signal: self.name.as_str().to_owned(),
                index,
                expected_type: <P as glib::types::StaticType>::static_type(),
                actual_type: value.type_(),
            })
        }
    }
}

impl<T> Signal<T> {
    pub fn new(name: Rc<String>, parameters: Vec<glib::Value>, tag: T) -> Self {
        Signal(SendWrapper::new(SignalData { name, parameters, tag }))
    }

    /// The name of the signal.
    ///
    /// * If the signal comes from a GTK builder, it is what's written in the "Handler" field of
    ///   the relevant signal in the "Signals" tabs of [the Cambalache
    ///   editor](https://gitlab.gnome.org/jpu/cambalache), or the `handler` attribute of the
    ///   `<signal>` element in the XML.
    /// * If the signal comes from [`woab::route_signal`](crate::route_signal), it is the third
    ///   argument (`actix_signal`) passed to that function.
    /// * If the signal comes from [`woab::route_action`](crate::route_action), it is the name of
    ///   the GIO action.
    pub fn name(&self) -> &str {
        &self.0.name
    }

    /// The tag of the signal.
    ///
    /// Tags are useful if the same GTK builder is instantiated many times, and all the
    /// instantiations are connected to the same actor. The actor can use the tag to identify the
    /// individual instantiation that sent the signal.
    pub fn tag(&self) -> &T {
        &self.0.tag
    }

    pub fn raw_param(&self, index: usize) -> Result<&glib::Value, crate::Error> {
        self.0.raw_param(index)
    }

    /// A parameter of the signal, converted to the appropriate type.
    pub fn param<'a, P>(&'a self, index: usize) -> Result<P, crate::Error>
    where
        P: glib::value::FromValue<'a>,
        P: glib::types::StaticType,
    {
        self.0.param(index)
    }

    /// The action parameter for stateless action signals, or the action state for stateful action signals.
    ///
    /// Convenience method - the parameter in actions signals needs to be converted to
    /// `glib::Variant` first before it can be converted to its concrete type. This method runs
    /// both steps.
    pub fn action_param<P: glib::variant::FromVariant>(&self) -> Result<P, crate::Error> {
        let value: glib::Variant = self.param(1)?;
        value.get().ok_or_else(|| crate::Error::IncorrectActionParameter {
            signal: self.name().to_owned(),
            expected_type: <P as glib::variant::StaticVariantType>::static_variant_type().into_owned(),
            actual_type: value.type_().to_owned(),
        })
    }

    /// An error indicating that an handler does not recognize the signal.
    ///
    /// To be used in the catch-all arm of a signal match:
    ///
    /// ```no_run
    /// # struct MyActor;
    /// # impl actix::Actor for MyActor { type Context = actix::Context<Self>; }
    ///
    /// impl actix::Handler<woab::Signal> for MyActor {
    ///     type Result = woab::SignalResult;
    ///
    ///     fn handle(&mut self, msg: woab::Signal, _ctx: &mut <Self as actix::Actor>::Context) -> Self::Result {
    ///         Ok(match msg.name() {
    ///             "signal1" => {
    ///                 // ...
    ///                 None
    ///             }
    ///             "signal2" => {
    ///                 // ...
    ///                 None
    ///             }
    ///             "signal3" => {
    ///                 // ...
    ///                 None
    ///             }
    ///             _ => msg.cant_handle()?,
    ///         })
    ///     }
    /// }
    pub fn cant_handle(&self) -> SignalResult {
        Err(crate::Error::NoSuchSignalError(self.0.name.as_str().to_owned()))
    }

    /// To be used with the [`woab::params!`](crate::params!) macro to extract all the signal's parameters.
    pub fn params<'a, R: SignalParamReceiver<'a>>(&'a self) -> Result<R, crate::Error> {
        R::fill_from_index(&*self.0, 0)
    }
}

#[doc(hidden)]
pub trait SignalParamReceiver<'a>: Sized {
    fn fill_from_index<D>(signal: &'a SignalData<D>, from_index: usize) -> Result<Self, crate::Error>;
}

impl SignalParamReceiver<'_> for () {
    fn fill_from_index<D>(signal: &SignalData<D>, from_index: usize) -> Result<Self, crate::Error> {
        if from_index < signal.parameters.len() {
            return Err(crate::Error::NotAllParametersExtracted {
                signal: signal.name.as_str().to_owned(),
                num_parameters: signal.parameters.len(),
                num_extracted: from_index,
            });
        }
        Ok(())
    }
}

impl<'a, T, R> SignalParamReceiver<'a> for (T, core::marker::PhantomData<T>, R)
where
    T: glib::value::FromValue<'a>,
    T: glib::types::StaticType,
    R: SignalParamReceiver<'a>,
{
    fn fill_from_index<D>(signal: &'a SignalData<D>, from_index: usize) -> Result<Self, crate::Error> {
        Ok((
            signal.param(from_index)?,
            core::marker::PhantomData,
            R::fill_from_index(signal, from_index + 1)?,
        ))
    }
}

impl<'a, R> SignalParamReceiver<'a> for ((&'a glib::Value,), R)
where
    R: SignalParamReceiver<'a>,
{
    fn fill_from_index<D>(signal: &'a SignalData<D>, from_index: usize) -> Result<Self, crate::Error> {
        Ok(((signal.raw_param(from_index)?,), R::fill_from_index(signal, from_index + 1)?))
    }
}
