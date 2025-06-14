use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use ratatui::crossterm::{event, execute};
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use std::error::Error;
use std::io;
use crate::app::{App, Screen};
use crate::ui::ui;

mod app;
mod ui;
mod words;

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stderr = std::io::stderr();
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Ok(do_print) = res {
        if do_print {
            // app.print_json()?;
        }
    } else if let Err(err) = res {
        println!("{}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<bool> {

    loop {
        // The ui function will the frame and draw to it
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            // Skip all key release events.
            if key.kind == event::KeyEventKind::Release {
                continue;
            }

            match app.current_screen {
                Screen::Game => match key.code {
                    // Pressing any character, while the game hasn't started, starts the game
                    KeyCode::Char(' ') => {
                        if !app.current_user_input.is_empty() {
                            app.words[app.current_word_offset].user_attempt = app.current_user_input.clone();
                            app.current_word_offset += 1;
                            app.current_user_input = String::new();
                        }
                    }
                    KeyCode::Char(char) => {
                        if !app.game_active {
                            app.game_active = true;
                        }
                        app.current_user_input.push(char);
                    }
                    // Pressing escape exits.
                    KeyCode::Esc => {
                        app.current_screen = Screen::Exiting;  // TODO
                        return Ok(true)
                    },
                    _ => {}
                }
            _ => {}
            }
        }
    }
}