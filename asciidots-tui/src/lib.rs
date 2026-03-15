use std::time::Duration;

use asciidots_core::presentation::{
    AppCommand, DotsView, GridCell, GridCellKind, GridModel, RuntimeController,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::Text,
    widgets::{Block, Borders, Clear, Widget},
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
        GridCellKind::ActiveDot => RGB::CORAL,
    };

    Style::default().fg(foreground.into())
}

fn style_for_grid_cell(cell: GridCell) -> Style {
    style_for_grid_cell_kind(cell.kind)
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
                    }else{
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

pub struct RatatuiView {
    term: DefaultTerminal,
    grid_scroll: (usize, usize),
    running: bool,
}

impl RatatuiView {
    pub fn new() -> Self {
        RatatuiView {
            term: ratatui::init(),
            grid_scroll: (0, 0),
            running: false,
        }
    }

    fn handle_key_event(&mut self, event: &KeyEvent) -> Option<AppCommand> {
        match event {
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
            _ => None,
        }
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
                let settings_inner = settings_block.inner(cols[1]);
                frame.render_widget(settings_block, cols[1]);
                let text = Text::raw(format!(
                    "Running: {}\nTick Speed: {} ms\nSteps/Tick: {}\nDots: {}\n\nControls:\nSpace: Run/Pause\nn: Single Step\nr: Reset\nj/k: Delay +/-10ms\nu/i: Steps/Tick -/+1\nwasd: Scroll\nEsc: Quit",
                    if controller.is_running() { "Yes" } else { "No" },
                    controller.tick_speed().as_millis(),
                    controller.steps_per_frame(),
                    controller.num_dots(),
                ));
                frame.render_widget(text, settings_inner);
            })
            .unwrap();
    }

    fn poll_command(&mut self) -> Option<AppCommand> {
        if event::poll(Duration::from_millis(1)).unwrap(){
            match event::read().unwrap() {
                Event::Key(event) => self.handle_key_event(&event),
                _ => None,
            }
        }else{
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
            app.handle_command(c);
        }
        app.tick();
        view.render(&app)
    }
}

