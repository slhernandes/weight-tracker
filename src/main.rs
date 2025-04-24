use chrono::{DateTime, Datelike, Days, Local, Months, NaiveDate, TimeDelta};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Row, Table, TableState},
    *,
};
use std::rc::Rc;
use std::{clone, io};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum SelectedFrame {
    Table,
    Chart,
}

#[allow(dead_code)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum ChartTimeFrame {
    Month,
    Year,
    WindowYear,
}

#[allow(dead_code)]
struct App {
    close: bool,
    data: Rc<Vec<(String, f64)>>,
    table_state: TableState,
    current_frame: SelectedFrame,
    current_tf: ChartTimeFrame,
    selected_date_wy: NaiveDate,
    selected_date_y: NaiveDate,
    selected_date_m: NaiveDate,
}

fn center_text(s: String) -> Text<'static> {
    return Text::styled(s, Style::default()).centered();
}

fn main() -> io::Result<()> {
    let mut term = ratatui::init();
    let mut app = App::default_test();
    app.table_state.select_last();
    let ret = app.run(&mut term);
    ratatui::try_restore()?;
    return ret;
}

impl App {
    #[allow(dead_code)]
    fn default_test() -> Self {
        let now = Local::now().date_naive();
        return App {
            close: false,
            data: Rc::new(vec![
                ("23-04-2025".to_string(), 90.1),
                ("24-04-2025".to_string(), 89.9),
                ("25-04-2025".to_string(), 90.5),
                ("26-04-2025".to_string(), 90.1),
                ("27-04-2025".to_string(), 89.9),
                ("28-04-2025".to_string(), 90.5),
                ("29-04-2025".to_string(), 90.1),
                ("30-04-2025".to_string(), 89.9),
                ("01-05-2025".to_string(), 90.5),
                ("02-05-2025".to_string(), 90.1),
                ("03-05-2025".to_string(), 89.9),
                ("04-05-2025".to_string(), 90.5),
                ("05-05-2025".to_string(), 90.1),
                ("06-05-2025".to_string(), 89.9),
                ("07-05-2025".to_string(), 90.5),
                ("08-05-2025".to_string(), 90.1),
                ("09-05-2025".to_string(), 89.9),
                ("10-05-2025".to_string(), 90.5),
                ("11-05-2025".to_string(), 90.1),
                ("12-05-2025".to_string(), 89.9),
                ("13-05-2025".to_string(), 90.5),
                ("14-05-2025".to_string(), 90.1),
                ("15-05-2025".to_string(), 89.9),
                ("16-05-2025".to_string(), 90.5),
                ("17-05-2025".to_string(), 90.1),
                ("18-05-2025".to_string(), 89.9),
                ("19-05-2025".to_string(), 90.5),
            ]),
            table_state: TableState::default(),
            current_frame: SelectedFrame::Table,
            current_tf: ChartTimeFrame::Month,
            selected_date_wy: now.clone(),
            selected_date_y: now.clone(),
            selected_date_m: now,
        };
    }

    #[allow(dead_code)]
    fn default() -> Self {
        let now = Local::now().date_naive();
        return App {
            close: false,
            data: Rc::new(Vec::new()),
            table_state: TableState::default(),
            current_frame: SelectedFrame::Table,
            current_tf: ChartTimeFrame::Month,
            selected_date_wy: now.clone(),
            selected_date_y: now.clone(),
            selected_date_m: now,
        };
    }

    fn run(&mut self, term: &mut DefaultTerminal) -> io::Result<()> {
        while !self.close {
            term.draw(|f| self.draw(f))?;
            self.handle_events()?;
        }
        return Ok(());
    }

    fn draw(&mut self, frame: &mut Frame) {
        // Vertical split
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(frame.area());

        // Title
        self.render_title(chunks[0], frame);

        // Middle split
        {
            let mid_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Length(21), Constraint::Min(20)])
                .split(chunks[1]);

            self.render_table(mid_chunks[0], frame);
            self.render_chart(mid_chunks[1], frame);
        }

        // Key hint
        self.render_key_hints(chunks[2], frame);
    }

    fn render_title(&self, area: Rect, frame: &mut Frame) {
        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default());
        let title = Paragraph::new(Text::styled("Weight Tracker", Style::default()))
            .centered()
            .block(title_block.clone());
        frame.render_widget(title, area);
    }

    fn render_table(&mut self, area: Rect, frame: &mut Frame) {
        let style = match self.current_frame {
            SelectedFrame::Table => Style::default(),
            _ => Style::default().dark_gray(),
        };
        let table_block = Block::default().borders(Borders::ALL).style(style);
        let widths = [Constraint::Length(12), Constraint::Length(7)];
        let cloned_data = Rc::clone(&self.data);
        let rows = cloned_data
            .iter()
            .map(|x| Row::new([center_text(x.0.clone()), center_text(format!("{:.1}", x.1))]));
        let table = Table::new(rows, widths)
            .header(
                Row::new([
                    center_text(String::from("Date")),
                    center_text(String::from("Weight")),
                ])
                .bottom_margin(1)
                .style(Style::default().on_blue().dark_gray()),
            )
            .block(table_block)
            .row_highlight_style(Style::new().on_dark_gray().white())
            .highlight_symbol("→");
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn render_chart(&mut self, area: Rect, frame: &mut Frame) {
        let style = match self.current_frame {
            SelectedFrame::Table => Style::default().dark_gray(),
            _ => Style::default(),
        };
        match self.current_tf {
            ChartTimeFrame::WindowYear => {
                let date_right = self.selected_date_wy;
                let date_left = date_right.checked_sub_months(Months::new(12)).unwrap();
                let delta = (date_right - date_left).num_days() as f64;
                let x_label = vec![
                    Span::styled(
                        format!("{}", date_left.format("%d-%m-%Y").to_string()),
                        Style::default(),
                    ),
                    Span::styled(
                        format!(
                            "{}",
                            date_left
                                .checked_add_months(Months::new(4))
                                .unwrap()
                                .format("%d-%m-%Y")
                                .to_string()
                        ),
                        Style::default(),
                    ),
                    Span::styled(
                        format!(
                            "{}",
                            date_left
                                .checked_add_months(Months::new(8))
                                .unwrap()
                                .format("%d-%m-%Y")
                                .to_string()
                        ),
                        Style::default(),
                    ),
                    Span::styled(
                        format!("{}", date_right.format("%d-%m-%Y").to_string()),
                        Style::default(),
                    ),
                ];
                let cloned_data = Rc::clone(&self.data);
                let data_points = cloned_data
                    .iter()
                    .map(|x| {
                        let date_point =
                            NaiveDate::parse_from_str(x.0.as_str(), "%d-%m-%Y").unwrap();
                        let diff = (date_point - date_left).num_days() as f64;
                        (diff, x.1.clone())
                    })
                    .collect::<Vec<_>>();
                let min_weight = cloned_data
                    .iter()
                    .fold(f64::MAX, |acc, x| x.1.clone().min(acc));
                let max_weight = cloned_data.iter().fold(0.0, |acc, x| x.1.clone().max(acc));
                let dataset = Dataset::default()
                    .marker(symbols::Marker::HalfBlock)
                    .style(Style::new().blue())
                    .graph_type(GraphType::Bar)
                    .data(&data_points);
                let chart = Chart::new(vec![dataset])
                    .block(
                        Block::bordered()
                            .title_top(Line::from("One Year Window").cyan().bold().centered())
                            .style(style),
                    )
                    .x_axis(
                        Axis::default()
                            .style(Style::default().gray())
                            .bounds([0.0, delta])
                            .labels(x_label)
                            .labels_alignment(Alignment::Right),
                    )
                    .y_axis(
                        Axis::default()
                            .style(Style::default().gray())
                            .bounds([min_weight - OFFSET_MIN, max_weight + OFFSET_MAX])
                            .labels([
                                format!("{:.1}", min_weight - OFFSET_MIN).bold(),
                                format!("{:.1}", max_weight + OFFSET_MAX).bold(),
                            ]),
                    )
                    .hidden_legend_constraints((Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)));
                frame.render_widget(chart, area);
            }
            ChartTimeFrame::Year => {
                let y = self.selected_date_y.year_ce().1;
                let date_left = NaiveDate::from_ymd_opt(y.try_into().unwrap(), 1, 1).unwrap();
                let date_right = NaiveDate::from_ymd_opt(y.try_into().unwrap(), 12, 31).unwrap();
                let delta = (date_right - date_left).num_days() as f64;
                let x_label = vec![
                    Span::styled(
                        format!("{}", date_left.format("%b").to_string()),
                        Style::default(),
                    ),
                    Span::styled(
                        format!(
                            "{}",
                            date_left
                                .checked_add_months(Months::new(4))
                                .unwrap()
                                .format("%b")
                                .to_string()
                        ),
                        Style::default(),
                    ),
                    Span::styled(
                        format!(
                            "{}",
                            date_left
                                .checked_add_months(Months::new(8))
                                .unwrap()
                                .format("%b")
                                .to_string()
                        ),
                        Style::default(),
                    ),
                    Span::styled(
                        format!("{}", date_right.format("%b").to_string()),
                        Style::default(),
                    ),
                ];
                let cloned_data = Rc::clone(&self.data);
                let data_points = cloned_data
                    .iter()
                    .map(|x| {
                        let date_point =
                            NaiveDate::parse_from_str(x.0.as_str(), "%d-%m-%Y").unwrap();
                        let diff = (date_point - date_left).num_days() as f64;
                        (diff, x.1.clone())
                    })
                    .collect::<Vec<_>>();
                let min_weight = cloned_data
                    .iter()
                    .fold(f64::MAX, |acc, x| x.1.clone().min(acc));
                let max_weight = cloned_data.iter().fold(0.0, |acc, x| x.1.clone().max(acc));
                let dataset = Dataset::default()
                    .marker(symbols::Marker::HalfBlock)
                    .style(Style::new().blue())
                    .graph_type(GraphType::Bar)
                    .data(&data_points);
                let chart = Chart::new(vec![dataset])
                    .block(
                        Block::bordered()
                            .title_top(
                                Line::from(self.selected_date_y.format("%Y").to_string())
                                    .cyan()
                                    .bold()
                                    .centered(),
                            )
                            .style(style),
                    )
                    .x_axis(
                        Axis::default()
                            .style(Style::default().gray())
                            .bounds([0.0, delta])
                            .labels(x_label)
                            .labels_alignment(Alignment::Right),
                    )
                    .y_axis(
                        Axis::default()
                            .style(Style::default().gray())
                            .bounds([min_weight - OFFSET_MIN, max_weight + OFFSET_MAX])
                            .labels([
                                format!("{:.1}", min_weight - OFFSET_MIN).bold(),
                                format!("{:.1}", max_weight + OFFSET_MAX).bold(),
                            ]),
                    )
                    .hidden_legend_constraints((Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)));
                frame.render_widget(chart, area);
            }
            ChartTimeFrame::Month => {
                let y = self.selected_date_m.year_ce().1;
                let m = self.selected_date_m.month();
                let date_left = NaiveDate::from_ymd_opt(y.try_into().unwrap(), m, 1).unwrap();
                let date_right = NaiveDate::from_ymd_opt(y.try_into().unwrap(), (m % 12) + 1, 1)
                    .unwrap()
                    .checked_sub_days(Days::new(1))
                    .unwrap();
                let delta = (date_right - date_left).num_days() as f64;
                let x_label = vec![
                    Span::styled(
                        format!("{}", date_left.format("%d").to_string()),
                        Style::default(),
                    ),
                    Span::styled(
                        format!("{}", date_right.format("%d").to_string()),
                        Style::default(),
                    ),
                ];
                let cloned_data = Rc::clone(&self.data);
                let data_points = cloned_data
                    .iter()
                    .map(|x| {
                        let date_point =
                            NaiveDate::parse_from_str(x.0.as_str(), "%d-%m-%Y").unwrap();
                        let diff = (date_point - date_left).num_days() as f64;
                        (diff, x.1.clone())
                    })
                    .collect::<Vec<_>>();
                let min_weight = cloned_data
                    .iter()
                    .fold(f64::MAX, |acc, x| x.1.clone().min(acc));
                let max_weight = cloned_data.iter().fold(0.0, |acc, x| x.1.clone().max(acc));
                let dataset = Dataset::default()
                    .marker(symbols::Marker::HalfBlock)
                    .style(Style::new().blue())
                    .graph_type(GraphType::Bar)
                    .data(&data_points);
                let chart = Chart::new(vec![dataset])
                    .block(
                        Block::bordered()
                            .title_top(
                                Line::from(self.selected_date_m.format("%b %Y").to_string())
                                    .cyan()
                                    .bold()
                                    .centered(),
                            )
                            .style(style),
                    )
                    .x_axis(
                        Axis::default()
                            .style(Style::default().gray())
                            .bounds([0.0, delta])
                            .labels(x_label)
                            .labels_alignment(Alignment::Right),
                    )
                    .y_axis(
                        Axis::default()
                            .style(Style::default().gray())
                            .bounds([min_weight - OFFSET_MIN, max_weight + OFFSET_MAX])
                            // .bounds([0.0, max_weight + OFFSET_MAX])
                            .labels([
                                format!("{:.1}", min_weight - OFFSET_MIN).bold(),
                                format!("{:.1}", max_weight + OFFSET_MAX).bold(),
                            ]),
                    );
                frame.render_widget(chart, area);
            }
        };
    }

    fn render_key_hints(&self, area: Rect, frame: &mut Frame) {
        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default());
        let key_hint = Paragraph::new(Span::styled("Press 'q' or C-c to quit.", Style::default()))
            .block(title_block);
        frame.render_widget(key_hint, area);
    }

    fn toggle_frame(&mut self) {
        self.current_frame = match self.current_frame {
            SelectedFrame::Chart => SelectedFrame::Table,
            SelectedFrame::Table => SelectedFrame::Chart,
        }
    }

    fn cycle_next_tf(&mut self) {
        self.current_tf = match self.current_tf {
            ChartTimeFrame::WindowYear => ChartTimeFrame::Month,
            ChartTimeFrame::Year => ChartTimeFrame::WindowYear,
            ChartTimeFrame::Month => ChartTimeFrame::Year,
        };
    }

    fn cycle_prev_tf(&mut self) {
        self.current_tf = match self.current_tf {
            ChartTimeFrame::Month => ChartTimeFrame::WindowYear,
            ChartTimeFrame::WindowYear => ChartTimeFrame::Year,
            ChartTimeFrame::Year => ChartTimeFrame::Month,
        };
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                return Ok(());
            }
            match (key.modifiers, key.code) {
                (_, KeyCode::Esc) | (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                    self.close = true;
                }
                (_, KeyCode::Tab) => self.toggle_frame(),
                (_, KeyCode::Char(ch)) => {
                    // Local key-binds
                    if self.current_frame == SelectedFrame::Table {
                        match ch {
                            'q' => self.close = true,
                            'k' => self.table_state.select_previous(),
                            'j' => self.table_state.select_next(),
                            _ => {}
                        };
                    } else if self.current_frame == SelectedFrame::Chart {
                        match ch {
                            'q' => self.close = true,
                            'k' => self.cycle_prev_tf(),
                            'j' => self.cycle_next_tf(),
                            'h' => match self.current_tf {
                                ChartTimeFrame::Month => {
                                    self.selected_date_m = self
                                        .selected_date_m
                                        .checked_sub_months(Months::new(1))
                                        .unwrap()
                                }
                                ChartTimeFrame::Year => {
                                    self.selected_date_y = self
                                        .selected_date_y
                                        .checked_sub_months(Months::new(12))
                                        .unwrap()
                                }
                                ChartTimeFrame::WindowYear => {
                                    self.selected_date_wy = self
                                        .selected_date_wy
                                        .checked_sub_days(Days::new(1))
                                        .unwrap()
                                }
                            },
                            'l' => match self.current_tf {
                                ChartTimeFrame::Month => {
                                    self.selected_date_m = self
                                        .selected_date_m
                                        .checked_add_months(Months::new(1))
                                        .unwrap()
                                }
                                ChartTimeFrame::Year => {
                                    self.selected_date_y = self
                                        .selected_date_y
                                        .checked_add_months(Months::new(12))
                                        .unwrap()
                                }
                                ChartTimeFrame::WindowYear => {
                                    self.selected_date_wy = self
                                        .selected_date_wy
                                        .checked_add_days(Days::new(1))
                                        .unwrap()
                                }
                            },
                            _ => {}
                        };
                    }
                }
                _ => {}
            }
        }
        return Ok(());
    }
}

const OFFSET_MIN: f64 = 2.0;
const OFFSET_MAX: f64 = 2.0;
