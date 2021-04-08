use core::cell::RefCell;
use core::future::Future;

type ScheudleOutsideDlgs = std::collections::VecDeque<Box<dyn FnOnce()>>;

thread_local! {
    static ACTIX_SYSTEM_RUNNER: RefCell<actix::SystemRunner> = RefCell::new(actix::System::new());
    static SCHEDULED_OUTSIDE: core::cell::RefCell<Option<ScheudleOutsideDlgs>> = Default::default();
}

/// Run a feature inside the Actix system GTK will be spinning.
///
/// Use this when starting actors from outside Tokio/Actix (e.g. - when creating the starteup
/// actors of the program) or when expecting message responses from inside GTK signal handlers that
/// are not routed to Actix.
pub fn block_on<F: Future>(fut: F) -> <F as Future>::Output {
    try_block_on(fut).map_err(|_| "Already inside Actix context").unwrap()
}

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
pub fn run_actix_inside_gtk_event_loop() -> std::io::Result<()> {
    SCHEDULED_OUTSIDE.with(|scheduled_outside| {
        scheduled_outside.borrow_mut().get_or_insert_with(&Default::default);
    });
    glib::idle_add_local(move || {
        try_block_on(async {
            actix::clock::sleep(core::time::Duration::new(0, 0)).await;
        })
        .map_err(|_| "`idle_add_local` called inside Actix context")
        .unwrap();
        while let Some(dlg) = pop_scheduled_outside_dlg() {
            dlg();
        }
        glib::source::Continue(true)
    });
    Ok(())
}

pub fn schedule_outside(dlg: impl FnOnce() + 'static) {
    SCHEDULED_OUTSIDE.with(|queue| {
        let mut queue = queue.borrow_mut();
        let queue = queue
            .as_mut()
            .ok_or("`scheduled_outside` can only be called from the thread that runs the GTK loop")
            .unwrap();
        queue.push_back(Box::new(dlg));
    });
}

fn pop_scheduled_outside_dlg() -> Option<Box<dyn FnOnce()>> {
    SCHEDULED_OUTSIDE.with(|queue| {
        let mut queue = queue.borrow_mut();
        let queue = queue
            .as_mut()
            .ok_or("The scheduled-outside queue can only be accessed from the thread that runs the GTK loop")
            .unwrap();
        queue.pop_front()
    })
}
