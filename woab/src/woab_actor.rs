use crate::BuilderSignal;

pub trait WoabActor: actix::Actor<Context = actix::Context<Self>> {
    type Widgets: for<'a> std::convert::TryFrom<&'a gtk::Builder, Error = crate::Error>;
    type Signal: BuilderSignal;

    fn woab_create<>(builder: &gtk::Builder, make_self: impl FnOnce(&mut Self::Context, Self::Widgets) -> Self) -> Result<actix::Addr<Self>, crate::Error>
        where Self: actix::StreamHandler<<Self as WoabActor>::Signal>
    {
        use std::convert::TryInto;
        let widgets: Self::Widgets = builder.try_into()?;
        Ok(Self::create(move |ctx| {
            Self::Signal::connect_builder_signals::<Self>(ctx, &builder);
            make_self(ctx, widgets)
        }))
    }
}
