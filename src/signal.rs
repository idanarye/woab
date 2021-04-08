use std::rc::Rc;

use send_wrapper::SendWrapper;

pub struct Signal<T = ()>(SendWrapper<SignalData<T>>);
pub type SignalResult = Result<Option<gtk::Inhibit>, crate::Error>;

impl<T> actix::Message for Signal<T> {
    type Result = SignalResult;
}

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

#[doc(hidden)]
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

    fn param<'a, P: glib::value::FromValueOptional<'a>>(&'a self, index: usize) -> Result<P, crate::Error> {
        let value = self.raw_param(index)?;
        if let Ok(Some(value)) = value.get() {
            Ok(value)
        } else {
            Err(crate::Error::IncorrectSignalParameter {
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

    pub fn name(&self) -> &str {
        &*self.0.name
    }

    pub fn tag(&self) -> &T {
        &self.0.tag
    }

    pub fn param<'a, P: glib::value::FromValueOptional<'a>>(&'a self, index: usize) -> Result<P, crate::Error> {
        self.0.param(index)
    }

    pub fn action_param<P: glib::variant::FromVariant>(&self) -> Result<P, crate::Error> {
        let value: glib::Variant = self.param(1)?;
        value.get().ok_or_else(|| crate::Error::IncorrectActionParameter {
            signal: self.name().to_owned(),
            expected_type: <P as glib::variant::StaticVariantType>::static_variant_type().into_owned(),
            actual_type: value.type_().to_owned(),
        })
    }

    pub fn cant_handle(&self) -> SignalResult {
        Err(crate::Error::NoSuchSignalError("Actor", (*self.0.name).to_owned()))
    }

    pub fn params<'a, R: SignalParamReceiver<'a>>(&'a self) -> Result<R, crate::Error> {
        R::fill_from_index(&*self.0, 0)
    }
}

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
    T: glib::value::FromValueOptional<'a>,
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
