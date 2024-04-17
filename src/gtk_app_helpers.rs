use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

enum ActivationState<S, F: 'static + FnOnce(&gtk4::Application) -> crate::Result<S>> {
    BeforeActivation(F),
    WaitingForActivationResult,
    ActivationSucceeded(S),
    ActivationFailed(crate::Error),
    ResultTakenOut,
}

impl<S, F: 'static + FnOnce(&gtk4::Application) -> crate::Result<S>> ActivationState<S, F> {
    fn take_activation_dlg(&mut self) -> Option<F> {
        match self {
            Self::BeforeActivation(_) => {
                let Self::BeforeActivation(dlg) = std::mem::replace(self, Self::WaitingForActivationResult) else {
                    panic!("We just checked that the variant is BeforeActivation...");
                };
                Some(dlg)
            }
            Self::WaitingForActivationResult => panic!(),
            Self::ActivationSucceeded(_) => None,
            Self::ActivationFailed(_) => None,
            Self::ResultTakenOut => panic!(),
        }
    }

    fn set_activation_result(&mut self, result: crate::Result<S>) {
        match self {
            Self::BeforeActivation(_) => panic!("Trying to set result before running the activation delegate"),
            Self::WaitingForActivationResult => {
                *self = match result {
                    Ok(ok) => Self::ActivationSucceeded(ok),
                    Err(err) => Self::ActivationFailed(err),
                };
            }
            Self::ActivationSucceeded(_) | Self::ActivationFailed(_) | Self::ResultTakenOut => {
                panic!("Trying to set result more than once")
            }
        }
    }

    fn take_activation_result(&mut self) -> Option<crate::Result<S>> {
        match std::mem::replace(self, Self::ResultTakenOut) {
            Self::BeforeActivation(_) => None,
            Self::WaitingForActivationResult => panic!("Trying to take the activation result when activation is still running"),
            Self::ActivationSucceeded(ok) => Some(Ok(ok)),
            Self::ActivationFailed(err) => Some(Err(err)),
            Self::ResultTakenOut => panic!(),
        }
    }
}

/// Run GTK and Actix.
///
/// The closure passed to this function will run inside the application's `connect` signal. Use it
/// to setup the application: build and run the initial window and launch any actors that need to
/// run at bootstrap.
pub fn main(app: gtk4::Application, dlg: impl 'static + FnOnce(&gtk4::Application) -> crate::Result<()>) -> crate::Result<()> {
    gtk4::init()?;
    crate::run_actix_inside_gtk_event_loop();

    let activation_state = Rc::new(RefCell::new(ActivationState::BeforeActivation(dlg)));

    app.connect_activate({
        let activation_state = activation_state.clone();
        move |app| {
            crate::block_on(async {
                let Some(dlg) = activation_state.borrow_mut().take_activation_dlg() else {
                    panic!("woab::main was used, but the `activate` signal was invoked more than once");
                };
                let result = dlg(app);
                let failed = result.is_err();
                activation_state.borrow_mut().set_activation_result(result);
                if failed {
                    app.quit();
                }
            })
        }
    });
    let exit_code = app.run();
    crate::close_actix_runtime()??;
    if exit_code != glib::ExitCode::SUCCESS {
        return Err(crate::Error::GtkBadExitCode(exit_code));
    }
    let result = activation_state
        .borrow_mut()
        .take_activation_result()
        .expect("activate signal was not called");
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
