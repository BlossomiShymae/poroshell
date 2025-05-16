use crate::ui::components::pages::Page;

use super::Model;

impl Model {
    pub fn view(&mut self) {
        if self.redraw {
            match self.page {
                Page::Home => self.view_page_home(),
            }
        }
    }
}
