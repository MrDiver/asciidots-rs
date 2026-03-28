use std::{
    path::Path,
    sync::{Arc, RwLock, mpsc},
    thread,
    time::{Duration, Instant},
};

use asciidots_core::display::DisplayBackend;
use asciidots_core::interpreter::InterpreterResult;
use asciidots_core::presentation::{
    AppCommand, DotsView, GridCell, GridCellKind, GridModel, ListCell, ListCellKind, ListModel,
    OutputEntry, OutputListModel, RuntimeController, TableCell, TableCellKind, TableModel,
    apply_result_to_output,
};
use asciidots_display_minifb::MinifbDisplayBackend;
use asciidots_display_ratatui::{DisplayBuffer, DisplayWidget, RatatuiDisplayBackend};
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEventKind,
    EnableMouseCapture, DisableMouseCapture,
};
use rustc_hash::FxHashMap;
use std::io::stdout;
use miette::NamedSource;
use ratatui::widgets::Paragraph;
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Layout, Position as LayoutPosition, Rect},
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
        GridCellKind::ActiveOperatorWithDots => RGB::SPEARMINT,
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Program,
    Displays,
}

pub enum TuiDisplayMode {
    Minifb,
    Ratatui,
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
    display_mode: TuiDisplayMode,
    display_backends: FxHashMap<u32, Box<dyn DisplayBackend>>,
    ratatui_buffers: FxHashMap<u32, Arc<RwLock<DisplayBuffer>>>,
    active_tab: ActiveTab,
    left_width_pct: u16,
    side_split_pct: u16,
    dots_scroll: usize,
    grid_rect: Rect,
    dots_rect: Rect,
    output_rect: Rect,
    settings_rect: Rect,
}

impl RatatuiView {
    pub fn new() -> Self {
        Self::with_display_mode(TuiDisplayMode::Ratatui)
    }

    pub fn with_display_mode(display_mode: TuiDisplayMode) -> Self {
        crossterm::execute!(stdout(), EnableMouseCapture).ok();
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
            display_mode,
            display_backends: FxHashMap::default(),
            ratatui_buffers: FxHashMap::default(),
            active_tab: ActiveTab::Program,
            left_width_pct: 67,
            side_split_pct: 56,
            dots_scroll: 0,
            grid_rect: Rect::default(),
            dots_rect: Rect::default(),
            output_rect: Rect::default(),
            settings_rect: Rect::default(),
        }
    }

    fn process_results(&mut self, results: &[InterpreterResult]) {
        for result in results {
            if let InterpreterResult::InputRequest(uuid) = result {
                self.input_string.clear();
                self.mode = ViewMode::ProcessInput(*uuid);
            }
            if let InterpreterResult::Display { display_id, command } = result {
                let id = *display_id;
                if !self.display_backends.contains_key(&id) {
                    let backend: Box<dyn DisplayBackend> = match self.display_mode {
                        TuiDisplayMode::Minifb => Box::new(MinifbDisplayBackend::new()),
                        TuiDisplayMode::Ratatui => {
                            let backend = RatatuiDisplayBackend::new();
                            self.ratatui_buffers.insert(id, backend.buffer());
                            Box::new(backend)
                        }
                    };
                    self.display_backends.insert(id, backend);
                }
                self.display_backends.get_mut(&id).unwrap().handle(command.clone());
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
        self.dots_scroll = 0;
        self.output_follow = true;
        self.display_backends.clear();
        self.ratatui_buffers.clear();
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
                    code: KeyCode::Tab,
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.active_tab = match self.active_tab {
                        ActiveTab::Program => ActiveTab::Displays,
                        ActiveTab::Displays => ActiveTab::Program,
                    };
                    None
                }
                KeyEvent {
                    code: KeyCode::Char('['),
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.left_width_pct = self.left_width_pct.saturating_sub(5).max(20);
                    None
                }
                KeyEvent {
                    code: KeyCode::Char(']'),
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.left_width_pct = (self.left_width_pct + 5).min(85);
                    None
                }
                KeyEvent {
                    code: KeyCode::Char('-'),
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.side_split_pct = self.side_split_pct.saturating_sub(5).max(20);
                    None
                }
                KeyEvent {
                    code: KeyCode::Char('='),
                    kind: KeyEventKind::Press,
                    ..
                } => {
                    self.side_split_pct = (self.side_split_pct + 5).min(80);
                    None
                }
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
        crossterm::execute!(stdout(), DisableMouseCapture).ok();
        ratatui::restore();
    }
}

impl RatatuiView {
    fn scroll_at(&mut self, col: u16, row: u16, delta: i32, shift: bool) {
        let pos = LayoutPosition { x: col, y: row };
        if self.grid_rect.contains(pos) {
            if shift {
                self.grid_scroll.0 = (self.grid_scroll.0 as i32 + delta).max(0) as usize;
            } else {
                self.grid_scroll.1 = (self.grid_scroll.1 as i32 + delta).max(0) as usize;
            }
        } else if self.output_rect.contains(pos) {
            self.output_follow = false;
            self.output_scroll = (self.output_scroll as i32 + delta).max(0) as usize;
        } else if self.dots_rect.contains(pos) {
            self.dots_scroll = (self.dots_scroll as i32 + delta).max(0) as usize;
        }
    }
}

impl DotsView for RatatuiView {
    fn render(&mut self, controller: &RuntimeController) {
        let active_tab = self.active_tab;
        let left_width_pct = self.left_width_pct;
        let side_split_pct = self.side_split_pct;
        self.term
            .draw(|frame| {
                let area = frame.area();

                let ratatui_buffers: Vec<_> = {
                    let mut ids: Vec<u32> = self.ratatui_buffers.keys().copied().collect();
                    ids.sort();
                    ids.into_iter()
                        .filter_map(|id| self.ratatui_buffers.get(&id).map(|b| (id, Arc::clone(b))))
                        .collect()
                };

                // ── tab bar ──────────────────────────────────────────────
                let [tab_bar_area, content_area] = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Min(0)])
                    .areas(area);

                let tab_labels = ["[1] Program", "[2] Displays"];
                let tab_text = tab_labels
                    .iter()
                    .enumerate()
                    .map(|(i, label)| {
                        let is_active = match i {
                            0 => active_tab == ActiveTab::Program,
                            _ => active_tab == ActiveTab::Displays,
                        };
                        if is_active {
                            ratatui::text::Span::styled(
                                format!(" {label} "),
                                Style::default()
                                    .fg(RGB::BLACK.into())
                                    .bg(RGB::LAVENDER.into())
                                    .add_modifier(Modifier::BOLD),
                            )
                        } else {
                            ratatui::text::Span::styled(
                                format!(" {label} "),
                                Style::default().fg(RGB::GRAY_400.into()),
                            )
                        }
                    })
                    .collect::<Vec<_>>();
                frame.render_widget(
                    Paragraph::new(ratatui::text::Line::from(tab_text)),
                    tab_bar_area,
                );

                match active_tab {
                    ActiveTab::Displays => {
                        if ratatui_buffers.is_empty() {
                            frame.render_widget(
                                Paragraph::new("No displays active. Run a program with %display."),
                                content_area,
                            );
                        } else {
                            let count = ratatui_buffers.len() as u32;
                            let display_cols = Layout::default()
                                .direction(Direction::Horizontal)
                                .constraints(vec![Constraint::Ratio(1, count); ratatui_buffers.len()])
                                .split(content_area);
                            for ((id, buffer), panel_area) in
                                ratatui_buffers.iter().zip(display_cols.iter())
                            {
                                let display_block = Block::default()
                                    .title(format!("Display {id}"))
                                    .borders(Borders::ALL)
                                    .border_style(Style::default().fg(RGB::BUBBLEGUM.into()));
                                let display_inner = display_block.inner(*panel_area);
                                frame.render_widget(display_block, *panel_area);
                                frame.render_widget(
                                    DisplayWidget::new(Arc::clone(buffer)),
                                    display_inner,
                                );
                            }
                        }
                    }
                    ActiveTab::Program => {
                        let grid_model = controller.grid_model();
                        let right_pct = 100u16.saturating_sub(left_width_pct);
                        let cols = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([
                                Constraint::Percentage(left_width_pct),
                                Constraint::Percentage(right_pct),
                            ])
                            .split(content_area);

                        let side_rows = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Percentage(side_split_pct / 2),
                                Constraint::Percentage(side_split_pct / 2),
                                Constraint::Percentage(100 - side_split_pct),
                            ])
                            .split(cols[1]);

                        let left_rows = if ratatui_buffers.is_empty() {
                            vec![cols[0]]
                        } else {
                            Layout::default()
                                .direction(Direction::Vertical)
                                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                                .split(cols[0])
                                .to_vec()
                        };

                        let program_block = Block::default()
                            .title("Program")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(RGB::LAVENDER.into()));
                        let program_inner = program_block.inner(left_rows[0]);
                        self.grid_rect = left_rows[0];
                        frame.render_widget(program_block, left_rows[0]);
                        frame.render_widget(Clear, program_inner);
                        frame.render_widget(
                            GridWidget {
                                grid_model: &grid_model,
                                scroll: self.grid_scroll,
                            },
                            program_inner,
                        );

                        if !ratatui_buffers.is_empty() {
                            let display_area = left_rows[1];
                            let count = ratatui_buffers.len() as u32;
                            let display_cols = Layout::default()
                                .direction(Direction::Horizontal)
                                .constraints(vec![Constraint::Ratio(1, count); ratatui_buffers.len()])
                                .split(display_area);
                            for ((id, buffer), panel_area) in
                                ratatui_buffers.iter().zip(display_cols.iter())
                            {
                                let display_block = Block::default()
                                    .title(format!("Display {id}"))
                                    .borders(Borders::ALL)
                                    .border_style(Style::default().fg(RGB::BUBBLEGUM.into()));
                                let display_inner = display_block.inner(*panel_area);
                                frame.render_widget(display_block, *panel_area);
                                frame.render_widget(
                                    DisplayWidget::new(Arc::clone(buffer)),
                                    display_inner,
                                );
                            }
                        }

                        // ── Status ──────────────────────────────────────────
                        self.settings_rect = side_rows[0];
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

                        // ── Dots ─────────────────────────────────────────────
                        self.dots_rect = side_rows[1];
                        let dots_block = Block::default()
                            .title("Dots")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(RGB::PEACH.into()));
                        let dots_inner = dots_block.inner(side_rows[1]);
                        frame.render_widget(dots_block, side_rows[1]);
                        let dots_model = controller.dot_debug_table_model();
                        let total_dot_rows = dots_model.row_count();
                        // -1 for the header row
                        let visible_dot_rows = (dots_inner.height as usize).saturating_sub(1);
                        let max_dot_scroll = total_dot_rows.saturating_sub(visible_dot_rows);
                        self.dots_scroll = self.dots_scroll.min(max_dot_scroll);
                        let dot_start = self.dots_scroll;
                        let dot_end = (dot_start + visible_dot_rows).min(total_dot_rows);
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
                        let dots_cols = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([Constraint::Min(1), Constraint::Length(1)])
                            .split(dots_inner);
                        let dot_rows = (dot_start..dot_end)
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
                            (0..dots_model.column_count()).map(|_| Constraint::Min(4)),
                        )
                        .header(Row::new(dot_header_cells))
                        .column_spacing(1);
                        frame.render_widget(dots_table, dots_cols[0]);
                        let mut dot_scrollbar_state = ScrollbarState::default()
                            .content_length(total_dot_rows.max(1))
                            .position(dot_start);
                        frame.render_stateful_widget(
                            Scrollbar::new(ScrollbarOrientation::VerticalRight),
                            dots_cols[1],
                            &mut dot_scrollbar_state,
                        );

                        // ── Output ───────────────────────────────────────────
                        self.output_rect = side_rows[2];
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
                        let output_model = OutputListModel::from_output(
                            &self.output_entries,
                            &self.current_output_line,
                        );
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
                        frame.render_widget(List::new(items), output_cols[0]);
                        let mut scrollbar_state = ScrollbarState::default()
                            .content_length(total_items.max(1))
                            .position(scroll);
                        frame.render_stateful_widget(
                            Scrollbar::new(ScrollbarOrientation::VerticalRight),
                            output_cols[1],
                            &mut scrollbar_state,
                        );
                    }
                }

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
        if event::poll(Duration::ZERO).unwrap() {
            match event::read().unwrap() {
                Event::Key(event) => self.handle_key_event(&event),
                Event::Mouse(mouse) => {
                    let shift = mouse.modifiers.contains(KeyModifiers::SHIFT);
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            self.scroll_at(mouse.column, mouse.row, -3, shift);
                        }
                        MouseEventKind::ScrollDown => {
                            self.scroll_at(mouse.column, mouse.row, 3, shift);
                        }
                        _ => {}
                    }
                    None
                }
                _ => None,
            }
        } else {
            None
        }
    }
}

// ─── Interpreter thread ───────────────────────────────────────────────────────

fn interpreter_thread(
    shared: Arc<RwLock<RuntimeController>>,
    cmd_rx: mpsc::Receiver<AppCommand>,
    result_tx: mpsc::Sender<Vec<InterpreterResult>>,
) {
    loop {
        // Process any pending commands before stepping.
        let mut stop = false;
        while let Ok(cmd) = cmd_rx.try_recv() {
            if matches!(cmd, AppCommand::StopProgram) {
                stop = true;
                break;
            }
            let results = shared.write().unwrap().handle_command(cmd);
            if !results.is_empty() && result_tx.send(results).is_err() {
                return;
            }
        }
        if stop {
            break;
        }

        // Run steps for at most 2 ms, then release the lock so the UI thread
        // can always get a read lock within that window.
        let results = shared.write().unwrap().tick_budgeted(Duration::from_millis(2));

        if results.is_empty() {
            // Not running or tick_speed hasn't elapsed yet — avoid busy-spin.
            thread::sleep(Duration::from_millis(1));
            continue;
        }

        let needs_input = results
            .iter()
            .any(|r| matches!(r, InterpreterResult::InputRequest(_)));

        if result_tx.send(results).is_err() {
            break;
        }

        if needs_input {
            // Block until InputAnswer (or Stop/Reset) so we don't busy-loop.
            loop {
                let cmd = match cmd_rx.recv() {
                    Ok(cmd) => cmd,
                    Err(_) => return,
                };
                let is_answer = matches!(cmd, AppCommand::InputAnswer(..));
                let is_stop = matches!(cmd, AppCommand::StopProgram);
                let results = shared.write().unwrap().handle_command(cmd);
                if !results.is_empty() {
                    result_tx.send(results).ok();
                }
                if is_stop {
                    return;
                }
                if is_answer {
                    break;
                }
            }
        }
    }
}

// ─── Shared main-loop helper ──────────────────────────────────────────────────

fn run_with_controller(
    controller: RuntimeController,
    display_mode: TuiDisplayMode,
) -> miette::Result<()> {
    let shared: Arc<RwLock<RuntimeController>> = Arc::new(RwLock::new(controller));

    let (cmd_tx, cmd_rx) = mpsc::channel::<AppCommand>();
    let (result_tx, result_rx) = mpsc::channel::<Vec<InterpreterResult>>();

    let interp_shared = Arc::clone(&shared);
    let _interp = thread::spawn(move || {
        interpreter_thread(interp_shared, cmd_rx, result_tx);
    });

    let mut view = RatatuiView::with_display_mode(display_mode);
    let frame_budget = Duration::from_millis(16); // ~60 fps cap

    loop {
        let frame_start = Instant::now();

        // Drain all results the interpreter produced since last frame.
        while let Ok(results) = result_rx.try_recv() {
            view.process_results(&results);
        }

        // Handle all pending UI events (non-blocking).
        loop {
            match view.poll_command() {
                None => break,
                Some(AppCommand::StopProgram) => {
                    cmd_tx.send(AppCommand::StopProgram).ok();
                    return Ok(());
                }
                Some(AppCommand::Reset) => {
                    view.clear_output();
                    cmd_tx.send(AppCommand::Reset).ok();
                }
                Some(cmd) => {
                    cmd_tx.send(cmd).ok();
                }
            }
        }

        // Render — takes a read lock; interpreter is free to write between frames.
        {
            let ctrl = shared.read().unwrap();
            view.render(&ctrl);
        }

        // Sleep out the rest of the frame budget so we don't burn CPU at idle.
        let elapsed = frame_start.elapsed();
        if elapsed < frame_budget {
            thread::sleep(frame_budget - elapsed);
        }
    }
}

// ─── Public entry points ──────────────────────────────────────────────────────

pub fn run(content: &str) -> miette::Result<()> {
    let controller = RuntimeController::new(content)
        .map_err(|e| e.with_source_code(NamedSource::new("Source", content.to_string())))?;
    run_with_controller(controller, TuiDisplayMode::Ratatui)
}

pub fn run_file(path: impl AsRef<Path>, display_mode: TuiDisplayMode) -> miette::Result<()> {
    let controller = RuntimeController::new_from_file(path.as_ref())?;
    run_with_controller(controller, display_mode)
}
