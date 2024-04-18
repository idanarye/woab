use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

enum ActivationState<S, F: 'static + FnOnce(&gtk4::Application) -> crate::Result<S>> {
    BeforeStartup(F),
    WaitingForStartupResult,
    StartupSucceeded(S),
    StartupFailed(crate::Error),
    ResultTakenOut,
}

impl<S, F: 'static + FnOnce(&gtk4::Application) -> crate::Result<S>> ActivationState<S, F> {
    fn take_startup_dlg(&mut self) -> Option<F> {
        match self {
            Self::BeforeStartup(_) => {
                let Self::BeforeStartup(dlg) = std::mem::replace(self, Self::WaitingForStartupResult) else {
                    panic!("We just checked that the variant is BeforeActivation...");
                };
                Some(dlg)
            }
            Self::WaitingForStartupResult => panic!(),
            Self::StartupSucceeded(_) => None,
            Self::StartupFailed(_) => None,
            Self::ResultTakenOut => panic!(),
        }
    }

    fn set_startup_result(&mut self, result: crate::Result<S>) {
        match self {
            Self::BeforeStartup(_) => panic!("Trying to set result before running the startup delegate"),
            Self::WaitingForStartupResult => {
                *self = match result {
                    Ok(ok) => Self::StartupSucceeded(ok),
                    Err(err) => Self::StartupFailed(err),
                };
            }
            Self::StartupSucceeded(_) | Self::StartupFailed(_) | Self::ResultTakenOut => {
                panic!("Trying to set result more than once")
            }
        }
    }

    fn take_startup_result(&mut self) -> Option<crate::Result<S>> {
        match std::mem::replace(self, Self::ResultTakenOut) {
            Self::BeforeStartup(_) => None,
            Self::WaitingForStartupResult => panic!("Trying to take the startup result when startup is still running"),
            Self::StartupSucceeded(ok) => Some(Ok(ok)),
            Self::StartupFailed(err) => Some(Err(err)),
            Self::ResultTakenOut => panic!(),
        }
    }
}

/// Run GTK and Actix.
///
/// The closure passed to this function will run inside the application's `startup` signal. Use it
/// to setup the application: build and run the initial window and launch any actors that need to
/// run at bootstrap.
pub fn main(app: gtk4::Application, dlg: impl 'static + FnOnce(&gtk4::Application) -> crate::Result<()>) -> crate::Result<()> {
    gtk4::init()?;

    let startup_state = Rc::new(RefCell::new(ActivationState::BeforeStartup(dlg)));

    app.connect_startup({
        let startup_state = startup_state.clone();
        move |app| {
            crate::run_actix_inside_gtk_event_loop();
            crate::block_on(async {
                let Some(dlg) = startup_state.borrow_mut().take_startup_dlg() else {
                    panic!("woab::main was used, but the `startup` signal was invoked more than once");
                };
                let result = dlg(app);
                let failed = result.is_err();
                startup_state.borrow_mut().set_startup_result(result);
                if failed {
                    app.quit();
                }
            })
        }
    });
    let exit_code = app.run();
    if matches!(*startup_state.borrow(), ActivationState::BeforeStartup(_)) {
        return if exit_code != glib::ExitCode::SUCCESS {
            Err(crate::Error::GtkBadExitCode(exit_code))
        } else {
            Ok(())
        };
    }
    crate::close_actix_runtime()??;
    if exit_code != glib::ExitCode::SUCCESS {
        return Err(crate::Error::GtkBadExitCode(exit_code));
    }
    let result = startup_state
        .borrow_mut()
        .take_startup_result()
        .expect("startup signal was not called");
    result
}

/// Helper function to configure the application so that when the last window is closed, the
/// application will shutdown.
///
/// Note that this will only work for windows that are attached to the application. To easily
/// attach windows to the application, use
/// [`BuilderWidgets::set_application`](crate::BuilderWidgets::set_application).
pub fn shutdown_when_last_window_is_closed(app: &gtk4::Application) {
    app.connect_window_removed(|app, _| {
        if app.windows().is_empty() {
            app.quit();
        }
    });
}
