use crate::words;
use rand::seq::IteratorRandom;
use std::ops::Div;
use std::time::Duration;
use tachyonfx::Interpolation::QuadOut;
use tachyonfx::{fx, Effect};

pub enum Screen {
    Game,
    Results,
    Info,
    Exiting,
}

const NUMBER_OF_WORDS_TO_PICK: usize = 500;
const DEFAULT_GAME_LENGTH: Duration = Duration::from_secs(30);

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

impl WordAttempt {
    pub fn new(word: String) -> WordAttempt {
        WordAttempt {
            word,
            user_attempt: String::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ScoreScreenState {
    pub menu_index: usize,
}

// Holds the state for the app
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
    
    pub score_screen_state: ScoreScreenState,
}

pub fn load_words_effect() -> Effect {
    fx::coalesce((180, QuadOut))
}

pub fn load_results_screen_effect() -> Effect {
    fx::coalesce((180, QuadOut))
}

impl App {
    pub fn new() -> App {
        App {
            current_user_input: String::new(),
            current_word_offset: 0,
            words: generate_words(),
            current_screen: Screen::Game,
            time_remaining: DEFAULT_GAME_LENGTH,
            game_active: false,
            millis_at_current_game_start: 0,
            current_millis: 0,
            score: Score::default(),
            load_words_effect: load_words_effect(),
            load_results_screen_effect: load_results_screen_effect(),
            last_tick_duration: Duration::ZERO,
            is_debug_mode: false,  // TODO - make cli switch
            debug_string: "".to_string(),
            score_screen_state: ScoreScreenState::default(),
        }
    }

    pub fn reset_game(&mut self) {
        *self = App::new();
    }

    pub fn game_time_elapsed_millis(&self) -> u64 {
        if self.game_active {
            self.current_millis - self.millis_at_current_game_start
        } else {
            0
        }
    }

    pub fn game_time_remaining_millis(&self) -> u64 {
        (DEFAULT_GAME_LENGTH.as_millis() as u64).saturating_sub(self.game_time_elapsed_millis())
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
        // We add the current_word_offset below as it represents the number of spaces, which should
        // be included in the WPM calculation.
        let wpm = ((character_matches + self.current_word_offset) as f32 / 5.) * (60. / seconds_elapsed);

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
