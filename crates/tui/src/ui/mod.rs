use color_eyre::eyre::Result;
use model::Model;
use tuirealm::{PollStrategy, Update};

pub mod components;
pub mod model;

pub struct UI {
    model: Model,
}

impl UI {
    pub async fn new() -> Result<Self> {
        let model = Model::new().await;
        Ok(Self { model })
    }

    pub async fn run(&mut self) -> Result<()> {
        self.model.init_terminal();
        let res = self.run_inner().await;
        self.model.finalize_terminal();

        res
    }

    async fn run_inner(&mut self) -> Result<()> {
        while !self.model.quit {
            // Tick
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

            // Redraw
            if self.model.redraw {
                self.model.view();
                self.model.redraw = false;
            }
        }

        Ok(())
    }
}
