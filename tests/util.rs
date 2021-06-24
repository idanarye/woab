#[macro_export]
macro_rules! wait_for {
    ($pred:expr) => {{
        let timeout = std::time::Duration::from_secs(1);
        let time_limit = std::time::Instant::now() + timeout;
        loop {
            let is_over = time_limit < std::time::Instant::now();
            if $pred {
                break Ok(());
            } else if is_over {
                break Err(anyhow::Error::msg(concat!("Timed out: ", stringify!($pred))));
            }
            gtk::main_iteration();
        }
    }};
}
