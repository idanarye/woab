use gtk4::prelude::*;

/// Run GTK and Actix.
///
/// The closure passed to this function will run inside the application's `connect` signal. Use it
/// to setup the application: build and run the initial window and launch any actors that need to
/// run at bootstrap.
pub fn main(app: gtk4::Application, dlg: impl 'static + Fn(&gtk4::Application)) -> crate::Result<()> {
    gtk4::init()?;
    crate::run_actix_inside_gtk_event_loop();

    app.connect_activate(move |app| {
        crate::block_on(async {
            dlg(app);
        })
    });
    let exit_code = app.run();
    crate::close_actix_runtime()??;
    if exit_code != glib::ExitCode::SUCCESS {
        return Err(crate::Error::GtkBadExitCode(exit_code));
    }
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
