#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    XmlError(#[from] quick_xml::Error),

    #[error(transparent)]
    WidgetMissingInBuilder(#[from] WidgetMissingInBuilder),
}

#[derive(thiserror::Error, Debug)]
#[error("Builder is missing widget with ID {0:?}")]
pub struct WidgetMissingInBuilder(pub &'static str);
