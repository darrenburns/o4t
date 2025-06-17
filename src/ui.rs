use crate::app::{App, Screen};
use ratatui::buffer::Buffer;
use ratatui::layout::Alignment;
use ratatui::prelude::{Line, Widget};
use ratatui::{
    Frame,
    layout::Constraint,
    layout::Constraint::{Length, Min},
    layout::Direction,
    layout::Flex,
    layout::Flex::SpaceBetween,
    layout::Layout,
    layout::Rect,
    style::Color::{Red, Yellow},
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::{Block, Padding, Paragraph, Wrap},
};
use std::rc::Rc;
use ratatui::layout::Flex::{Center, SpaceAround};
use tachyonfx::{EffectRenderer, Shader};

#[derive(Default, Debug)]
struct ResultData {
    pub value: String,
    pub subtext: String,
}

impl Widget for ResultData {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let key_style = Style::default().add_modifier(Modifier::DIM);
        let value_style = Style::default().fg(Yellow).add_modifier(Modifier::BOLD);
        let text = Text::from(vec![
            Line::styled(self.value, value_style),
            Line::styled(self.subtext, key_style),
        ]);
        text.render(area, buf);
    }
}

pub fn ui(screen_frame: &mut Frame, app: &mut App) {
    match app.current_screen {
        Screen::Game => build_game_screen(screen_frame, app),
        Screen::Results => build_results_screen(screen_frame, app),
        Screen::Info => {}
        Screen::Exiting => {}
    }
}

fn build_game_screen(screen_frame: &mut Frame, app: &mut App) {
    let screen_sections = Layout::default()
        .horizontal_margin(3)
        .vertical_margin(1)
        .direction(Direction::Vertical)
        .constraints([
            Length(1), // Header
            Min(6),    // Body (1 timer row, 5 lines of text)
            Length(1), // Footer
        ])
        .split(screen_frame.area());

    // Header
    let header = build_header();
    screen_frame.render_widget(header, screen_sections[0]);

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
                false,
            );
            if app.current_user_input.len() >= word.len() {
                words_text.push_span(Span::styled(
                    " ",
                    Style::default().add_modifier(Modifier::UNDERLINED),
                ))
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
                true,
            );
            if index != words.len() - 1 {
                words_text.push_span(Span::default().content(" "));
            }
        }
    }

    // The body has 2 rows - a single cell height row for the timer, and 5 rows for the text to type
    let centered_body = center(
        screen_sections[1],
        Length(screen_frame.area().width),
        Length(6),  // 1 timer row, 5 lines of text
    );
    let centered_body_sections = Layout::vertical([Length(1), Min(5)]).split(centered_body);

    // The game timer
    if app.game_active {
        let game_time_remaining_secs = app.game_time_remaining_millis().div_ceil(1000);
        let mut timer_style = Style::default().fg(Yellow).add_modifier(Modifier::BOLD);
        if game_time_remaining_secs <= 3 {
            timer_style = timer_style.add_modifier(Modifier::UNDERLINED);
        }
        let game_timer = Paragraph::new(Text::styled(
            game_time_remaining_secs.to_string(),
            timer_style,
        ))
        .block(Block::default().padding(Padding::horizontal(8)));
        screen_frame.render_widget(game_timer, centered_body_sections[0]);
    }

    // The words to be typed
    let words_paragraph = Paragraph::new(words_text)
        .wrap(Wrap::default())
        .block(Block::default().padding(Padding::horizontal(8)))
        .scroll((0, 0)); // TODO - scroll as we move through the paragraph

    let launch_effect = &mut app.load_words_effect;
    screen_frame.render_widget(words_paragraph, centered_body_sections[1]);
    if launch_effect.running() {
        screen_frame.render_effect(
            launch_effect,
            centered_body_sections[1],
            app.last_tick_duration.into(),
        );
    }

    // Footer
    build_footer(screen_frame, screen_sections, app, true, true);
}

fn build_header() -> Paragraph<'static> {
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
    title
}

fn build_results_screen(screen_frame: &mut Frame, app: &mut App) {
    let screen_sections = Layout::default()
        .horizontal_margin(3)
        .vertical_margin(1)
        .direction(Direction::Vertical)
        .constraints([
            Length(1), // Header
            Min(2),   // Body
            Length(1), // Footer
        ])
        .split(screen_frame.area());

    screen_frame.render_widget(build_header(), screen_sections[0]);

    // Score screen body
    let score = &app.score;
    let score_data = vec![
        ResultData {
            value: format!("{:.0} ", score.words_per_minute),
            subtext: "wpm".to_string(),
        },
        ResultData {
            value: format!("{:.0}%", score.accuracy * 100.),
            subtext: "accuracy".to_string(),
        },
        ResultData {
            value: score.character_hits.to_string(),
            subtext: "hits".to_string(),
        },
        ResultData {
            value: score.character_misses.to_string(),
            subtext: "misses".to_string(),
        },
        ResultData {
            value: score.best_char_streak.to_string(),
            subtext: "streak".to_string(),
        },
        ResultData {
            value: score.num_words.to_string(),
            subtext: "words".to_string(),
        },
    ];
    let col_constraints = (0..4).map(|_| Length(10));
    let row_constraints = (0..3).map(|_| Length(2));
    let horizontal = Layout::horizontal(col_constraints).spacing(1);
    let vertical = Layout::vertical(row_constraints).flex(Center).spacing(1).horizontal_margin(1);

    let rows = vertical.split(screen_sections[1]);
    let cells = rows.iter().flat_map(|&row| horizontal.split(row).to_vec());

    for (score_data, cell_area) in score_data.into_iter().zip(cells) {
        screen_frame.render_widget(score_data, cell_area);
    }

    let load_effect = &mut app.load_results_screen_effect;
    if load_effect.running() {
        screen_frame.render_effect(
            load_effect,
            screen_sections[1],
            app.last_tick_duration.into(),
        );
    }
    build_footer(screen_frame, screen_sections, app, false, true);
}

fn build_footer(
    screen_frame: &mut Frame,
    sections: Rc<[Rect]>,
    app: &mut App,
    show_scoring: bool,
    show_reset: bool,
) {
    let footer_sections: [Rect; 2] = Layout::horizontal([Constraint::Fill(1), Min(10)])
        .flex(SpaceBetween)
        .areas(sections[2]);

    let keys_block = Block::default().padding(Padding::left(1));

    let key_style = Style::default().fg(Yellow).add_modifier(Modifier::BOLD);
    let value_style = Style::default().add_modifier(Modifier::DIM);
    let mut keys = Line::from(vec![
        Span::styled("ESC ", key_style),
        Span::styled("quit  ", value_style),
    ]);
    if show_reset {
        keys.push_span(Span::styled("TAB ", key_style));
        keys.push_span(Span::styled("restart ", value_style));
    }
    let keys_paragraph = Paragraph::new(keys).block(keys_block);

    let footer_left_corner = footer_sections[0];
    screen_frame.render_widget(keys_paragraph, footer_left_corner);

    let footer_right_corner = footer_sections[1];
    if show_scoring {
        let empty_score_placeholder = "-";
        let score = &app.score;
        let score_block = Block::default().padding(Padding::right(1));
        let accuracy = if app.game_active && !score.accuracy.is_nan() {
            format!("{:.0}%", score.accuracy * 100.0)
        } else {
            empty_score_placeholder.to_string()
        };
        let wpm = if app.game_active && !score.words_per_minute.is_nan() {
            format!("{:.0}", score.words_per_minute)
        } else {
            empty_score_placeholder.to_string()
        };
        let score_string = format!(
            "{}/{} · acc: {} · wpm: {}",
            score.character_matches.to_string(),
            score.character_mismatches.to_string(),
            accuracy,
            wpm,
        );
        let score_text = Text::styled(score_string, Style::default().fg(Yellow));
        let score_paragraph = Paragraph::new(score_text)
            .alignment(Alignment::Right)
            .block(score_block);

        screen_frame.render_widget(score_paragraph, footer_right_corner);
    }
}

fn build_styled_word(
    words_text: &mut Text,
    char_style: Style,
    user_attempt: String,
    expected_word: String,
    is_current_word: bool,
    is_past_word: bool,
) {
    let zipped_chars = expected_word
        .chars()
        .zip(user_attempt.chars())
        .collect::<Vec<_>>();
    let min_len = zipped_chars.len();
    for (expected_char, user_char) in zipped_chars {
        let mut style = char_style;
        if user_char == expected_char {
            style = style.remove_modifier(Modifier::DIM);
            words_text.push_span(Span::styled(expected_char.to_string(), style));
        } else {
            words_text.push_span(Span::styled(expected_char.to_string(), char_style.fg(Red)));
        }
    }

    // Render text we expected the user to type that they didn't type
    let mut missed_char_style = char_style;
    if is_past_word {
        missed_char_style = missed_char_style.fg(Red).add_modifier(Modifier::UNDERLINED);
    }

    let mut missed_chars_iter = expected_word.chars().skip(min_len);
    if let Some(cursor_char) = missed_chars_iter.next() {
        if is_current_word {
            words_text.push_span(Span::styled(
                cursor_char.to_string(),
                char_style.add_modifier(Modifier::UNDERLINED),
            ));
        } else {
            words_text.push_span(Span::styled(cursor_char.to_string(), missed_char_style));
        }
    }

    words_text.push_span(Span::styled(
        missed_chars_iter.collect::<String>(),
        missed_char_style,
    ));

    // Render extra chars that the user typed beyond the length of the word
    let extra_chars_iter = user_attempt.chars().skip(min_len);
    words_text.push_span(Span::styled(
        extra_chars_iter.collect::<String>(),
        char_style.fg(Red).add_modifier(Modifier::CROSSED_OUT),
    ));
}

fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal])
        .flex(Flex::Center)
        .areas(area);
    let [area] = Layout::vertical([vertical]).flex(Flex::Center).areas(area);
    area
}

fn center_vertical(area: Rect, height: u16) -> Rect {
    let [area] = Layout::vertical([Length(height)])
        .flex(Flex::Center)
        .areas(area);
    area
}
