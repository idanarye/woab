use gtk::Builder;

use crate::WoabActor;

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
    pub fn create(&self, make_actor: impl FnOnce(&mut A::Context, A::Widgets) -> A) -> Result<actix::Addr<A>, crate::errors::WidgetMissingInBuilder> {
        A::woab_create(&self.builder_factory.build(), make_actor)
    }
}
