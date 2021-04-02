use std::rc::Rc;

use send_wrapper::SendWrapper;

pub struct Signal<T = ()>(SendWrapper<SignalData<T>>);
pub type SignalResult = Result<Option<gtk::Inhibit>, crate::Error>;

impl<T> actix::Message for Signal<T> {
    type Result = SignalResult;
}

struct SignalData<T> {
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
        // TODO: handle errors
        let value = &self.0.parameters[index];
        let value = value.get().unwrap().unwrap();
        Ok(value)
    }

    pub fn cant_handle(&self) -> SignalResult {
        Err(crate::Error::NoSuchSignalError("Actor", (*self.0.name).to_owned()))
    }
}
