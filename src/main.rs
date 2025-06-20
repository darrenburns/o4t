use crate::app::{load_score_screen_effect, load_words_effect, score_screen_perfect_round_effect, App, Screen};
use crate::ui::ui;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers,
};
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::crossterm::{event, execute};
use ratatui::Terminal;
use std::cmp::max;
use std::error::Error;
use std::time::Instant;
use std::{io, thread};
use std::rc::Rc;
use tachyonfx::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

mod app;
mod ui;
mod words;
mod wrap;
mod theme;

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
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
    let (tx, mut rx) = mpsc::channel(100);

    let _tokio_handle = thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(background_task(tx));
    });
    terminal.clear()?;

    let mut last_frame_instant = Instant::now();
    app.load_words_effect = load_words_effect();
    loop {
        app.last_tick_duration = last_frame_instant.elapsed().into();
        last_frame_instant = Instant::now();

        // The ui function will the frame and draw to it
        terminal.draw(|f| ui(f, app))?;

        if let Ok(millis_elapsed) = rx.try_recv() {
            app.current_millis = app.current_millis + app.last_tick_duration.as_millis() as u64;
            if app.game_time_remaining_millis() == 0 {
                app.load_results_screen_effect = load_score_screen_effect();
                if app.score.accuracy == 100. {
                    app.perfect_score_effect = score_screen_perfect_round_effect(Rc::clone(&app.theme));
                }
                app.game_active = false;
                app.current_screen = Screen::Results;
            }
            if app.game_active {
                app.refresh_internal_score();
            }
        }

        if !event::poll(Duration::from_millis(32).into())? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                continue;
            }

            match key.code {
                KeyCode::Esc => return Ok(true),
                KeyCode::Tab => app.reset_game(),
                _ => {}
            }

            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            let alt = key.modifiers.contains(KeyModifiers::ALT);

            match app.current_screen {
                Screen::Game => match key.code {
                    // Pressing any character, while the game hasn't started, starts the game
                    KeyCode::Char(' ') => {
                        if !app.current_user_input.is_empty() {
                            app.words[app.current_word_offset].user_attempt =
                                app.current_user_input.clone();
                            app.current_word_offset += 1;
                            app.current_user_input = String::new();
                        }
                    }
                    KeyCode::Char(char) => {
                        if ctrl && char == 'w' {
                            app.current_user_input = String::new();
                        }

                        let current_word = &app.words[app.current_word_offset].word;
                        let cursor_offset = app.current_user_input.len();
                        let expected_char = current_word.chars().nth(cursor_offset);
                        if let Some(expected_char) = expected_char {
                            if char == expected_char {
                                app.score.current_char_streak += 1;
                                app.score.character_hits += 1;
                            } else {
                                app.score.current_char_streak = 0;
                                app.score.character_misses += 1;
                            }
                        } else {
                            // User has gone beyond the word and is typing extra characters.
                            app.score.character_misses += 1;
                            app.score.current_char_streak = 0;
                        }
                        app.score.best_char_streak =
                            max(app.score.best_char_streak, app.score.current_char_streak);

                        if !app.game_active {
                            app.game_active = true;
                            app.millis_at_current_game_start = app.current_millis;
                        }
                        app.current_user_input.push(char);
                    }
                    KeyCode::Backspace if app.game_active => {
                        if ctrl || alt {
                            app.current_user_input = String::new();
                        }
                        match app.current_user_input.pop() {
                            Some(_) => {}
                            None => {
                                // Go back into the previous word if possible.
                                if app.current_word_offset != 0
                                    && app.words[app.current_word_offset - 1].user_attempt
                                        != app.words[app.current_word_offset - 1].word
                                {
                                    app.current_word_offset -= 1;
                                    app.current_user_input =
                                        app.words[app.current_word_offset].user_attempt.clone();
                                }
                            }
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

async fn background_task(tx: mpsc::Sender<u64>) {
    let mut interval = interval(Duration::from_millis(32).into());
    let mut millis_elapsed = 0u64;
    loop {
        interval.tick().await;
        millis_elapsed += 50;
        // If the receiver is dropped, the task will gracefully exit.
        if tx.send(millis_elapsed).await.is_err() {
            break;
        }
    }
}
