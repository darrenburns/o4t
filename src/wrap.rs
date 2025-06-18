use ratatui::layout::Alignment;
use ratatui::text::StyledGrapheme;
use std::{collections::VecDeque, mem};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;


const NBSP: &str = "\u{00a0}";
const ZWSP: &str = "\u{200b}";


/// A state machine to pack styled symbols into lines.
/// Cannot implement it as Iterator since it yields slices of the internal buffer (need streaming
/// iterators for that).
pub trait LineComposer<'a> {
    fn next_line<'lend>(&'lend mut self) -> Option<WrappedLine<'lend, 'a>>;
}

pub struct WrappedLine<'lend, 'text> {
    /// One line reflowed to the correct width
    pub line: &'lend [StyledGrapheme<'text>],
    /// The width of the line
    pub width: u16,
    /// Whether the line was aligned left or right
    pub alignment: Alignment,
}

/// A state machine that wraps lines on word boundaries.
#[derive(Debug, Default, Clone)]
pub struct WordWrapper<'a, O, I>
where
// Outer iterator providing the individual lines
    O: Iterator<Item = (I, Alignment)>,
// Inner iterator providing the styled symbols of a line Each line consists of an alignment and
// a series of symbols
    I: Iterator<Item = StyledGrapheme<'a>>,
{
    /// The given, unprocessed lines
    input_lines: O,
    max_line_width: u16,
    wrapped_lines: VecDeque<Vec<StyledGrapheme<'a>>>,
    current_alignment: Alignment,
    current_line: Vec<StyledGrapheme<'a>>,
    /// Removes the leading whitespace from lines
    trim: bool,

    // These are cached allocations that hold no state across next_line invocations
    pending_word: Vec<StyledGrapheme<'a>>,
    pending_whitespace: VecDeque<StyledGrapheme<'a>>,
    pending_line_pool: Vec<Vec<StyledGrapheme<'a>>>,
}

impl<'a, O, I> WordWrapper<'a, O, I>
where
    O: Iterator<Item = (I, Alignment)>,
    I: Iterator<Item = StyledGrapheme<'a>>,
{
    pub const fn new(lines: O, max_line_width: u16, trim: bool) -> Self {
        Self {
            input_lines: lines,
            max_line_width,
            wrapped_lines: VecDeque::new(),
            current_alignment: Alignment::Left,
            current_line: vec![],
            trim,

            pending_word: Vec::new(),
            pending_line_pool: Vec::new(),
            pending_whitespace: VecDeque::new(),
        }
    }
    
    fn is_whitespace_grapheme(&self, grapheme: &StyledGrapheme) -> bool {
        let symbol = grapheme.symbol;
        symbol == ZWSP || symbol.chars().all(char::is_whitespace) && symbol != NBSP
    }

    /// Split an input line (`line_symbols`) into wrapped lines
    /// and cache them to be emitted later
    fn process_input(&mut self, line_symbols: impl IntoIterator<Item = StyledGrapheme<'a>>) {
        let mut pending_line = self.pending_line_pool.pop().unwrap_or_default();
        let mut line_width = 0;
        let mut word_width = 0;
        let mut whitespace_width = 0;
        let mut non_whitespace_previous = false;

        self.pending_word.clear();
        self.pending_whitespace.clear();
        pending_line.clear();

        for grapheme in line_symbols {
            let is_whitespace = self.is_whitespace_grapheme(&grapheme);
            let symbol_width = grapheme.symbol.width() as u16;

            // ignore symbols wider than line limit
            if symbol_width > self.max_line_width {
                continue;
            }

            let word_found = non_whitespace_previous && is_whitespace;
            // current word would overflow after removing whitespace
            let trimmed_overflow = pending_line.is_empty()
                && self.trim
                && word_width + symbol_width > self.max_line_width;
            // separated whitespace would overflow on its own
            let whitespace_overflow = pending_line.is_empty()
                && self.trim
                && whitespace_width + symbol_width > self.max_line_width;
            // current full word (including whitespace) would overflow
            let untrimmed_overflow = pending_line.is_empty()
                && !self.trim
                && word_width + whitespace_width + symbol_width > self.max_line_width;

            // append finished segment to current line
            if word_found || trimmed_overflow || whitespace_overflow || untrimmed_overflow {
                if !pending_line.is_empty() || !self.trim {
                    pending_line.extend(self.pending_whitespace.drain(..));
                    line_width += whitespace_width;
                }

                pending_line.append(&mut self.pending_word);
                line_width += word_width;

                self.pending_whitespace.clear();
                whitespace_width = 0;
                word_width = 0;
            }

            // pending line fills up limit
            let line_full = line_width >= self.max_line_width;
            // pending word would overflow line limit
            let pending_word_overflow = symbol_width > 0
                && line_width + whitespace_width + word_width >= self.max_line_width;

            // add finished wrapped line to remaining lines
            if line_full || pending_word_overflow {
                let mut remaining_width = u16::saturating_sub(self.max_line_width, line_width);

                self.wrapped_lines.push_back(mem::take(&mut pending_line));
                line_width = 0;

                // remove whitespace up to the end of line
                while let Some(grapheme) = self.pending_whitespace.front() {
                    let width = grapheme.symbol.width() as u16;

                    if width > remaining_width {
                        break;
                    }

                    whitespace_width -= width;
                    remaining_width -= width;
                    self.pending_whitespace.pop_front();
                }

                // don't count first whitespace toward next word
                if is_whitespace && self.pending_whitespace.is_empty() {
                    continue;
                }
            }

            // append symbol to a pending buffer
            if is_whitespace {
                whitespace_width += symbol_width;
                self.pending_whitespace.push_back(grapheme);
            } else {
                word_width += symbol_width;
                self.pending_word.push(grapheme);
            }

            non_whitespace_previous = !is_whitespace;
        }

        // append remaining text parts
        if pending_line.is_empty()
            && self.pending_word.is_empty()
            && !self.pending_whitespace.is_empty()
        {
            self.wrapped_lines.push_back(vec![]);
        }
        if !pending_line.is_empty() || !self.trim {
            pending_line.extend(self.pending_whitespace.drain(..));
        }
        pending_line.append(&mut self.pending_word);

        #[allow(clippy::else_if_without_else)]
        if !pending_line.is_empty() {
            self.wrapped_lines.push_back(pending_line);
        } else if pending_line.capacity() > 0 {
            self.pending_line_pool.push(pending_line);
        }
        if self.wrapped_lines.is_empty() {
            self.wrapped_lines.push_back(vec![]);
        }
    }

    fn replace_current_line(&mut self, line: Vec<StyledGrapheme<'a>>) {
        let cache = mem::replace(&mut self.current_line, line);
        if cache.capacity() > 0 {
            self.pending_line_pool.push(cache);
        }
    }
}

impl<'a, O, I> LineComposer<'a> for WordWrapper<'a, O, I>
where
    O: Iterator<Item = (I, Alignment)>,
    I: Iterator<Item = StyledGrapheme<'a>>,
{
    #[allow(clippy::too_many_lines)]
    fn next_line<'lend>(&'lend mut self) -> Option<WrappedLine<'lend, 'a>> {
        if self.max_line_width == 0 {
            return None;
        }

        loop {
            // emit next cached line if present
            if let Some(line) = self.wrapped_lines.pop_front() {
                let line_width = line
                    .iter()
                    .map(|grapheme| grapheme.symbol.width() as u16)
                    .sum();

                self.replace_current_line(line);
                return Some(WrappedLine {
                    line: &self.current_line,
                    width: line_width,
                    alignment: self.current_alignment,
                });
            }

            // otherwise, process pending wrapped lines from input
            let (line_symbols, line_alignment) = self.input_lines.next()?;
            self.current_alignment = line_alignment;
            self.process_input(line_symbols);
        }
    }
}

/// A state machine that truncates overhanging lines.
#[derive(Debug, Default, Clone)]
pub struct LineTruncator<'a, O, I>
where
// Outer iterator providing the individual lines
    O: Iterator<Item = (I, Alignment)>,
// Inner iterator providing the styled symbols of a line Each line consists of an alignment and
// a series of symbols
    I: Iterator<Item = StyledGrapheme<'a>>,
{
    /// The given, unprocessed lines
    input_lines: O,
    max_line_width: u16,
    current_line: Vec<StyledGrapheme<'a>>,
    /// Record the offset to skip render
    horizontal_offset: u16,
}

impl<'a, O, I> LineTruncator<'a, O, I>
where
    O: Iterator<Item = (I, Alignment)>,
    I: Iterator<Item = StyledGrapheme<'a>>,
{
    pub const fn new(lines: O, max_line_width: u16) -> Self {
        Self {
            input_lines: lines,
            max_line_width,
            horizontal_offset: 0,
            current_line: vec![],
        }
    }

    pub fn set_horizontal_offset(&mut self, horizontal_offset: u16) {
        self.horizontal_offset = horizontal_offset;
    }
}

impl<'a, O, I> LineComposer<'a> for LineTruncator<'a, O, I>
where
    O: Iterator<Item = (I, Alignment)>,
    I: Iterator<Item = StyledGrapheme<'a>>,
{
    fn next_line<'lend>(&'lend mut self) -> Option<WrappedLine<'lend, 'a>> {
        if self.max_line_width == 0 {
            return None;
        }

        self.current_line.truncate(0);
        let mut current_line_width = 0;

        let mut lines_exhausted = true;
        let mut horizontal_offset = self.horizontal_offset as usize;
        let mut current_alignment = Alignment::Left;
        if let Some((current_line, alignment)) = &mut self.input_lines.next() {
            lines_exhausted = false;
            current_alignment = *alignment;

            for StyledGrapheme { symbol, style } in current_line {
                // Ignore characters wider that the total max width.
                if symbol.width() as u16 > self.max_line_width {
                    continue;
                }

                if current_line_width + symbol.width() as u16 > self.max_line_width {
                    // Truncate line
                    break;
                }

                let symbol = if horizontal_offset == 0 || Alignment::Left != *alignment {
                    symbol
                } else {
                    let w = symbol.width();
                    if w > horizontal_offset {
                        let t = trim_offset(symbol, horizontal_offset);
                        horizontal_offset = 0;
                        t
                    } else {
                        horizontal_offset -= w;
                        ""
                    }
                };
                current_line_width += symbol.width() as u16;
                self.current_line.push(StyledGrapheme { symbol, style });
            }
        }

        if lines_exhausted {
            None
        } else {
            Some(WrappedLine {
                line: &self.current_line,
                width: current_line_width,
                alignment: current_alignment,
            })
        }
    }
}

/// This function will return a str slice which start at specified offset.
/// As src is a unicode str, start offset has to be calculated with each character.
fn trim_offset(src: &str, mut offset: usize) -> &str {
    let mut start = 0;
    for c in UnicodeSegmentation::graphemes(src, true) {
        let w = c.width();
        if w <= offset {
            offset -= w;
            start += c.len();
        } else {
            break;
        }
    }
    #[allow(clippy::string_slice)] // Is safe as it comes from UnicodeSegmentation
    &src[start..]
}
