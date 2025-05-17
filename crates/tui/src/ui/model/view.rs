use tracing::error;

use crate::ui::components::pages::Page;

use super::Model;

impl Model {
    pub fn view(&mut self) {
        if self.redraw {
            if let Err(err) = self.terminal.raw_mut().draw(|f| {
                match self.page {
                    Page::Home => Self::view_page_home(&mut self.app, f),
                }
                Self::view_quit_dialog(&mut self.app, f);
            }) {
                error!(error = err.get_ref(), "Failed to draw");
                panic!();
            }
        }
    }
}
