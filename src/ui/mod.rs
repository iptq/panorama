//! UI library

mod colon_prompt;
mod input;
// mod keybinds;
mod mail_view;
// mod messages;
mod windows;

use std::any::Any;
use std::collections::HashMap;
use std::io::Stdout;
use std::mem;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use anyhow::Result;
use chrono::{Local, TimeZone};
use downcast_rs::Downcast;
use futures::{future::FutureExt, select, stream::StreamExt};
use panorama_imap::response::{AttributeValue, Envelope};
use panorama_tui::{
    crossterm::{
        cursor,
        event::{self, Event, EventStream, KeyCode, KeyEvent},
        execute, queue, style, terminal,
    },
    tui::{
        backend::CrosstermBackend,
        layout::{Constraint, Direction, Layout},
        style::{Color, Modifier, Style},
        text::{Span, Spans},
        widgets::*,
    },
    Frame, Terminal,
};
use tokio::{sync::mpsc, time};

use crate::config::ConfigWatcher;
use crate::mail::{EmailMetadata, MailEvent, MailStore};

use self::colon_prompt::ColonPrompt;
use self::input::{BaseInputHandler, HandlesInput, InputResult};
use self::mail_view::MailView;
// pub(crate) use self::messages::*;
use self::windows::*;

pub(crate) type FrameType<'a, 'b> = Frame<'a, &'b mut Stdout>;
// pub(crate) type FrameType<'a, 'b, 'c> = &'c mut Frame<'a, CrosstermBackend<&'b mut Stdout>>;
pub(crate) type TermType<'b> = &'b mut Terminal<Stdout>;

/// Parameters for passing to the UI thread
pub struct UiParams {
    /// Config updates
    pub config_update: ConfigWatcher,

    /// Mail store
    pub mail_store: MailStore,

    /// Handle to the screen
    pub stdout: Stdout,

    /// A channel for telling the UI to quit
    pub exit_tx: mpsc::Sender<()>,

    /// All the events coming in from the mail thread
    pub mail2ui_rx: mpsc::UnboundedReceiver<MailEvent>,
}

/// Main entrypoint for the UI
pub async fn run_ui2(params: UiParams) -> Result<()> {
    let mut stdout = params.stdout;
    let mut mail2ui_rx = params.mail2ui_rx;
    let exit_tx = params.exit_tx;

    execute!(stdout, cursor::Hide, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    // let backend = CrosstermBackend::new(&mut stdout);
    let mut term = Terminal::new(&mut stdout)?;
    let mut ui_events = EventStream::new();

    let should_exit = Arc::new(AtomicBool::new(false));

    let mail_store = params.mail_store;
    let mut ui = UI {
        should_exit: should_exit.clone(),
        window_layout: WindowLayout::default(),
        windows: HashMap::new(),
        page_names: HashMap::new(),
        mail_store: mail_store.clone(),
    };

    ui.open_window(MailView::new(mail_store));

    // let mut input_states: Vec<Box<dyn HandlesInput>> = vec![];

    while !should_exit.load(Ordering::Relaxed) {
        term.pre_draw()?;
        {
            let mut frame = term.get_frame();
            ui.draw(&mut frame).await?;
        }
        term.post_draw()?;

        select! {
            // got an event from the mail thread
            evt = mail2ui_rx.recv().fuse() => if let Some(evt) = evt {
                ui.process_mail_event(evt).await?;
            },

            // got an event from the ui thread
            evt = ui_events.next().fuse() => if let Some(evt) = evt {
                let evt = evt?;
                ui.process_event(evt);
            }

            // wait for approx 60fps
            // _ = time::sleep(FRAME_DURATION).fuse() => {},
        }
    }

    mem::drop(ui);
    mem::drop(term);

    execute!(
        stdout,
        style::ResetColor,
        cursor::Show,
        terminal::LeaveAlternateScreen
    )?;
    terminal::disable_raw_mode()?;

    exit_tx.send(()).await?;
    Ok(())
}

/// UI
pub struct UI {
    should_exit: Arc<AtomicBool>,
    window_layout: WindowLayout,
    windows: HashMap<LayoutId, Box<dyn Window>>,
    page_names: HashMap<PageId, String>,
    mail_store: MailStore,
}

impl UI {
    async fn draw(&mut self, f: &mut FrameType<'_, '_>) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Max(5000), Constraint::Length(1)])
            .split(f.size());

        let pages = self.window_layout.list_pages();

        // draw a list of pages at the bottom
        let titles = self
            .window_layout
            .list_pages()
            .iter()
            .enumerate()
            .map(|(i, id)| {
                self.page_names
                    .get(id)
                    .cloned()
                    .unwrap_or_else(|| i.to_string())
            })
            .map(Spans::from)
            .collect();
        let tabs = Tabs::new(titles).style(Style::default().bg(Color::DarkGray));
        f.render_widget(tabs, chunks[1]);
        debug!("drew chunks");

        // render all other windows
        let visible = self.window_layout.visible_windows(chunks[0]);
        for (layout_id, area) in visible.into_iter() {
            if let Some(window) = self.windows.get(&layout_id) {
                window.draw(f, area, self).await?;
                debug!("drew {:?} {:?}", layout_id, area);
            }
        }

        Ok(())
    }

    fn open_window(&mut self, window: impl Window) {
        debug!("opened window {:?}", window.name());
        let (layout_id, page_id) = self.window_layout.new_page();

        let window = Box::new(window);
        self.windows.insert(layout_id, window);
    }

    /// Main entrypoint for handling any kind of event coming from the terminal
    fn process_event(&mut self, evt: Event) {
        if let Event::Key(evt) = evt {
            if let KeyEvent {
                code: KeyCode::Char('q'),
                ..
            } = evt
            {
                self.should_exit.store(true, Ordering::Relaxed);
            }

            // handle states in the state stack
            // although this is written in a for loop, every case except one should break
            let should_pop = false;
            // for input_state in input_states.iter_mut().rev() {
            //     match input_state.handle_key(&mut term, evt)? {
            //         InputResult::Ok => break,
            //         InputResult::Push(state) => {
            //             input_states.push(state);
            //             break;
            //         }
            //         InputResult::Pop => {
            //             should_pop = true;
            //             break;
            //         }
            //     }
            // }

            if should_pop {
                debug!("pop state");
                // input_states.pop();
            }
        }
    }

    async fn process_mail_event(&mut self, evt: MailEvent) -> Result<()> {
        self.mail_store.handle_mail_event(evt).await?;
        Ok(())
    }
}
