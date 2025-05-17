use tuirealm::{
    Application, Component, Frame, MockComponent, NoUserEvent,
    props::{Alignment, Color},
    ratatui::widgets::Clear,
};

use crate::{
    ids::Id,
    msgs::Msg,
    ui::{model::Model, utils::draw_area_in_absolute},
};

use super::{Dialog, DialogStyle, DialogType};

#[derive(MockComponent)]
pub struct QuitDialog {
    component: Dialog,
}

impl QuitDialog {
    pub fn new() -> Self {
        let component = Dialog::new(
            " Are you sure you want to quit? ",
            DialogStyle {
                dialog_type: DialogType::Warning,
                title_alignment: Alignment::Center,
            },
        );

        Self { component }
    }
}

impl Component<Msg, NoUserEvent> for QuitDialog {
    fn on(&mut self, ev: tuirealm::Event<NoUserEvent>) -> Option<Msg> {
        self.component
            .on(ev, Msg::QuitDialogOk, Msg::QuitDialogCancel)
    }
}

impl Model {
    pub fn mount_quit_dialog(&mut self) {
        self.app
            .mount(Id::QuitDialog, Box::new(QuitDialog::new()), Vec::new())
            .ok();
        self.app.active(&Id::QuitDialog).ok();
    }

    pub fn umount_quit_dialog(&mut self) {
        self.app.umount(&Id::QuitDialog).ok();
    }

    pub fn view_quit_dialog(app: &mut Application<Id, Msg, NoUserEvent>, f: &mut Frame<'_>) {
        if app.mounted(&Id::QuitDialog) {
            let dialog = draw_area_in_absolute(f.area(), 35, 3);
            f.render_widget(Clear, dialog);
            app.view(&Id::QuitDialog, f, dialog);
        }
    }
}
