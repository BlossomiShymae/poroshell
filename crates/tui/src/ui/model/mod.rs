pub mod update;
pub mod view;
use std::time::Duration;

use color_eyre::eyre::Result;
use data::RiotAPILibrary;
use tokio::sync::mpsc::UnboundedSender;
use tuirealm::{
    Application, EventListenerCfg, NoUserEvent, Sub, SubClause, SubEventClause,
    event::{Key, KeyEvent, KeyModifiers},
    terminal::{CrosstermTerminalAdapter, TerminalBridge},
};

use crate::{cmds::BackgroundCmd, ids::Id, msgs::Msg};

use super::components::{global_listener::GlobalListener, pages::Page};

pub struct Model {
    pub app: Application<Id, Msg, NoUserEvent>,
    pub terminal: TerminalBridge<CrosstermTerminalAdapter>,
    pub quit: bool,
    pub redraw: bool,
    pub page: Page,
    pub bg_tx: UnboundedSender<BackgroundCmd>,
    pub libraries: Option<Vec<RiotAPILibrary>>,
}

impl Model {
    pub fn new(bg_tx: UnboundedSender<BackgroundCmd>) -> Self {
        let terminal = TerminalBridge::init_crossterm().expect("Cannot create terminal bridge");

        let app = Self::init_app();

        Self {
            app,
            terminal,
            quit: false,
            redraw: true,
            page: Page::Home,
            bg_tx,
            libraries: None,
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
        app.active(&Id::Navigation).ok();
        app.active(&Id::Libraries).ok();

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
