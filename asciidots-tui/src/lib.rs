use std::time::Duration;

use asciidots_core::interpreter::InterpreterResult;
use asciidots_core::presentation::{
    AppCommand, DotsView, GridCell, GridCellKind, GridModel, ListCell, ListCellKind, ListModel,
    OutputEntry, OutputListModel, RuntimeController, TableCell, TableCellKind, TableModel,
    apply_result_to_output,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::widgets::Paragraph;
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, Widget,
    },
};

// ╔══════════════════════════════════════╗
// ║                                      ║
// ║                Colors                ║
// ║                                      ║
// ╚══════════════════════════════════════╝

pub struct RGB {
    r: u8,
    g: u8,
    b: u8,
}

impl RGB {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    // pastel palette
    pub const PINK: RGB = RGB::new(255, 182, 193);
    pub const LAVENDER: RGB = RGB::new(200, 182, 255);
    pub const MINT: RGB = RGB::new(182, 255, 214);
    pub const PEACH: RGB = RGB::new(255, 218, 182);
    pub const SKY: RGB = RGB::new(182, 228, 255);
    pub const LEMON: RGB = RGB::new(255, 255, 182);
    pub const LILAC: RGB = RGB::new(221, 182, 255);
    pub const CORAL: RGB = RGB::new(255, 182, 182);
    pub const SAGE: RGB = RGB::new(182, 255, 182);
    pub const PERIWINKLE: RGB = RGB::new(182, 196, 255);
    // grays
    pub const WHITE: RGB = RGB::new(255, 255, 255);
    pub const GRAY_100: RGB = RGB::new(240, 240, 240);
    pub const GRAY_200: RGB = RGB::new(220, 220, 220);
    pub const GRAY_300: RGB = RGB::new(180, 180, 180);
    pub const GRAY_400: RGB = RGB::new(140, 140, 140);
    pub const GRAY_500: RGB = RGB::new(100, 100, 100);
    pub const GRAY_600: RGB = RGB::new(70, 70, 70);
    pub const GRAY_700: RGB = RGB::new(50, 50, 50);
    pub const GRAY_800: RGB = RGB::new(30, 30, 30);
    pub const GRAY_900: RGB = RGB::new(15, 15, 15);
    pub const BLACK: RGB = RGB::new(0, 0, 0);
}

// ╔══════════════════════════════════════╗
// ║                                      ║
// ║              From< X >               ║
// ║                                      ║
// ╚══════════════════════════════════════╝

impl From<RGB> for ratatui::style::Color {
    fn from(c: RGB) -> Self {
        ratatui::style::Color::Rgb(c.r, c.g, c.b)
    }
}

fn style_for_grid_cell_kind(kind: GridCellKind) -> Style {
    let foreground = match kind {
        GridCellKind::Default => RGB::GRAY_500,
        GridCellKind::Quote => RGB::MINT,
        GridCellKind::OperatorDelimiter => RGB::LEMON,
        GridCellKind::Operator => RGB::WHITE,
        GridCellKind::Command => RGB::PERIWINKLE,
        GridCellKind::ActiveDot => RGB::CORAL,
        GridCellKind::ActiveOperator => RGB::MINT,
    };

    Style::default().fg(foreground.into())
}

fn style_for_grid_cell(cell: GridCell) -> Style {
    style_for_grid_cell_kind(cell.kind)
}

fn style_for_table_cell_kind(kind: TableCellKind) -> Style {
    let foreground = match kind {
        TableCellKind::Default => RGB::GRAY_500,
        TableCellKind::Quote => RGB::MINT,
        TableCellKind::Changeable => RGB::LEMON,
        TableCellKind::Changing => RGB::WHITE,
        TableCellKind::Active => RGB::CORAL,
    };

    Style::default().fg(foreground.into())
}

fn style_for_table_cell(cell: &TableCell) -> Style {
    let mut style = style_for_table_cell_kind(cell.kind);
    if cell.bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if cell.underline {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if cell.strikethrough {
        style = style.add_modifier(Modifier::CROSSED_OUT);
    }
    style
}

fn style_for_list_cell_kind(kind: ListCellKind) -> Style {
    let foreground = match kind {
        ListCellKind::Default => RGB::GRAY_500,
        ListCellKind::Quote => RGB::MINT,
        ListCellKind::Changeable => RGB::LEMON,
        ListCellKind::Changing => RGB::WHITE,
        ListCellKind::Active => RGB::CORAL,
    };

    Style::default().fg(foreground.into())
}

fn style_for_list_cell(cell: &ListCell) -> Style {
    let mut style = style_for_list_cell_kind(cell.kind);
    if cell.bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if cell.underline {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if cell.strikethrough {
        style = style.add_modifier(Modifier::CROSSED_OUT);
    }
    style
}

// ╔══════════════════════════════════════╗
// ║                                      ║
// ║             Grid Widget              ║
// ║                                      ║
// ╚══════════════════════════════════════╝

pub struct GridWidget<'a> {
    grid_model: &'a dyn GridModel,
    scroll: (usize, usize),
}

impl<'a> Widget for GridWidget<'a> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        for y in 0..area.height {
            for x in 0..area.width {
                let xs = x as usize + self.scroll.0;
                let ys = y as usize + self.scroll.1;
                if let Some(cell) = buf.cell_mut(ratatui::layout::Position {
                    x: area.x + x,
                    y: area.y + y,
                }) {
                    if let Some(grid_cell) = self.grid_model.get_cell(xs, ys) {
                        let c = grid_cell.c;
                        let style = style_for_grid_cell(grid_cell);
                        cell.set_char(c);
                        cell.set_style(style);
                    } else {
                        cell.set_char(' ');
                        cell.set_bg(RGB::BLACK.into());
                    }
                }
            }
        }
    }
}

// ╔══════════════════════════════════════╗
// ║                                      ║
// ║            Ratatui View              ║
// ║                                      ║
// ╚══════════════════════════════════════╝

pub enum ViewMode {
    Normal,
    ProcessInput(u64),
}

pub struct RatatuiView {
    term: DefaultTerminal,
    grid_scroll: (usize, usize),
    output_scroll: usize,
    output_follow: bool,
    output_entries: Vec<OutputEntry>,
    current_output_line: String,
    running: bool,
    mode: ViewMode,
    input_string: String,
}

impl RatatuiView {
    pub fn new() -> Self {
        RatatuiView {
            term: ratatui::init(),
            grid_scroll: (0, 0),
            output_scroll: 0,
            output_follow: true,
            output_entries: Vec::new(),
            current_output_line: String::new(),
            running: false,
            mode: ViewMode::Normal,
            input_string: String::new(),
        }
    }

    fn process_results(&mut self, results: &[InterpreterResult]) {
        for result in results {
            match result {
                InterpreterResult::InputRequest(uuid) => {
                    self.input_string.clear();
                    self.mode = ViewMode::ProcessInput(*uuid);
                }
                _ => {}
            }
            apply_result_to_output(
                &mut self.output_entries,
                &mut self.current_output_line,
                result,
            );
        }
    }

    fn clear_output(&mut self) {
        self.output_entries.clear();
        self.current_output_line.clear();
        self.output_scroll = 0;
        self.output_follow = true;
    }

    fn handle_key_event(&mut self, event: &KeyEvent) -> Option<AppCommand> {
        match self.mode {
            ViewMode::Normal => match event {
                KeyEvent {
                    code: KeyCode::Esc,
                    kind: KeyEventKind::Press,
                    ..
                } => Some(AppCommand::StopProgram),
                KeyEvent {
                    code: KeyCode::Char('w'),
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.grid_scroll = (self.grid_scroll.0, self.grid_scroll.1.saturating_sub(1));
                    None
                }
                KeyEvent {
                    code: KeyCode::Char('s'),
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.grid_scroll = (self.grid_scroll.0, self.grid_scroll.1.saturating_add(1));
                    None
                }
                KeyEvent {
                    code: KeyCode::Char('a'),
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.grid_scroll = (self.grid_scroll.0.saturating_sub(1), self.grid_scroll.1);
                    None
                }
                KeyEvent {
                    code: KeyCode::Char('d'),
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.grid_scroll = (self.grid_scroll.0.saturating_add(1), self.grid_scroll.1);
                    None
                }
                KeyEvent {
                    code: KeyCode::Char(' '),
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.running = !self.running;
                    if self.running {
                        Some(AppCommand::Start)
                    } else {
                        Some(AppCommand::Pause)
                    }
                }
                KeyEvent {
                    code: KeyCode::Char('r'),
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.running = false;
                    Some(AppCommand::Reset)
                }
                KeyEvent {
                    code: KeyCode::Char('n'),
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.running = false;
                    Some(AppCommand::Step)
                }
                KeyEvent {
                    code: KeyCode::Char('j'),
                    kind: KeyEventKind::Press,
                    ..
                } => Some(AppCommand::AdjustSpeed(10)),
                KeyEvent {
                    code: KeyCode::Char('k'),
                    kind: KeyEventKind::Press,
                    ..
                } => Some(AppCommand::AdjustSpeed(-10)),
                KeyEvent {
                    code: KeyCode::Char('u'),
                    kind: KeyEventKind::Press,
                    ..
                } => Some(AppCommand::AdjustStepsPerFrame(-1)),
                KeyEvent {
                    code: KeyCode::Char('i'),
                    kind: KeyEventKind::Press,
                    ..
                } => Some(AppCommand::AdjustStepsPerFrame(1)),
                KeyEvent {
                    code: KeyCode::Up,
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.output_follow = false;
                    self.output_scroll = self.output_scroll.saturating_sub(1);
                    None
                }
                KeyEvent {
                    code: KeyCode::Down,
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.output_scroll = self.output_scroll.saturating_add(1);
                    None
                }
                KeyEvent {
                    code: KeyCode::PageUp,
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.output_follow = false;
                    self.output_scroll = self.output_scroll.saturating_sub(10);
                    None
                }
                KeyEvent {
                    code: KeyCode::PageDown,
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.output_scroll = self.output_scroll.saturating_add(10);
                    None
                }
                _ => None,
            },
            ViewMode::ProcessInput(uuid) => match event {
                KeyEvent {
                    code: KeyCode::Esc,
                    kind: KeyEventKind::Press,
                    ..
                } => Some(AppCommand::StopProgram),
                KeyEvent {
                    code: KeyCode::Enter,
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.mode = ViewMode::Normal;
                    Some(AppCommand::InputAnswer(
                        uuid,
                        self.input_string
                            .trim()
                            .parse()
                            .expect("Something went wrong while parsing input"),
                    ))
                }
                KeyEvent {
                    code: KeyCode::Char(c),
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    if c.is_ascii_digit() || *c == '.' && !self.input_string.contains(*c) {
                        self.input_string.push(*c);
                        None
                    } else {
                        None
                    }
                }
                _ => None,
            },
        }
    }
}

impl Default for RatatuiView {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RatatuiView {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

impl DotsView for RatatuiView {
    fn render(&mut self, controller: &RuntimeController) {
        self.term
            .draw(|frame| {
                let area = frame.area();
                let grid_model = controller.grid_model();
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(Constraint::from_ratios([(2, 3), (1, 3)]))
                    .split(area);
                let side_rows = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(28),
                        Constraint::Percentage(28),
                        Constraint::Percentage(44),
                    ])
                    .split(cols[1]);
                let program_block = Block::default()
                    .title("Program")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(RGB::LAVENDER.into()));
                let program_inner = program_block.inner(cols[0]);

                frame.render_widget(program_block, cols[0]);
                frame.render_widget(Clear, program_inner);
                frame.render_widget(
                    GridWidget {
                        grid_model: &grid_model,
                        scroll: self.grid_scroll,
                    },
                    program_inner,
                );
                let settings_block = Block::default()
                    .title("Status")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(RGB::SKY.into()));
                let settings_inner = settings_block.inner(side_rows[0]);
                frame.render_widget(settings_block, side_rows[0]);
                let settings_model = controller.settings_table_model();

                let header_cells = (0..settings_model.column_count())
                    .map(|column| {
                        let text = settings_model.header(column).unwrap_or("");
                        Cell::from(text).style(
                            Style::default()
                                .fg(RGB::SKY.into())
                                .add_modifier(Modifier::BOLD),
                        )
                    })
                    .collect::<Vec<_>>();

                let rows = (0..settings_model.row_count())
                    .map(|row| {
                        let cells = (0..settings_model.column_count())
                            .map(|column| {
                                if let Some(table_cell) = settings_model.get_cell(row, column) {
                                    let style = style_for_table_cell(&table_cell);
                                    Cell::from(table_cell.text).style(style)
                                } else {
                                    Cell::from("")
                                }
                            })
                            .collect::<Vec<_>>();
                        Row::new(cells)
                    })
                    .collect::<Vec<_>>();

                let table = Table::new(
                    rows,
                    [Constraint::Percentage(35), Constraint::Percentage(65)],
                )
                .header(Row::new(header_cells))
                .column_spacing(1);

                frame.render_widget(table, settings_inner);

                let dots_block = Block::default()
                    .title("Dots")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(RGB::PEACH.into()));
                let dots_inner = dots_block.inner(side_rows[1]);
                frame.render_widget(dots_block, side_rows[1]);
                let dots_model = controller.dot_debug_table_model();

                let dot_header_cells = (0..dots_model.column_count())
                    .map(|column| {
                        let text = dots_model.header(column).unwrap_or("");
                        Cell::from(text).style(
                            Style::default()
                                .fg(RGB::PEACH.into())
                                .add_modifier(Modifier::BOLD),
                        )
                    })
                    .collect::<Vec<_>>();

                let dot_rows = (0..dots_model.row_count())
                    .map(|row| {
                        let cells = (0..dots_model.column_count())
                            .map(|column| {
                                if let Some(table_cell) = dots_model.get_cell(row, column) {
                                    let style = style_for_table_cell(&table_cell);
                                    Cell::from(table_cell.text).style(style)
                                } else {
                                    Cell::from("")
                                }
                            })
                            .collect::<Vec<_>>();
                        Row::new(cells)
                    })
                    .collect::<Vec<_>>();

                let dots_table = Table::new(
                    dot_rows,
                    [
                        Constraint::Length(4),
                        Constraint::Length(8),
                        Constraint::Length(4),
                        Constraint::Length(4),
                        Constraint::Min(6),
                    ],
                )
                .header(Row::new(dot_header_cells))
                .column_spacing(1);

                frame.render_widget(dots_table, dots_inner);

                let output_block = Block::default()
                    .title("Output")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(RGB::SAGE.into()));
                let output_inner = output_block.inner(side_rows[2]);
                frame.render_widget(output_block, side_rows[2]);
                let output_cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Min(1), Constraint::Length(1)])
                    .split(output_inner);
                let output_model =
                    OutputListModel::from_output(&self.output_entries, &self.current_output_line);
                let visible_count = output_cols[0].height as usize;
                let total_items = output_model.item_count();
                let max_scroll = total_items.saturating_sub(visible_count);
                if self.output_follow {
                    self.output_scroll = max_scroll;
                } else {
                    self.output_scroll = self.output_scroll.min(max_scroll);
                    if self.output_scroll >= max_scroll {
                        self.output_follow = true;
                    }
                }
                let scroll = self.output_scroll;
                let end = (scroll + visible_count).min(total_items);
                let items = (scroll..end)
                    .filter_map(|index| output_model.get_item(index))
                    .map(|cell| {
                        let style = style_for_list_cell(&cell);
                        ListItem::new(cell.text).style(style)
                    })
                    .collect::<Vec<_>>();
                let list = List::new(items);
                frame.render_widget(list, output_cols[0]);

                let mut scrollbar_state = ScrollbarState::default()
                    .content_length(total_items.max(1))
                    .position(scroll);
                frame.render_stateful_widget(
                    Scrollbar::new(ScrollbarOrientation::VerticalRight),
                    output_cols[1],
                    &mut scrollbar_state,
                );

                match self.mode {
                    ViewMode::ProcessInput(uuid) => {
                        let input_area = area.centered(Constraint::Max(40), Constraint::Length(4));
                        let input_text = Paragraph::new(self.input_string.as_str()).block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(format!("Input for Dot({uuid})")),
                        );
                        frame.render_widget(Clear, input_area);
                        frame.render_widget(input_text, input_area);
                    }
                    _ => {}
                }
            })
            .unwrap();
    }

    fn poll_command(&mut self) -> Option<AppCommand> {
        if event::poll(Duration::from_millis(1)).unwrap() {
            match event::read().unwrap() {
                Event::Key(event) => self.handle_key_event(&event),
                _ => None,
            }
        } else {
            None
        }
    }
}

pub fn run(content: &str) {
    let mut app = RuntimeController::new(content).unwrap();
    let mut view = RatatuiView::new();
    loop {
        let command = view.poll_command();
        if let Some(c) = command {
            if matches!(c, AppCommand::StopProgram) {
                break;
            }
            if matches!(c, AppCommand::Reset) {
                view.clear_output();
            }
            let results = app.handle_command(c);
            view.process_results(&results);
        }
        if matches!(view.mode, ViewMode::Normal) {
            let results = app.tick();
            view.process_results(&results);
        }
        view.render(&app)
    }
}
