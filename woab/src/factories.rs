use gtk::Builder;

use crate::WoabActor;

/// Holds instructions for generating a GTK builder.
///
/// ```no_run
/// # use gtk::prelude::*;
/// # use woab::BuilderFactory;
/// let builder_xml = r#"
///     <interface>
///       <requires lib="gtk+" version="3.22"/>
///       <object class="GtkButton" id="my_button">
///         ...
///       </object>
///     </interface>
/// "#;
/// let builder_factory: BuilderFactory = builder_xml.to_owned().into();
/// let builder = builder_factory.build();
/// let my_button: gtk::Button = builder.get_object("my_button").unwrap();
/// ```
pub struct BuilderFactory(String);

impl From<String> for BuilderFactory {
    fn from(xml: String) -> Self {
        Self(xml)
    }
}

impl BuilderFactory {
    pub fn build(&self) -> Builder {
        Builder::from_string(&self.0)
    }
}

/// Factory for creating `WoabActor` and connecting it to GTK builder signals.
pub struct ActorFactory<A: WoabActor> {
    builder_factory: BuilderFactory,
    _phantom: std::marker::PhantomData<A>,
}

impl<A: WoabActor> From<String> for ActorFactory<A> {
    fn from(xml: String) -> Self {
        Self {
            builder_factory: BuilderFactory(xml),
            _phantom: Default::default(),
        }
    }
}

impl<A: WoabActor + actix::StreamHandler<<A as WoabActor>::Signal>> ActorFactory<A> {
    pub fn create(&self, make_actor: impl FnOnce(&mut A::Context, A::Widgets) -> A) -> Result<actix::Addr<A>, crate::Error> {
        A::woab_create(&self.builder_factory.build(), make_actor)
    }
}
