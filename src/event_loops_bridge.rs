use core::cell::RefCell;
use core::future::Future;

thread_local! {
    static ACTIX_SYSTEM_RUNNER: RefCell<actix::SystemRunner> = RefCell::new(actix::System::new());
}

/// Run a feature inside the Actix system GTK will be spinning.
///
/// Use this when starting actors from outside Tokio/Actix (e.g. - when creating the starteup
/// actors of the program) or when expecting message responses from inside GTK signal handlers that
/// are not routed to Actix.
pub fn block_on<F: Future>(fut: F) -> <F as Future>::Output {
    try_block_on(fut).expect("Already inside Actix context")
}

pub fn try_block_on<F: Future>(fut: F) -> Option<<F as Future>::Output> {
    ACTIX_SYSTEM_RUNNER.with(|system_runner| {
        if let Ok(system_runner) = system_runner.try_borrow_mut() {
            let result = Some(system_runner.block_on(fut));
            result
        } else {
            None
        }
    })
}

/// Start an Actix `System` that runs inside the GTK thread.
pub fn run_actix_inside_gtk_event_loop() -> std::io::Result<()> {
    glib::idle_add_local(move || {
        try_block_on(async {
            tokio::time::sleep(tokio::time::Duration::new(0, 0)).await;
        });
        glib::source::Continue(true)
    });
    Ok(())
}
