use crate::BuilderSignal;

/// Represents an actor coupled with part of the GUI.
///
/// Typically created from a GTK builder using `ActorFactory`, `WoabActor` is an Actix `Actor` that
/// runs in the GTK thread and can interact directly with the widgets it represents. It can receive
/// signals specified in the builder through a `Stream` and handle them in its actor context.
pub trait WoabActor: actix::Actor<Context = actix::Context<Self>> + actix::StreamHandler<<Self as WoabActor>::Signal> {
    /// Handles for all the widgets this actor will need.
    type Widgets: for<'a> std::convert::TryFrom<&'a gtk::Builder, Error = crate::Error>;

    /// Signal emitted from the builder that created the widgets.
    type Signal: BuilderSignal;

    fn woab_create(builder: &gtk::Builder, make_self: impl FnOnce(&mut Self::Context, Self::Widgets) -> Self) -> Result<actix::Addr<Self>, crate::Error> {
        use std::convert::TryInto;
        let widgets: Self::Widgets = builder.try_into()?;
        Ok(Self::create(move |ctx| {
            Self::Signal::connect_builder_signals::<Self>(ctx, &builder);
            make_self(ctx, widgets)
        }))
    }
}
