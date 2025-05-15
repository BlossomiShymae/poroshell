pub mod libraries;
pub mod navigation;
pub mod welcome;

use color_eyre::eyre::Result;
use libraries::Libraries;
use navigation::Navigation;
use tracing::error;
use tuirealm::{
    Application, NoUserEvent,
    ratatui::layout::{Constraint, Direction, Layout},
};

use welcome::Welcome;

use crate::{ids::Id, msgs::Msg, ui::model::Model};

impl Model {
    pub fn mount_home(app: &mut Application<Id, Msg, NoUserEvent>) -> Result<()> {
        app.mount(Id::Libraries, Box::new(Libraries::new()), Vec::new())?;
        app.mount(Id::Navigation, Box::new(Navigation::new()), Vec::new())?;
        app.mount(Id::Welcome, Box::new(Welcome::new()), Vec::new())?;

        Ok(())
    }

    pub fn view_page_home(&mut self) {
        if let Err(err) = self.terminal.raw_mut().draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Fill(1), Constraint::Fill(2)].as_ref())
                .split(f.area());

            let sub_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Fill(1), Constraint::Fill(2)].as_ref())
                .split(chunks[1]);

            self.app.view(&Id::Libraries, f, sub_chunks[1]);
            self.app.view(&Id::Navigation, f, chunks[0]);
            self.app.view(&Id::Welcome, f, sub_chunks[0]);
        }) {
            error!(error = err.get_ref(), "Failed to draw");
            panic!()
        }
    }
}
