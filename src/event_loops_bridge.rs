use core::cell::RefCell;
use core::future::Future;

struct WoabRuntime {
    actix_system_runner: actix::SystemRunner,
    runtime_cranker_source_id: glib::SourceId,
}

thread_local! {
    static WOAB_RUNTIME: RefCell<Option<WoabRuntime>> = RefCell::new(None);
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
    WOAB_RUNTIME.with(|woab_runtime| {
        if let Ok(woab_runtime) = woab_runtime.try_borrow_mut() {
            let woab_runtime = woab_runtime
                .as_ref()
                .expect("`try_block_on` called without `run_actix_inside_gtk_event_loop`");
            let result = woab_runtime.actix_system_runner.block_on(fut);
            Ok(result)
        } else {
            Err(fut)
        }
    })
}

/// Start an Actix `System` that runs inside the GTK thread.
pub fn run_actix_inside_gtk_event_loop() {
    WOAB_RUNTIME.with(|woab_runtime| {
        let mut woab_runtime = woab_runtime.borrow_mut();
        if woab_runtime.is_some() {
            panic!("WoAB is already running Actix inside the GTK event loop");
        }
        let runtime_cranker_source_id = glib::idle_add(|| {
            try_block_on(async {
                actix::clock::sleep(core::time::Duration::new(0, 10_000_000)).await;
            })
            .map_err(|_| "`idle_add` function called inside Actix context")
            .unwrap();
            glib::source::Continue(true)
        });
        *woab_runtime = Some(WoabRuntime {
            actix_system_runner: actix::System::new(),
            runtime_cranker_source_id,
        });
    });
}

/// Shut down the Actix `System` that runs inside the GTK thread.
///
/// This will close the Actix runtime and stop GTK from idly cranking it to check for new events
/// from external sources (e.g. network) but will not disconnect the routed GTK signals. If the GTK
/// loop is still running and these signals are fired, WoAB will panic.
pub fn close_actix_runtime() -> Result<(), std::io::Error> {
    WOAB_RUNTIME.with(|woab_runtime| {
        if let Ok(mut woab_runtime) = woab_runtime.try_borrow_mut() {
            let woab_runtime = woab_runtime
                .take()
                .expect("`close_actix_runtime` called before `run_actix_inside_gtk_event_loop`");
            woab_runtime.actix_system_runner.block_on(async {
                actix::System::current().stop();
            });
            glib::source::source_remove(woab_runtime.runtime_cranker_source_id);
            woab_runtime.actix_system_runner.run()
        } else {
            panic!("`close_actix_runtime` function called inside Actix context");
        }
    })
}
