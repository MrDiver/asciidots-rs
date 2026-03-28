use std::sync::{Arc, RwLock};

use asciidots_core::{
    display::DisplayBackend,
    interpreter::{ColorMode, DisplayCommand},
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    widgets::Widget,
};

#[derive(Debug, Clone)]
pub struct DisplayBuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
    pub mode: ColorMode,
}

impl DisplayBuffer {
    fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            pixels: Vec::new(),
            mode: ColorMode::default(),
        }
    }

    fn resize_if_needed(&mut self) {
        let len = self.width * self.height;
        if self.pixels.len() != len {
            self.pixels.resize(len, 0);
        }
    }
}

pub struct RatatuiDisplayBackend {
    buffer: Arc<RwLock<DisplayBuffer>>,
}

impl RatatuiDisplayBackend {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(RwLock::new(DisplayBuffer::new())),
        }
    }

    pub fn buffer(&self) -> Arc<RwLock<DisplayBuffer>> {
        Arc::clone(&self.buffer)
    }
}

impl Default for RatatuiDisplayBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayBackend for RatatuiDisplayBackend {
    fn handle(&mut self, command: DisplayCommand) {
        let Ok(mut buf) = self.buffer.write() else { return; };
        Self::apply_to_buf(&mut buf, command);
    }

    fn handle_batch(&mut self, commands: &[DisplayCommand]) {
        let Ok(mut buf) = self.buffer.write() else { return; };
        for command in commands {
            Self::apply_to_buf(&mut buf, command.clone());
        }
    }
}

impl RatatuiDisplayBackend {
    fn apply_to_buf(buf: &mut DisplayBuffer, command: DisplayCommand) {
        match command {
            DisplayCommand::SetWidth(w) => {
                buf.width = w;
                buf.resize_if_needed();
            }
            DisplayCommand::SetHeight(h) => {
                buf.height = h;
                buf.resize_if_needed();
            }
            DisplayCommand::SetColorMode(mode) => {
                buf.mode = mode;
            }
            DisplayCommand::SetPixel { index, value } => {
                if index < buf.pixels.len() {
                    buf.pixels[index] = value_to_rgb(buf.mode, value);
                }
            }
        }
    }
}

fn normalized_phase(value: f64) -> f64 {
    if !value.is_finite() { 0.0 } else { value.abs().fract() }
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = if h < 60.0 { (c, x, 0.0) }
        else if h < 120.0 { (x, c, 0.0) }
        else if h < 180.0 { (0.0, c, x) }
        else if h < 240.0 { (0.0, x, c) }
        else if h < 300.0 { (x, 0.0, c) }
        else { (c, 0.0, x) };
    let to_u8 = |v: f64| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    (to_u8(r1), to_u8(g1), to_u8(b1))
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if h < 60.0 { (c, x, 0.0) }
        else if h < 120.0 { (x, c, 0.0) }
        else if h < 180.0 { (0.0, c, x) }
        else if h < 240.0 { (0.0, x, c) }
        else if h < 300.0 { (x, 0.0, c) }
        else { (c, 0.0, x) };
    let to_u8 = |v: f64| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    (to_u8(r1), to_u8(g1), to_u8(b1))
}

fn value_to_rgb(mode: ColorMode, value: f64) -> u32 {
    let (r, g, b) = match mode {
        ColorMode::Rgb => {
            let bits = value.to_bits();
            let r = (bits & 0xFF) as u8;
            let g = ((bits >> 8) & 0xFF) as u8;
            let b = ((bits >> 16) & 0xFF) as u8;
            (r, g, b)
        }
        ColorMode::Hsv => {
            let hue = normalized_phase(value) * 360.0;
            hsv_to_rgb(hue, 1.0, 1.0)
        }
        ColorMode::Hsl => {
            let hue = normalized_phase(value) * 360.0;
            hsl_to_rgb(hue, 1.0, 0.5)
        }
        ColorMode::HueOnly => {
            let hue = normalized_phase(value) * 360.0;
            hsv_to_rgb(hue, 1.0, 1.0)
        }
        ColorMode::BlackAndWhite => {
            let v = if normalized_phase(value) >= 0.5 { 255u8 } else { 0u8 };
            (v, v, v)
        }
    };
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// A ratatui widget that renders a DisplayBuffer using half-block characters.
/// Each terminal cell renders two vertical pixels using '▀' with fg/bg colors.
pub struct DisplayWidget {
    buffer: Arc<RwLock<DisplayBuffer>>,
}

impl DisplayWidget {
    pub fn new(buffer: Arc<RwLock<DisplayBuffer>>) -> Self {
        Self { buffer }
    }
}

fn u32_to_ratatui_color(packed: u32) -> Color {
    let r = ((packed >> 16) & 0xFF) as u8;
    let g = ((packed >> 8) & 0xFF) as u8;
    let b = (packed & 0xFF) as u8;
    Color::Rgb(r, g, b)
}

impl Widget for DisplayWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let Ok(display) = self.buffer.read() else { return; };
        if display.width == 0 || display.height == 0 {
            return;
        }

        let cell_cols = area.width.min(display.width as u16);
        let cell_rows = area.height.min(((display.height + 1) / 2) as u16);

        for cell_y in 0..cell_rows {
            let top_row = (cell_y as usize) * 2;
            let bot_row = top_row + 1;

            for cell_x in 0..cell_cols {
                let px = cell_x as usize;
                let top_idx = top_row * display.width + px;
                let bot_idx = bot_row * display.width + px;

                let top_color = display.pixels.get(top_idx).copied().unwrap_or(0);
                let bot_color = display.pixels.get(bot_idx).copied().unwrap_or(0);

                let cell = buf.cell_mut((area.x + cell_x, area.y + cell_y));
                if let Some(cell) = cell {
                    cell.set_char('▀');
                    cell.fg = u32_to_ratatui_color(top_color);
                    cell.bg = u32_to_ratatui_color(bot_color);
                }
            }
        }
    }
}