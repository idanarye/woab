/// A message for removing actors along with their GUI
///
/// Refer to [`#[derive(woab::Removable)]`](derive.Removable.html) docs for usage instructions.
/// ```no_run
/// #[derive(woab::Removable)]
/// #[removable(self.widgets.row in gtk4::ListBox)]
/// struct MyActor {
///     widgets: MyWidgets,
/// }
///
/// # impl actix::Actor for MyActor { type Context = actix::Context<Self>; }
///
/// #[derive(woab::WidgetsFromBuilder)]
/// struct MyWidgets {
///     row: gtk4::ListBoxRow,
/// }
///
/// let my_actor: actix::Addr<MyActor>;
/// # let mut my_actor: actix::Addr<MyActor> = panic!();
/// my_actor.do_send(woab::Remove);
/// ```
pub struct Remove;

impl actix::Message for Remove {
    type Result = ();
}
