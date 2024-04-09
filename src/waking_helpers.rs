use core::future::Future;

use tokio::sync::mpsc;

use crate::WakerPerished;

/// Asynchronously wait for something to happen somewhere.
///
/// Accepts a closure that accepts a `Sender`. The closure must "plant" the sender somewhere -
/// usually inside a signal - and once the sender's `do_send` is called, `wake_from` will be woken
/// and return the value passed to the sender.
///
/// Note that unless explicitly removed, any signal handler registered inside the closure would
/// remain active afterwards - just like any other signal handler registered in GTK. To wake from a
/// signal it is better to use [`woab::wake_from_signal`](wake_from_signal).
///
/// ```no_run
/// # use gtk4::prelude::*;
/// # async fn asyncfunc() {
/// let button1: gtk4::Button;
/// let button2: gtk4::Button;
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
pub async fn wake_from<T>(setup_dlg: impl FnOnce(mpsc::Sender<T>)) -> Result<T, WakerPerished> {
    let (tx, mut rx) = mpsc::channel(1);
    setup_dlg(tx);
    let result = rx.recv().await;
    rx.close();
    result.ok_or(WakerPerished)
}

/// Asynchronously wait for a signal to be called.
///
/// Accepts a GLib object and a closure that accepts a `Sender`. The closure must "plant" the
/// sender inside a signal handler and return the signal handler ID - and once the sender's
/// `do_send` is called, `wake_from_signal` will be woken, remove the signal from the object, and
/// return the value passed to the sender.
///
/// ```no_run
/// # use gtk4::prelude::*;
/// # async fn asyncfunc() {
/// let button: gtk4::Button;
/// # button = panic!();
/// let button_clicked = woab::wake_from_signal(&button, |tx| {
///     button.connect_clicked(move |_| {
///         let _ = tx.try_send(());
///     })
/// }).await.unwrap();
/// # }
/// ```
pub async fn wake_from_signal<T>(
    obj: &impl glib::object::ObjectExt,
    setup_dlg: impl FnOnce(mpsc::Sender<T>) -> glib::SignalHandlerId,
) -> Result<T, WakerPerished> {
    let (tx, mut rx) = mpsc::channel(1);
    let signal_handler_id = setup_dlg(tx);
    let result = rx.recv().await;
    rx.close();
    obj.disconnect(signal_handler_id);
    result.ok_or(WakerPerished)
}

/// Run a future outside the Actix system.
///
/// If operation that generate GTK signals are executed inside the Actix runtime, they'll be
/// queued to run later by the Actix event loop. This may be a problem if the signal is expecting
/// an propagation decision, because the handler will not be able to generate it before the signal
/// returns. This may also be a problem for signals that use a context parameter (like `draw`),
/// because by the time the handler is invoked the context will already expire. In such cases, the
/// code that triggers the signal must run outside.
///
/// **Note**: Most GTK signals that have these requirements are already queued by GTK. This
/// function was more useful before version 0.6 of WoAB , when signals were never queued by WoAB
/// and all signals GTK handles immediately were required to run outside. Now it is not needed, but
/// not deprecated either because the possibility of misbehaving custom signals cannot be ruled
/// out.
///
/// Similar to [`outside`], but returns immediately without waiting for the future to finish.
///
/// ```no_run
/// # use gtk4::prelude::*;
/// #
/// # struct WindowActor {
/// #     widgets: WindowWidgets,
/// # }
/// #
/// # struct WindowWidgets {
/// #     some_container: gtk4::ListBox,
/// #     some_widget: gtk4::ListBoxRow,
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
/// If operation that generate GTK signals are executed inside the Actix runtime, they'll be
/// queued to run later by the Actix event loop. This may be a problem if the signal is expecting
/// an propagation decision, because the handler will not be able to generate it before the signal
/// returns. This may also be a problem for signals that use a context parameter (like `draw`),
/// because by the time the handler is invoked the context will already expire. In such cases, the
/// code that triggers the signal must run outside.
///
/// **Note**: Most GTK signals that have these requirements are already queued by GTK. This
/// function was more useful before version 0.6 of WoAB , when signals were never queued by WoAB
/// and all signals GTK handles immediately were required to run outside. Now it is not needed, but
/// not deprecated either because the possibility of misbehaving custom signals cannot be ruled
/// out.
///
/// Similar to [`spawn_outside`], but waits for the future to finish and returns its result.
///
/// ```no_run
/// # use actix::prelude::*;
/// # use gtk4::prelude::*;
/// #
/// # struct WindowActor {
/// #     widgets: WindowWidgets,
/// # }
/// #
/// # struct WindowWidgets {
/// #     some_container: gtk4::ListBox,
/// #     some_widget: gtk4::ListBoxRow,
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
pub async fn outside<T: 'static>(fut: impl Future<Output = T> + 'static) -> Result<T, WakerPerished> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    glib::MainContext::ref_thread_default().spawn_local(async move {
        let result = fut.await;
        tx.send(result).map_err(|_| "Unable to send future result").unwrap();
    });
    rx.await.map_err(|_| WakerPerished)
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
///     &gtk4::MessageDialog::new::<gtk4::ApplicationWindow>(
///         None,
///         gtk4::DialogFlags::all(),
///         gtk4::MessageType::Question,
///         gtk4::ButtonsType::YesNo,
///         "What would it be?",
///     ),
///     true,
/// )
/// .await;
/// println!("User says {}", dialog_response);
/// # }
/// ```
pub async fn run_dialog(
    dialog: &(impl Clone + gtk4::prelude::DialogExt + gtk4::prelude::GtkWindowExt + gtk4::prelude::WidgetExt + 'static),
    close_after: bool,
) -> gtk4::ResponseType {
    dialog.set_modal(true);
    spawn_outside({
        let dialog = dialog.clone();
        async move {
            dialog.show();
        }
    });
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
