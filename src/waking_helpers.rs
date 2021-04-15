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
