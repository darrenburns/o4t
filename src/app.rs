use crate::theme::Theme;
use crate::{words, Cli};
use clap::ValueEnum;
use derive_setters::Setters;
use rand::seq::IteratorRandom;
use ratatui::prelude::Color;
use ratatui::style::{Style, Stylize};
use serde::{Deserialize, Serialize};
use std::ops::Div;
use std::rc::Rc;
use std::time::Duration;
use tachyonfx::Interpolation::QuadOut;
use tachyonfx::{fx, Effect};
use crate::config::Config;

pub enum Screen {
    Game,
    Results,
}

const NUMBER_OF_WORDS_TO_PICK: usize = 500;
#[derive(Debug, PartialOrd, PartialEq)]
pub struct WordAttempt {
    // the word the user was asked and attempted to type
    pub word: String,
    // what the user typed for this word
    pub user_attempt: String,
}

#[derive(Debug, Default)]
pub struct Score {
    // Number of characters matching what they should be at the current point in time.
    pub character_matches: usize,
    // Number of characters which don't match what they should be at the current point in time.
    // This value can decrease if the user corrects a typo.
    pub character_mismatches: usize,
    // The number of correctly typed characters in this session. Always increasing.
    pub character_hits: usize,
    // The number of characters which were typed which shouldn't have been in this session.
    // This number cannot decrease. If you make a typo, it remains in this value.
    pub character_misses: usize,
    // The ratio of character_hits / character_hits + character_misses
    pub accuracy: f32,
    // Number of characters typed per minute.
    pub chars_per_minute: f32,
    // WPM = (character_matches * 5) * (60 / session_length_secs)
    pub wpm: f32,
    // Number of words typed CORRECTLY per minute.
    pub real_words_per_minute: f32,
    // Total number of CORRECTLY typed words.
    pub num_words: usize,
    // The number of words typed correctly in a row. Always increasing. Words that were typed
    // incorrectly then changed don't count.
    pub best_char_streak: usize,
    pub current_char_streak: usize,
}

impl Score {
    pub fn is_perfect(&self) -> bool {
        self.character_misses == 0 && self.num_words > 0
    }
}

impl WordAttempt {
    pub fn new(word: String) -> WordAttempt {
        WordAttempt {
            word,
            user_attempt: String::new(),
        }
    }
}

// Holds the state for the app
#[derive(Setters)]
pub struct App {
    // the current input the user has typed while trying to type words[0]
    pub current_user_input: String,
    // The index of the word in words that is being attempted by the user
    pub current_word_offset: usize,
    // contains the history of words for the current session.
    // the current word the user is being asked to type is words[0]
    pub words: Vec<WordAttempt>,
    pub current_screen: Screen,
    pub time_remaining: Duration,
    pub game_active: bool,
    pub millis_at_current_game_start: u64,
    pub current_millis: u64,
    pub score: Score,
    pub load_results_screen_effect: Effect,
    pub load_words_effect: Effect,
    pub last_tick_duration: Duration,

    pub is_debug_mode: bool,
    // Debug string that can be rendered to screen
    pub debug_string: String,

    pub theme_name: String,
    pub cursor_style: CursorType,
    pub themes: Vec<Theme>,
    pub config: Rc<Config>,
}

pub fn load_words_effect(theme: Theme) -> Effect {
    fx::parallel(&[
        fx::fade_from_fg(theme.secondary, (180, QuadOut)),
        fx::coalesce((180, QuadOut)),
    ])
}

pub fn load_score_screen_effect() -> Effect {
    fx::coalesce((180, QuadOut))
}

#[derive(ValueEnum, Clone, Debug, Copy, Serialize, Deserialize)]
#[clap(rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum CursorType {
    Block,
    Underline,
    None,
}

#[derive(ValueEnum, Clone, Debug, Copy, Serialize, Deserialize)]
#[clap(rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum CurrentWord {
    Bold,
    Highlight,
    None
}

impl App {
    pub fn with_config(config: Rc<Config>) -> App {
        let theme_name = &config.theme;
        let theme = get_theme(theme_name);
        App {
            current_user_input: String::new(),
            current_word_offset: 0,
            words: generate_words(),
            current_screen: Screen::Game,
            time_remaining: Duration::from_secs(config.time as u64),
            game_active: false,
            millis_at_current_game_start: 0,
            current_millis: 0,
            score: Score::default(),
            load_words_effect: load_words_effect(theme.clone()),
            load_results_screen_effect: load_score_screen_effect(),
            last_tick_duration: Duration::ZERO,
            is_debug_mode: false, // TODO - make cli switch
            debug_string: "".to_string(),
            theme_name: theme_name.to_string(),
            themes: get_themes(),
            cursor_style: config.cursor,
            config,
        }
    }

    pub fn next_theme(&mut self) {
        let mut themes = self
            .themes
            .iter()
            .cycle()
            .skip_while(|theme| theme.name != self.theme_name)
            .skip(1);

        self.theme_name = themes.next().unwrap().name.to_string();
    }

    pub fn get_current_theme(&self) -> Theme {
        self.themes
            .iter()
            .find(|t| t.name == self.theme_name)
            .unwrap()
            .clone()
    }

    pub fn reset_game(&mut self) {
        let config = self.config.clone();
        *self = App::with_config(config).theme_name(self.theme_name.to_string());
    }

    pub fn game_time_elapsed_millis(&self) -> u64 {
        if self.game_active {
            self.current_millis - self.millis_at_current_game_start
        } else {
            0
        }
    }

    pub fn game_time_remaining_millis(&self) -> u64 {
        (self.time_remaining.as_millis() as u64).saturating_sub(self.game_time_elapsed_millis())
    }

    pub fn refresh_internal_score(&mut self) {
        let mut character_matches: usize = 0;
        let mut character_mismatches: usize = 0;
        let mut num_correct_words: usize = 0;

        // Count hits and misses
        for (index, attempt) in self.words.iter().enumerate() {
            let zipped_chars;
            if index != self.current_word_offset {
                zipped_chars = attempt.user_attempt.chars().zip(attempt.word.chars());
            } else {
                zipped_chars = self.current_user_input.chars().zip(attempt.word.chars());
            }
            let mut this_word_hits = 0;
            for (user_char, expected_char) in zipped_chars {
                let is_hit = user_char == expected_char;
                if is_hit {
                    character_matches += 1;
                    this_word_hits += 1;
                } else {
                    character_mismatches += 1;
                }
            }
            if this_word_hits == attempt.word.len() {
                num_correct_words += 1
            }
        }

        let character_hits = self.score.character_hits;
        let character_misses = self.score.character_misses;
        let accuracy =
            (character_hits as f32).div(character_hits.saturating_add(character_misses) as f32);

        let num_chars = character_matches.saturating_add(character_mismatches);

        // Chars and words per minute
        let seconds_elapsed = (self.game_time_elapsed_millis() as f32) / 1000.;
        let minutes_elapsed = seconds_elapsed / 60.;
        let chars_per_minute = num_chars as f32 / minutes_elapsed;
        let real_words_per_minute = num_correct_words as f32 / minutes_elapsed;
        // We add the num_correct_words below as it represents the number of spaces, which should
        // be included in the WPM calculation.
        let mut wpm =
            ((character_matches + num_correct_words) as f32 / 5.) * (60. / seconds_elapsed);

        if wpm.is_infinite() {
            wpm = 0.;
        }

        self.score = Score {
            character_matches,
            character_mismatches,
            character_hits: self.score.character_hits,
            character_misses: self.score.character_misses,
            accuracy,
            chars_per_minute,
            wpm,
            real_words_per_minute,
            num_words: num_correct_words,
            best_char_streak: self.score.best_char_streak,
            current_char_streak: self.score.current_char_streak,
        }
    }
}

fn generate_words() -> Vec<WordAttempt> {
    let mut rng = rand::rng();
    words::ENGLISH_1K_WORDS
        .iter()
        .choose_multiple(&mut rng, NUMBER_OF_WORDS_TO_PICK)
        .iter()
        .map(|s| WordAttempt::new(s.to_string()))
        .collect()
}

fn get_themes() -> Vec<Theme> {
    vec![
        Theme {
            name: "terminal-yellow",
            fg: Color::Reset,
            bg: Color::Reset,
            primary: Color::Yellow,
            secondary: Color::Yellow,
            success: Color::Green,
            error: Color::Red,
            supports_alpha: false,
            character_match: Style::default().not_dim(),
            character_mismatch: Style::default().fg(Color::Red),
            character_upcoming: Style::default().dim(),
        },
        Theme {
            name: "terminal-cyan",
            fg: Color::White,
            bg: Color::Blue,
            primary: Color::Cyan,
            secondary: Color::Cyan,
            success: Color::Green,
            error: Color::Yellow,
            supports_alpha: false,
            character_match: Style::default().not_dim(),
            character_mismatch: Style::default().fg(Color::Red),
            character_upcoming: Style::default().dim(),
        },
        Theme {
            name: "nord",
            fg: Color::from_u32(0xD8DEE9),        // nord4
            bg: Color::from_u32(0x2E3440),        // nord0
            primary: Color::from_u32(0x88C0D0),   // nord8
            secondary: Color::from_u32(0xB48EAD), // nord15
            success: Color::from_u32(0xA3BE8C),   // nord14
            error: Color::from_u32(0xBF616A),     // nord11
            supports_alpha: true,
            character_match: Style::default().fg(Color::from_u32(0xA3BE8C)).not_dim(),
            character_mismatch: Style::default().fg(Color::from_u32(0xBF616A)),
            character_upcoming: Style::default().fg(Color::from_u32(0xD8DEE9)),
        },
        Theme {
            name: "catppuccin-mocha",
            fg: Color::from_u32(0xCDD6F4),        // Text
            bg: Color::from_u32(0x1E1E2E),        // Base
            primary: Color::from_u32(0x89B4FA),   // Blue
            secondary: Color::from_u32(0xCBA6F7), // Mauve
            success: Color::from_u32(0xA6E3A1),   // Green
            error: Color::from_u32(0xF38BA8),     // Red
            supports_alpha: true,
            character_match: Style::default().not_dim(),
            character_mismatch: Style::default().fg(Color::from_u32(0xF38BA8)),
            character_upcoming: Style::default().fg(Color::from_u32(0xCDD6F4)),
        },
        Theme {
            name: "dracula",
            fg: Color::from_u32(0xF8F8F2),        // Foreground
            bg: Color::from_u32(0x282A36),        // Background
            primary: Color::from_u32(0xBD93F9),   // Purple
            secondary: Color::from_u32(0x8BE9FD), // Cyan
            success: Color::from_u32(0x50FA7B),   // Green
            error: Color::from_u32(0xFF5555),     // Red
            supports_alpha: true,
            character_match: Style::default().fg(Color::from_u32(0x50FA7B)).not_dim(),
            character_mismatch: Style::default().fg(Color::from_u32(0xFF5555)),
            character_upcoming: Style::default().fg(Color::from_u32(0xF8F8F2)),
        },
        Theme {
            name: "gruvbox",
            fg: Color::from_u32(0xEBDBB2),        // fg1
            bg: Color::from_u32(0x282828),        // bg0
            primary: Color::from_u32(0xFABD2F),   // yellow
            secondary: Color::from_u32(0x8EC07C), // aqua
            success: Color::from_u32(0xB8BB26),   // green
            error: Color::from_u32(0xFB4934),     // red
            supports_alpha: true,
            character_match: Style::default().not_dim(),
            character_mismatch: Style::default().fg(Color::from_u32(0xFB4934)),
            character_upcoming: Style::default().fg(Color::from_u32(0xA89984)),  // fg4
        },
        Theme {
            name: "solarized-dark",
            fg: Color::from_u32(0x839496),        // base0
            bg: Color::from_u32(0x002B36),        // base03
            primary: Color::from_u32(0x268BD2),   // blue
            secondary: Color::from_u32(0x2AA198), // cyan
            success: Color::from_u32(0x859900),   // green
            error: Color::from_u32(0xDC322F),     // red
            supports_alpha: true,
            character_match: Style::default().fg(Color::from_u32(0x859900)),
            character_mismatch: Style::default().fg(Color::from_u32(0xDC322F)),
            character_upcoming: Style::default().fg(Color::from_u32(0x839496)),
        },
        Theme {
            name: "tokyo-night",
            fg: Color::from_u32(0xC0CAF5),        // fg
            bg: Color::from_u32(0x1A1B26),        // bg
            primary: Color::from_u32(0x7AA2F7),   // blue
            secondary: Color::from_u32(0xff4499), // orange
            success: Color::from_u32(0x9ECE6A),   // green
            error: Color::from_u32(0xf7768e),     // red
            supports_alpha: true,
            character_match: Style::default().not_dim(),
            character_mismatch: Style::default().fg(Color::from_u32(0xff9e64)),
            character_upcoming: Style::default().fg(Color::from_u32(0x6584C9)),
        },
        Theme {
            name: "monokai",
            fg: Color::from_u32(0xF8F8F2),
            bg: Color::from_u32(0x272822),
            primary: Color::from_u32(0xF92672),   // pink
            secondary: Color::from_u32(0xA6E22E), // green
            success: Color::from_u32(0xA6E22E),   // green
            error: Color::from_u32(0xfd971f),
            supports_alpha: true,
            character_match: Style::default().not_dim(),
            character_mismatch: Style::default().fg(Color::from_u32(0xfd971f)),
            character_upcoming: Style::default().fg(Color::from_u32(0x999999)),
        },
        Theme {
            name: "galaxy",
            fg: Color::from_u32(0xC0CAF5),
            bg: Color::from_u32(0x0F0F1F),
            primary: Color::from_u32(0xC45AFF),
            secondary: Color::from_u32(0xa684e8),
            success: Color::from_u32(0x50FA7B), // bright green
            error: Color::from_u32(0xFF4500),
            supports_alpha: true,
            character_match: Style::default().fg(Color::from_u32(0x50FA7B)).not_dim(),
            character_mismatch: Style::default().fg(Color::from_u32(0xFF4500)),
            character_upcoming: Style::default().fg(Color::from_u32(0xC0CAF5)),
        },
    ]
}

fn get_theme(theme_name: &str) -> Theme {
    let themes = get_themes();
    themes.iter().find(|t| t.name == theme_name).unwrap().clone()
}
