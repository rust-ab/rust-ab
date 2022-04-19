use crate::{utils::monitoring::app::App, utils::monitoring::ui};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};

use ui::UI;

pub fn run(tick_rate: Duration) -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let res = run_app(&mut terminal, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let mut ui = UI::new();

    loop {
        
        terminal.draw(|f| ui.draw(f))?;
        
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char(c) => ui.on_key(c),
                    KeyCode::Left => ui.on_left(),
                    KeyCode::Up => ui.on_up(),
                    KeyCode::Right => ui.on_right(),
                    KeyCode::Down => ui.on_down(),
                    _ => {}
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            ui.on_tick(0, 0.);
            last_tick = Instant::now();
        }
        if ui.should_quit {
            return Ok(());
        }
    }
}
