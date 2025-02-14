use iced::{
    widget::{column, container, horizontal_rule, row, scrollable, text, Column, Space},
    Alignment, Application, Color, Command, Element, Length, Settings, Subscription, Theme
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::process::Command as StdCommand;
use std::time::Duration;

const HEADER_COLOR: Color = Color::from_rgb(0.53, 0.81, 0.92);
const TEXT_COLOR: Color = Color::from_rgb(0.9, 0.9, 0.9);
const BACKGROUND_COLOR: Color = Color::from_rgb(0.1, 0.1, 0.1);
const ROW_ALT_COLOR: Color = Color::from_rgb(0.15, 0.15, 0.15);
const ERROR_COLOR: Color = Color::from_rgb(0.8, 0.2, 0.2);

// Font sizes (converted to u16)-(Also remember to add Body)
const HEADER_FONT_SIZE: u16 = 18;

fn main() -> iced::Result {
    SensorViewer::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(700.0, 900.0), // Use iced::Size::new for the window size
            resizable: true,  // You can toggle whether the window should be resizable
            ..Default::default()
        },
        ..Default::default()
    })
}

#[derive(Debug, Clone)]
enum Message {
    Refresh,
}

struct SensorViewer {
    sensor_data: Result<Vec<SensorSection>, String>,
}

#[derive(Debug, Clone)]
struct SensorSection {
    name: String,
    adapter: String,
    entries: Vec<SensorEntry>,
}

#[derive(Debug, Clone)]
struct SensorEntry {
    key: String,
    value: String,
    additional_info: Option<String>,
}

impl Application for SensorViewer {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        (
            SensorViewer {
                sensor_data: read_sensor_data(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Sensory")
    }

    fn update(&mut self, message: Message) -> Command<Self::Message> {
        match message {
            Message::Refresh => {
                self.sensor_data = read_sensor_data();
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let content = match &self.sensor_data {
            Ok(data) => Column::with_children(
                data.iter()
                    .map(|section| sensor_section(section))
                    .collect::<Vec<_>>(),
            )
            .spacing(20),
            Err(e) => column![
                horizontal_rule(1).style(iced::theme::Rule::Custom(Box::new(error_rule_style))),
                text(format!("Error: {}", e))
                    .size(HEADER_FONT_SIZE)
                    .style(ERROR_COLOR)
            ]
            .spacing(10),
        };

        container(scrollable(
            column![content.spacing(20)]
                .spacing(20)
                .padding(20),
        ))
        .style(iced::theme::Container::Custom(Box::new(AppContainerStyle)))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        iced::time::every(Duration::from_millis(500)).map(|_| Message::Refresh)
    }
}

fn sensor_section(section: &SensorSection) -> Element<'static, Message> {
    let header = row![
        text(&section.name)
            .size(HEADER_FONT_SIZE)
            .style(HEADER_COLOR),
        Space::with_width(Length::Fill),
        text(format!("Adapter: {}", section.adapter))
            .style(Color::from_rgb(0.6, 0.6, 0.6))
    ];

    let mut rows = Column::new().spacing(5);
    for (i, entry) in section.entries.iter().enumerate() {
        let row_color = if i % 2 == 0 {
            BACKGROUND_COLOR
        } else {
            ROW_ALT_COLOR
        };

        let row = container(
            row![
                text(&entry.key).style(TEXT_COLOR).width(Length::Fixed(200.0)),
                text(&entry.value)
                    .style(Color::from_rgb(0.4, 0.8, 0.4))
                    .width(Length::Fixed(150.0)),
                text(entry.additional_info.clone().unwrap_or_default())
                    .style(Color::from_rgb(0.8, 0.8, 0.4))
                    .width(Length::Fill),
            ]
            .spacing(20)
            .align_items(Alignment::Center),
        )
        .style(iced::theme::Container::Custom(Box::new(RowStyle(row_color))))
        .padding(10)
        .width(Length::Fill);

        rows = rows.push(row);
    }

    container(column![header, rows].spacing(10))
        .padding(20)
        .style(iced::theme::Container::Custom(Box::new(SectionContainerStyle)))
        .into()
}

// Custom rule style function
fn error_rule_style(_theme: &Theme) -> iced::widget::rule::Appearance {
    iced::widget::rule::Appearance {
        color: ERROR_COLOR,
        width: 1,
        radius: 0.0.into(),
        fill_mode: iced::widget::rule::FillMode::Full,
    }
}

// Custom styles
struct AppContainerStyle;
struct SectionContainerStyle;
struct RowStyle(Color);

impl iced::widget::container::StyleSheet for AppContainerStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(BACKGROUND_COLOR.into()),
            ..Default::default()
        }
    }
}

impl iced::widget::container::StyleSheet for SectionContainerStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(Color::from_rgb(0.15, 0.15, 0.15).into()),
            border: iced::Border {
                radius: 8.0.into(),
                width: 1.0,
                color: Color::from_rgb(0.3, 0.3, 0.3),
            },
            ..Default::default()
        }
    }
}

impl iced::widget::container::StyleSheet for RowStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            background: Some(self.0.into()),
            ..Default::default()
        }
    }
}

// Sensor data reading and Parsing functions(add the graph too[real-time])
fn read_sensor_data() -> Result<Vec<SensorSection>, String> {
    let output = StdCommand::new("sensors")
        .output()
        .map_err(|e| format!("Failed to execute sensors command: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "sensors command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    parse_sensor_output(&output_str)
}

fn parse_sensor_output(input: &str) -> Result<Vec<SensorSection>, String> {
    let mut sections = Vec::new();
    let mut current_section: Option<SensorSection> = None;

    static ENTRY_REGEX: Lazy<Regex> = Lazy::new(|| {
        Regex::new(
            r"(?x)
            ^(?P<key>.+?):\s+
            (?P<value>[+-]?\d+\.?\d*\s?(°C|RPM|V|W|%|mA)?)
            (\s+\((?P<info>.+?)\))?$
            ",
        )
        .unwrap()
    });

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if !line.contains(':') && !line.starts_with("Adapter:") {
            if let Some(section) = current_section.take() {
                sections.push(section);
            }
            current_section = Some(SensorSection {
                name: line.to_string(),
                adapter: String::new(),
                entries: Vec::new(),
            });
        } else if let Some(ref mut section) = current_section {
            if line.starts_with("Adapter:") {
                section.adapter = line.replace("Adapter:", "").trim().to_string();
            } else if let Some(caps) = ENTRY_REGEX.captures(line) {
                let entry = SensorEntry {
                    key: caps["key"].to_string(),
                    value: caps["value"].trim().to_string(),
                    additional_info: caps.name("info").map(|m| m.as_str().to_string()),
                };
                section.entries.push(entry);
            }
        }
    }

    if let Some(section) = current_section.take() {
        sections.push(section);
    }

    if sections.is_empty() {
        Err("No sensor data found".to_string())
    } else {
        Ok(sections)
    }
}
