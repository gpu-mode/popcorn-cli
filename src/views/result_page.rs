use crate::utils;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    prelude::Buffer,
    style::{Color, Style},
    widgets::{Block, BorderType, Paragraph, Widget},
};

pub struct ResultPage {
    result_text: Paragraph<'static>,
    pub ack: bool,
}

impl ResultPage {
    pub fn new(result_text: String) -> Self {
        Self {
            result_text: Paragraph::new(result_text),
            ack: false,
        }
    }

    fn render_left(&self, buf: &mut Buffer, left: Rect) {
        let left_block = Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Yellow));

        let left_text = Paragraph::new(utils::get_ascii_art());

        left_text.block(left_block).render(left, buf);
    }

    fn render_right(&self, buf: &mut Buffer, right: Rect) {
        let right_block = Block::bordered()
            .border_type(BorderType::Plain)
            .border_style(Style::default().fg(Color::Yellow))
            .title_bottom("Press q to quit...")
            .title_style(Style::default().fg(Color::Red))
            .title_alignment(Alignment::Right);

        let result_text = self.result_text.clone().block(right_block);
        result_text.render(right, buf);
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('q') {
            self.ack = true;
        }
    }
}

impl Widget for &ResultPage {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::horizontal([Constraint::Percentage(45), Constraint::Percentage(55)]);
        let [left, right] = layout.areas(area);

        self.render_left(buf, left);
        self.render_right(buf, right);
    }
}
