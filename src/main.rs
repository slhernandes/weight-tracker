use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Span, Text},
    widgets::{Block, Borders, Paragraph, Row, Table, TableState},
    *,
};
use std::io;
use std::rc::Rc;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum SelectedFrame {
    Table,
    Graph,
}

#[allow(dead_code)]
struct App<'a> {
    close: bool,
    data: Rc<Vec<Row<'a>>>,
    table_state: TableState,
    current_frame: SelectedFrame,
}

fn center_text(s: &str) -> Text {
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

impl App<'_> {
    #[allow(dead_code)]
    fn default_test() -> Self {
        return App {
            close: false,
            data: Rc::new(vec![
                Row::new(vec![center_text("23-04-2025"), center_text("90.1")]),
                Row::new(vec![center_text("24-04-2025"), center_text("89.9")]),
                Row::new(vec![center_text("25-04-2025"), center_text("90.5")]),
            ]),
            table_state: TableState::default(),
            current_frame: SelectedFrame::Table,
        };
    }

    #[allow(dead_code)]
    fn default() -> Self {
        return App {
            close: false,
            data: Rc::new(Vec::new()),
            table_state: TableState::default(),
            current_frame: SelectedFrame::Table,
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
            self.render_graph(mid_chunks[1], frame);
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
        let table = Table::new(self.data.to_vec(), widths)
            .header(
                Row::new([center_text("Date"), center_text("Weight")])
                    .bottom_margin(1)
                    .style(Style::default().on_blue().dark_gray()),
            )
            .block(table_block)
            .row_highlight_style(Style::new().on_dark_gray().white())
            .highlight_symbol("→");
        frame.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn render_graph(&mut self, area: Rect, frame: &mut Frame) {
        let style = match self.current_frame {
            SelectedFrame::Table => Style::default().dark_gray(),
            _ => Style::default(),
        };
        let temp = Block::default().borders(Borders::ALL).style(style);
        frame.render_widget(temp, area);
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
            SelectedFrame::Graph => SelectedFrame::Table,
            SelectedFrame::Table => SelectedFrame::Graph,
        }
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
                    }
                }
                _ => {}
            }
        }
        return Ok(());
    }
}
