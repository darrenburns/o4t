use ratatui::style::{Color, Style};

#[derive(Default, Clone, Debug)]
pub struct Theme {
    pub(crate) name: &'static str,
    pub(crate) fg: Color,
    pub(crate) bg: Color,
    pub(crate) primary: Color,
    pub(crate) secondary: Color,
    pub(crate) error: Color,
    pub(crate) success: Color,
    pub(crate) character_match: Style,
    pub(crate) character_mismatch: Style,
    pub(crate) character_upcoming: Style,
    pub(crate) supports_alpha: bool,
}
