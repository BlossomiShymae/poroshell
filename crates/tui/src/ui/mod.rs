pub mod background;
use std::sync::Arc;

use color_eyre::eyre::Result;
use model::Model;
use tokio::sync::{
    Mutex,
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};
use tuirealm::{PollStrategy, Update};

use crate::cmds::{BackgroundCmd, BackgroundCmdResult};

pub mod components;
pub mod model;

pub struct UI {
    model: Model,
    bg_rx: Arc<Mutex<UnboundedReceiver<BackgroundCmd>>>,
    result_tx: Arc<Mutex<UnboundedSender<BackgroundCmdResult>>>,
    result_rx: UnboundedReceiver<BackgroundCmdResult>,
}

impl UI {
    pub async fn new() -> Result<Self> {
        let (bg_tx, bg_rx) = unbounded_channel::<BackgroundCmd>();
        let (result_tx, result_rx) = unbounded_channel::<BackgroundCmdResult>();
        let model = Model::new(bg_tx).await;
        Ok(Self {
            model,
            bg_rx: Arc::new(Mutex::new(bg_rx)),
            result_tx: Arc::new(Mutex::new(result_tx)),
            result_rx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.model.init_terminal();
        let res = self.run_inner().await;
        self.model.finalize_terminal();

        res
    }

    async fn run_inner(&mut self) -> Result<()> {
        self.run_background()?;

        while !self.model.quit {
            // Tick background results
            while let Ok(result) = self.result_rx.try_recv() {
                self.model.redraw = true;
                match result {
                    BackgroundCmdResult::LibrariesReady(libraries) => {
                        self.model.update_libraries(libraries)
                    }
                }
            }

            // Tick UI
            match self.model.app.tick(PollStrategy::Once) {
                Ok(messages) => {
                    for msg in messages.into_iter() {
                        let mut msg = Some(msg);
                        while msg.is_some() {
                            msg = self.model.update(msg);
                        }
                    }
                }
                Err(_) => todo!(),
            }

            // Redraw view
            if self.model.redraw {
                self.model.view();
                self.model.redraw = false;
            }
        }

        Ok(())
    }
}
