mod event_loops_bridge;
pub mod errors;

pub use woab_macros::WidgetsFromBuilder;

pub use event_loops_bridge::run_actix_inside_gtk_event_loop;
