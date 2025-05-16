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
    let mut ui = UI::new().await?;
    debug!("Running UI");
    ui.run().await?;

    Ok(())
}
