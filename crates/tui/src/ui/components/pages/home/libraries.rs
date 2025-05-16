use tui_realm_stdlib::List;
use tuirealm::{
    Component, MockComponent, NoUserEvent,
    command::CmdResult,
    props::{Alignment, BorderType, Borders},
};

use crate::msgs::Msg;

#[derive(MockComponent)]
pub struct Libraries {
    component: List,
}

impl Libraries {
    pub fn new() -> Self {
        Self {
            component: List::default()
                .title("Libraries", Alignment::Center)
                .borders(Borders::default().modifiers(BorderType::Rounded)),
        }
    }
}

impl Component<Msg, NoUserEvent> for Libraries {
    fn on(&mut self, ev: tuirealm::Event<NoUserEvent>) -> Option<Msg> {
        let _ = match ev {
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
