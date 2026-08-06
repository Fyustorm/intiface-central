#[macro_use]
extern crate log;

mod api;
mod embedded;
mod frb_generated;
mod in_process_frontend;
mod logging;
mod mobile_init;

#[cfg(target_os = "android")]
mod bridge;

pub use api::runtime::EngineOptionsExternal;
