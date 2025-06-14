use std::cmp::min;
use std::iter::Zip;
use std::str::Chars;
use crate::app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::Color::{Red, Yellow};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Span, Text};
use ratatui::widgets::{Block, Padding, Paragraph, Wrap};

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
    let title = Paragraph::new(Text::styled("tigertype 🐯", Style::default().fg(Yellow)))
        .block(header_block);
    frame.render_widget(title, sections[0]);

    // Body text containing the words to show the user that they must type.
    let words = app
        .words
        .iter()
        .map(|word_attempt| word_attempt.word.clone())
        .collect::<Vec<_>>();

    let mut words_text = Text::default();
    for (index, word) in words.iter().enumerate() {
        let mut char_style = Style::default().fg(Color::Gray).add_modifier(Modifier::DIM);
        let user_attempt = &app.words[index].user_attempt;
        if app.current_word_offset == index {
            // Check which characters match and which ones don't in order to build up the styling for this word.
            char_style = char_style.add_modifier(Modifier::BOLD);
            build_styled_word(&mut words_text, char_style, app.current_user_input.to_string(), word.to_string());
        } else if user_attempt.is_empty() {
            // It's not the current word, and there's no attempt yet, basic rendering.
            words_text.push_span(Span::styled(word, char_style));
        } else {
            // It's not the current word, but we have attempted it - render the word attempt.
            build_styled_word(&mut words_text, char_style, user_attempt.to_string(), word.to_string());
        }

        if index != words.len() - 1 {
            words_text.push_span(Span::default().content(" "));
        }
    }

    let words_paragraph = Paragraph::new(words_text)
        .wrap(Wrap::default())
        .block(Block::default().padding(Padding::horizontal(8)))
        .scroll((0, 0)); // TODO - scroll as we move through the paragraph
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

fn build_styled_word(words_text: &mut Text, char_style: Style, user_attempt: String, expected_word: String) {
    let zipped_chars = expected_word.chars().zip(user_attempt.chars()).collect::<Vec<_>>();
    let n = zipped_chars.len();
    for (expected_char, user_char) in zipped_chars {
        if user_char == expected_char {
            words_text.push_span(Span::styled(
                expected_char.to_string(),
                char_style.remove_modifier(Modifier::DIM),
            ));
        } else {
            words_text.push_span(Span::styled(expected_char.to_string(), char_style.fg(Red)));
        }
    }
    words_text.push_span(Span::styled(expected_word.chars().skip(n).collect::<String>(), char_style));
}

fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}
