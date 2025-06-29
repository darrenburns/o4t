use crate::app::{App, CurrentWord, CursorType, Screen};
use crate::theme::Theme;
use crate::wrap::{LineComposer, WordWrapper};
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint::Max;
use ratatui::layout::Flex::Center;
use ratatui::layout::{Alignment, Margin, Offset};
use ratatui::prelude::{Line, Widget};
use ratatui::style::{Color, Stylize};
use ratatui::widgets::Clear;
use ratatui::{
    Frame,
    layout::Constraint,
    layout::Constraint::{Length, Min},
    layout::Direction,
    layout::Flex::SpaceBetween,
    layout::Layout,
    layout::Rect,
    style::{Modifier, Style},
    text::{Span, Text},
    widgets::{Block, Padding, Paragraph, Wrap},
};
use std::cmp::max;
use tachyonfx::{EffectRenderer, Shader, ToRgbComponents};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Default, Debug)]
struct ResultData {
    pub value: String,
    pub subtext: String,
    pub theme: Theme,
}

impl Widget for ResultData {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let key_style = Style::default().add_modifier(Modifier::DIM);
        let value_style = Style::default()
            .fg(self.theme.primary)
            .add_modifier(Modifier::BOLD);
        let text = Text::from(vec![
            Line::styled(self.value, value_style),
            Line::styled(self.subtext, key_style),
        ]);
        text.render(area, buf);
    }
}

pub fn ui(screen_frame: &mut Frame, app: &mut App) {
    let current_theme = app.get_current_theme();
    Clear.render(screen_frame.area(), screen_frame.buffer_mut());
    Block::default()
        .fg(current_theme.fg)
        .bg(current_theme.bg)
        .render(screen_frame.area(), screen_frame.buffer_mut());
    match app.current_screen {
        Screen::Game => build_game_screen(screen_frame, app),
        Screen::Results => build_score_screen(screen_frame, app),
    }
}

fn build_game_screen(screen_frame: &mut Frame, app: &mut App) {
    let current_theme = app.get_current_theme();
    let screen_sections = Layout::default()
        .horizontal_margin(3)
        .vertical_margin(1)
        .direction(Direction::Vertical)
        .constraints([
            Length(1), // Header
            Min(7),    // Body (timer row, plus text)
            Length(1), // Footer
        ])
        .split(screen_frame.area());

    // Header (actual render call is at bottom since we may need to include debug info).
    if app.is_debug_mode {
        let debug_text = Line::from(vec![Span::raw("debug: "), Span::raw(&app.debug_string)]);
        let header = Paragraph::new(debug_text).bg(current_theme.bg);
        screen_frame.render_widget(header, screen_sections[0]);
    } else {
        let header = build_header(app);
        screen_frame.render_widget(header, screen_sections[0]);
    }

    // Body text containing the words to show the user that they must type.
    let words = app
        .words
        .iter()
        .map(|word_attempt| word_attempt.word.clone())
        .collect::<Vec<_>>();

    let mut words_text = Text::default();
    let mut cursor_offset = 0;
    let mut expected_char_offset = 0;

    for (index, word) in words.iter().enumerate() {
        let mut char_style = Style::default().fg(current_theme.fg);
        let user_attempt = &app.words[index].user_attempt;

        let expected_word_num_graphemes = word.graphemes(false).count();

        // Compute the cursor offset
        if index < app.current_word_offset {
            cursor_offset += max(
                app.words[index].user_attempt.graphemes(false).count(),
                expected_word_num_graphemes,
            );
        } else if index == app.current_word_offset {
            let user_input_num_graphemes = app.current_user_input.graphemes(false).count();
            cursor_offset += user_input_num_graphemes;
        }

        // The ghost offset should ignore current user input and user attempts, and only look
        // at the words the user is expected to type. +1 for space.
        expected_char_offset += expected_word_num_graphemes + 1;
        let mut ghost_cursor_word_offset = None;
        match app.ghost_offset {
            None => {}
            Some(app_ghost_offset) => {
                if expected_char_offset >= (app_ghost_offset as usize)
                    && app_ghost_offset as usize
                        >= expected_char_offset - expected_word_num_graphemes
                {
                    // The ghost cursor is within this word.
                    let offset_from_end_of_word =
                        expected_char_offset - (app_ghost_offset as usize);
                    ghost_cursor_word_offset =
                        Some(expected_word_num_graphemes.saturating_sub(offset_from_end_of_word));
                    app.debug_string = format!(
                        "{:.2}, {:.2}",
                        app.ghost_offset.unwrap_or(0.0),
                        ghost_cursor_word_offset.unwrap_or(0)
                    );
                }
            }
        }

        if app.current_word_offset == index {
            // Check which characters match and which ones don't in order to build up the styling for this word.
            build_styled_word(
                app,
                &mut words_text,
                char_style,
                app.current_user_input.to_string(),
                word.to_string(),
                true,
                false,
                ghost_cursor_word_offset,
            );
            if app.current_user_input.len() >= word.len() {
                words_text.push_span(Span::styled(
                    " ",
                    Style::default().patch(cursor_type_to_ratatui_style(&app.cursor_style, app)),
                ))
            } else {
                let mut space = Span::default().content(" ");
                if let Some(ghost_cursor_word_offset) = ghost_cursor_word_offset {
                    if ghost_cursor_word_offset == word.len() {
                        space = space.bg(current_theme.ghost_cursor_color())
                    }
                }
                words_text.push_span(space);
            }
        } else if user_attempt.is_empty() {
            // It's not the current word, and there's no attempt yet, basic rendering.
            // Isolate the ghost cursor character
            if let Some(ghost_cursor_word_offset) = ghost_cursor_word_offset {
                let ghost_cursor_char = word.chars().nth(ghost_cursor_word_offset);
                let (before, after) = word.split_at(ghost_cursor_word_offset);

                words_text.push_span(Span::styled(
                    before,
                    char_style.patch(current_theme.character_upcoming),
                ));
                ghost_cursor_char.map(|ghost_cursor_char| {
                    words_text.push_span(Span::styled(
                        ghost_cursor_char.to_string(),
                        char_style
                            .patch(current_theme.character_upcoming)
                            .bg(current_theme.ghost_cursor_color()),
                    ));
                });
                words_text.push_span(Span::styled(
                    after.chars().skip(1).collect::<String>(),
                    char_style.patch(current_theme.character_upcoming),
                ));
            } else {
                let current_word_span =
                    Span::styled(word, char_style.patch(current_theme.character_upcoming));
                words_text.push_span(current_word_span);
            }
            let mut space = Span::default().content(" ");
            if index != words.len() - 1 {
                if let Some(ghost_cursor_word_offset) = ghost_cursor_word_offset {
                    if ghost_cursor_word_offset == word.len() {
                        space = space.bg(current_theme.ghost_cursor_color())
                    }
                }
                words_text.push_span(space);
            }
        } else {
            // It's not the current word, but we have attempted it - render the word attempt.
            build_styled_word(
                app,
                &mut words_text,
                char_style,
                user_attempt.to_string(),
                word.to_string(),
                false,
                true,
                ghost_cursor_word_offset,
            );
            if index != words.len() - 1 {
                let mut space = Span::default().content(" ");
                if let Some(ghost_cursor_word_offset) = ghost_cursor_word_offset {
                    if ghost_cursor_word_offset == word.len() {
                        space = space.bg(current_theme.ghost_cursor_color())
                    }
                }
                words_text.push_span(space);
            }
        }
    }

    // The body has 2 rows - a single cell height row for the timer, and 5 rows for the text to type
    let centered_body = center(
        screen_sections[1],
        Length(screen_frame.area().width),
        Length(7), // 1 timer row, 6 lines of text
    );

    // Split the centered rows into space for the timer and space for the challenge words.
    let [timer_section, words_section] =
        Layout::vertical([Length(1), Min(5)]).areas::<2>(centered_body);

    // Horizontal padding for the centered content (timer + challenge words).
    let h_pad = 8;

    // The game timer - shows as dim until the game starts.
    let game_time_remaining_secs = app.game_time_remaining_millis().div_ceil(1000);
    let mut timer_style = Style::default()
        .fg(current_theme.primary)
        .add_modifier(Modifier::DIM);

    if app.game_active {
        timer_style = timer_style
            .add_modifier(Modifier::BOLD)
            .remove_modifier(Modifier::DIM);
    }

    // When the game is almost over, we underline the timer.
    if game_time_remaining_secs <= 3 {
        timer_style = timer_style.add_modifier(Modifier::UNDERLINED);
    }

    let game_timer = Paragraph::new(Text::styled(
        game_time_remaining_secs.to_string(),
        timer_style,
    ))
    .bg(current_theme.bg)
    .block(Block::default().padding(Padding::horizontal(h_pad)));
    screen_frame.render_widget(game_timer, timer_section);

    let styled = &words_text.iter().map(|line| {
        let graphemes = line
            .spans
            .iter()
            .flat_map(|span| span.styled_graphemes(span.style));
        let alignment = line.alignment.unwrap_or(Alignment::Left);
        (graphemes, alignment)
    });

    let text_render_area_width = screen_sections[1].inner(Margin::new(h_pad, 0)).width;
    let mut wrapper = WordWrapper::new(styled.clone().into_iter(), text_render_area_width, false);

    // Continuously sum the widths until we get to the cursor offset.
    // At that point we know we're at the cursor char, and can check the line number
    // from there.
    let (mut row, mut offset_from_start_of_text) = (0, 0);
    let mut cursor_row = 0;
    let mut cursor_found = false;
    let mut wrapped_lines = vec![];
    let mut line_alpha = 1.0;
    while let Some(wrapped_line) = wrapper.next_line() {
        let line_symbols = wrapped_line
            .line
            .iter()
            .map(|grapheme| {
                Span::styled(
                    grapheme.symbol,
                    grapheme
                        .style
                        .patch(grapheme.style.fg.map_or(current_theme.fg, |fg| {
                            if current_theme.supports_alpha {
                                blend_colors(fg, current_theme.bg, line_alpha)
                            } else {
                                fg
                            }
                        })),
                )
            })
            .collect::<Line>();

        wrapped_lines.push(line_symbols);
        for grapheme in wrapped_line.line {
            if grapheme.symbol != " " {
                offset_from_start_of_text += grapheme.symbol.width();
                if offset_from_start_of_text > cursor_offset && !cursor_found {
                    cursor_row = row;
                    cursor_found = true;
                }
            }
        }

        // Start dimming towards the bottom
        if cursor_found && row > cursor_row && row > 2 {
            line_alpha -= 0.42;
        }
        row += 1;
    }

    let mut words_paragraph = Paragraph::new(Text::from(wrapped_lines))
        .wrap(Wrap { trim: false })
        .block(Block::default().padding(Padding::horizontal(h_pad)));

    if cursor_row > 2 {
        words_paragraph = words_paragraph.scroll((cursor_row - 2, 0));
    }

    screen_frame.render_widget(words_paragraph, words_section);

    let launch_effect = &mut app.load_words_effect;
    if launch_effect.running() {
        screen_frame.render_effect(launch_effect, words_section, app.last_tick_duration.into());
    }

    // Footer
    build_footer(screen_frame, screen_sections[2], app, true, true);
}

fn build_header(app: &App) -> Paragraph<'static> {
    let current_theme = app.get_current_theme();
    let header_block = Block::default()
        .padding(Padding::horizontal(1))
        .bg(current_theme.bg);
    let mut title_text = Line::styled(
        "o4t ",
        Style::default()
            .fg(current_theme.primary)
            .add_modifier(Modifier::BOLD),
    );
    title_text += Span::styled(
        env!("CARGO_PKG_VERSION"),
        Style::default()
            .fg(current_theme.fg)
            .add_modifier(Modifier::DIM)
            .remove_modifier(Modifier::BOLD),
    );
    let title = Paragraph::new(title_text).block(header_block);
    title
}

fn build_score_screen(screen_frame: &mut Frame, app: &mut App) {
    let current_theme = app.get_current_theme();
    let [header_rect, body_rect, footer_rect] = Layout::default()
        .horizontal_margin(3)
        .vertical_margin(1)
        .direction(Direction::Vertical)
        .constraints([
            Length(1), // Header
            Min(2),    // Body
            Length(1), // Footer
        ])
        .areas(screen_frame.area());

    screen_frame.render_widget(build_header(app), header_rect);

    // Score screen body
    let score = &app.score;
    let score_data = vec![
        ResultData {
            theme: current_theme.clone(),
            value: format!("{:.0} ", score.wpm),
            subtext: "wpm".to_string(),
        },
        ResultData {
            theme: current_theme.clone(),
            value: format!("{:.0}%", score.accuracy * 100.),
            subtext: "accuracy".to_string(),
        },
        ResultData {
            theme: current_theme.clone(),
            value: score.character_hits.to_string(),
            subtext: "hits".to_string(),
        },
        ResultData {
            theme: current_theme.clone(),
            value: score.character_misses.to_string(),
            subtext: "misses".to_string(),
        },
        ResultData {
            theme: current_theme.clone(),
            value: score.best_char_streak.to_string(),
            subtext: "streak".to_string(),
        },
        ResultData {
            theme: current_theme.clone(),
            value: score.num_words.to_string(),
            subtext: "words".to_string(),
        },
    ];
    let col_constraints = (0..3).map(|_| Length(10));
    let mut row_constraints = (0..2).map(|_| Length(3)).collect::<Vec<_>>();
    let is_perfect_score = app.score.is_perfect();
    if is_perfect_score {
        row_constraints.insert(0, Length(1));
    }

    let horizontal = Layout::horizontal(col_constraints).spacing(1);
    let vertical = Layout::vertical(row_constraints)
        .flex(Center)
        .spacing(1)
        .horizontal_margin(1);

    let rows = vertical.split(body_rect);
    // If the score is perfect, then we've added an extra constraint to insert "PERFECT" text,
    // so skip that as it's not one of the "table cells" we'll insert our data into.
    let num_skips = if is_perfect_score { 1 } else { 0 };
    let cells = rows
        .iter()
        .skip(num_skips)
        .flat_map(|&row| horizontal.split(row).to_vec())
        .collect::<Vec<_>>();

    if is_perfect_score {
        let perfect_score_section = rows.iter().next().unwrap();
        screen_frame.render_widget(
            Line::styled(
                "Perfect!",
                Style::default().fg(current_theme.secondary).italic(),
            ),
            *perfect_score_section,
        );
    }
    for (score_data, cell_area) in score_data.into_iter().zip(cells) {
        screen_frame.render_widget(score_data, cell_area);
    }

    let load_effect = &mut app.load_results_screen_effect;
    if load_effect.running() {
        screen_frame.render_effect(load_effect, body_rect, app.last_tick_duration.into());
    }
    build_footer(screen_frame, footer_rect, app, false, true);
}

fn build_footer(
    screen_frame: &mut Frame,
    rect: Rect,
    app: &mut App,
    show_scoring: bool,
    show_reset: bool,
) {
    let current_theme = app.get_current_theme();
    let score_constraint = if show_scoring { Min(10) } else { Max(0) };
    let footer_sections: [Rect; 2] = Layout::horizontal([Constraint::Fill(1), score_constraint])
        .flex(SpaceBetween)
        .areas(rect);

    let keys_block = Block::default()
        .padding(Padding::left(1))
        .fg(current_theme.primary)
        .bg(current_theme.bg);

    let key_style = Style::default().add_modifier(Modifier::BOLD);
    let value_style = Style::default()
        .fg(current_theme.fg)
        .add_modifier(Modifier::DIM);
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
        let score_block = Block::default()
            .padding(Padding::right(1))
            .fg(current_theme.primary)
            .bg(current_theme.bg);

        let accuracy = if app.game_active && !score.accuracy.is_nan() {
            format!("{:.0}%", score.accuracy * 100.0)
        } else {
            empty_score_placeholder.to_string()
        };
        let wpm = if app.game_active && !score.wpm.is_nan() && score.wpm != 0.0 {
            format!("{:.0}", score.wpm)
        } else {
            empty_score_placeholder.to_string()
        };

        let score_text = Line::from(vec![
            Span::raw("acc "),
            Span::raw(accuracy).fg(current_theme.fg).dim(),
            Span::raw("  ").fg(current_theme.fg).dim(),
            Span::raw("wpm "),
            Span::raw(wpm).fg(current_theme.fg).dim(),
        ]);
        let score_text = Text::from(score_text);
        let score_paragraph = Paragraph::new(score_text)
            .alignment(Alignment::Right)
            .block(score_block);

        screen_frame.render_widget(score_paragraph, footer_right_corner);
    }
}

fn build_styled_word(
    app: &App,
    words_text: &mut Text,
    char_style: Style,
    user_attempt: String,
    expected_word: String,
    is_current_word: bool,
    is_past_word: bool,
    ghost_cursor_offset: Option<usize>,
) {
    let current_theme = app.get_current_theme();
    let mut offset_in_word = 0;
    let zipped_chars = expected_word
        .chars()
        .zip(user_attempt.chars())
        .collect::<Vec<_>>();
    let min_len = zipped_chars.len();

    let mut char_style = char_style;

    if is_current_word {
        match app.config.current_word {
            CurrentWord::Bold => char_style = char_style.add_modifier(Modifier::BOLD),
            CurrentWord::Highlight if current_theme.supports_alpha => {
                char_style = char_style.bg(blend_colors(
                    current_theme.character_upcoming,
                    current_theme.bg,
                    0.08,
                ))
            }
            _ => {}
        }
    }

    for (expected_char, user_char) in zipped_chars {
        let mut style = char_style;
        let mut span;
        if user_char == expected_char {
            style = style.patch(current_theme.character_match);
            span = Span::styled(expected_char.to_string(), style);
        } else {
            span = Span::styled(
                expected_char.to_string(),
                char_style.patch(current_theme.character_mismatch),
            );
        }

        match ghost_cursor_offset {
            Some(ghost_cursor_offset) => {
                if ghost_cursor_offset == offset_in_word {
                    span = span.bg(current_theme.ghost_cursor_color());
                }
            }
            None => {}
        }

        words_text.push_span(span);
        offset_in_word += 1;
    }

    let current_theme = app.get_current_theme();

    // Render text we expected the user to type that they didn't type at all
    // (not that they typed incorrectly - this is for when you press space too
    // early, before getting to the end of a word).
    let mut missed_char_style = char_style;
    if is_past_word {
        missed_char_style = missed_char_style
            .patch(current_theme.character_mismatch)
            .add_modifier(Modifier::UNDERLINED);
    }

    let mut missed_chars_iter = expected_word.chars().skip(min_len);
    if let Some(cursor_char) = missed_chars_iter.next() {
        if is_current_word {
            let upcoming_chars_in_this_word = Span::styled(
                cursor_char.to_string(),
                char_style.patch(cursor_type_to_ratatui_style(&app.cursor_style, app)),
            );
            words_text.push_span(upcoming_chars_in_this_word);
        } else {
            let missed_chars_span = Span::styled(cursor_char.to_string(), missed_char_style);
            words_text.push_span(missed_chars_span);
        }
    }

    let is_upcoming = !is_current_word && !is_past_word;
    if is_upcoming {
        missed_char_style = missed_char_style.patch(current_theme.character_upcoming)
    }

    for (idx, missed_char) in missed_chars_iter.enumerate() {
        let mut char_style = missed_char_style;
        if let Some(ghost_cursor_offset) = ghost_cursor_offset {
            if ghost_cursor_offset == min_len + idx + 1 {
                char_style = char_style.bg(current_theme.ghost_cursor_color());
            }
        }
        words_text.push_span(Span::styled(missed_char.to_string(), char_style));
    }

    // Render extra chars that the user typed beyond the length of the word
    let extra_chars_iter = user_attempt.chars().skip(min_len);
    let extra_chars_span = Span::styled(
        extra_chars_iter.collect::<String>(),
        char_style
            .patch(current_theme.character_mismatch)
            .add_modifier(Modifier::CROSSED_OUT),
    );
    words_text.push_span(extra_chars_span);
}

fn center(area: Rect, horizontal: Constraint, vertical: Constraint) -> Rect {
    let [area] = Layout::horizontal([horizontal]).flex(Center).areas(area);
    let [area] = Layout::vertical([vertical])
        .flex(Center)
        .areas(area)
        .map(|rect| rect.offset(Offset { x: 0, y: -1 }));
    area
}

fn cursor_type_to_ratatui_style(cursor_style: &CursorType, app: &App) -> Style {
    let current_theme = app.get_current_theme();
    match cursor_style {
        CursorType::Block => Style::default()
            .fg(current_theme.bg)
            .bg(current_theme.secondary),
        CursorType::Underline => Style::default()
            .underlined()
            .underline_color(current_theme.secondary),
        CursorType::None => Style::default(),
    }
}

/// Blends a foreground color over a background color using a given alpha.
///
/// - `fg`: The foreground color.
/// - `bg`: The background color.
/// - `alpha`: The opacity of the foreground color, from 0.0 (fully transparent)
///   to 1.0 (fully opaque).
///
/// Returns the resulting blended `Color`.
pub fn blend_colors(fg: Color, bg: Color, alpha: f32) -> Color {
    let fg_rgb = fg.to_rgb();
    let bg_rgb = bg.to_rgb();

    // Clamp alpha to the valid range [0.0, 1.0] to prevent invalid calculations.
    let alpha = alpha.clamp(0.0, 1.0);
    let beta = 1.0 - alpha; // The inverse of alpha, for the background.

    // Perform alpha blending for each channel.
    // The formula is: output = (foreground * alpha) + (background * (1.0 - alpha))
    let r = (fg_rgb.0 as f32 * alpha + bg_rgb.0 as f32 * beta).round() as u8;
    let g = (fg_rgb.1 as f32 * alpha + bg_rgb.1 as f32 * beta).round() as u8;
    let b = (fg_rgb.2 as f32 * alpha + bg_rgb.2 as f32 * beta).round() as u8;

    Color::Rgb(r, g, b)
}
