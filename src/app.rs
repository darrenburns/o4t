use crate::words;
use rand::seq::IteratorRandom;
use std::time::Duration;

pub enum Screen {
    Game,
    Info,
    Exiting,
}

const NUMBER_OF_WORDS_TO_PICK: usize = 500;
const DEFAULT_GAME_LENGTH: Duration = Duration::from_secs(4);

#[derive(Debug, PartialOrd, PartialEq)]
pub struct WordAttempt {
    // the word the user was asked and attempted to type
    pub word: String,
    // what the user typed for this word
    pub user_attempt: String,
}

pub struct Score {
    character_hits: u16,
    character_misses: u16,
    accuracy: f32,
    chars_per_minute: f32,
    words_per_minute: f32,
    num_words: u16,
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
        }
    }

    pub fn start_game(&mut self) {
        self.game_active = true;
    }

    pub fn game_time_elapsed_millis(&self) -> u64 {
        if self.game_active {self.current_millis - self.millis_at_current_game_start} else { 0 }
    }

    pub fn game_time_remaining_millis(&self) -> u64 {
        (DEFAULT_GAME_LENGTH.as_millis() as u64).saturating_sub(self.game_time_elapsed_millis())
    }

    pub fn compute_scoring(&self) -> Score {
        let mut character_hits = 0;
        let mut character_misses = 0;
        let mut num_words = 0;

        // Count hits and misses
        for (index, attempt) in self.words.iter().enumerate() {
            num_words += 1;
            if self.current_word_offset > index {
                let zipped_chars = attempt.user_attempt.chars().zip(attempt.word.chars());
                for (user_char, expected_char) in zipped_chars {
                    let is_hit = user_char == expected_char;
                    character_hits += is_hit as u16;
                    character_misses += !(is_hit as u16);
                }
            }
        }

        // Compute accuracy based on character hits and misses
        let num_chars = character_hits.saturating_add(character_misses);
        let accuracy = character_hits as f32 / num_chars as f32;
        
        // Chars and words per minute
        let minutes_elapsed = (self.game_time_elapsed_millis() * 1000 * 60) as f32;
        let chars_per_minute = num_chars as f32 / minutes_elapsed;
        let words_per_minute = num_words as f32 / minutes_elapsed;

        Score {
            character_hits,
            character_misses,
            accuracy,
            chars_per_minute,
            words_per_minute,
            num_words,
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
