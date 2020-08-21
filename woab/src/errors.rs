use std::fmt::{Display, Formatter};
use std::error::Error;

#[derive(Debug)]
pub struct WidgetMissingInBuilder(pub &'static str);

impl Display for WidgetMissingInBuilder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Builder is missing widget with ID {:?}", self.0)
    }
}

impl Error for WidgetMissingInBuilder {}
