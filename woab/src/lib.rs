pub mod errors;
mod event_loops_bridge;
mod builder_signal;
mod woab_actor;
mod builder_dissect;
mod factories;

pub use woab_macros::{WidgetsFromBuilder, BuilderSignal};
pub use event_loops_bridge::run_actix_inside_gtk_event_loop;
pub use builder_signal::BuilderSignal;
pub use woab_actor::WoabActor;
pub use builder_dissect::dissect_builder_xml;
pub use factories::{BuilderFactory, ActorFactory};
