use tui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{StatefulWidget, Widget},
};

pub struct MailTabState {}

impl MailTabState {
    pub fn new() -> Self {
        MailTabState {}
    }
}

pub struct MailTab;

impl StatefulWidget for MailTab {
    type State = MailTabState;

    fn render(self, rect: Rect, buffer: &mut Buffer, state: &mut Self::State) {}
}
