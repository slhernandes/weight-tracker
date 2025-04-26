use chrono::{Datelike, Days, Local, Months, NaiveDate};
use directories::BaseDirs;
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
    cell::RefCell, cmp::Ordering, collections::VecDeque, fs::{self, File, OpenOptions}, io::{self, Error, Read, Write}, time::{Duration, Instant}
};
use tui_textarea::{CursorMove, Input, TextArea};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum FrameType {
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

#[allow(unused)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum MessageType {
    Info,
    Warning,
    Error,
}

#[allow(dead_code)]
struct App<'a> {
    close: bool,
    current_window: WindowType,
    data: RefCell<Vec<(String, f64)>>,
    table_state: TableState,
    current_frame: FrameType,
    current_tf: ChartTimeFrame,
    selected_date_wy: NaiveDate,
    selected_date_y: NaiveDate,
    selected_date_m: NaiveDate,
    text_area: [TextArea<'a>; 2],
    text_is_valid: [bool; 2],
    selected_area: usize,
    text_mode: Option<TextMode>,
    message: Option<(String, MessageType)>,
    msg_time_elapsed: Option<Instant>,
    wait_time_elapsed: Option<Instant>,
    scroll_offset: usize,
    reversed_offset: bool,
    rm_confirm: bool,
}

fn center_text(s: String) -> Text<'static> {
    return Text::styled(s, Style::default()).centered();
}

fn get_data_file() -> io::Result<String> {
    let base_dirs = BaseDirs::new();
    if let None = base_dirs {
        return Err(Error::other("BaseDirs::new() failed"));
    }
    let mut data_path = base_dirs.unwrap().data_local_dir().to_path_buf();
    data_path.push("weight-tracker");
    if !data_path.try_exists()? {
        let data_path_str = data_path.to_str().unwrap();
        fs::create_dir_all(data_path_str)?;
    }
    data_path.push("weight-tracker.csv");
    let ret = data_path.to_str();
    if let Some(ret) = ret {
        return Ok(ret.to_string());
    }
    return Err(Error::other("Cannot create path str"));
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
                ("26-04-2024".to_string(), 90.1),
                ("27-04-2024".to_string(), 89.9),
                ("01-05-2024".to_string(), 91.5),
                ("02-05-2024".to_string(), 94.1),
                ("03-05-2024".to_string(), 87.9),
            ]),
            table_state: TableState::default(),
            current_frame: FrameType::Table,
            current_tf: ChartTimeFrame::Month,
            selected_date_wy: now.clone(),
            selected_date_y: now.clone(),
            selected_date_m: now,
            text_area: [TextArea::default(), TextArea::default()],
            text_is_valid: [false, false],
            selected_area: 1,
            text_mode: None,
            message: None,
            msg_time_elapsed: None,
            wait_time_elapsed: None,
            scroll_offset: 0,
            reversed_offset: false,
            rm_confirm: false,
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
            current_frame: FrameType::Table,
            current_tf: ChartTimeFrame::Month,
            selected_date_wy: now.clone(),
            selected_date_y: now.clone(),
            selected_date_m: now,
            text_area: [TextArea::default(), TextArea::default()],
            text_is_valid: [false, false],
            selected_area: 1,
            text_mode: None,
            message: None,
            msg_time_elapsed: None,
            wait_time_elapsed: None,
            scroll_offset: 0,
            reversed_offset: false,
            rm_confirm: false,
        };
    }

    fn modify_data(&mut self, element: (String, Option<f64>)) -> bool {
        let idx = self.table_state.selected_mut();
        if let None = idx {
            return false;
        }
        let idx = idx.unwrap();
        let data_ref = self.data.get_mut();
        if let (s, Some(num)) = element {
            if self.text_mode == Some(TextMode::Edit) {
                data_ref[idx].1 = num;
            } else if self.text_mode == Some(TextMode::Append) {
                let l_bound = data_ref.binary_search_by(|x| {
                    // Format should already checked beforehand.
                    let lhs = NaiveDate::parse_from_str(x.0.as_str(), "%d-%m-%Y").unwrap();
                    let rhs = NaiveDate::parse_from_str(s.as_str(), "%d-%m-%Y").unwrap();
                    if lhs < rhs {
                        Ordering::Less
                    } else if lhs == rhs {
                        Ordering::Equal
                    } else {
                        Ordering::Greater
                    }
                });
                if let Ok(_) = l_bound {
                    self.message = Some((String::from("Cannot add element. Did you mean to edit?"), MessageType::Error));
                    return false;
                } else {
                    data_ref.insert(l_bound.unwrap_err(), (s, num));
                }
            }
        } else if let (_, None) = element {
            data_ref.remove(idx);
            self.rm_confirm = false;
            self.message = None;
            self.msg_time_elapsed = None;
        }
        return true;
    }

    fn import_data(&mut self, path: &String) -> io::Result<()> {
        let file = File::open(&path);
        if let Err(_) = file {
            // Do nothing in case of file does not exist
            return Ok(());
        }
        let mut file = file.unwrap();
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
        let tick_rate = Duration::from_micros(16667);
        let mut now = Instant::now();
        while !self.close {
            term.draw(|f| self.draw(f))?;
            let timeout = tick_rate.saturating_add(now.elapsed());
            if event::poll(timeout)? {
                self.handle_events()?;
            }
            if now.elapsed() >= tick_rate {
                now = Instant::now();
            }
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
        let area = frame.area();
        let w = area.width;
        let h = area.height;
        if w >= MIN_WIDTH && h >= MIN_HEIGHT {
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
        } else {
            self.render_window_too_small(frame, w, h);
        }
    }

    fn render_window_too_small(&self, frame: &mut Frame, w: u16, h: u16) {
        let layout = Layout::vertical([Constraint::Length(4)]).flex(Flex::Center);
        let w_span = if w < MIN_WIDTH {
            Span::styled(format!("{}", w), Style::new().light_red())
        } else {
            Span::styled(format!("{}", w), Style::new().light_green())
        };
        let h_span = if h < MIN_HEIGHT {
            Span::styled(format!("{}", h), Style::new().light_red())
        } else {
            Span::styled(format!("{}", h), Style::new().light_green())
        };
        let lines = vec![
            Line::from("Terminal size too small:"),
            Line::from(vec![w_span, Span::raw(" x "), h_span]),
            Line::from("Required size:"),
            Line::from(format!("{} x {}", MIN_WIDTH, MIN_HEIGHT)),
        ];
        let [area] = layout.areas(frame.area());
        frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
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
            FrameType::Table => Style::default(),
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
            FrameType::Table => Style::default().dark_gray(),
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
                    .filter_map(|x| {
                        let date_point =
                            NaiveDate::parse_from_str(x.0.as_str(), "%d-%m-%Y").unwrap();
                        let diff = (date_point - date_left).num_days() as f64;
                        if diff >= 0f64 && diff <= delta {
                            Some((diff, x.1.clone()))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                let min_weight = if !data_points.is_empty() {
                    data_points
                        .iter()
                        .fold(f64::MAX, |acc, x| x.1.clone().min(acc))
                } else {
                    0f64 + OFFSET_MIN
                };
                let max_weight = if !data_points.is_empty() {
                    data_points.iter().fold(0f64, |acc, x| x.1.clone().max(acc))
                } else {
                    100f64 - OFFSET_MAX
                };
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
                    .filter_map(|x| {
                        let date_point =
                            NaiveDate::parse_from_str(x.0.as_str(), "%d-%m-%Y").unwrap();
                        let diff = (date_point - date_left).num_days() as f64;
                        if diff >= 0f64 && diff <= delta {
                            Some((diff, x.1.clone()))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                let min_weight = if !data_points.is_empty() {
                    data_points
                        .iter()
                        .fold(f64::MAX, |acc, x| x.1.clone().min(acc))
                } else {
                    0f64 + OFFSET_MIN
                };
                let max_weight = if !data_points.is_empty() {
                    data_points.iter().fold(0f64, |acc, x| x.1.clone().max(acc))
                } else {
                    100f64 - OFFSET_MAX
                };
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
                    .filter_map(|x| {
                        let date_point =
                            NaiveDate::parse_from_str(x.0.as_str(), "%d-%m-%Y").unwrap();
                        let diff = (date_point - date_left).num_days() as f64;
                        if diff >= 0f64 && diff <= delta {
                            Some((diff, x.1.clone()))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                let min_weight = if !data_points.is_empty() {
                    data_points
                        .iter()
                        .fold(f64::MAX, |acc, x| x.1.clone().min(acc))
                } else {
                    0f64 + OFFSET_MIN
                };
                let max_weight = if !data_points.is_empty() {
                    data_points.iter().fold(0f64, |acc, x| x.1.clone().max(acc))
                } else {
                    100f64 - OFFSET_MAX
                };
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

    fn render_message_box(&mut self, area: Rect, frame: &mut Frame) {
        let title_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default());
        if let Some((msg, msg_type)) = &self.message {
            let msg_str = msg.as_str();
            let style = match msg_type {
                MessageType::Warning => {
                    Style::default().fg(Color::LightYellow)
                }
                MessageType::Info => {
                    Style::default().fg(Color::LightGreen)
                }
                MessageType::Error => {
                    Style::default().fg(Color::LightRed)
                }
            };
            let message = Paragraph::new(Span::styled(msg_str, style))
                .centered()
                .block(title_block);
            frame.render_widget(message, area);
            if let Some(msg_time_elapsed) = self.msg_time_elapsed {
                if msg_time_elapsed.elapsed() >= MSG_TIMEOUT {
                    self.rm_confirm = false;
                    self.message = None;
                    self.msg_time_elapsed = None;
                }
            } else {
                self.msg_time_elapsed = Some(Instant::now());
            }
        } else {
            let message = match self.current_window {
                WindowType::ClosePopup => {
                    String::from("Esc/n => back to main window | Enter/y => quit app")
                }
                WindowType::InputPopup => String::from(
                    "Esc => go to main window | Tab => switch input box | Enter => submit form",
                ),
                WindowType::MainWindow => match self.current_frame {
                    FrameType::Chart => String::from(
                        "Esc/q: quit app | a: append table | e: edit selected row | j/k: cycle chart | h/l: (-/+)x-axis",
                    ),
                    FrameType::Table => String::from(
                        "Esc/q: quit app | a: append table | e: edit selected row | j/k: (↓/↑) 1 row | d: delete 1 row",
                    ),
                },
            };
            let tick_count = 3;
            let max_offset = (message.len() + 2).saturating_sub(usize::from(area.width));
            self.scroll_offset = self.scroll_offset.clamp(0, max_offset * tick_count);
            let message = if usize::from(area.width) >= message.len() + 2 {
                self.wait_time_elapsed = None;
                Paragraph::new(Span::styled(message, Style::default()))
                    .centered()
                    .block(title_block)
            } else {
                Paragraph::new(Span::styled(message, Style::default()))
                    .scroll((
                        0,
                        (self.scroll_offset / tick_count)
                            .clamp(0, usize::from(u16::MAX))
                            .try_into()
                            .unwrap(),
                    ))
                    .block(title_block)
            };
            if let Some(wait_time_elapsed) = self.wait_time_elapsed {
                if wait_time_elapsed.elapsed() > Duration::from_secs(1) {
                    self.wait_time_elapsed = None;
                }
            } else {
                if self.reversed_offset {
                    if self.scroll_offset > 0 {
                        self.scroll_offset -= 1;
                    } else {
                        self.reversed_offset = false;
                        if self.wait_time_elapsed == None {
                            self.wait_time_elapsed = Some(Instant::now());
                        }
                    }
                } else {
                    if self.scroll_offset + 1 <= max_offset * tick_count {
                        self.scroll_offset += 1;
                    } else {
                        self.scroll_offset = max_offset * tick_count;
                        self.reversed_offset = true;
                        if self.wait_time_elapsed == None {
                            self.wait_time_elapsed = Some(Instant::now());
                        }
                    }
                }
            }
            frame.render_widget(message, area);
        }
    }

    fn toggle_frame(&mut self) {
        self.current_frame = match self.current_frame {
            FrameType::Chart => FrameType::Table,
            FrameType::Table => FrameType::Chart,
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
                        self.current_window = WindowType::ClosePopup;
                        self.scroll_offset = 0;
                    }
                    _ => {
                        self.current_window = WindowType::MainWindow;
                        self.scroll_offset = 0;
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
                            if self.modify_data((date, Some(weight.unwrap()))) {
                                self.current_window = WindowType::MainWindow;
                                self.scroll_offset = 0;
                                self.table_state.select_last();
                                self.text_mode = None;
                            }
                        } else if date_is_valid {
                            self.message = Some((String::from("Invalid weight format!"), MessageType::Error));
                        } else if weight_is_valid {
                            self.message = Some((String::from("Invalid date format!"), MessageType::Error));
                        } else {
                            self.message = Some((String::from("Invalid weight & date format!"), MessageType::Error));
                        }
                    }
                },
                (_, KeyCode::Tab) => match self.current_window {
                    WindowType::MainWindow => self.toggle_frame(),
                    WindowType::InputPopup => match self.text_mode {
                        Some(TextMode::Append) => self.selected_area = (self.selected_area + 1) % 2,
                        _ => {}
                    },
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
                            if self.current_frame == FrameType::Table {
                                match ch {
                                    'q' => {
                                        self.current_window = WindowType::ClosePopup;
                                        self.scroll_offset = 0;
                                    }
                                    'k' => self.table_state.select_previous(),
                                    'j' => self.table_state.select_next(),
                                    'a' => {
                                        self.current_window = WindowType::InputPopup;
                                        self.scroll_offset = 0;
                                        self.text_mode = Some(TextMode::Append);
                                        self.init_text_area();
                                    }
                                    'e' => {
                                        self.current_window = WindowType::InputPopup;
                                        self.scroll_offset = 0;
                                        self.text_mode = Some(TextMode::Edit);
                                        self.init_text_area();
                                    }
                                    'd' => {
                                        if self.rm_confirm {
                                            let idx = self.table_state.selected_mut();
                                            if let None = idx {
                                                return Err(Error::other("No row is selected."));
                                            }
                                            let idx = idx.unwrap();
                                            let data_ref = self.data.get_mut();
                                            let selected = data_ref[idx].clone();
                                            self.modify_data((selected.0, None));
                                        } else {
                                            self.rm_confirm = true;
                                            self.message = Some((String::from("Press 'd' again to confirm deletion"), MessageType::Warning));
                                        }
                                    }
                                    _ => {}
                                };
                            } else if self.current_frame == FrameType::Chart {
                                match ch {
                                    'q' => {
                                        self.current_window = WindowType::ClosePopup;
                                        self.scroll_offset = 0;
                                    }
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
                        WindowType::ClosePopup => match ch {
                            'y' => self.close = true,
                            'n' => {
                                self.current_window = WindowType::MainWindow;
                                self.scroll_offset = 0;
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
const MSG_TIMEOUT: Duration = Duration::from_secs(3);
const MIN_WIDTH: u16 = 60u16;
const MIN_HEIGHT: u16 = 20u16;
