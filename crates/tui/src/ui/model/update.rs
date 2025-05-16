use tuirealm::Update;

use crate::{cmds::BackgroundCmd, msgs::Msg};

use super::Model;

impl Update<Msg> for Model {
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        self.redraw = true;
        match msg.unwrap_or(Msg::None) {
            Msg::AppClose => {
                self.quit = true;
                None
            }
            Msg::LibrariesInit => {
                self.bg_tx.send(BackgroundCmd::LibrariesLoad).ok();
                None
            }
            Msg::LibrariesSubmit(index) => {
                if let Some(libraries) = self.libraries.take() {
                    if let Some(library) = libraries.get(index) {
                        let link = format!("https://github.com/{}/{}", library.owner, library.repo);
                        self.bg_tx.send(BackgroundCmd::LibrariesOpenLink(link)).ok();
                    }
                }
                None
            }
            Msg::None => None,
        }
    }
}
