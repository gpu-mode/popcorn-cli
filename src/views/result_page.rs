use crate::utils;
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Paragraph},
    Frame,
};

pub struct ResultPage {
    result_text: Paragraph<'static>,
}

impl ResultPage {
    pub fn new(result_text: String) -> Self {
        Self {
            result_text: Paragraph::new(result_text),
        }
    }

    fn render_left(&self, frame: &mut Frame, left: Rect) {
        let left_block = Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Yellow));

        let left_text = Paragraph::new(utils::get_ascii_art());

        frame.render_widget(left_text.block(left_block), left);
    }

    fn render_right(&self, frame: &mut Frame, right: Rect) {
        let right_block = Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Yellow))
            .title_bottom("Press q to quit...")
            .title_style(Style::default().fg(Color::Red))
            .title_alignment(Alignment::Right);

        frame.render_widget(self.result_text.clone().block(right_block), right);
    }

    pub fn render(&self, frame: &mut Frame) {
        let layout = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]);
        let [left, right] = layout.areas(frame.size());

        self.render_left(frame, left);
        self.render_right(frame, right);
    }
}
