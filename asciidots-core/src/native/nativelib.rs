use crate::{
    native::native_fun::NativeFunction,
    state_machine::DotMachine,
};

#[cfg(feature = "display_window")]
use crate::primitives::Direction;

#[cfg(feature = "display_window")]
use std::env;

#[cfg(feature = "display_window")]
use minifb::{Window, WindowOptions};

#[cfg(feature = "display_window")]
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

macro_rules! define_unary_native {
    ($struct_name:ident, $native_name:literal, $operation:expr) => {
        pub struct $struct_name {
            queued: Vec<DotMachine>,
        }

        impl $struct_name {
            pub fn new() -> Self {
                Self { queued: Vec::new() }
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl NativeFunction for $struct_name {
            fn name(&self) -> &str {
                $native_name
            }

            fn dot_enter(&mut self, dot: DotMachine) {
                self.queued.push(dot);
            }

            fn tick(&mut self) -> Vec<DotMachine> {
                self.queued
                    .drain(..)
                    .map(|mut dot| {
                        let value = dot.selected();
                        dot.dot_mut().set_selected(($operation)(value));
                        dot
                    })
                    .collect()
            }

            fn reset(&mut self) {
                self.queued.clear();
            }
        }
    };
}

define_unary_native!(AbsNative, "abs", f64::abs);
define_unary_native!(SinNative, "sin", f64::sin);
define_unary_native!(AsinNative, "asin", f64::asin);
define_unary_native!(CosNative, "cos", f64::cos);
define_unary_native!(AcosNative, "acos", f64::acos);
define_unary_native!(TanNative, "tan", f64::tan);
define_unary_native!(AtanNative, "atan", f64::atan);
define_unary_native!(SinhNative, "sinh", f64::sinh);
define_unary_native!(AsinhNative, "asinh", f64::asinh);
define_unary_native!(CoshNative, "cosh", f64::cosh);
define_unary_native!(AcoshNative, "acosh", f64::acosh);
define_unary_native!(TanhNative, "tanh", f64::tanh);
define_unary_native!(AtanhNative, "atanh", f64::atanh);
define_unary_native!(SqrtNative, "sqrt", f64::sqrt);
define_unary_native!(CbrtNative, "cbrt", f64::cbrt);
define_unary_native!(ExpNative, "exp", f64::exp);
define_unary_native!(LnNative, "ln", f64::ln);
define_unary_native!(Log10Native, "log10", f64::log10);
define_unary_native!(FloorNative, "floor", f64::floor);
define_unary_native!(CeilNative, "ceil", f64::ceil);
define_unary_native!(RoundNative, "round", f64::round);
define_unary_native!(TruncNative, "trunc", f64::trunc);
define_unary_native!(SignumNative, "signum", f64::signum);

#[derive(Clone)]
struct LoopIterState {
    template: DotMachine,
    current: f64,
    end: f64,
}

pub struct ForNative {
    active: Vec<LoopIterState>,
}

impl ForNative {
    pub fn new() -> Self {
        Self { active: Vec::new() }
    }
}

impl Default for ForNative {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeFunction for ForNative {
    fn name(&self) -> &str {
        "for"
    }

    fn dot_enter(&mut self, dot: DotMachine) {
        let end = dot.selected();
        self.active.push(LoopIterState {
            template: dot,
            current: 0.0,
            end,
        });
    }

    fn tick(&mut self) -> Vec<DotMachine> {
        let mut output = Vec::new();

        for iter_state in &mut self.active {
            if iter_state.current < iter_state.end {
                let mut dot = iter_state.template.clone();
                dot.dot_mut().set_selected(iter_state.current);
                output.push(dot);
                iter_state.current += 1.0;
            }
        }

        self.active
            .retain(|iter_state| iter_state.current < iter_state.end);

        output
    }

    fn reset(&mut self) {
        self.active.clear();
    }
}

#[cfg(feature = "display_window")]
#[derive(Clone, Copy)]
enum ColorMode {
    Rgb,
    Hsv,
    Hsl,
    HueOnly,
}

#[cfg(feature = "display_window")]
struct DisplayShared {
    width: usize,
    height: usize,
    scale: usize,
    buffer: Vec<u32>,
}

#[cfg(feature = "display_window")]
struct DisplayWorker {
    shared: Arc<Mutex<DisplayShared>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

#[cfg(feature = "display_window")]
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
                    Ok(shared) => (shared.width, shared.height, shared.scale, shared.buffer.clone()),
                    Err(_) => break,
                };

                if let Some((scaled_width, scaled_height)) =
                    DisplayNative::build_scaled_buffer_snapshot(
                        &buffer,
                        width,
                        height,
                        scale,
                        &mut scaled_buffer,
                    )
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
            // give the window thread time to finish its own cleanup loop
            // and properly release all Wayland proxies before we join
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

#[cfg(feature = "display_window")]
impl Drop for DisplayWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(feature = "display_window")]
pub struct DisplayNative {
    queued: Vec<DotMachine>,
    width: Option<usize>,
    height: Option<usize>,
    scale: usize,
    mode: ColorMode,
    worker: Option<DisplayWorker>,
}

#[cfg(feature = "display_window")]
impl DisplayNative {
    pub fn new() -> Self {
        Self {
            queued: Vec::new(),
            width: None,
            height: None,
            scale: Self::display_scale_from_env(),
            mode: ColorMode::Rgb,
            worker: None,
        }
    }

    fn display_scale_from_env() -> usize {
        env::var("DISPLAY_SCALE")
            .ok()
            .and_then(|value| value.trim().parse::<usize>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(1)
    }

    fn dimensions(&self) -> Option<(usize, usize)> {
        match (self.width, self.height) {
            (Some(width), Some(height)) if width > 0 && height > 0 => Some((width, height)),
            _ => None,
        }
    }

    fn parse_dimension(value: f64) -> Option<usize> {
        if !value.is_finite() {
            return None;
        }

        let rounded = value.round();
        if rounded < 1.0 {
            return None;
        }

        Some(rounded as usize)
    }

    fn update_dimensions_if_ready(&mut self) {
        let Some((width, height)) = self.dimensions() else {
            return;
        };

        if width == 0 || height == 0 {
            return;
        }

        if self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
        {
            self.shutdown_worker();
        }

        let worker_matches = self
            .worker
            .as_ref()
            .and_then(|worker| worker.shared.lock().ok())
            .is_some_and(|shared| {
                shared.width == width && shared.height == height && shared.scale == self.scale
            });

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

    fn clear_worker_buffer(&mut self) {
        let Some(worker) = self.worker.as_ref() else {
            return;
        };

        let Ok(mut shared) = worker.shared.lock() else {
            return;
        };

        shared.buffer.fill(0);
    }

    fn build_scaled_buffer_snapshot(
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

    fn set_mode_from_value(&mut self, value: f64) {
        if !value.is_finite() {
            self.mode = ColorMode::Rgb;
            return;
        }

        self.mode = match value.round() as i64 {
            1 => ColorMode::Hsv,
            2 => ColorMode::Hsl,
            3 => ColorMode::HueOnly,
            _ => ColorMode::Rgb,
        };
    }

    fn normalized_phase(value: f64) -> f64 {
        if !value.is_finite() {
            return 0.0;
        }

        value.abs().fract()
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

        let to_u8 = |component: f64| ((component + m).clamp(0.0, 1.0) * 255.0).round() as u8;
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

        let to_u8 = |component: f64| ((component + m).clamp(0.0, 1.0) * 255.0).round() as u8;
        (to_u8(r1), to_u8(g1), to_u8(b1))
    }

    fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }

    fn color_for_value(&self, value: f64) -> u32 {
        match self.mode {
            ColorMode::Rgb => {
                let bits = value.to_bits();
                let r = (bits & 0xFF) as u8;
                let g = ((bits >> 8) & 0xFF) as u8;
                let b = ((bits >> 16) & 0xFF) as u8;
                Self::pack_rgb(r, g, b)
            }
            ColorMode::Hsv => {
                let hue = Self::normalized_phase(value) * 360.0;
                let (r, g, b) = Self::hsv_to_rgb(hue, 1.0, 1.0);
                Self::pack_rgb(r, g, b)
            }
            ColorMode::Hsl => {
                let hue = Self::normalized_phase(value) * 360.0;
                let (r, g, b) = Self::hsl_to_rgb(hue, 1.0, 0.5);
                Self::pack_rgb(r, g, b)
            }
            ColorMode::HueOnly => {
                let hue = Self::normalized_phase(value) * 360.0;
                let (r, g, b) = Self::hsv_to_rgb(hue, 1.0, 1.0);
                Self::pack_rgb(r, g, b)
            }
        }
    }

    fn draw_dot(&mut self, dot: &DotMachine) {
        let Some(worker) = self.worker.as_ref() else {
            return;
        };

        let addr = dot.dot().addr();
        if !addr.is_finite() || addr < 0.0 {
            return;
        }

        let Ok(mut shared) = worker.shared.lock() else {
            return;
        };

        let index = addr.trunc() as usize;
        if index >= shared.buffer.len() {
            return;
        }

        shared.buffer[index] = self.color_for_value(dot.dot().value());
    }
}

#[cfg(feature = "display_window")]
impl Drop for DisplayNative {
    fn drop(&mut self) {
        self.shutdown_worker();
    }
}

#[cfg(feature = "display_window")]
impl Default for DisplayNative {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "display_window")]
impl NativeFunction for DisplayNative {
    fn name(&self) -> &str {
        "display"
    }

    fn dot_enter(&mut self, dot: DotMachine) {
        self.queued.push(dot);
    }

    fn tick(&mut self) -> Vec<DotMachine> {
        let queued: Vec<DotMachine> = self.queued.drain(..).collect();

        for dot in queued {
            match dot.direction() {
                Direction::Right => {
                    self.width = Self::parse_dimension(dot.selected());
                    self.update_dimensions_if_ready();
                }
                Direction::Up => {
                    self.height = Self::parse_dimension(dot.selected());
                    self.update_dimensions_if_ready();
                }
                Direction::Left => {
                    self.set_mode_from_value(dot.selected());
                }
                Direction::Down => {
                    self.draw_dot(&dot);
                }
            }
        }

        vec![]
    }

    fn reset(&mut self) {
        self.queued.clear();
        self.width = None;
        self.height = None;
        self.mode = ColorMode::Rgb;
        self.clear_worker_buffer();
        self.shutdown_worker();
    }
}

pub fn default_native_library() -> Vec<Box<dyn NativeFunction>> {
    let natives: Vec<Box<dyn NativeFunction>> = vec![
        Box::new(AbsNative::new()),
        Box::new(SinNative::new()),
        Box::new(AsinNative::new()),
        Box::new(CosNative::new()),
        Box::new(AcosNative::new()),
        Box::new(TanNative::new()),
        Box::new(AtanNative::new()),
        Box::new(SinhNative::new()),
        Box::new(AsinhNative::new()),
        Box::new(CoshNative::new()),
        Box::new(AcoshNative::new()),
        Box::new(TanhNative::new()),
        Box::new(AtanhNative::new()),
        Box::new(SqrtNative::new()),
        Box::new(CbrtNative::new()),
        Box::new(ExpNative::new()),
        Box::new(LnNative::new()),
        Box::new(Log10Native::new()),
        Box::new(FloorNative::new()),
        Box::new(CeilNative::new()),
        Box::new(RoundNative::new()),
        Box::new(TruncNative::new()),
        Box::new(SignumNative::new()),
        Box::new(ForNative::new()),
    ];

    #[cfg(feature = "display_window")]
    let natives = {
        let mut natives = natives;
        natives.push(Box::new(DisplayNative::new()));
        natives
    };

    natives
}
