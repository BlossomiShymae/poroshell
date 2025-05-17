use color_eyre::owo_colors::OwoColorize;
use data::RiotAPILibrary;
use tui_realm_stdlib::Table;
use tuirealm::{
    AttrValue, Attribute, Component, Event, MockComponent, NoUserEvent,
    command::{Cmd, Direction, Position},
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, Color, TableBuilder, TextSpan},
};

use crate::{ids::Id, msgs::Msg, ui::model::Model};

#[derive(MockComponent)]
pub struct Libraries {
    component: Table,
    init: bool,
}

impl Libraries {
    pub fn new() -> Self {
        Self {
            component: Table::default()
                .title("Libraries", Alignment::Center)
                .borders(Borders::default().modifiers(BorderType::Rounded))
                .scroll(true)
                .rewind(true)
                .highlighted_color(Color::White)
                .step(4)
                .row_height(1)
                .headers(&["Owner", "Repo", "Language"])
                .column_spacing(3)
                .widths(&[40, 40, 20]),
            init: false,
        }
    }
}

impl Component<Msg, NoUserEvent> for Libraries {
    fn on(&mut self, ev: tuirealm::Event<NoUserEvent>) -> Option<Msg> {
        match ev {
            Event::Tick if !self.init => {
                self.init = true;
                Some(Msg::LibrariesInit)
            }
            Event::Keyboard(KeyEvent { code: Key::Tab, .. }) => Some(Msg::LibrariesBlur),
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => {
                self.perform(Cmd::Move(Direction::Down));
                Some(Msg::None)
            }
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                self.perform(Cmd::Move(Direction::Up));
                Some(Msg::None)
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                ..
            }) => {
                self.perform(Cmd::Scroll(Direction::Down));
                Some(Msg::None)
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageUp, ..
            }) => {
                self.perform(Cmd::Scroll(Direction::Up));
                Some(Msg::None)
            }
            Event::Keyboard(KeyEvent {
                code: Key::Home, ..
            }) => {
                self.perform(Cmd::GoTo(Position::Begin));
                Some(Msg::None)
            }
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => {
                self.perform(Cmd::GoTo(Position::End));
                Some(Msg::None)
            }
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => Some(Msg::LibrariesSubmit(self.component.states.list_index)),
            _ => Some(Msg::None),
        }
    }
}

impl Model {
    pub fn blur_libraries(&mut self) {
        self.app
            .attr(&Id::Libraries, Attribute::Focus, AttrValue::Flag(false))
            .ok();
        self.app.active(&Id::Navigation).ok();
    }

    pub fn update_libraries(&mut self, libraries: Vec<RiotAPILibrary>) {
        let current_libraries = libraries
            .into_iter()
            .filter(is_lcu_or_ingame_library)
            .collect::<Vec<RiotAPILibrary>>();
        self.libraries = Some(current_libraries.clone());
        let mut table = TableBuilder::default();
        for library in current_libraries.into_iter() {
            table.add_col(TextSpan::from(library.owner));
            table.add_col(TextSpan::from(library.repo));
            table.add_col(TextSpan::from(library.language));
            table.add_row();
        }
        self.app
            .attr(
                &Id::Libraries,
                Attribute::Content,
                AttrValue::Table(table.build()),
            )
            .ok();
    }
}

fn is_lcu_or_ingame_library(x: &RiotAPILibrary) -> bool {
    if let Some(tags) = &x.tags {
        if tags.contains(&String::from("lcu")) || tags.contains(&String::from("ingame")) {
            return true;
        }
    }
    false
}
