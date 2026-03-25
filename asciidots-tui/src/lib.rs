use std::{path::Path, time::Duration};

use asciidots_core::interpreter::InterpreterResult;
use asciidots_core::presentation::{
    AppCommand, DotsView, GridCell, GridCellKind, GridModel, ListCell, ListCellKind, ListModel,
    OutputEntry, OutputListModel, RuntimeController, TableCell, TableCellKind, TableModel,
    apply_result_to_output,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use miette::NamedSource;
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

    // ── pinks & reds ──────────────────────────────────────────────
    pub const ROSE: RGB = RGB::new(255, 120, 150);
    pub const BLUSH: RGB = RGB::new(255, 145, 175);
    pub const BUBBLEGUM: RGB = RGB::new(255, 120, 200);
    pub const CARNATION: RGB = RGB::new(255, 110, 130);
    pub const WATERMELON: RGB = RGB::new(255, 100, 120);

    // ── oranges ───────────────────────────────────────────────────
    pub const PEACH: RGB = RGB::new(255, 190, 120);
    pub const APRICOT: RGB = RGB::new(255, 175, 100);
    pub const CANTALOUPE: RGB = RGB::new(255, 185, 95);
    pub const TANGERINE: RGB = RGB::new(255, 160, 80);
    pub const SHERBET: RGB = RGB::new(255, 180, 130);

    // ── yellows ───────────────────────────────────────────────────
    pub const LEMON: RGB = RGB::new(255, 245, 100);
    pub const BUTTER: RGB = RGB::new(255, 240, 130);
    pub const BANANA: RGB = RGB::new(255, 235, 90);
    pub const CREAM: RGB = RGB::new(255, 245, 160);
    pub const CUSTARD: RGB = RGB::new(255, 230, 110);

    // ── yellow greens ─────────────────────────────────────────────
    pub const HONEYDEW: RGB = RGB::new(190, 255, 130);
    pub const PISTACHIO: RGB = RGB::new(175, 245, 110);
    pub const LIME_SORBET: RGB = RGB::new(180, 255, 100);
    pub const CELERY: RGB = RGB::new(170, 230, 120);

    // ── greens ────────────────────────────────────────────────────
    pub const MINT: RGB = RGB::new(100, 255, 180);
    pub const SAGE: RGB = RGB::new(130, 220, 130);
    pub const SEAFOAM: RGB = RGB::new(100, 240, 190);
    pub const SPEARMINT: RGB = RGB::new(120, 235, 170);
    pub const MATCHA: RGB = RGB::new(145, 210, 140);

    // ── teals & cyans ─────────────────────────────────────────────
    pub const AQUA: RGB = RGB::new(100, 235, 220);
    pub const TEAL_MIST: RGB = RGB::new(100, 220, 210);
    pub const ICE: RGB = RGB::new(130, 240, 235);
    pub const GLACIER: RGB = RGB::new(140, 225, 225);
    pub const POOL: RGB = RGB::new(100, 210, 230);

    // ── blues ─────────────────────────────────────────────────────
    pub const SKY: RGB = RGB::new(100, 185, 255);
    pub const BABY_BLUE: RGB = RGB::new(120, 175, 255);
    pub const CORNFLOWER: RGB = RGB::new(120, 155, 255);
    pub const POWDER: RGB = RGB::new(140, 185, 255);
    pub const MIST: RGB = RGB::new(150, 175, 240);

    // ── blue purples ──────────────────────────────────────────────
    pub const PERIWINKLE: RGB = RGB::new(130, 140, 255);
    pub const BLUEBELL: RGB = RGB::new(140, 135, 255);
    pub const IRIS: RGB = RGB::new(150, 130, 255);
    pub const HYACINTH: RGB = RGB::new(160, 130, 255);
    pub const WISTERIA: RGB = RGB::new(170, 130, 255);

    // ── purples ───────────────────────────────────────────────────
    pub const LAVENDER: RGB = RGB::new(185, 130, 255);
    pub const LILAC: RGB = RGB::new(200, 130, 255);
    pub const AMETHYST: RGB = RGB::new(190, 120, 255);
    pub const THISTLE: RGB = RGB::new(195, 140, 255);
    pub const VIOLET: RGB = RGB::new(175, 115, 255);

    // ── pink purples ──────────────────────────────────────────────
    pub const ORCHID: RGB = RGB::new(220, 120, 255);
    pub const PLUM_BLOSSOM: RGB = RGB::new(230, 120, 245);
    pub const MAUVE: RGB = RGB::new(220, 130, 225);
    pub const COTTON_CANDY: RGB = RGB::new(255, 140, 230);
    pub const PINK_LAVENDER: RGB = RGB::new(245, 130, 215);

    // ── neutrals ──────────────────────────────────────────────────
    pub const WHITE: RGB = RGB::new(255, 255, 255);
    pub const GRAY_100: RGB = RGB::new(230, 230, 235);
    pub const GRAY_200: RGB = RGB::new(200, 200, 210);
    pub const GRAY_300: RGB = RGB::new(165, 165, 175);
    pub const GRAY_400: RGB = RGB::new(130, 130, 145);
    pub const GRAY_500: RGB = RGB::new(95, 95, 110);
    pub const GRAY_600: RGB = RGB::new(70, 70, 85);
    pub const GRAY_700: RGB = RGB::new(48, 48, 60);
    pub const GRAY_800: RGB = RGB::new(28, 28, 38);
    pub const GRAY_900: RGB = RGB::new(14, 14, 20);
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
        GridCellKind::Quote => RGB::SEAFOAM,
        GridCellKind::OperatorDelimiter => RGB::IRIS,
        GridCellKind::Operator => RGB::PERIWINKLE,
        GridCellKind::Command => RGB::WISTERIA,
        GridCellKind::ActiveDot => RGB::CARNATION,
        GridCellKind::ActiveOperator => RGB::HONEYDEW,
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
        TableCellKind::Active => RGB::PERIWINKLE,
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
        ListCellKind::Default => RGB::GRAY_200,
        ListCellKind::Quote => RGB::MINT,
        ListCellKind::Changeable => RGB::LEMON,
        ListCellKind::Changing => RGB::WHITE,
        ListCellKind::Active => RGB::PERIWINKLE,
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
            if let InterpreterResult::InputRequest(uuid) = result {
                self.input_string.clear();
                self.mode = ViewMode::ProcessInput(*uuid);
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
                } => Some(AppCommand::AdjustStepsPerFrame(-100)),
                KeyEvent {
                    code: KeyCode::Char('i'),
                    kind: KeyEventKind::Press,
                    ..
                } => Some(AppCommand::AdjustStepsPerFrame(100)),
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
                    (0..dots_model.column_count()).map(|_column| Constraint::Min(4)),
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

                if let ViewMode::ProcessInput(uuid) = self.mode {
                    let input_area = area.centered(Constraint::Max(40), Constraint::Length(4));
                    let input_text = Paragraph::new(self.input_string.as_str()).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!("Input for Dot({uuid})")),
                    );
                    frame.render_widget(Clear, input_area);
                    frame.render_widget(input_text, input_area);
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

pub fn run(content: &str) -> miette::Result<()> {
    let mut app = RuntimeController::new(content)
        .map_err(|e| e.with_source_code(NamedSource::new("Source", content.to_string())))?;
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
    Ok(())
}

pub fn run_file(path: impl AsRef<Path>) -> miette::Result<()> {
    let path_ref = path.as_ref();
    let mut app = RuntimeController::new_from_file(path_ref)?;
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
    Ok(())
}
