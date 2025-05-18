pub mod background;
pub mod utils;
use std::sync::Arc;

use model::Model;
use tokio::sync::{
    Mutex,
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};
use tracing::debug;
use tuirealm::{PollStrategy, Update};

use crate::{
    cmds::{BackgroundCmd, BackgroundCmdResult},
    msgs::Msg,
};

pub mod components;
pub mod model;

pub struct UI {
    model: Model,
    bg_rx: Arc<Mutex<UnboundedReceiver<BackgroundCmd>>>,
    result_tx: Arc<Mutex<UnboundedSender<BackgroundCmdResult>>>,
    result_rx: UnboundedReceiver<BackgroundCmdResult>,
}

impl UI {
    pub fn new() -> Self {
        let (bg_tx, bg_rx) = unbounded_channel::<BackgroundCmd>();
        let (result_tx, result_rx) = unbounded_channel::<BackgroundCmdResult>();
        let model = Model::new(bg_tx);
        Self {
            model,
            bg_rx: Arc::new(Mutex::new(bg_rx)),
            result_tx: Arc::new(Mutex::new(result_tx)),
            result_rx,
        }
    }

    pub fn run(&mut self) {
        self.model.init_terminal();
        self.run_inner();
        self.model.finalize_terminal();
    }

    fn run_inner(&mut self) {
        debug!("Spinning background");
        self.run_background();

        debug!("Spinning UI");
        while !self.model.quit {
            // Tick background results
            while let Ok(result) = self.result_rx.try_recv() {
                self.model.redraw = true;
                match result {
                    BackgroundCmdResult::LibrariesReady(libraries) => {
                        self.model.update_libraries(libraries);
                    }
                }
            }

            // Tick UI
            match self.model.app.tick(PollStrategy::UpTo(20)) {
                Ok(messages) => {
                    for msg in messages {
                        let mut msg = Some(msg);
                        while msg.is_some() {
                            if matches!(msg, Some(msg) if msg != Msg::None) {
                                let printed_msg = format!("{msg:?}");
                                debug!(msg = printed_msg, "Received UI message");
                            }
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
    }
}
