use std::sync::Arc;

use color_eyre::eyre::Result;
use data::RiotAPILibrary;
use tokio::sync::{
    Mutex,
    mpsc::{UnboundedReceiver, UnboundedSender},
};
use tracing::error;

use crate::cmds::{BackgroundCmd, BackgroundCmdResult};

use super::UI;

impl UI {
    pub fn run_background(&self) -> Result<()> {
        let rx: Arc<Mutex<UnboundedReceiver<BackgroundCmd>>> = self.bg_rx.clone();
        let tx: Arc<Mutex<UnboundedSender<BackgroundCmdResult>>> = self.result_tx.clone();
        tokio::spawn(async move {
            let mut lock = rx.lock().await;
            // Tick background
            while let Some(msg) = lock.recv().await {
                let result = match msg {
                    BackgroundCmd::LibrariesLoad => Self::load_libraries(tx.clone()).await,
                    BackgroundCmd::LibrariesOpenLink(link) => Self::open_library_link(link),
                };
                if let Err(err) = result {
                    error!(
                        error = err.root_cause(),
                        "Failed to execute background command"
                    );
                }
            }
        });

        Ok(())
    }

    async fn load_libraries(
        result_tx: Arc<Mutex<UnboundedSender<BackgroundCmdResult>>>,
    ) -> Result<()> {
        let libraries = reqwest::get("https://raw.githubusercontent.com/BlossomiShymae/poroschema/refs/heads/main/other/libraries.json")
            .await?
            .error_for_status()?
            .json::<Vec<RiotAPILibrary>>()
            .await?;

        let lock = result_tx.lock().await;
        lock.send(BackgroundCmdResult::LibrariesReady(libraries))
            .ok();

        Ok(())
    }

    fn open_library_link(link: String) -> Result<()> {
        open::that(link)?;

        Ok(())
    }
}
