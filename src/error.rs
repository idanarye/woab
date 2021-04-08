#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    XmlError(#[from] quick_xml::Error),

    #[error(transparent)]
    GtkBoolError(#[from] glib::BoolError),

    /// When extracting widgets using
    /// [`BuilderConnector::widgets`](crate::BuilderConnector::widgets) and one of the widgets is
    /// missing.
    #[error("Builder is missing widget with ID {0:?}")]
    WidgetMissingInBuilder(String),

    /// When extracting widgets using
    /// [`BuilderConnector::widgets`](crate::BuilderConnector::widgets) and one of the widgets has
    /// the wrong type.
    #[error("Expected widget {widget_id:?} to be {expected_type} - not {actual_type}")]
    IncorrectWidgetTypeInBuilder {
        widget_id: String,
        expected_type: glib::types::Type,
        actual_type: glib::types::Type,
    },

    /// When a signal handler does not recognize the name of the signal routed to it.
    #[error("Cannot handle the signal named {0:?}")]
    NoSuchSignalError(String),

    /// When a signal parameter has the wrong type.
    #[error("Expected the parameter at index {index} of {signal:?} to be {expected_type} - not {actual_type}")]
    IncorrectSignalParameterType {
        signal: String,
        index: usize,
        expected_type: glib::types::Type,
        actual_type: glib::types::Type,
    },

    /// When a signal has less parameters than what the handler expects.
    #[error("{signal:?} does not have a parameter at index {index} - it only has {num_parameters} parameters")]
    SignalParameterIndexOutOfBound {
        signal: String,
        index: usize,
        num_parameters: usize,
    },

    /// When an action signal's parameter is of the the wrong type.
    #[error("Expected the action parameter of {signal:?} to be {expected_type} - not {actual_type}")]
    IncorrectActionParameter {
        signal: String,
        expected_type: glib::VariantType,
        actual_type: glib::VariantType,
    },

    /// When a signal has more parameters than what the handler expects.
    #[error("{signal:?} has {num_parameters} parameters - only {num_extracted} extracted")]
    NotAllParametersExtracted {
        signal: String,
        num_parameters: usize,
        num_extracted: usize,
    },
}
