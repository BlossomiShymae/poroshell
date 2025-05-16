use std::time::Duration;

use color_eyre::eyre::Result;
use tuirealm::{
    Application, EventListenerCfg, NoUserEvent, Sub, SubClause, SubEventClause, Update,
    event::{Key, KeyEvent, KeyModifiers},
    terminal::{CrosstermTerminalAdapter, TerminalBridge},
};

use crate::{ids::Id, msgs::Msg};

use super::components::{global_listener::GlobalListener, pages::Page};

pub struct Model {
    pub app: Application<Id, Msg, NoUserEvent>,
    pub terminal: TerminalBridge<CrosstermTerminalAdapter>,
    pub quit: bool,
    pub redraw: bool,
    pub page: Page,
}

impl Model {
    pub async fn new() -> Self {
        let terminal = TerminalBridge::init_crossterm().expect("Cannot create terminal bridge");

        let app = Self::init_app();

        Self {
            app,
            terminal,
            quit: false,
            redraw: true,
            page: Page::Home,
        }
    }

    pub fn init_app() -> Application<Id, Msg, NoUserEvent> {
        let mut app = Application::init(
            EventListenerCfg::default()
                .crossterm_input_listener(Duration::from_millis(20), 10)
                .poll_timeout(Duration::from_millis(10))
                .tick_interval(Duration::from_secs(1)),
        );

        Self::mount_main(&mut app).unwrap();
        let _ = app.active(&Id::Libraries);

        app
    }

    fn mount_main(app: &mut Application<Id, Msg, NoUserEvent>) -> Result<()> {
        app.mount(
            Id::GlobalListener,
            Box::new(GlobalListener::new()),
            vec![
                Sub::new(
                    SubEventClause::Keyboard(KeyEvent {
                        code: Key::Esc,
                        modifiers: KeyModifiers::NONE,
                    }),
                    SubClause::Always,
                ),
                Sub::new(
                    SubEventClause::Keyboard(KeyEvent {
                        code: Key::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                    }),
                    SubClause::Always,
                ),
            ],
        )?;

        Self::mount_home(app)?;

        Ok(())
    }

    pub fn view(&mut self) {
        if self.redraw {
            match self.page {
                Page::Home => self.view_page_home(),
            }
        }
    }

    pub fn init_terminal(&mut self) {
        let _ = self.terminal.enable_raw_mode();
        let _ = self.terminal.enter_alternate_screen();
        let _ = self.terminal.clear_screen();
    }

    pub fn finalize_terminal(&mut self) {
        let _ = self.terminal.disable_raw_mode();
        let _ = self.terminal.leave_alternate_screen();
    }
}

impl Update<Msg> for Model {
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        self.redraw = true;
        match msg.unwrap_or(Msg::None) {
            Msg::AppClose => {
                self.quit = true;
                None
            }
            Msg::None => None,
        }
    }
}
