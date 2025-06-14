use std::ops::Add;
use crate::app::App;
use color_eyre::owo_colors::OwoColorize;
use ratatui::Frame;
use ratatui::layout::Constraint::{Fill, Length, Min, Percentage};
use ratatui::layout::Flex::SpaceBetween;
use ratatui::layout::{Alignment, Constraint, Direction, Flex, Layout, Rect};
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
            Length(1), // Header
            Min(4),    // Body
            Length(1), // Footer
        ])
        .split(frame.area());

    // Header
    let header_block = Block::default().padding(Padding::horizontal(1));
    let mut title_text = Text::styled(
        "o4t ",
        Style::default().fg(Yellow).add_modifier(Modifier::BOLD),
    );
    title_text.push_span(Span::styled(
        env!("CARGO_PKG_VERSION"),
        Style::default()
            .remove_modifier(Modifier::BOLD)
            .add_modifier(Modifier::DIM),
    ));
    let title = Paragraph::new(title_text).block(header_block);
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
            build_styled_word(
                &mut words_text,
                char_style,
                app.current_user_input.to_string(),
                word.to_string(),
                true,
            );
            if word.len() == app.current_user_input.len() {
                words_text.push_span(Span::styled(" ", Style::default().add_modifier(Modifier::UNDERLINED)))
            } else {
                words_text.push_span(Span::default().content(" "));
            }
        } else if user_attempt.is_empty() {
            // It's not the current word, and there's no attempt yet, basic rendering.
            words_text.push_span(Span::styled(word, char_style));
            if index != words.len() - 1 {
                words_text.push_span(Span::default().content(" "));
            }
        } else {
            // It's not the current word, but we have attempted it - render the word attempt.
            build_styled_word(
                &mut words_text,
                char_style,
                user_attempt.to_string(),
                word.to_string(),
                false,
            );
            if index != words.len() - 1 {
                words_text.push_span(Span::default().content(" "));
            }
        }

    }

    // The body has 2 rows - a single cell height row for the timer, and 5 rows for the text to type
    let centered_body = center(sections[1], Length(frame.area().width), Length(6));
    let centered_body_sections = Layout::vertical([Length(1), Min(5)]).split(centered_body);

    // The game timer
    if app.game_active {
        let game_time_remaining_secs = app.game_time_remaining_millis().div_ceil(1000);
        let game_timer = Paragraph::new(Text::styled(
            game_time_remaining_secs.to_string() + " ",
            Style::default().fg(Yellow).add_modifier(Modifier::BOLD),
        ))
        .block(Block::default().padding(Padding::horizontal(8)));
        frame.render_widget(game_timer, centered_body_sections[0]);
    }

    // The words to be typed
    let words_paragraph = Paragraph::new(words_text)
        .wrap(Wrap::default())
        .block(Block::default().padding(Padding::horizontal(8)))
        .scroll((0, 0)); // TODO - scroll as we move through the paragraph
    frame.render_widget(words_paragraph, centered_body_sections[1]);

    // Footer
    let footer_block = Block::default().padding(Padding::horizontal(1));
    let footer_text = Text::styled("[esc] quit", Style::default().fg(Yellow));
    let footer_paragraph = Paragraph::new(footer_text).block(footer_block);
    let footer_area: [Rect; 2] = Layout::horizontal([Percentage(50), Percentage(50)])
        .flex(SpaceBetween)
        .areas(sections[2]);

    let footer_left_corner = footer_area[0];
    let footer_right_corner = footer_area[1];
    frame.render_widget(footer_paragraph, footer_left_corner);
}

fn build_styled_word(
    words_text: &mut Text,
    char_style: Style,
    user_attempt: String,
    expected_word: String,
    is_current_word: bool,
) {
    let zipped_chars = expected_word
        .chars()
        .zip(user_attempt.chars())
        .collect::<Vec<_>>();
    let n = zipped_chars.len();
    for (expected_char, user_char) in zipped_chars {
        let mut style = char_style;
        if user_char == expected_char {
            style = style.remove_modifier(Modifier::DIM);
            words_text.push_span(Span::styled(expected_char.to_string(), style));
        } else {
            words_text.push_span(Span::styled(expected_char.to_string(), char_style.fg(Red)));
        }
    }
    let mut remaining_chars_iter = expected_word.chars().skip(n);
    if let Some(cursor_char) = remaining_chars_iter.next() {
        if is_current_word {
            words_text.push_span(Span::styled(
                cursor_char.to_string(),
                char_style.add_modifier(Modifier::UNDERLINED),
            ));
        } else {
            words_text.push_span(Span::styled(cursor_char.to_string(), char_style));
        }
    }
    words_text.push_span(Span::styled(
        remaining_chars_iter.collect::<String>(),
        char_style,
    ));
}

fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}
