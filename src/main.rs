use crate::app::{load_score_screen_effect, load_words_effect, App, Screen};
use crate::cli::Cli;
use crate::config::Config;
use crate::ui::ui;
use clap::{CommandFactory, FromArgMatches};
use etcetera::{choose_base_strategy, BaseStrategy};
use figment::providers::Env;
use figment::providers::{Format, Serialized, Toml};
use figment::{Figment};
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
use std::rc::Rc;
use std::time::Instant;
use std::{io, thread};
use tachyonfx::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

mod app;
mod theme;
mod ui;
mod words;
mod wrap;
mod cli;
mod config;

fn main() -> Result<(), Box<dyn Error>> {
    let xdg = choose_base_strategy().unwrap();
    let config_file = xdg.config_dir().join("o4t/config.toml");
    let mut cmd = Cli::command();
    let dynamic_help_text = format!(
        "CONFIGURATION:\n    Config file: {}\n    Environment variables are prefixed with O4T_",
        config_file.display()
    );
    cmd = cmd.after_help(dynamic_help_text);
    let matches = cmd.get_matches_mut();
    let parsed_cli = match Cli::from_arg_matches(&matches) {
        Ok(config) => {
            config
        },
        Err(err) => {
            err.exit();
        }
    };
    let config: Config = Figment::new()
        .merge(Serialized::defaults(Config::default()))
        .merge(Toml::file(config_file))
        .merge(Env::prefixed("O4T_"))
        .merge(Serialized::defaults(parsed_cli))
        .extract()?;

    let mut app = App::with_config(Rc::from(config));

    let mut stderr = io::stderr();
    execute!(stderr, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stderr);
    let mut terminal = Terminal::new(backend)?;
    enable_raw_mode()?;
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
    app.load_words_effect = load_words_effect(app.get_current_theme().clone());
    loop {
        app.last_tick_duration = last_frame_instant.elapsed().into();
        last_frame_instant = Instant::now();

        // The ui function will the frame and draw to it
        terminal.draw(|f| ui(f, app))?;

        if let Ok(_) = rx.try_recv() {
            let last_tick_millis = app.last_tick_duration.as_millis() as u64;
            app.current_millis = app.current_millis + last_tick_millis;
            if app.game_time_remaining_millis() == 0 {
                app.load_results_screen_effect = load_score_screen_effect();
                app.game_active = false;
                app.current_screen = Screen::Results;
            }
            if app.game_active {
                app.refresh_internal_score();
                if app.config.target_wpm > 0 {
                    match app.ghost_offset {
                        Some(current_ghost) => {
                            let last_tick_secs = app.last_tick_duration.as_secs_f64();
                            let target_chars_per_minute = 5 * app.config.target_wpm;
                            let target_chars_per_second = target_chars_per_minute as f64 / 60.;
                            let delta = target_chars_per_second * last_tick_secs;
                            let next_ghost = current_ghost + delta;
                            app.ghost_offset = Some(next_ghost);
                        }
                        None => {}
                    }
                }
            }
        }

        if !event::poll(Duration::from_millis(32).into())? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                continue;
            }

            let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
            let alt = key.modifiers.contains(KeyModifiers::ALT);

            // Global bindings
            match key.code {
                KeyCode::Char('t') if ctrl => {
                    app.next_theme();
                    continue;
                }
                KeyCode::Esc => return Ok(true),
                KeyCode::Tab => app.reset_game(),
                _ => {}
            }

            // Screen-specific bindings
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
                            continue;
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
