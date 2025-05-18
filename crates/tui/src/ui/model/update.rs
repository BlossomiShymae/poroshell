use tuirealm::Update;

use crate::{cmds::BackgroundCmd, msgs::Msg};

use super::Model;

impl Update<Msg> for Model {
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        self.redraw = true;
        match msg.unwrap_or(Msg::None) {
            Msg::QuitDialogShow => {
                self.mount_quit_dialog();
            }
            Msg::QuitDialogOk | Msg::AppClose => {
                self.quit = true;
            }
            Msg::QuitDialogCancel => {
                self.umount_quit_dialog();
            }
            Msg::LibrariesInit => {
                self.bg_tx.send(BackgroundCmd::LibrariesLoad).ok();
            }
            Msg::LibrariesSubmit(index) => {
                if let Some(libraries) = self.libraries.take() {
                    if let Some(library) = libraries.get(index) {
                        let link = format!("https://github.com/{}/{}", library.owner, library.repo);
                        self.bg_tx.send(BackgroundCmd::LibrariesOpenLink(link)).ok();
                    }
                }
            }
            Msg::LibrariesBlur => {
                self.blur_libraries();
            }
            Msg::NavigationBlur => {
                self.blur_navigation();
            }
            Msg::None => (),
        }

        None
    }
}
