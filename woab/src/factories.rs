use core::convert::TryInto;

use gtk::Builder;

use crate::BuilderSignal;

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

pub struct Factory<A, W, S> {
    xml: String,
    _phantom: std::marker::PhantomData<(A, W, S)>,
}

impl<A, W, S> From<String> for Factory<A, W, S> {
    fn from(xml: String) -> Self {
        Self {
            xml,
            _phantom: Default::default(),
        }
    }
}

impl<A, W, S> Factory<A, W, S> {
    pub fn build(&self) -> BuilderUtilizer<A, W, S> {
        Builder::from_string(&self.xml).into()
    }
}

pub struct BuilderUtilizer<A, W, S> {
    builder: gtk::Builder,
    _phantom: std::marker::PhantomData<(A, W, S)>,
}

impl<A, W, S> From<gtk::Builder> for BuilderUtilizer<A, W, S> {
    fn from(builder: gtk::Builder) -> Self {
        Self {
            builder,
            _phantom: Default::default(),
        }
    }
}

impl<A, W, S> BuilderUtilizer<A, W, S>
where
    for<'a> &'a gtk::Builder: TryInto<W>
{
    pub fn widgets(&self) -> Result<W, <&gtk::Builder as TryInto<W>>::Error>  {
        (&self.builder).try_into()
    }
}

impl<A, W, S> BuilderUtilizer<A, W, S>
where
    A: actix::Actor<Context = actix::Context<A>>,
    for<'a> &'a gtk::Builder: TryInto<W>,
    S: BuilderSignal,
    A: actix::StreamHandler<S>
{
    pub fn actor(&self, make_actor: impl FnOnce(&mut A::Context, W) -> A) -> Result<actix::Addr<A>, <&gtk::Builder as TryInto<W>>::Error> {
        let widgets: W = self.widgets()?;
        Ok(<A as actix::Actor>::create(move |ctx| {
            S::connect_builder_signals::<A>(ctx, &self.builder);
            make_actor(ctx, widgets)
        }))
    }
}
