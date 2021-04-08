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

    #[error("Builder is missing widget with ID {0:?}")]
    WidgetMissingInBuilder(String),

    #[error("Expected widget {widget_id:?} to be {expected_type} - not {actual_type}")]
    IncorrectWidgetTypeInBuilder {
        widget_id: String,
        expected_type: glib::types::Type,
        actual_type: glib::types::Type,
    },

    #[error("{} does not have a signal named {0:?}")]
    NoSuchSignalError(&'static str, String),

    #[error("Expected the parameter at index {index} of {signal:?} to be {expected_type} - not {actual_type}")]
    IncorrectSignalParameter {
        signal: String,
        index: usize,
        expected_type: glib::types::Type,
        actual_type: glib::types::Type,
    },

    #[error("{signal:?} does not have a parameter at index {index} - it only has {num_parameters} parameters")]
    SignalParameterIndexOutOfBound {
        signal: String,
        index: usize,
        num_parameters: usize,
    },

    #[error("Expected the action parameter of {signal:?} to be {expected_type} - not {actual_type}")]
    IncorrectActionParameter {
        signal: String,
        expected_type: glib::VariantType,
        actual_type: glib::VariantType,
    },

    #[error("{signal:?} has {num_parameters} parameters - only {num_extracted} extracted")]
    NotAllParametersExtracted {
        signal: String,
        num_parameters: usize,
        num_extracted: usize,
    },
}
