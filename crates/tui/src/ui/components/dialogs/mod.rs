pub mod quit;
use tui_realm_stdlib::Radio;
use tuirealm::{
    Event, MockComponent, NoUserEvent, State, StateValue,
    command::{Cmd, CmdResult, Direction},
    event::{Key, KeyEvent},
    props::{Alignment, BorderType, Borders, Color},
};

use crate::msgs::Msg;

#[derive(Debug, Clone, PartialEq)]
pub struct DialogStyle {
    pub dialog_type: DialogType,
    pub title_alignment: Alignment,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum DialogType {
    Warning,
}

#[derive(MockComponent)]
pub struct Dialog {
    component: Radio,
}

impl Dialog {
    pub fn new<T: Into<String>>(title: T, style: &DialogStyle) -> Self {
        let border_color = match style.dialog_type {
            DialogType::Warning => Color::LightYellow,
        };

        Self {
            component: Radio::default()
                .borders(
                    Borders::default()
                        .color(border_color)
                        .modifiers(BorderType::Rounded),
                )
                .title(title, style.title_alignment)
                .rewind(true)
                .choices(&["Ok", "Cancel"])
                .value(0),
        }
    }

    pub fn on(&mut self, ev: &Event<NoUserEvent>, on_ok: Msg, on_cancel: Msg) -> Option<Msg> {
        let cmd_result = match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => return Some(on_cancel),
            Event::Keyboard(KeyEvent {
                code: Key::Left, ..
            }) => self.perform(Cmd::Move(Direction::Left)),
            Event::Keyboard(KeyEvent {
                code: Key::Right, ..
            }) => self.perform(Cmd::Move(Direction::Right)),
            Event::Keyboard(KeyEvent {
                code: Key::Enter, ..
            }) => self.perform(Cmd::Submit),
            _ => return None,
        };

        match cmd_result {
            CmdResult::Submit(State::One(StateValue::Usize(0))) => Some(on_ok),
            CmdResult::Submit(State::One(StateValue::Usize(1))) => Some(on_cancel),
            CmdResult::None => None,
            _ => Some(Msg::None),
        }
    }
}
