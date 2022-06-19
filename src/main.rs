//! Pipeline runner.

#![deny(warnings)]
#![deny(missing_docs, rustdoc::missing_crate_level_docs)]

pub mod error;
pub mod yutil;

use log::info;

fn main() {
    env_logger::init();
    info!("Running pipeline");
}
