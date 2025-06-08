use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Layout, Rect},
    style::{Color, Stylize},
    widgets::{Block, Gauge, Padding, Paragraph, StatefulWidget, Widget},
};

#[derive(Debug, Default, Clone)]
pub struct LoadingPageState {
    pub loop_count: u16,
    pub progress_column: u16,
    pub progress_bar: f64,
}

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct LoadingPage {
    header_area: Rect,
    gauge_area: Rect,
    footer_area: Rect,
}

const GAUGE_COLOR: Color = ratatui::style::palette::tailwind::RED.c800;

impl StatefulWidget for &LoadingPage {
    type State = LoadingPageState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        use ratatui::layout::Constraint::Percentage;

        let layout = Layout::vertical([Percentage(45), Percentage(10), Percentage(45)]);

        let [_, gauge_area, footer_area] = layout.areas(area);

        render_gauge(gauge_area, buf, state);
        render_footer(footer_area, buf, state);
    }
}

fn render_gauge(area: Rect, buf: &mut Buffer, state: &mut LoadingPageState) {
    let blk = Block::default().padding(Padding::horizontal(20));
    Gauge::default()
        .block(blk)
        .gauge_style(GAUGE_COLOR)
        .ratio(state.progress_bar / 100.0)
        .render(area, buf);
}

fn get_footer_text(state: &LoadingPageState) -> String {
    let percentage = state.progress_bar;

    if state.loop_count > 0 {
        return "Did you know we have zero idea how long this will take?".to_string();
    }

    if percentage > 75.0 {
        return "Almost there!".to_string();
    } else if percentage > 35.0 {
        return "Crunching numbers...".to_string();
    } else {
        return "This is taking a while, huh?".to_string();
    }
}

fn render_footer(area: Rect, buf: &mut Buffer, state: &LoadingPageState) {
    let blk = Block::default().padding(Padding::vertical(1));
    let text = Paragraph::new(get_footer_text(state))
        .alignment(Alignment::Center)
        .fg(Color::White)
        .bold()
        .block(blk);

    text.render(area, buf);
}
