use tui_realm_stdlib::{Container, Label};
use tuirealm::{
    Component, MockComponent, NoUserEvent,
    props::{BorderType, Borders, Layout},
    ratatui::layout::{Constraint, Direction},
};

use crate::msgs::Msg;

#[derive(MockComponent)]
pub struct Welcome {
    component: Container,
}

impl Welcome {
    pub fn new() -> Self {
        Self {
            component: Container::default()
                .borders(Borders::default().modifiers(BorderType::Rounded))
                .layout(
                    Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints([Constraint::Percentage(100)].as_ref()),
                )
                .children(vec![Box::new(
                    Label::default().text("Welcome to Poroshell"),
                )]),
        }
    }
}

impl Component<Msg, NoUserEvent> for Welcome {
    fn on(&mut self, _ev: tuirealm::Event<NoUserEvent>) -> Option<Msg> {
        Some(Msg::None)
    }
}
