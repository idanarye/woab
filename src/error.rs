pub type Result<T> = core::result::Result<T, Error>;

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

    #[error("GTK exited with code {0:?}")]
    GtkBadExitCode(glib::ExitCode),

    /// When extracting widgets using
    /// [`BuilderWidgets::widgets`](crate::BuilderWidgets::widgets) and one of the widgets is
    /// missing.
    #[error("Builder is missing widget with ID {0:?}")]
    WidgetMissingInBuilder(String),

    /// When extracting widgets using
    /// [`BuilderWidgets::widgets`](crate::BuilderWidgets::widgets) and one of the widgets has the
    /// wrong type.
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

    /// When an event signal's parameter is of the the wrong event type.
    #[error("Expected the event parameter of {signal:?} to be {expected_type} - not {actual_type:?}")]
    IncorrectEventParameter {
        signal: String,
        expected_type: &'static str,
        actual_type: gdk4::EventType,
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

    #[error(transparent)]
    WakerPerished(#[from] WakerPerished),

    #[error(transparent)]
    RuntimeStopError(#[from] crate::RuntimeStopError),
}

/// When a future cannot be woken.
///
/// When using a function like [`woab::wake_from_signal`](crate::wake_from_signal) there is no
/// compile-time guarantee the waker will be used. For example - the widget that holds the signal
/// handler that can be deleted before the signal is triggered. In that scenario, the future will
/// be return with this error instead.
#[derive(thiserror::Error, Debug)]
#[error("The object that was supposed to wake this future was dropped")]
pub struct WakerPerished;
