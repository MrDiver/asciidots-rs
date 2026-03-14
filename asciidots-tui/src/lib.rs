use std::time::{Duration, Instant};

use asciidots_core::{
    dot::Position,
    interpreter::{AsciiField, DotsInterpreter, InterpreterResult},
    parser::LexError,
    state_machine::DotMachine,
    token::Token,
};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame, Terminal,
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::Text,
    widgets::{Block, Borders, Clear, Widget},
};
use rustc_hash::FxHashSet;

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
// ║              GridModel               ║
// ║                                      ║
// ╚══════════════════════════════════════╝

pub struct GridCell {
    c: char,
    foreground: RGB,
    background: Option<RGB>,
}

impl GridCell {
    fn new(c: char, foreground: RGB, background: Option<RGB>) -> Self {
        GridCell {
            c,
            foreground,
            background,
        }
    }
}

impl Default for GridCell {
    fn default() -> Self {
        Self {
            c: ' ',
            foreground: RGB {
                r: 255,
                g: 255,
                b: 255,
            },
            background: None,
        }
    }
}

pub trait GridModel {
    fn width(&self) -> usize;
    fn height(&self) -> usize;
    fn get_cell(&self, x: usize, y: usize) -> Option<GridCell>;
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

impl From<GridCell> for ratatui::style::Style {
    fn from(value: GridCell) -> Self {
        let mut style = Style::default().fg(value.foreground.into());
        if let Some(c) = value.background {
            style = style.bg(c.into());
        }
        style
    }
}

// ╔══════════════════════════════════════╗
// ║                                      ║
// ║           Ascii Map Model            ║
// ║           (Impl GridModel)           ║
// ╚══════════════════════════════════════╝

pub struct AsciiMapModel<'a> {
    field: &'a AsciiField,
    dots: &'a [DotMachine],
    dot_positions: FxHashSet<(usize, usize)>,
}

impl<'a> AsciiMapModel<'a> {
    fn new(field: &'a AsciiField, dots: &'a [DotMachine]) -> Self {
        let dot_positions: FxHashSet<(usize, usize)> = dots
            .iter()
            .map(|d| (d.position().x as usize, d.position().y as usize))
            .collect();
        AsciiMapModel {
            field,
            dots,
            dot_positions,
        }
    }
}

impl<'a> GridModel for AsciiMapModel<'a> {
    fn width(&self) -> usize {
        self.field.width()
    }

    fn height(&self) -> usize {
        self.field.height()
    }

    fn get_cell(&self, x: usize, y: usize) -> Option<GridCell> {
        let token = self.field.at(&Position {
            x: x as i32,
            y: y as i32,
        });
        let (mut foreground, background) = match token {
            Token::Quote => (RGB::MINT, None),
            Token::OperatorBegin(_) | Token::OperatorEnd(_) => (RGB::LEMON, None),
            Token::Operator { .. } => (RGB::WHITE, None),
            _ => (RGB::GRAY_500, None),
        };

        if self.dot_positions.contains(&(x, y)){
            foreground = RGB::CORAL;
        }
        let c = char::from(token);
        Some(GridCell::new(c, foreground, background))
    }
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
                        let style: Style = grid_cell.into();
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
// ║            App Controller            ║
// ║                                      ║
// ╚══════════════════════════════════════╝
pub struct AppController {
    interpreter: DotsInterpreter,
    source: String,
    tick_speed: Duration,
    steps_per_frame: usize,
    running: bool,
    clock: Instant,
}

pub enum AppCommand {
    Step,
    Reverse,
    Pause,
    Start,
    Reset,
    SetSpeed(Duration),
    StopProgram,
}

impl AppController {
    pub fn new(content: &str) -> Result<Self, Vec<LexError>> {
        let interpreter = DotsInterpreter::from_string(content)?;
        Ok(Self { interpreter, source: content.to_string(), tick_speed: Duration::from_millis(100), steps_per_frame: 1, running: false, clock: Instant::now() })
    }

    pub fn reset(&mut self) {
        match DotsInterpreter::from_string(&self.source) {
            Ok(interpreter) => {
                self.interpreter = interpreter;
                self.running = false;
                self.clock = Instant::now();
            }
            Err(_) => {} // source was valid at construction, so this shouldn't happen
        }
    }

    pub fn tick(&mut self) -> Vec<InterpreterResult>{
        if self.running && self.clock.elapsed() >= self.tick_speed {
            for _ in 0..self.steps_per_frame {
                let results:Vec<InterpreterResult>= self.interpreter.step().unwrap().iter().map(|r| r.clone()).collect();
                if results.len() > 0 {
                    return results;
                }
            }
            self.clock = Instant::now();
        }
        Vec::new()
    }

    pub fn handle_command(&mut self, command: AppCommand) {
        match command {
            AppCommand::Step => {
                self.interpreter.step().unwrap();
            }
            AppCommand::Reverse => {
                todo!()
            }
            AppCommand::Pause => {
                self.running = false;
            }
            AppCommand::Start => {
                self.running = true;
            }
            AppCommand::SetSpeed(speed) => {
                self.tick_speed = speed;
            }
            AppCommand::Reset => {
                self.reset();
            }
            AppCommand::StopProgram => {
                // Handled in main loop
            }
        } 
    }

    pub fn get_grid_model(&self) -> AsciiMapModel<'_> {
        AsciiMapModel::new(self.interpreter.get_field(), self.interpreter.get_dots())
    }
}

// ╔══════════════════════════════════════╗
// ║                                      ║
// ║             View Trait               ║
// ║                                      ║
// ╚══════════════════════════════════════╝

pub trait DotsView {
    fn render(&mut self, controller: &AppController);
    fn poll_command(&mut self) -> Option<AppCommand>;
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
    fn render(&mut self, controller: &AppController) {
        self.term
            .draw(|frame| {
                let area = frame.area();
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
                // Clear the inner area
                frame.render_widget(Clear, program_inner);
                frame.render_widget(
                    GridWidget {
                        grid_model: &controller.get_grid_model(),
                        scroll: self.grid_scroll, // TODO: Make scrollable
                    },
                    program_inner,
                );
                // Settings Panel
                let settings_block = Block::default()
                        .title("Status")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(RGB::SKY.into()));
                let settings_inner = settings_block.inner(cols[1]);
                frame.render_widget(
                    settings_block,
                    cols[1],
                );
                let text = Text::raw(format!(
                    "Running: {}\nTick Speed: {:?}\nSteps/Frame: {}\nDots: {}",
                    if controller.running { "Yes" } else { "No" },
                    controller.tick_speed,
                    controller.steps_per_frame,
                    controller.interpreter.num_dots(),
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
    let mut app = AppController::new(content).unwrap();
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

