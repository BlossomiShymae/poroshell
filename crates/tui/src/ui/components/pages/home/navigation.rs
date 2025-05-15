use tui_realm_stdlib::List;
use tuirealm::{
    Component, MockComponent, NoUserEvent,
    command::CmdResult,
    props::{Alignment, BorderType, Borders, TableBuilder, TextSpan},
};

use crate::msgs::Msg;

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
        let _ = match ev {
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
