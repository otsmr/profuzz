use crate::fuzz::stats::{SerializableInstant, Stats};
use crossterm::event::{self, Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::{Layout, Rect};
use ratatui::prelude::Constraint::Length;
use ratatui::prelude::Stylize;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use ratatui::{DefaultTerminal, Frame, symbols};
use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::time::Duration;
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget};

#[derive(Debug, Default)]
pub struct Ui {
    stats: Stats,
}

impl Ui {
    /// runs the application's main loop until the user quits
    pub(crate) fn run(&mut self, terminal: &mut DefaultTerminal, stats: &Arc<RwLock<Stats>>) {
        self.stats.running = true;
        while self.stats.running {
            if let Ok(stats) = stats.read() {
                self.stats.clone_from(&stats);
            }
            if terminal.draw(|frame| self.draw(frame)).is_err() {
                tracing::error!("Error showing ui");
                break;
            }
            sleep(Duration::from_millis(10));
        }
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }
}
fn format_instant(duration: Option<SerializableInstant>) -> String {
    if let Some(duration) = duration {
        format_duration(duration.elapsed())
    } else {
        "0 days, 0 hrs, 0 min, 0 secs".into()
    }
}

fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let days = total_secs / 86400; // 60 * 60 * 24
    let hours = (total_secs % 86400) / 3600; // 60 * 60
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    format!("{days} days, {hours} hrs, {minutes} min, {seconds} secs",)
}

fn format_number(num: u64) -> String {
    #[allow(clippy::cast_precision_loss)]
    if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}k", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}

fn format_text_row<'a>(rows: &[(&'static str, Span<'a>)]) -> Text<'a> {
    let mut result = vec![];

    let mut max = 0;
    for (title, _) in rows {
        if title.len() > max {
            max = title.len();
        }
    }
    for (title, desc) in rows {
        result.push(Line::from(vec![format!("{}{title} : ", " ".repeat(max - title.len() + 1)).dark_gray(), desc.clone()]));
    }

    Text::from(result)
}

impl Widget for &Ui {
    #[allow(clippy::too_many_lines)]
    fn render(self, area: Rect, buf: &mut Buffer) {
        let hor = Layout::horizontal([Length(90)]).split(area);
        let main_area = Layout::vertical([Length(3), Length(40)]).split(hor[0]);

        #[allow(clippy::useless_conversion)]
        Paragraph::new(Text::from([Line::from(""), Line::from(" ProFUZZ 0.0.1 ".bold().yellow() + self.stats.title.clone().green())].to_vec()))
            .centered()
            .render(main_area[0], buf);

        let columns = Layout::vertical([Length(6), Length(4), Length(10)]).split(main_area[1]);
        let instructions = Line::from(vec!["  Started :)  ".blue().bold()]).right_aligned();

        let row1 = Layout::horizontal([Length(60), Length(30)]).split(columns[0]);
        let row2 = Layout::horizontal([Length(35), Length(55)]).split(columns[1]);

        // let formatter = MyLogFormatter();

        TuiLoggerWidget::default()
            .block(
                Block::new()
                    .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                    // .title(Line::from(" logs ".to_string().cyan()))
                    .title_bottom(instructions),
            )
            .output_separator(' ')
            .output_timestamp(Some("%H:%M:%S".to_string()))
            .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
            .output_target(false)
            .output_file(false)
            .output_line(false)
            .style_error(Style::default().fg(Color::Red))
            // .style_debug(Style::default().fg(Color::Green))
            // .style_warn(Style::default().fg(Color::Yellow))
            // .style_trace(Style::default().fg(Color::Magenta))
            // .style_info(Style::default().fg(Color::Cyan))
            // .style(Style::default().fg(Color::Gray))
            // .formatter(formatter)
            // .state(&filter_state)
            .render(columns[2], buf);

        {
            let text_rows = [
                ("run time", format_instant(self.stats.started).white()),
                ("last new path", format_instant(self.stats.last_new_path).white()),
                ("last unique crash", format_instant(self.stats.last_unique_crash).white()),
                ("last health check", format_instant(self.stats.last_healt_check).white()),
                ("backoff time", format_duration(Duration::from_millis(self.stats.backoff_time)).white()),
            ];
            let text = format_text_row(&text_rows);
            let border_set = symbols::border::Set {
                top_right: symbols::line::NORMAL.horizontal_down,
                ..symbols::border::PLAIN
            };

            Paragraph::new(text)
                .block(
                    Block::new()
                        .border_set(border_set)
                        .borders(Borders::TOP | Borders::RIGHT | Borders::LEFT)
                        .title(Line::from(" process timing ".to_string().cyan())),
                )
                .render(row1[0], buf);
        }

        {
            let text_rows = [
                ("cycles done", format_number(self.stats.cylcles_done as u64).white()),
                ("corpus count", format_number(self.stats.corpus_count as u64).white()),
                ("total responses", format_number(self.stats.total_unique_responses as u64).white()),
            ];
            let text = format_text_row(&text_rows);

            Paragraph::new(text)
                .block(
                    Block::new().borders(Borders::TOP | Borders::RIGHT).title(Line::from(" overall results ".to_string().cyan())), // .border_set(border::ROUNDED),
                )
                .render(row1[1], buf);
        }

        {
            let execs = self.stats.executions_per_second.get() as u64;
            let mut exec_speed = (format_number(execs) + "/sec").white();
            if execs < 30 {
                exec_speed = (format_number(execs) + "/sec (slow!)").red();
            }

            let text_rows = [("total execs", (format_number(self.stats.total_executions)).white()), ("exec speed", exec_speed)];
            let text = format_text_row(&text_rows);

            let border_set = symbols::border::Set {
                top_left: symbols::line::NORMAL.vertical_right,
                top_right: symbols::line::NORMAL.horizontal_down,
                bottom_left: symbols::line::NORMAL.vertical_right,
                bottom_right: symbols::line::NORMAL.horizontal_up,
                ..symbols::border::PLAIN
            };

            Paragraph::new(text)
                .block(
                    Block::new()
                        .border_set(border_set)
                        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
                        .title_bottom(Line::from(" logs ".to_string().cyan()))
                        .title(Line::from(" stage progress ".to_string().cyan())),
                )
                .render(row2[0], buf);
        }

        {
            let mut total_crashes = format_number(self.stats.total_crashes as u64).white();
            if self.stats.total_crashes > 0 {
                total_crashes = format_number(self.stats.total_crashes as u64).red();
            }

            let text_rows = [("total crashes", total_crashes), ("total timeouts", format_number(self.stats.total_timeouts as u64).white())];
            let text = format_text_row(&text_rows);

            let border_set = symbols::border::Set {
                top_right: symbols::line::NORMAL.vertical_left,
                bottom_left: symbols::line::NORMAL.horizontal_up,
                bottom_right: symbols::line::NORMAL.vertical_left,
                ..symbols::border::PLAIN
            };

            Paragraph::new(text)
                .block(
                    Block::new()
                        .border_set(border_set)
                        .borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM)
                        .title(Line::from(" findings in depth ".to_string().cyan())), // .title_bottom(instructions.right_aligned()),
                )
                .render(row2[1], buf);
        }
    }
}

pub fn show_ui(stats: &Arc<RwLock<Stats>>) {
    let mut terminal = ratatui::init();

    let stats_cloned = stats.clone();
    std::thread::spawn(move || -> std::io::Result<()> {
        loop {
            {
                if !stats_cloned.read().expect("closing").running {
                    return Ok(());
                }
            }
            if event::poll(std::time::Duration::from_millis(100))?
                && let Event::Key(key_event) = event::read()?
                && key_event.code == KeyCode::Char('c')
                && key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
            {
                stats_cloned.write().expect("closing").running = false;
            }
        }
    });
    Ui::default().run(&mut terminal, stats);
    ratatui::restore();
}
