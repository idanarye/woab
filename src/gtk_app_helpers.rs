use gtk4::prelude::*;

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

pub fn shutdown_when_last_window_is_closed(app: &gtk4::Application) {
    app.connect_window_removed(|app, _| {
        if app.windows().is_empty() {
            app.quit();
        }
    });
}
