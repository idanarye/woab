use core::convert::TryInto;

use gtk::Builder;
use tokio::sync::mpsc;
use tokio::stream::StreamExt;

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
///
/// Refer to [`#[derive(woab::Factories)]`](derive.Factories.html) for how to create instances of
/// this struct.
pub struct BuilderFactory(String);

impl From<String> for BuilderFactory {
    fn from(xml: String) -> Self {
        Self(xml)
    }
}

impl BuilderFactory {
    /// Create a `gtk::Builder` from the instructions inside this factory.
    ///
    /// Note that "creating a builder" means that the GTK widgets are created (but not yet shown)
    pub fn build(&self) -> Builder {
        Builder::from_string(&self.0)
    }
}

/// Holds instructions for generating GTK widgets and connecing them to Actix actors.
///
/// 1. The first generic parameter, `A`, is the actor type.
/// 2. The second generic parameter, `W`, is the widgets type. Typically created with
///    [`#[derive(woab::WidgetsFromBuilder)]`](derive.WidgetsFromBuilder.html) on a struct that
///    specifies the widgets of the Glade XML file that the code needs to access.
/// 3. The third generic parameter, `S`, is the signal type. Typically created with
///    [`#[derive(woab::BuilderSignal)]`](derive.BuilderSignal.html) on an enum that lists the
///    signals from the Glade XML file that the code wants to handle.
///
/// `A` can be `()` if the widgets are to be handled by an existing actor - usually the one that
/// handles their parent widget. `S` can also be `()` if it is desired to just generate widgets
/// without connecting a signal.
///
/// Refer to [`#[derive(woab::Factories)]`](derive.Factories.html) for how to create instances of
/// this struct.
///
/// ```no_run
/// # use gtk::prelude::*;
/// #[derive(woab::Factories)]
/// struct Factories {
///     window: woab::Factory<WindowActor, WindowWidgets, WindowSignal>,
///     row: woab::Factory<(), RowWidgets, RowSignal>,
/// }
///
/// struct WindowActor {
///     widgets: WindowWidgets,
/// }
/// # impl actix::Actor for WindowActor {
/// #     type Context = actix::Context<Self>;
/// # }
/// # #[derive(woab::BuilderSignal)]
/// # enum WindowSignal {}
///
/// impl actix::StreamHandler<WindowSignal> for WindowActor {
///     fn handle(&mut self, signal: WindowSignal, _ctx: &mut <Self as actix::Actor>::Context) {
///         match signal {
///             // Handle the signals of the main window
///         }
///     }
/// }
///
/// #[derive(woab::WidgetsFromBuilder)]
/// struct WindowWidgets {
///     window: gtk::ApplicationWindow,
///     list_box: gtk::ListBox,
/// }
///
/// #[derive(woab::WidgetsFromBuilder)]
/// struct RowWidgets {
///     row: gtk::ListBoxRow,
///     label: gtk::Label,
/// }
/// # #[derive(woab::BuilderSignal)]
/// # enum RowSignal {}
///
/// impl actix::StreamHandler<(usize, RowSignal)> for WindowActor {
///     fn handle(&mut self, (row_number, signal): (usize, RowSignal), _ctx: &mut <Self as actix::Actor>::Context) {
///         match signal {
///             // Handle the signals of row #row_number
///         }
///     }
///
///     // ******************************************************
///     // * VERY IMPORTANT! Otherwise once one row is deleted, *
///     // * its signal stream will be closed and the default   *
///     // * implementation will close the WindowActor.         *
///     // ******************************************************
///     fn finished(&mut self, _ctx: &mut Self::Context) {}
/// }
///
/// fn create_window_with_rows(factory: &Factories) {
///     factory.window.build().actor(|ctx, widgets| {
///         for row_number in 0..10 {
///             let row_widgets = factory.row.build()
///                 .connect_tagged_builder_signals(ctx, row_number)
///                 .widgets().unwrap();
///             row_widgets.label.set_text(&format!("Roe number {}", row_number));
///             widgets.list_box.add(&row_widgets.row);
///         }
///         WindowActor { widgets }
///     }).unwrap();
/// }
/// ```
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
    /// Create the `gtk::Builder` (inside a [`woab::BuilderUtilizer`](struct.BuilderUtilizer.html))
    ///
    /// Note that this causes the GTK widgets to be created (but not to be shown or be connected to
    /// anything)
    pub fn build(&self) -> BuilderUtilizer<A, W, S> {
        Builder::from_string(&self.xml).into()
    }
}

/// Context for utilizing a `gtk::Builder` and connecting it to he Actix world.
///
/// See [`woab::Factory`](struct.Factory.html) for usage example.
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
    /// Create a widgets struct (as defined by the `W` generic parameter of
    /// [`woab::Factory`](struct.Factory.html)) and map the builder's widgets to its fields.
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
    /// Create an Actix actor and connect it to the builder's widgets and signals.
    ///
    /// `make_actor` is a function that receives the actor context and the widgets, and is
    /// responsible for constructing the actor struct with the widgets inside it. It can also be
    /// used for configuring and or showing the widgets GTK-wise (though this can also be handled
    /// by the actor afterwards)
    pub fn actor(&self, make_actor: impl FnOnce(&mut A::Context, W) -> A) -> Result<actix::Addr<A>, <&gtk::Builder as TryInto<W>>::Error> {
        let widgets: W = self.widgets()?;
        Ok(<A as actix::Actor>::create(move |ctx| {
            S::connect_builder_signals::<A>(ctx, &self.builder);
            make_actor(ctx, widgets)
        }))
    }
}

impl<A, W, S> BuilderUtilizer<A, W, S>
where
    S: BuilderSignal,
{
    /// Create a stream (based on Tokio's MSPC) of signals that arrive from the builder.
    ///
    /// * The signals are all represented by the third generic parameter (`S`) of
    ///   [`woab::Factory`](struct.Factory.html) - if the builder sends signals not covered by
    ///   `S`'s variants they'll be ignored.
    /// * If the builder defines no signals, or if none of the signals it defines are covered by
    ///   `S`, this method will return `None`. This is important because otherwise it would have
    ///   returned a stream stream will be closed automatically for having no transmitters, which -
    ///   by default - will make Actix close the actor.
    pub fn stream_builder_signals(&self) -> Option<mpsc::Receiver<S>> {
        S::stream_builder_signals(&self.builder)
    }

    /// Stream the signals generated by the builder to an actor represented by `ctx`, together with
    /// a tag.
    ///
    /// When using the same actor to handle multiple copies of the same set of widgets (e.g.
    /// multiple `GtkListBoxRow`s) the tag can be used to determine which copy generated the
    /// signal.
    ///
    /// **If multiple tagged signals are streamed to the same actor - which is the typical use case
    /// for tagged signals - `StreamHandler::finished` should be overridden to avoid stopping the
    /// actor when one instance of the widgets is removed!!!**
    pub fn connect_tagged_builder_signals<T, C, AA>(&self, ctx: &mut C, tag: T) -> &Self
    where
        T: Clone + 'static,
        AA: actix::Actor<Context = C>,
        C: actix::AsyncContext<AA>,
        AA: actix::StreamHandler<(T, S)>
    {
        if let Some(rx) = self.stream_builder_signals() {
            let stream = rx.map(move |s| (tag.clone(), s));
            ctx.add_stream(stream);
        }
        self
    }
}
