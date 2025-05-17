use tracing::debug;
use tui_realm_stdlib::Phantom;
use tuirealm::{
    Component, MockComponent, NoUserEvent,
    event::{Key, KeyModifiers},
};

use crate::msgs::Msg;

#[derive(MockComponent)]
pub struct GlobalListener {
    component: Phantom,
}

impl GlobalListener {
    pub fn new() -> Self {
        Self {
            component: Phantom::default(),
        }
    }
}

impl Component<Msg, NoUserEvent> for GlobalListener {
    fn on(&mut self, ev: tuirealm::Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            tuirealm::Event::Keyboard(key_event) => {
                let printed_modifier = format!("{:?}", key_event.modifiers);
                let printed_code = format!("{:?}", key_event.code);
                debug!(
                    code = printed_code,
                    modifier = printed_modifier,
                    "Key pressed"
                );
                match key_event.code {
                    Key::Esc => Some(Msg::QuitDialogShow),
                    Key::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => {
                        Some(Msg::AppClose)
                    }
                    _ => Some(Msg::None),
                }
            }
            _ => Some(Msg::None),
        }
    }
}
