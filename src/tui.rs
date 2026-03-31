use std::time::Duration;

use color_eyre::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, EventStream, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::{
    sync::mpsc,
    time::interval,
};
use tokio_util::sync::CancellationToken;

use crate::action::Action;

pub type Tui = Terminal<CrosstermBackend<std::io::Stderr>>;

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Action>,
    cancel: CancellationToken,
}

impl EventHandler {
    pub fn new(tick_rate: f64, frame_rate: f64) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();

        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            let mut event_stream = EventStream::new();
            let mut tick_interval = interval(Duration::from_secs_f64(1.0 / tick_rate));
            let mut render_interval = interval(Duration::from_secs_f64(1.0 / frame_rate));

            loop {
                let action = tokio::select! {
                    _ = cancel_clone.cancelled() => break,
                    _ = tick_interval.tick() => Action::Tick,
                    _ = render_interval.tick() => Action::Render,
                    event = event_stream.next() => match event {
                        Some(Ok(crossterm::event::Event::Key(key))) if key.kind == KeyEventKind::Press => {
                            Action::Key(key)
                        }
                        Some(Ok(crossterm::event::Event::Resize(w, h))) => Action::Resize(w, h),
                        Some(Err(_)) | None => break,
                        _ => continue,
                    },
                };
                if tx.send(action).is_err() {
                    break;
                }
            }
        });

        Self { rx, cancel }
    }

    pub async fn next(&mut self) -> Result<Action> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| color_eyre::eyre::eyre!("Event channel closed"))
    }

    pub fn stop(&self) {
        self.cancel.cancel();
    }
}

pub fn init() -> Result<Tui> {
    enable_raw_mode()?;
    execute!(std::io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;
    let terminal = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;
    Ok(terminal)
}

pub fn restore() -> Result<()> {
    execute!(std::io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
    disable_raw_mode()?;
    Ok(())
}
