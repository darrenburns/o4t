use ratatui::style::{Color, Style};

#[derive(Default, Clone, Debug)]
pub struct Theme {
    pub(crate) name: &'static str,
    pub(crate) fg: Color,
    pub(crate) bg: Color,
    pub(crate) primary: Color,
    pub(crate) secondary: Color,
    pub(crate) error: Color,
    pub(crate) supports_alpha: bool,
}
