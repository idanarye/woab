use core::cell::RefCell;
use core::future::Future;

use tokio::sync::mpsc;

thread_local! {
    static ACTIX_SYSTEM_RUNNER: RefCell<actix::SystemRunner> = RefCell::new(actix::System::new());
}

/// Run a feature inside the Actix system GTK will be spinning.
///
/// Use this when starting actors from outside Tokio/Actix (e.g. - when creating the starteup
/// actors of the program) or when expecting message responses from inside GTK signal handlers that
/// are not routed to Actix.
///
/// Will panic if called from inside the Tokio runtime Actix is using.
pub fn block_on<F: Future>(fut: F) -> <F as Future>::Output {
    try_block_on(fut).map_err(|_| "Already inside Actix context").unwrap()
}

/// Run a feature inside the Actix system GTK will be spinning.
///
/// Similar to [`block_on`], but will not panic if called from inside the Tokio runtime Actix is
/// using, and instead return the future as the error value so that it could be executed in some
/// other fashion.
pub fn try_block_on<F: Future>(fut: F) -> Result<<F as Future>::Output, F> {
    ACTIX_SYSTEM_RUNNER.with(|system_runner| {
        if let Ok(system_runner) = system_runner.try_borrow_mut() {
            let result = system_runner.block_on(fut);
            Ok(result)
        } else {
            Err(fut)
        }
    })
}

/// Start an Actix `System` that runs inside the GTK thread.
pub fn run_actix_inside_gtk_event_loop() -> std::io::Result<glib::SourceId> {
    let source_id = glib::idle_add_local(|| {
        try_block_on(async {
            actix::clock::sleep(core::time::Duration::new(0, 0)).await;
        })
        .map_err(|_| "`idle_add_local` called inside Actix context")
        .unwrap();
        glib::source::Continue(true)
    });
    Ok(source_id)
}

/// Run a future outside the Actix system.
///
/// Useful for GTK operations that generate synchronous signals that are handled by actors. If
/// these operations are executed inside the Actix runtime, they'll try to rerun the Actix runtime
/// again to handle the signal - and fail. Therefore - they must be handled outside.
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

pub async fn wake_from<T>(setup_dlg: impl FnOnce(mpsc::Sender<T>)) -> Option<T> {
    let (tx, mut rx) = mpsc::channel(1);
    setup_dlg(tx);
    let result = rx.recv().await;
    rx.close();
    result
}

pub async fn outside<T: 'static>(fut: impl Future<Output = T> + 'static) -> Option<T> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    glib::MainContext::ref_thread_default().spawn_local(async move {
        let result = fut.await;
        tx.send(result).map_err(|_| "Unable to send future result").unwrap();
    });
    rx.await.ok()
}

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
