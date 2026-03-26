use iced::widget::{column, scrollable, text};
use iced::Element;

const MAX_LINES: usize = 500;

#[derive(Debug, Clone)]
pub struct Logger {
    lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum LoggerMessage {
    AddLine(String),
    Clear,
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    pub fn update(&mut self, message: LoggerMessage) {
        match message {
            LoggerMessage::AddLine(line) => {
                self.lines.push(line);
                if self.lines.len() > MAX_LINES {
                    self.lines.remove(0);
                }
            }
            LoggerMessage::Clear => {
                self.lines.clear();
            }
        }
    }

    pub fn view(&self) -> Element<LoggerMessage> {
        let content = column(
            self.lines
                .iter()
                .map(|line| {
                    text(line)
                        .size(12)
                        .style(|_| text::Style {
                            color: Some(iced::Color::from_rgb(0.9, 0.9, 0.9)),
                        })
                        .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(2);

        scrollable(content).height(iced::Length::Fill).into()
    }
}
