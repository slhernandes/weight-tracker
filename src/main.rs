use chrono::{Datelike, Days, Local, Months, NaiveDate};
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols::Marker,
    text::{Line, Span, Text},
    widgets::{
        Axis, Block, Borders, Chart, Clear, Dataset, GraphType, Paragraph, Row, Table, TableState,
    },
};
use std::{
    cell::RefCell,
    collections::VecDeque,
    env,
    fs::{self, File, OpenOptions},
    io::{self, Error, Read, Write},
};
use tui_textarea::{CursorMove, Input, TextArea};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum SelectedFrame {
    Table,
    Chart,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum ChartTimeFrame {
    Month,
    Year,
    WindowYear,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum WindowType {
    ClosePopup,
    InputPopup,
    MainWindow,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum TextMode {
    Edit,
    Append,
}

#[allow(dead_code)]
struct App<'a> {
    close: bool,
    current_window: WindowType,
    data: RefCell<Vec<(String, f64)>>,
    table_state: TableState,
    current_frame: SelectedFrame,
    current_tf: ChartTimeFrame,
    selected_date_wy: NaiveDate,
    selected_date_y: NaiveDate,
    selected_date_m: NaiveDate,
    text_area: [TextArea<'a>; 2],
    text_is_valid: [bool; 2],
    selected_area: usize,
    text_mode: Option<TextMode>,
    message: Option<String>,
}

fn center_text(s: String) -> Text<'static> {
    return Text::styled(s, Style::default()).centered();
}

fn get_data_file() -> io::Result<String> {
    let home_path = env::var("HOME");
    let mut path;
    if home_path.is_ok() {
        path = home_path.unwrap();
        path.push('/');
        path.push_str(DEFAULT_DIR);
        if !fs::exists(&path)? {
            fs::create_dir_all(&path)?;
        }
        path.push_str(DEFAULT_FILE_NAME);
    } else {
        return Err(Error::other("$HOME is not defined."));
    }
    return Ok(path);
}

fn main() -> io::Result<()> {
    let mut term = ratatui::init();
    let mut app = App::default();
    let path = get_data_file()?;
    if fs::exists(&path)? {
        app.import_data(&path)?;
    }
    app.table_state.select_last();
    let ret = app.run(&mut term);
    let mut out_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&path)?;
    app.export_data(&mut out_file)?;
    ratatui::try_restore()?;
    return ret;
}

impl App<'_> {
    #[allow(dead_code)]
    fn default_test() -> Self {
        let now = Local::now().date_naive();
        return App {
            close: false,
            current_window: WindowType::MainWindow,
            data: RefCell::new(vec![
                ("23-04-2024".to_string(), 90.1),
                ("24-04-2024".to_string(), 89.9),
                ("25-04-2024".to_string(), 90.5),
                ("26-04-2024".to_string(), 90.1),
                ("27-04-2024".to_string(), 89.9),
                ("28-04-2024".to_string(), 90.5),
                ("29-04-2024".to_string(), 90.1),
                ("30-04-2024".to_string(), 89.9),
                ("01-05-2024".to_string(), 90.5),
                ("02-05-2024".to_string(), 90.1),
                ("03-05-2024".to_string(), 89.9),
                ("04-05-2024".to_string(), 90.5),
                ("05-05-2024".to_string(), 90.1),
                ("06-05-2024".to_string(), 89.9),
                ("07-05-2024".to_string(), 90.5),
                ("08-05-2024".to_string(), 90.1),
                ("09-05-2024".to_string(), 89.9),
                ("10-05-2024".to_string(), 90.5),
                ("11-05-2024".to_string(), 90.1),
                ("12-05-2024".to_string(), 89.9),
                ("13-05-2024".to_string(), 90.5),
                ("14-05-2024".to_string(), 90.1),
                ("15-05-2024".to_string(), 89.9),
                ("16-05-2024".to_string(), 90.5),
                ("17-05-2024".to_string(), 90.1),
                ("18-05-2024".to_string(), 89.9),
                ("19-05-2024".to_string(), 90.5),
            ]),
            table_state: TableState::default(),
            current_frame: SelectedFrame::Table,
            current_tf: ChartTimeFrame::Month,
            selected_date_wy: now.clone(),
            selected_date_y: now.clone(),
            selected_date_m: now,
            text_area: [TextArea::default(), TextArea::default()],
            text_is_valid: [false, false],
            selected_area: 1,
            text_mode: None,
            message: None,
        };
    }

    #[allow(dead_code)]
    fn default() -> Self {
        let now = Local::now().date_naive();
        return App {
            close: false,
            current_window: WindowType::MainWindow,
            data: RefCell::new(Vec::new()),
            table_state: TableState::default(),
            current_frame: SelectedFrame::Table,
            current_tf: ChartTimeFrame::Month,
            selected_date_wy: now.clone(),
            selected_date_y: now.clone(),
            selected_date_m: now,
            text_area: [TextArea::default(), TextArea::default()],
            text_is_valid: [false, false],
            selected_area: 1,
            text_mode: None,
            message: None,
        };
    }

    fn modify_data(&mut self, element: (String, f64)) -> bool {
        let cloned_data = RefCell::clone(&self.data);
        let mut idx = None;
        for (i, val) in cloned_data.into_inner().iter().enumerate() {
            if val.0 == element.0 {
                idx = Some(i);
                break;
            }
        }
        let data_ref = self.data.get_mut();
        if let Some(idx) = idx {
            data_ref[idx].1 = element.1;
        } else {
            data_ref.push(element.clone());
        }
        return true;
    }

    fn import_data(&mut self, path: &String) -> io::Result<()> {
        let mut file = File::open(&path)?;
        let mut lines = String::new();
        file.read_to_string(&mut lines)?;
        let mut ret = lines
            .split(['\r', '\n'])
            .filter_map(|x| {
                if x.is_empty() {
                    None
                } else {
                    Some(
                        x.trim()
                            .split(',')
                            .filter_map(|x| if x.is_empty() { None } else { Some(x.trim()) })
                            .collect::<Vec<_>>(),
                    )
                }
            })
            .collect::<VecDeque<_>>();
        let header = ret.pop_front();
        if let Some(header) = header {
            if header.len() != 2 {
                return Err(Error::other("Invalid Header"));
            }
            if header[0] != "Date" && header[1] != "Weight" {
                return Err(Error::other("Invalid Header"));
            }
            let temp = ret
                .iter()
                .filter_map(|x| {
                    if let Ok(num) = x[1].trim().parse::<f64>() {
                        Some((String::from(x[0]), num))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            self.data = RefCell::new(temp);
        }
        return Ok(());
    }

    fn export_data(&self, file: &mut File) -> io::Result<()> {
        let cloned_data = RefCell::clone(&self.data);
        write!(file, "Date, Weight\n")?;
        for (date, weight) in cloned_data.into_inner().iter() {
            write!(file, "{}, {:.1}\n", date, weight)?;
        }
        return Ok(());
    }

    fn run(&mut self, term: &mut DefaultTerminal) -> io::Result<()> {
        while !self.close {
            term.draw(|f| self.draw(f))?;
            self.handle_events()?;
        }
        return Ok(());
    }

    fn init_text_area(&mut self) {
        let date_text = match self.text_mode {
            Some(TextMode::Edit) => {
                let idx = self.table_state.selected();
                if let Some(idx) = idx {
                    let data_ref = self.data.get_mut();
                    self.text_is_valid[0] = true;
                    data_ref[idx].0.clone()
                } else {
                    self.text_is_valid[0] = false;
                    String::from("")
                }
            }
            Some(TextMode::Append) => {
                self.text_is_valid[0] = true;
                Local::now().date_naive().format("%d-%m-%Y").to_string()
            }
            None => {
                self.text_is_valid[0] = false;
                String::from("")
            }
        };

        let weight_text = match self.text_mode {
            Some(TextMode::Edit) => {
                let idx = self.table_state.selected();
                if let Some(idx) = idx {
                    let data_ref = self.data.get_mut();
                    self.text_is_valid[0] = true;
                    format!("{:.1}", data_ref[idx].1)
                } else {
                    self.text_is_valid[0] = false;
                    String::from("")
                }
            }
            _ => {
                self.text_is_valid[0] = false;
                String::from("")
            }
        };

        self.selected_area = 1;

        self.text_area[0] = TextArea::new(vec![date_text]);
        self.text_area[1] = TextArea::new(vec![weight_text]);

        self.text_area[0].move_cursor(CursorMove::End);
        self.text_area[1].move_cursor(CursorMove::End);
    }

    fn activate_text(&mut self) {
        if self.selected_area == 0 {
            let text = self.text_area[0].lines()[0].clone();
            let date = NaiveDate::parse_from_str(text.as_str(), "%d-%m-%Y");
            if let Ok(_) = date {
                self.text_area[0].set_cursor_line_style(Style::default().fg(Color::LightGreen));
                self.text_area[0]
                    .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
                self.text_area[0].set_block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Color::LightGreen)
                        .title(" Date ")
                        .title_bottom(" Valid "),
                );
                self.text_is_valid[0] = true;
            } else {
                self.text_area[0].set_cursor_line_style(Style::default().fg(Color::LightRed));
                self.text_area[0]
                    .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
                self.text_area[0].set_block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Color::LightRed)
                        .title(" Date ")
                        .title_bottom(" Invalid "),
                );
                self.text_is_valid[0] = false;
            }
        } else if self.selected_area == 1 {
            let text = self.text_area[1].lines()[0].clone();
            let weight = text.parse::<f64>();
            match weight {
                Ok(w) if w > 0f64 => {
                    self.text_area[1].set_cursor_line_style(Style::default().fg(Color::LightGreen));
                    self.text_area[1]
                        .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
                    self.text_area[1].set_block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Color::LightGreen)
                            .title(" Weight ")
                            .title_bottom(" Valid "),
                    );
                    self.text_is_valid[0] = true;
                }
                _ => {
                    self.text_area[1].set_cursor_line_style(Style::default().fg(Color::LightRed));
                    self.text_area[1]
                        .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
                    self.text_area[1].set_block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Color::LightRed)
                            .title(" Weight ")
                            .title_bottom(" Invalid "),
                    );
                    self.text_is_valid[1] = false;
                }
            }
        }
    }

    fn inactivate_text(&mut self) {
        let inactive_area = (self.selected_area + 1) % 2;
        self.text_area[inactive_area].set_cursor_line_style(Style::default());
        self.text_area[inactive_area].set_cursor_style(Style::default());
        let title = if inactive_area == 0 {
            " Date "
        } else if inactive_area == 1 {
            " Weight "
        } else {
            unreachable!("Invalid index");
        };
        if inactive_area == 0 {
            let text = self.text_area[0].lines()[0].clone();
            let date = NaiveDate::parse_from_str(text.as_str(), "%d-%m-%Y");
            if let Ok(_) = date {
                self.text_is_valid[0] = true;
            } else {
                self.text_is_valid[0] = false;
            }
        } else if inactive_area == 1 {
            let text = self.text_area[1].lines()[0].parse::<f64>();
            if let Ok(_) = text {
                self.text_is_valid[1] = true;
            } else {
                self.text_is_valid[1] = false;
            }
        }
        self.text_area[inactive_area].set_block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::DarkGray))
                .title(title),
        );
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
        self.render_message_box(chunks[2], frame);
        if self.current_window == WindowType::ClosePopup {
            self.render_close_popup(frame);
        } else if self.current_window == WindowType::InputPopup {
            self.render_input_popup(frame);
        }
    }

    fn render_input_popup(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let vertical = Layout::vertical([Constraint::Length(3)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Length(25)]).flex(Flex::Center);
        let [area_popup] = vertical.areas(area);
        let [area_popup] = horizontal.areas(area_popup);
        let horizontal =
            Layout::horizontal([Constraint::Length(15), Constraint::Length(11)]).flex(Flex::Center);
        let area: [Rect; 2] = horizontal.areas(area_popup);

        self.activate_text();
        self.inactivate_text();

        frame.render_widget(Clear, area_popup); //this clears out the background
        frame.render_widget(&self.text_area[0], area[0]);
        frame.render_widget(&self.text_area[1], area[1]);
    }
    fn render_close_popup(&self, frame: &mut Frame) {
        let area = frame.area();
        let vertical = Layout::vertical([Constraint::Length(3)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Length(21)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        let block = Block::bordered();
        let text = Paragraph::new(center_text(String::from("Quit the app? [Y/n]"))).block(block);
        frame.render_widget(Clear, area); //this clears out the background
        frame.render_widget(text, area);
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
        let cloned_data = RefCell::clone(&self.data).into_inner();
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
                let cloned_data = RefCell::clone(&self.data).into_inner();
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
                    .marker(Marker::Dot)
                    .style(Style::new().blue())
                    .graph_type(GraphType::Scatter)
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
                let cloned_data = RefCell::clone(&self.data).into_inner();
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
                    .marker(Marker::Dot)
                    .style(Style::new().blue())
                    .graph_type(GraphType::Scatter)
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
                let cloned_data = RefCell::clone(&self.data).into_inner();
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
                    // .marker(Marker::HalfBlock)
                    .marker(Marker::Dot)
                    .style(Style::new().blue())
                    // .graph_type(GraphType::Bar) // Bar is fucked on v0.29.0
                    .graph_type(GraphType::Scatter)
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

    fn render_message_box(&self, area: Rect, frame: &mut Frame) {
        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default());
        let msg = self.message.clone();
        if let Some(msg) = msg {
            let msg_str = msg.as_str();
            let message = Paragraph::new(Span::styled(msg_str, Style::default()))
                .centered()
                .block(title_block);
            frame.render_widget(message, area);
        } else {
            let message = Paragraph::new(Span::styled(DEFAULT_MESSAGE, Style::default()))
                .centered()
                .block(title_block);
            frame.render_widget(message, area);
        }
    }

    fn toggle_frame(&mut self) {
        self.current_frame = match self.current_frame {
            SelectedFrame::Chart => {
                self.message = None;
                SelectedFrame::Table
            }
            SelectedFrame::Table => {
                self.message = Some(String::from(
                    "Esc/q => quit app | a => append table | e => edit selected row | j/k => cycle chart | h => decrease x-axis | l => increase x-axis",
                ));
                SelectedFrame::Chart
            }
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
                (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                    self.close = true;
                }
                (_, KeyCode::Esc) => match self.current_window {
                    WindowType::MainWindow => {
                        self.message = Some(String::from(
                            "Esc/n => back to main window | Enter/y => quit app",
                        ));
                        self.current_window = WindowType::ClosePopup
                    }
                    _ => {
                        if self.current_frame == SelectedFrame::Table {
                            self.message = None;
                        } else {
                            self.message = Some(String::from(
                                "Esc/q => quit app | a => append table | e => edit selected row | j/k => cycle chart | h => decrease x-axis | l => increase x-axis",
                            ));
                        }
                        self.current_window = WindowType::MainWindow
                    }
                },
                (_, KeyCode::Enter) => match self.current_window {
                    WindowType::MainWindow => {}
                    WindowType::ClosePopup => self.close = true,
                    WindowType::InputPopup => {
                        let (date, weight) = (
                            self.text_area[0].lines()[0].clone(),
                            self.text_area[1].lines()[0].parse::<f64>(),
                        );
                        let date_is_valid = if let Ok(_) =
                            NaiveDate::parse_from_str(date.clone().as_str(), "%d-%m-%Y")
                        {
                            true
                        } else {
                            false
                        };
                        let weight_is_valid = if let Ok(w) = weight { w > 0f64 } else { false };
                        if date_is_valid && weight_is_valid {
                            if self.modify_data((date, weight.unwrap())) {
                                if self.current_frame == SelectedFrame::Table {
                                    self.message = None;
                                } else {
                                    self.message = Some(String::from(
                                        "Esc/q => quit app | a => append table | e => edit selected row | j/k => cycle chart | h => decrease x-axis | l => increase x-axis",
                                    ));
                                }
                                self.current_window = WindowType::MainWindow;
                                self.table_state.select_last();
                            }
                        } else {
                        }
                    }
                },
                (_, KeyCode::Tab) => match self.current_window {
                    WindowType::MainWindow => self.toggle_frame(),
                    WindowType::InputPopup => self.selected_area = (self.selected_area + 1) % 2,
                    _ => {}
                },
                (_, KeyCode::Backspace) => match self.current_window {
                    WindowType::InputPopup => {
                        let _ = self.text_area[self.selected_area].delete_char();
                    }
                    _ => {}
                },
                (_, KeyCode::Char(ch)) => {
                    // Local key-binds
                    match self.current_window {
                        WindowType::MainWindow => {
                            if self.current_frame == SelectedFrame::Table {
                                match ch {
                                    'q' => {
                                        self.current_window = WindowType::ClosePopup;
                                        self.message = Some(String::from(
                                            "Esc/n => back to main window | Enter/y => quit app",
                                        ));
                                    }
                                    'k' => self.table_state.select_previous(),
                                    'j' => self.table_state.select_next(),
                                    'a' => {
                                        self.current_window = WindowType::InputPopup;
                                        self.text_mode = Some(TextMode::Append);
                                        self.init_text_area();
                                        self.message = Some(String::from(
                                            "Esc => go to main window | Tab => switch input box | Enter => submit form if valid",
                                        ));
                                    }
                                    'e' => {
                                        self.current_window = WindowType::InputPopup;
                                        self.text_mode = Some(TextMode::Edit);
                                        self.init_text_area();
                                        self.message = Some(String::from(
                                            "Esc => go to main window | Tab => switch input box | Enter => submit form if valid",
                                        ));
                                    }
                                    _ => {}
                                };
                            } else if self.current_frame == SelectedFrame::Chart {
                                match ch {
                                    'q' => {
                                        self.current_window = WindowType::ClosePopup;
                                        self.message = Some(String::from(
                                            "Esc/n => back to main window | Enter/y => quit app",
                                        ));
                                    }
                                    'k' => self.cycle_prev_tf(),
                                    'j' => self.cycle_next_tf(),
                                    'a' => {
                                        self.current_window = WindowType::InputPopup;
                                        self.text_mode = Some(TextMode::Append);
                                        self.init_text_area();
                                        self.message = Some(String::from(
                                            "Esc => go to main window | Tab => switch input box | Enter => submit form if valid",
                                        ));
                                    }
                                    'e' => {
                                        self.current_window = WindowType::InputPopup;
                                        self.text_mode = Some(TextMode::Edit);
                                        self.init_text_area();
                                        self.message = Some(String::from(
                                            "Esc => go to main window | Tab => switch input box | Enter => submit form if valid",
                                        ));
                                    }
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
                        WindowType::ClosePopup => match ch {
                            'y' => self.close = true,
                            'n' => {
                                self.current_window = WindowType::MainWindow;
                                if self.current_frame == SelectedFrame::Table {
                                    self.message = None;
                                } else {
                                    self.message = Some(String::from(
                                        "Esc/q => quit app | a => append table | e => edit selected row | j/k => cycle chart | h => decrease x-axis | l => increase x-axis",
                                    ));
                                }
                            }
                            _ => {}
                        },
                        WindowType::InputPopup => {
                            let input: Input = Event::Key(key).into();
                            if self.text_area[self.selected_area].input(input) {
                                self.activate_text();
                            }
                        }
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
const DEFAULT_MESSAGE: &str = "Esc/q => quit app | a => append table | e => edit selected row | j => go down 1 row | k => go up 1 row";
const DEFAULT_DIR: &str = ".cache/weight-tracker/";
const DEFAULT_FILE_NAME: &str = "weight-tracker.csv";
