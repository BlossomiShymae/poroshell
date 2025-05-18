use tui_realm_stdlib::List;
use tuirealm::{
    AttrValue, Attribute, Component, Event, MockComponent, NoUserEvent,
    command::CmdResult,
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, TableBuilder, TextSpan},
};

use crate::{ids::Id, msgs::Msg, ui::model::Model};

#[derive(MockComponent)]
pub struct Navigation {
    component: List,
}

impl Navigation {
    pub fn new() -> Self {
        Self {
            component: List::default()
                .borders(Borders::default().modifiers(BorderType::Rounded))
                .scroll(true)
                .title("Nav", Alignment::Left)
                .rows(
                    TableBuilder::default()
                        .add_col(TextSpan::from("Documents"))
                        .add_row()
                        .build(),
                )
                .selected_line(0),
        }
    }
}

impl Component<Msg, NoUserEvent> for Navigation {
    fn on(&mut self, ev: tuirealm::Event<NoUserEvent>) -> Option<Msg> {
        let _cmd_result = match ev {
            Event::Keyboard(KeyEvent { code: Key::Tab, .. }) => return Some(Msg::NavigationBlur),
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}

impl Model {
    pub fn blur_navigation(&mut self) {
        self.app
            .attr(&Id::Navigation, Attribute::Focus, AttrValue::Flag(false))
            .ok();
        self.app.active(&Id::Libraries).ok();
    }
}
