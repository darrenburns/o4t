use crate::app::{CurrentWord, CursorType};
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
#[command(version, about)]
pub struct Cli {

    #[clap(short, long, value_parser, value_name = "SECS")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub time: Option<usize>,

    #[clap(
        long,
        value_parser,
        value_name = "THEME_NAME",
    )]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub theme: Option<String>,

    #[clap(long, value_parser)]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub target_wpm: Option<usize>,
    
    #[clap(short, long, value_enum, value_name = "STYLE")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub cursor: Option<CursorType>,
    
    #[clap(long, value_enum, value_name = "FOCUS_STYLE")]
    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
    pub current_word: Option<CurrentWord>,
}
