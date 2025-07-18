use crate::utils;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    prelude::Buffer,
    style::{Color, Style},
    symbols::scrollbar,
    widgets::{Block, BorderType, Paragraph, Scrollbar, ScrollbarState, StatefulWidget, Widget},
    Frame,
};

#[derive(Debug, PartialEq)]
pub enum ResultAction {
    Handled,
    NotHandled,
    Quit,
}

#[derive(Default, Debug)]
pub struct ResultPageState {
    pub vertical_scroll: u16,
    pub vertical_scroll_state: ScrollbarState,
    pub horizontal_scroll: u16,
    pub horizontal_scroll_state: ScrollbarState,
    pub animation_frame: u16,
}

pub struct ResultView {
    result_text: String,
}

impl ResultView {
    pub fn new(result_text: String) -> Self {
        Self { result_text }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> ResultAction {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => ResultAction::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => ResultAction::Quit,
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Char('k') | KeyCode::Up |
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Char('l') | KeyCode::Right => {
                // Scrolling is handled by updating state
                ResultAction::Handled
            }
            _ => ResultAction::NotHandled,
        }
    }

    pub fn update_scroll(&self, key: KeyEvent, state: &mut ResultPageState) {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                state.vertical_scroll = state.vertical_scroll.saturating_add(1);
                state.vertical_scroll_state = state
                    .vertical_scroll_state
                    .position(state.vertical_scroll as usize);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                state.vertical_scroll = state.vertical_scroll.saturating_sub(1);
                state.vertical_scroll_state = state
                    .vertical_scroll_state
                    .position(state.vertical_scroll as usize);
            }
            KeyCode::Char('h') | KeyCode::Left => {
                state.horizontal_scroll = state.horizontal_scroll.saturating_sub(1);
                state.horizontal_scroll_state = state
                    .horizontal_scroll_state
                    .position(state.horizontal_scroll as usize);
            }
            KeyCode::Char('l') | KeyCode::Right => {
                state.horizontal_scroll = state.horizontal_scroll.saturating_add(1);
                state.horizontal_scroll_state = state
                    .horizontal_scroll_state
                    .position(state.horizontal_scroll as usize);
            }
            _ => {}
        }
    }

    pub fn render(&self, frame: &mut Frame, state: &mut ResultPageState) {
        // Initialize scroll state based on content
        let max_width = self.result_text
            .lines()
            .map(|line| line.len())
            .max()
            .unwrap_or(0);

        let num_lines = self.result_text.lines().count();

        state.vertical_scroll_state = state
            .vertical_scroll_state
            .content_length(num_lines);

        state.horizontal_scroll_state = state.horizontal_scroll_state.content_length(max_width);

        // Increment animation frame on every render
        state.animation_frame = state.animation_frame.wrapping_add(1);

        let layout = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]);
        let [left, right] = layout.areas(frame.size());

        self.render_left(frame.buffer_mut(), left, state);
        self.render_right(frame.buffer_mut(), right, state);

        let vertical_scrollbar =
            Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalLeft)
                .symbols(scrollbar::VERTICAL);

        let horizontal_scrollbar =
            Scrollbar::new(ratatui::widgets::ScrollbarOrientation::HorizontalBottom)
                .symbols(scrollbar::HORIZONTAL);

        vertical_scrollbar.render(
            right.inner(&Margin {
                vertical: 1,
                horizontal: 0,
            }),
            frame.buffer_mut(),
            &mut state.vertical_scroll_state,
        );
        horizontal_scrollbar.render(
            right.inner(&Margin {
                vertical: 0,
                horizontal: 1,
            }),
            frame.buffer_mut(),
            &mut state.horizontal_scroll_state,
        );
    }

    fn render_left(&self, buf: &mut Buffer, left: Rect, state: &mut ResultPageState) {
        let left_block = Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Rgb(255, 165, 0)))
            .title("GPU MODE")
            .title_alignment(Alignment::Center);

        let left_text = Paragraph::new(utils::get_ascii_art_frame(state.animation_frame / 5));

        left_text.block(left_block).render(left, buf);
    }

    fn render_right(&self, buf: &mut Buffer, right: Rect, state: &mut ResultPageState) {
        let right_block = Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Rgb(255, 165, 0)))
            .title_alignment(Alignment::Center)
            .title("Submission Results")
            .title_bottom("Press q to quit...")
            .title_style(Style::default().fg(Color::Magenta));

        let result_text = Paragraph::new(self.result_text.clone())
            .block(right_block)
            .scroll((state.vertical_scroll as u16, state.horizontal_scroll as u16));
        result_text.render(right, buf);
    }
}

#[derive(Default, Debug)]
pub struct ResultPage {
    result_text: Paragraph<'static>,
}

impl ResultPage {
    pub fn new(result_text: String, state: &mut ResultPageState) -> Self {
        let max_width = result_text
            .lines()
            .map(|line| line.len())
            .max()
            .unwrap_or(0);

        let num_lines = result_text.lines().count();

        state.vertical_scroll_state = state
            .vertical_scroll_state
            .content_length(num_lines);

        state.horizontal_scroll_state = state.horizontal_scroll_state.content_length(max_width);
        state.animation_frame = 0;

        Self {
            result_text: Paragraph::new(result_text),
        }
    }

    fn render_left(&self, buf: &mut Buffer, left: Rect, state: &mut ResultPageState) {
        let left_block = Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Rgb(255, 165, 0)))
            .title("GPU MODE")
            .title_alignment(Alignment::Center);

        let left_text = Paragraph::new(utils::get_ascii_art_frame(state.animation_frame / 5));

        left_text.block(left_block).render(left, buf);
    }

    fn render_right(&self, buf: &mut Buffer, right: Rect, state: &mut ResultPageState) {
        let right_block = Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Rgb(255, 165, 0)))
            .title_alignment(Alignment::Center)
            .title("Submission Results")
            .title_bottom("Press q to quit...")
            .title_style(Style::default().fg(Color::Magenta));

        let result_text = self
            .result_text
            .clone()
            .block(right_block)
            .scroll((state.vertical_scroll as u16, state.horizontal_scroll as u16));
        result_text.render(right, buf);
    }

}

impl StatefulWidget for &ResultPage {
    type State = ResultPageState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut ResultPageState) {
        // Increment animation frame on every render
        state.animation_frame = state.animation_frame.wrapping_add(1);

        let layout = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]);
        let [left, right] = layout.areas(area);

        self.render_left(buf, left, state);
        self.render_right(buf, right, state);

        let vertical_scrollbar =
            Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalLeft)
                .symbols(scrollbar::VERTICAL);

        let horizontal_scrollbar =
            Scrollbar::new(ratatui::widgets::ScrollbarOrientation::HorizontalBottom)
                .symbols(scrollbar::HORIZONTAL);

        vertical_scrollbar.render(
            right.inner(&Margin {
                vertical: 1,
                horizontal: 0,
            }),
            buf,
            &mut state.vertical_scroll_state,
        );
        horizontal_scrollbar.render(
            right.inner(&Margin {
                vertical: 0,
                horizontal: 1,
            }),
            buf,
            &mut state.horizontal_scroll_state,
        );
    }
}
