use std::{
    env,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use asciidots_core::{
    display::DisplayBackend,
    interpreter::{ColorMode, DisplayCommand},
};
use minifb::{Window, WindowOptions};

struct DisplayShared {
    width: usize,
    height: usize,
    scale: usize,
    buffer: Vec<u32>,
}

struct DisplayWorker {
    shared: Arc<Mutex<DisplayShared>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl DisplayWorker {
    fn spawn(width: usize, height: usize, scale: usize) -> Option<Self> {
        let len = width.checked_mul(height)?;
        let shared = Arc::new(Mutex::new(DisplayShared {
            width,
            height,
            scale,
            buffer: vec![0; len],
        }));
        let stop = Arc::new(AtomicBool::new(false));

        let shared_thread = Arc::clone(&shared);
        let stop_thread = Arc::clone(&stop);

        let handle = thread::spawn(move || {
            let (initial_width, initial_height, initial_scale) = match shared_thread.lock() {
                Ok(shared) => (shared.width, shared.height, shared.scale),
                Err(_) => return,
            };

            let Some(initial_window_width) = initial_width.checked_mul(initial_scale) else {
                return;
            };
            let Some(initial_window_height) = initial_height.checked_mul(initial_scale) else {
                return;
            };

            let mut window = match Window::new(
                "Asciidots Display",
                initial_window_width,
                initial_window_height,
                WindowOptions {
                    resize: true,
                    ..WindowOptions::default()
                },
            ) {
                Ok(window) => window,
                Err(_) => return,
            };

            let mut scaled_buffer = Vec::new();

            while !stop_thread.load(Ordering::Relaxed) && window.is_open() {
                let (width, height, scale, buffer) = match shared_thread.lock() {
                    Ok(shared) => (
                        shared.width,
                        shared.height,
                        shared.scale,
                        shared.buffer.clone(),
                    ),
                    Err(_) => break,
                };

                if let Some((scaled_width, scaled_height)) =
                    build_scaled_buffer(&buffer, width, height, scale, &mut scaled_buffer)
                {
                    let _ = window.update_with_buffer(&scaled_buffer, scaled_width, scaled_height);
                }

                thread::sleep(Duration::from_millis(16));
            }

            drop(window);
            thread::sleep(Duration::from_millis(32));
        });

        Some(Self {
            shared,
            stop,
            handle: Some(handle),
        })
    }

    fn is_finished(&self) -> bool {
        self.handle.as_ref().is_some_and(JoinHandle::is_finished)
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let deadline = std::time::Instant::now() + Duration::from_millis(500);
            while !handle.is_finished() {
                if std::time::Instant::now() >= deadline {
                    break;
                }
                thread::sleep(Duration::from_millis(8));
            }
            let _ = handle.join();
        }
    }
}

impl Drop for DisplayWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn build_scaled_buffer(
    buffer: &[u32],
    width: usize,
    height: usize,
    scale: usize,
    scaled_buffer: &mut Vec<u32>,
) -> Option<(usize, usize)> {
    let scaled_width = width.checked_mul(scale)?;
    let scaled_height = height.checked_mul(scale)?;
    let scaled_len = scaled_width.checked_mul(scaled_height)?;

    if scaled_buffer.len() != scaled_len {
        scaled_buffer.resize(scaled_len, 0);
    }

    if scale == 1 {
        scaled_buffer.copy_from_slice(buffer);
        return Some((scaled_width, scaled_height));
    }

    for y in 0..height {
        for x in 0..width {
            let color = buffer[y * width + x];
            let base_y = y * scale;
            let base_x = x * scale;
            for sy in 0..scale {
                let row_start = (base_y + sy) * scaled_width;
                for sx in 0..scale {
                    scaled_buffer[row_start + base_x + sx] = color;
                }
            }
        }
    }

    Some((scaled_width, scaled_height))
}

fn normalized_phase(value: f64) -> f64 {
    if !value.is_finite() {
        0.0
    } else {
        value.abs().fract()
    }
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let to_u8 = |v: f64| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    (to_u8(r1), to_u8(g1), to_u8(b1))
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let h = ((h % 360.0) + 360.0) % 360.0;
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let to_u8 = |v: f64| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    (to_u8(r1), to_u8(g1), to_u8(b1))
}

fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn color_for_value(mode: ColorMode, value: f64) -> u32 {
    match mode {
        ColorMode::Rgb => {
            let bits = value.to_bits();
            let r = (bits & 0xFF) as u8;
            let g = ((bits >> 8) & 0xFF) as u8;
            let b = ((bits >> 16) & 0xFF) as u8;
            pack_rgb(r, g, b)
        }
        ColorMode::Hsv => {
            let hue = normalized_phase(value) * 360.0;
            let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
            pack_rgb(r, g, b)
        }
        ColorMode::Hsl => {
            let hue = normalized_phase(value) * 360.0;
            let (r, g, b) = hsl_to_rgb(hue, 1.0, 0.5);
            pack_rgb(r, g, b)
        }
        ColorMode::HueOnly => {
            let hue = normalized_phase(value) * 360.0;
            let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
            pack_rgb(r, g, b)
        }
        ColorMode::BlackAndWhite => {
            let phase = normalized_phase(value);
            let v = if phase >= 0.5 { 255u8 } else { 0u8 };
            pack_rgb(v, v, v)
        }
    }
}

pub struct MinifbDisplayBackend {
    width: Option<usize>,
    height: Option<usize>,
    scale: usize,
    mode: ColorMode,
    worker: Option<DisplayWorker>,
}

impl MinifbDisplayBackend {
    pub fn new() -> Self {
        Self {
            width: None,
            height: None,
            scale: display_scale_from_env(),
            mode: ColorMode::default(),
            worker: None,
        }
    }

    fn dimensions(&self) -> Option<(usize, usize)> {
        match (self.width, self.height) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Some((w, h)),
            _ => None,
        }
    }

    fn update_dimensions_if_ready(&mut self) {
        let Some((width, height)) = self.dimensions() else {
            return;
        };

        if self.worker.as_ref().is_some_and(|w| w.is_finished()) {
            self.shutdown_worker();
        }

        let worker_matches = self
            .worker
            .as_ref()
            .and_then(|w| w.shared.lock().ok())
            .is_some_and(|s| s.width == width && s.height == height && s.scale == self.scale);

        if worker_matches {
            return;
        }

        self.shutdown_worker();
        self.worker = DisplayWorker::spawn(width, height, self.scale);
    }

    fn shutdown_worker(&mut self) {
        if let Some(mut worker) = self.worker.take() {
            worker.stop();
        }
    }
}

impl Default for MinifbDisplayBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MinifbDisplayBackend {
    fn drop(&mut self) {
        self.shutdown_worker();
    }
}

impl DisplayBackend for MinifbDisplayBackend {
    fn handle(&mut self, command: DisplayCommand) {
        match command {
            DisplayCommand::SetWidth(w) => {
                self.width = Some(w);
                self.update_dimensions_if_ready();
            }
            DisplayCommand::SetHeight(h) => {
                self.height = Some(h);
                self.update_dimensions_if_ready();
            }
            DisplayCommand::SetColorMode(mode) => {
                self.mode = mode;
            }
            DisplayCommand::SetPixel { index, value } => {
                let Some(worker) = self.worker.as_ref() else {
                    return;
                };
                let Ok(mut shared) = worker.shared.lock() else {
                    return;
                };
                if index < shared.buffer.len() {
                    shared.buffer[index] = color_for_value(self.mode, value);
                }
            }
        }
    }
}

fn display_scale_from_env() -> usize {
    env::var("DISPLAY_SCALE")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(1)
}
