use crate::app::App;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::Color::Yellow;
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Padding, Paragraph, Wrap};
use ratatui::Frame;

#[derive(Debug)]
pub struct Theme {
    name: String,
    title_fg: Color,
    title_bg: Color,
}

pub fn ui(frame: &mut Frame, app: &App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(4),    // Body
            Constraint::Length(1), // Footer
        ])
        .split(frame.area());

    // Header
    let header_block = Block::default().padding(Padding::horizontal(1));
    let title = Paragraph::new(Text::styled(
        "tigertype 🐯",
        Style::default().fg(Yellow),
    ))
    .block(header_block);
    frame.render_widget(title, sections[0]);

    // Body text containing the words to show the user that they must type.
    let words = app.words.iter().map(|word_attempt| word_attempt.word.clone()).collect::<Vec<_>>();
    let words_paragraph = Paragraph::new(Text::styled(
        words.join(" "),
        Style::default().fg(Color::Gray),
    ))
        .wrap(Wrap::default())
        .block(Block::default()
        .padding(Padding::horizontal(8)))
        .scroll((0, 0));  // TODO - scroll as we move through the paragraph
    let body_text_render_area = center(
        sections[1],
        Constraint::Length(frame.area().width),
        Constraint::Length(6),
    );


    frame.render_widget(words_paragraph, body_text_render_area);

    // Footer
    let footer_block = Block::default().padding(Padding::horizontal(1));
    let footer_text = Text::styled("[esc] quit", Style::default().fg(Yellow));
    let footer_paragraph = Paragraph::new(footer_text).block(footer_block);
    frame.render_widget(footer_paragraph, sections[2]);
}

fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}
