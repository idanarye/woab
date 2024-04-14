use core::future::Future;
use std::cell::Cell;
use std::rc::Rc;

use gtk4::prelude::*;

#[macro_export]
macro_rules! wait_for {
    ($pred:expr) => {{
        let timeout = std::time::Duration::from_secs(1);
        let time_limit = std::time::Instant::now() + timeout;
        loop {
            let is_over = time_limit < std::time::Instant::now();
            if $pred {
                break Ok(());
                // break;
            } else if is_over {
                break Err(anyhow::Error::msg(concat!("Timed out: ", stringify!($pred))));
                // panic!(concat!("Timed out: ", stringify!($pred)));
            }
            actix::clock::sleep(core::time::Duration::new(0, 1_000)).await;
        }
    }};
}

pub fn test_main(fut: impl 'static + Future<Output = anyhow::Result<()>>) -> anyhow::Result<()> {
    gtk4::init().unwrap();
    woab::run_actix_inside_gtk_event_loop();
    let app = gtk4::Application::default();

    let fut = Cell::new(Some(fut));
    let res = Rc::new(Cell::new(Ok(())));
    app.connect_activate({
        let res = res.clone();
        move |app| {
            let fut = fut.take().unwrap();
            res.set(woab::block_on(fut));
            app.quit();
        }
    });
    app.run();
    woab::close_actix_runtime().unwrap().unwrap();
    res.replace(Ok(()))
}
