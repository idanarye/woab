use core::cell::RefCell;
use core::future::Future;

struct WoabRuntime {
    actix_system_runner: actix::SystemRunner,
    runtime_cranker_source_id: glib::SourceId,
}

thread_local! {
    static WOAB_RUNTIME: RefCell<Option<WoabRuntime>> = const { RefCell::new(None) };
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
            glib::ControlFlow::Continue
        });
        *woab_runtime = Some(WoabRuntime {
            actix_system_runner: actix::System::new(),
            runtime_cranker_source_id,
        });
    });
}

#[derive(thiserror::Error, Debug)]
pub enum RuntimeStopError {
    #[error("Cannot stop the WoAB runtime because it was not started")]
    RuntimeNotStarted,
    #[error("Cannot stop the WoAB runtime because it is currently in use. Try stopping it with `actix::System::current().stop();` instead")]
    RuntimeInUse,
}

/// Shut down the Actix `System` that runs inside the GTK thread.
///
/// This will close the Actix runtime and stop GTK from idly cranking it to check for new events
/// from external sources (e.g. network) but will not disconnect the routed GTK signals. If the GTK
/// loop is still running and these signals are fired, WoAB will panic.
pub fn close_actix_runtime() -> Result<Result<(), std::io::Error>, RuntimeStopError> {
    let woab_runtime = WOAB_RUNTIME.with(|woab_runtime| {
        woab_runtime
            .try_borrow_mut()
            .map_err(|_| RuntimeStopError::RuntimeInUse)?
            .take()
            .ok_or(RuntimeStopError::RuntimeNotStarted)
    })?;
    woab_runtime.actix_system_runner.block_on(async {
        actix::System::current().stop();
    });
    woab_runtime.runtime_cranker_source_id.remove();
    Ok(woab_runtime.actix_system_runner.run())
}

/// Determine if the Actix `System` that runs inside the GTK thread is running.
///
/// Returns `true` if and only if called after
/// [`woab::run_actix_inside_gtk_event_loop`](run_actix_inside_gtk_event_loop) but before any
/// successful call to [`woab::close_actix_runtime`](close_actix_runtime).
pub fn is_runtime_running() -> bool {
    WOAB_RUNTIME.with(|woab_runtime| {
        if let Ok(woab_runtime) = woab_runtime.try_borrow() {
            woab_runtime.is_some()
        } else {
            true
        }
    })
}
