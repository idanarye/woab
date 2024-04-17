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
    if app.application_id().is_some() && !app.flags().contains(gio::ApplicationFlags::NON_UNIQUE) {
        return Err(crate::Error::IncorrectMain {
            method_used: "woab::main",
            should_have_used: "woab::main_shared",
            reason: concat!(
                "Application ID is set and the NON_UNIQUE flag was not passed",
                " - ",
                "GTK will make all invocations share a single process.",
                " ",
                "This needs to be addressed."
            ),
        });
    }
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

pub fn main_shared<T: 'static>(
    app: gtk4::Application,
    initial_dlg: impl 'static + FnOnce(&gtk4::Application) -> crate::Result<T>,
    activation_dlg: impl 'static + Fn(&mut T, &gtk4::Application),
) -> crate::Result<()> {
    if app.application_id().is_none() {
        return Err(crate::Error::IncorrectMain {
            method_used: "woab::main_shared",
            should_have_used: "woab::main",
            reason: "Application ID is not set - GTK will not make invocation share a process.",
        });
    }
    if app.flags().contains(gio::ApplicationFlags::NON_UNIQUE) {
        return Err(crate::Error::IncorrectMain {
            method_used: "woab::main_shared",
            should_have_used: "woab::main",
            reason: "Application has the NON_UNIQUE flag - GTK will not make invocation share a process.",
        });
    }

    gtk4::init()?;
    crate::run_actix_inside_gtk_event_loop();

    let activation_state = Rc::new(RefCell::new(ActivationState::BeforeActivation(initial_dlg)));

    app.connect_activate({
        let activation_state = activation_state.clone();
        move |app| {
            crate::block_on(async {
                let initial_dlg = activation_state.borrow_mut().take_activation_dlg();
                if let Some(initial_dlg) = initial_dlg {
                    let result = initial_dlg(app);
                    let failed = result.is_err();
                    activation_state.borrow_mut().set_activation_result(result);
                    if failed {
                        app.quit();
                        return;
                    }
                }
                match &mut *activation_state.borrow_mut() {
                    ActivationState::BeforeActivation(_) => panic!("We've already called take_activation_dlg"),
                    ActivationState::WaitingForActivationResult => panic!("should have already called set_activation_result"),
                    ActivationState::ActivationSucceeded(state) => {
                        activation_dlg(state, app);
                    }
                    ActivationState::ActivationFailed(_) => panic!("Application should have already quit due to the activation failure"),
                    ActivationState::ResultTakenOut => panic!("Main application already exited"),
                }
            });
        }
    });
    let exit_code = app.run();
    crate::close_actix_runtime()??;
    if exit_code != glib::ExitCode::SUCCESS {
        return Err(crate::Error::GtkBadExitCode(exit_code));
    }
    activation_state
        .borrow_mut()
        .take_activation_result()
        .expect("activate signal was not called")?;
    Ok(())
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
