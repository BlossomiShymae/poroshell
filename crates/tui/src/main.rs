#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![deny(warnings)]
#![forbid(unsafe_code)]

use std::error::Error;

use tracing::debug;
use ui::UI;

mod cmds;
mod ids;
mod logger;
mod msgs;
mod ui;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    logger::setup();

    debug!("Creating UI");
    let mut ui = UI::new();
    debug!("Running UI");
    ui.run();

    Ok(())
}
