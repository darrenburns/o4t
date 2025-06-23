use crate::app::{CurrentWord, CursorType};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub time: usize,
    pub theme: String,
    pub target_wpm: usize,
    pub cursor: CursorType,
    pub current_word: CurrentWord,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            time: 30,
            theme: "dracula".to_string(),
            target_wpm: 0,
            cursor: CursorType::Underline,
            current_word: CurrentWord::Highlight,
        }
    }
}
