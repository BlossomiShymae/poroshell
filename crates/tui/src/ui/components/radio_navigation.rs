use tui_realm_stdlib::Radio;
use tuirealm::{
    Component, MockComponent, NoUserEvent,
    props::{Alignment, BorderType, Borders, Color},
};

use crate::msgs::Msg;

#[derive(MockComponent)]
pub struct RadioNavigation {
    component: Radio,
}

impl RadioNavigation {
    pub fn new() -> Self {
        Self {
            component: Radio::default()
                .borders(
                    Borders::default()
                        .modifiers(BorderType::Rounded)
                        .color(Color::LightYellow),
                )
                .foreground(Color::LightYellow)
                .title("Navigation", Alignment::Center)
                .rewind(false)
                .choices(&["Home", "Endpoints"]),
        }
    }
}

impl Component<Msg, NoUserEvent> for RadioNavigation {
    fn on(&mut self, _ev: tuirealm::Event<NoUserEvent>) -> Option<Msg> {
        Some(Msg::None)
    }
}
