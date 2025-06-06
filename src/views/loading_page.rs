use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph, Widget},
};

pub struct LoadingPage {
}

impl LoadingPage {
    pub fn new() -> Self {
        Self {
        }
    }
}

impl Widget for LoadingPage {

    fn render(self, area: Rect, buf: &mut Buffer) {

        let loading_paragraph = Paragraph::new("Crunching some matmuls...")
            .block(Block::default().title("Loading").borders(Borders::ALL))
            .alignment(Alignment::Center);


        loading_paragraph.render(area, buf);
    }
}
