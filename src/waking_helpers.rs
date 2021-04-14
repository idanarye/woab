use core::future::Future;
use core::task::Poll;

use tokio::sync::{mpsc, oneshot};

/// Asynchronously wait for something to happen somewhere.
///
/// Accepts a closure that accepts a `Sender`. The closure must "plant" the sender somewhere -
/// usually inside a signal - and once the sender's `do_send` is called, `wake_from` will be woken
/// and return the value passed to the sender. If all the senders were dropped, `wake_from` will
/// return `None`.
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
///   be called multiple times. Note that unless explicitly removed, any signal handler registered
///   inside the closure would remain active afterwards - just like any other signal handler
///   registered in GTK.
/// * The closure runs inside the same runtime that `await`s the `wake_from` result. If that
///   runtime is the Actix runtime and the closure needs to do anything that must not be ran from
///   the Actix runtime, it'd need to either use [`spawn_outside`] inside the closure or use
///   [`outside`] before `wake_from` is called.
pub async fn wake_from<T>(setup_dlg: impl FnOnce(mpsc::Sender<T>)) -> Option<T> {
    let (tx, mut rx) = mpsc::channel(1);
    setup_dlg(tx);
    let result = rx.recv().await;
    rx.close();
    result
}

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

pub fn wait_for_signal<'a, O: glib::ObjectExt>(obj: &'a O, signal: &'a str) -> WaitForSignal<'a, O, ()> {
    WaitForSignal {
        obj,
        signal,
        signal_handler_id: None,
        receiver: None,
        inhibit: None,
    }
}

pub struct WaitForSignal<'a, O, T>
where
    O: glib::ObjectExt,
    T: WaitForSignalExtractParams,
{
    obj: &'a O,
    signal: &'a str,
    signal_handler_id: Option<glib::SignalHandlerId>,
    receiver: Option<core::pin::Pin<Box<oneshot::Receiver<T>>>>,
    inhibit: Option<bool>,
}

impl<O, T> WaitForSignal<'_, O, T>
where
    O: glib::ObjectExt,
    T: WaitForSignalExtractParams,
{
    pub fn inhibit(mut self, inhibit: bool) -> Self {
        self.inhibit = Some(inhibit);
        self
    }
}

impl<'a, O> WaitForSignal<'a, O, ()>
where
    O: glib::ObjectExt,
{
    pub fn params_as_signal(self) -> WaitForSignal<'a, O, crate::Signal> {
        let Self {
            obj,
            signal,
            signal_handler_id,
            receiver: _,
            inhibit,
        } = self;
        WaitForSignal {
            obj,
            signal,
            signal_handler_id,
            receiver: None,
            inhibit,
        }
    }
}

impl<O, T> Future for WaitForSignal<'_, O, T>
where
    O: glib::ObjectExt,
    T: WaitForSignalExtractParams,
    T: Unpin,
    T: 'static,
{
    type Output = Option<T>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        match &mut self.receiver {
            None => {
                assert!(self.signal_handler_id.is_none());
                let (tx, rx) = oneshot::channel();

                let mut receiver = Box::pin(rx);
                let initial_poll_result = receiver.as_mut().poll(cx);
                assert!(matches!(initial_poll_result, Poll::Pending));
                self.receiver = Some(receiver);

                let tx = std::rc::Rc::new(core::cell::Cell::new(Some(tx)));
                let inhibit = self.inhibit;
                let signal_handler_id = self.obj.connect_local(self.signal, false, move |params| {
                    if let Some(tx) = tx.take() {
                        // Swallow the result because the waiting future could be gone
                        let _ = tx.send(T::extract_params(params));
                    }
                    use glib::value::ToValue;
                    inhibit.map(|inhibit| inhibit.to_value())
                }).unwrap();
                self.signal_handler_id = Some(signal_handler_id);
                Poll::Pending
            }
            Some(receiver) => {
                if let Poll::Ready(result) = receiver.as_mut().poll(cx) {
                    self.obj.disconnect(self.signal_handler_id.take().expect("Signal handler should still exists"));
                    Poll::Ready(result.ok())
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

pub trait WaitForSignalExtractParams {
    fn extract_params(params: &[glib::Value]) -> Self;
}

impl WaitForSignalExtractParams for () {
    fn extract_params(_: &[glib::Value]) -> Self {
        ()
    }
}

impl WaitForSignalExtractParams for crate::Signal {
    fn extract_params(params: &[glib::Value]) -> Self {
        crate::Signal::new(std::rc::Rc::new("<woab::wait_for_signal>".to_owned()), params.to_owned(), ())
    }
}
