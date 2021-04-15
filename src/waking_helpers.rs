use core::future::Future;

use tokio::sync::mpsc;

/// Asynchronously wait for something to happen somewhere.
///
/// Accepts a closure that accepts a `Sender`. The closure must "plant" the sender somewhere -
/// usually inside a signal - and once the sender's `do_send` is called, `wake_from` will be woken
/// and return the value passed to the sender. If all the senders were dropped, `wake_from` will
/// return `None`.
///
/// Note that unless explicitly removed, any signal handler registered inside the closure would
/// remain active afterwards - just like any other signal handler registered in GTK. To wake from a
/// signal it is better to use [`woab::wake_from_signal`](wake_from_signal).
///
/// ```no_run
/// # use gtk::prelude::*;
/// # async fn asyncfunc() {
/// let button1: gtk::Button;
/// let button2: gtk::Button;
/// # button1 = panic!();
/// # button2 = panic!();
/// let button_clicked = woab::wake_from(|tx| {
///     button1.connect_clicked({
///         let tx = tx.clone();
///         move |_| {
///             let _ = tx.try_send(1);
///         }
///     });
///     button2.connect_clicked(move |_| {
///         let _ = tx.try_send(2);
///     });
/// }).await.unwrap();
/// println!("Clicked button {}", button_clicked);
/// # }
/// ```
///
/// # Pitfalls
///
/// * The sender's `do_send` can only be called once, and will fail if called multiple times.
///   Because GTK signals can be called multiple times, this cannot be guaranteed by the type
///   system and the result of `do_send` needs to be swallowed if there is a possibility for it to
///   be called multiple times.
/// * The closure runs inside the same runtime that `await`s the `wake_from` result. If that
///   runtime is the Actix runtime and the closure needs to do anything that must not be ran from
///   the Actix runtime, it'd need to either use [`spawn_outside`](crate::spawn_outside) inside the
///   closure or use [`outside`](crate::outside) before `wake_from` is called.
pub async fn wake_from<T>(setup_dlg: impl FnOnce(mpsc::Sender<T>)) -> Option<T> {
    let (tx, mut rx) = mpsc::channel(1);
    setup_dlg(tx);
    let result = rx.recv().await;
    rx.close();
    result
}

/// Asynchronously wait for a signal to be called.
///
/// Accepts a GLib object and a closure that accepts a `Sender`. The closure must "plant" the
/// sender inside a signal handler and return the signal handler ID - and once the sender's
/// `do_send` is called, `wake_from_signal` will be woken, remove the signal from the object, and
/// return the value passed to the sender. If all the senders were dropped (e.g. - the object was
/// deleted), `wake_from` will return `None`.
///
/// ```no_run
/// # use gtk::prelude::*;
/// # async fn asyncfunc() {
/// let button: gtk::Button;
/// # button = panic!();
/// let button_clicked = woab::wake_from_signal(&button, |tx| {
///     button.connect_clicked(move |_| {
///         let _ = tx.try_send(());
///     })
/// }).await.unwrap();
/// # }
/// ```
pub async fn wake_from_signal<T>(
    obj: &impl glib::ObjectExt,
    setup_dlg: impl FnOnce(mpsc::Sender<T>) -> glib::SignalHandlerId,
) -> Option<T> {
    let (tx, mut rx) = mpsc::channel(1);
    let signal_handler_id = setup_dlg(tx);
    let result = rx.recv().await;
    rx.close();
    obj.disconnect(signal_handler_id);
    result
}

/// Run a future outside the Actix system.
///
/// Useful for GTK operations that generate synchronous signals that are handled by actors. If
/// these operations are executed inside the Actix runtime, they'll try to rerun the Actix runtime
/// again to handle the signal - and fail. Therefore - they must be handled outside.
///
/// Similar to [`outside`], but returns immediately without waiting for the future to finish.
///
/// ```no_run
/// # use gtk::prelude::*;
/// #
/// # struct WindowActor {
/// #     widgets: WindowWidgets,
/// # }
/// #
/// # struct WindowWidgets {
/// #     some_container: gtk::ListBox,
/// #     some_widget: gtk::ListBoxRow,
/// # }
/// #
/// # impl actix::Actor for WindowActor {
/// #     type Context = actix::Context<Self>;
/// # }
/// impl actix::Handler<woab::Signal> for WindowActor {
///     type Result = woab::SignalResult;
///
///     fn handle(&mut self, msg: woab::Signal, _ctx: &mut Self::Context) -> Self::Result {
///         Ok(match msg.name() {
///             "remove_button_clicked" => {
///                 // BAD! This will panic at runtime:
///                 self.widgets.some_container.remove(&self.widgets.some_widget);
///
///                 // Use this instead:
///                 let container = self.widgets.some_container.clone();
///                 let widget = self.widgets.some_widget.clone();
///                 woab::spawn_outside(async move {
///                     container.remove(&widget)
///                 });
///
///                 None
///             }
///             _ => msg.cant_handle()?
///         })
///     }
/// }
/// ```
pub fn spawn_outside(fut: impl Future<Output = ()> + 'static) {
    glib::MainContext::ref_thread_default().spawn_local(fut);
}

/// Run a future outside the Actix runtime.
///
/// Useful for GTK operations that generate synchronous signals that are handled by actors. If
/// these operations are executed inside the Actix runtime, they'll try to rerun the Actix runtime
/// again to handle the signal - and fail. Therefore - they must be handled outside.
///
/// Similar to [`spawn_outside`], but waits for the future to finish and returns its result.
///
/// ```no_run
/// # use actix::prelude::*;
/// # use gtk::prelude::*;
/// #
/// # struct WindowActor {
/// #     widgets: WindowWidgets,
/// # }
/// #
/// # struct WindowWidgets {
/// #     some_container: gtk::ListBox,
/// #     some_widget: gtk::ListBoxRow,
/// # }
/// #
/// # impl actix::Actor for WindowActor {
/// #     type Context = actix::Context<Self>;
/// # }
/// impl actix::Handler<woab::Signal> for WindowActor {
///     type Result = woab::SignalResult;
///
///     fn handle(&mut self, msg: woab::Signal, ctx: &mut Self::Context) -> Self::Result {
///         Ok(match msg.name() {
///             "remove_button_clicked" => {
///                 let container = self.widgets.some_container.clone();
///                 let widget = self.widgets.some_widget.clone();
///
///                 ctx.spawn(async move {
///                     // BAD! This will panic at runtime:
///                     container.remove(&widget);
///
///                     // Use this instead:
///                     woab::outside(async move {
///                         container.remove(&widget)
///                     }).await;
///                 }.into_actor(self));
///
///                 None
///             }
///             _ => msg.cant_handle()?
///         })
///     }
/// }
/// ```
pub async fn outside<T: 'static>(fut: impl Future<Output = T> + 'static) -> Option<T> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    glib::MainContext::ref_thread_default().spawn_local(async move {
        let result = fut.await;
        tx.send(result).map_err(|_| "Unable to send future result").unwrap();
    });
    rx.await.ok()
}

/// Run a GTK dialog and get its response.
///
/// Use this instead of gtk-rs' `dialog.run()`, because `dialog.run()` runs a nested GTK loop which
/// does not play nice with WoAB and because `run_dialog` is `async` and can let Actix continue
/// running in the background.
///
/// GTK does not automatically close dialogs when the user responds to them, but `run_dialog` can
/// do it if `true` is passed as the second argument.
///
/// ```no_run
/// # async fn asyncfunc() {
/// let dialog_response = woab::run_dialog(
///     &gtk::MessageDialog::new::<gtk::ApplicationWindow>(
///         None,
///         gtk::DialogFlags::all(),
///         gtk::MessageType::Question,
///         gtk::ButtonsType::YesNo,
///         "What would it be?",
///     ),
///     true,
/// )
/// .await;
/// println!("User says {}", dialog_response);
/// # }
/// ```
pub async fn run_dialog(
    dialog: &(impl gtk::DialogExt + gtk::GtkWindowExt + gtk::WidgetExt),
    close_after: bool,
) -> gtk::ResponseType {
    dialog.set_modal(true);
    dialog.show();
    wake_from(|tx| {
        dialog.connect_response(move |dialog, response| {
            let _ = tx.try_send(response);
            if close_after {
                dialog.close();
            }
        });
    })
    .await
    .unwrap()
}
