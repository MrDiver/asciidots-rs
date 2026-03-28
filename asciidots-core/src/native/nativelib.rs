use crate::{
    native::native_fun::NativeFunction,
    primitives::{Orientation, Oriented, Position},
    state_machine::DotMachine,
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

            fn tick_into(&mut self, output: &mut Vec<DotMachine>) {
                output.extend(self.queued.drain(..).map(|mut dot| {
                    let value = dot.selected();
                    dot.dot_mut().set_selected(($operation)(value));
                    dot
                }));
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

    fn tick_into(&mut self, output: &mut Vec<DotMachine>) {
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
    }

    fn reset(&mut self) {
        self.active.clear();
    }
}

pub struct DupIterState {
    current: f64,
    end: f64,
    template: DotMachine,
}

pub struct DupNative {
    active: Vec<DupIterState>,
}

impl DupNative {
    pub fn new() -> Self {
        Self {
            active: Vec::default(),
        }
    }
}

impl Default for DupNative {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeFunction for DupNative {
    fn name(&self) -> &str {
        "dup"
    }

    fn dot_enter(&mut self, dot: DotMachine) {
        self.active.push(DupIterState {
            current: 0.0,
            end: dot.dot().addr(),
            template: dot,
        });
    }

    fn tick_into(&mut self, output: &mut Vec<DotMachine>) {
        for iter_state in &mut self.active {
            if iter_state.current < iter_state.end {
                let dot = iter_state.template.clone();
                output.push(dot);
                iter_state.current += 1.0;
            }
        }

        self.active
            .retain(|iter_state| iter_state.current < iter_state.end);
    }

    fn reset(&mut self) {
        self.active.clear()
    }
}

pub struct MemNative {
    queued: Vec<DotMachine>,
    current: f64,
}

impl MemNative {
    pub fn new() -> Self {
        Self {
            queued: Vec::default(),
            current: 0.0,
        }
    }
}

impl Default for MemNative {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeFunction for MemNative {
    fn name(&self) -> &str {
        "mem"
    }

    fn dot_enter(&mut self, dot: DotMachine) {
        match dot.orientation() {
            Orientation::Horizontal => self.queued.push(dot),
            Orientation::Vertical => self.current = dot.selected(),
        }
    }

    fn tick_into(&mut self, output: &mut Vec<DotMachine>) {
        let v: Vec<DotMachine> = self
            .queued
            .drain(..)
            .map(|mut dot| {
                dot.dot_mut().set_selected(self.current);
                dot
            })
            .collect();
        output.extend(v);
    }

    fn reset(&mut self) {
        self.queued.clear();
        self.current = 0.0;
    }
}

pub struct PushScopeNative {
    queued: Vec<DotMachine>,
}

impl PushScopeNative {
    pub fn new() -> Self {
        Self { queued: Vec::new() }
    }
}

impl Default for PushScopeNative {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeFunction for PushScopeNative {
    fn name(&self) -> &str {
        "push"
    }

    fn dot_enter(&mut self, dot: DotMachine) {
        self.queued.push(dot);
    }

    fn tick_into(&mut self, output: &mut Vec<DotMachine>) {
        output.extend(self.queued.drain(..).map(|mut dot| {
            let return_pos = dot
                .dot_mut()
                .pop_callstack()
                .unwrap_or_else(|| *dot.position());

            let addr = dot.dot().addr();
            let dummy_scope = if addr.is_finite() {
                Position::new(addr as i32, addr as i32)
            } else {
                Position::default()
            };

            dot.dot_mut()
                .push_callstack(dummy_scope)
                .push_callstack(return_pos);
            dot
        }));
    }

    fn reset(&mut self) {
        self.queued.clear();
    }
}

pub struct PopScopeNative {
    queued: Vec<DotMachine>,
}

impl PopScopeNative {
    pub fn new() -> Self {
        Self { queued: Vec::new() }
    }
}

impl Default for PopScopeNative {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeFunction for PopScopeNative {
    fn name(&self) -> &str {
        "pop"
    }

    fn dot_enter(&mut self, dot: DotMachine) {
        self.queued.push(dot);
    }

    fn tick_into(&mut self, output: &mut Vec<DotMachine>) {
        output.extend(self.queued.drain(..).map(|mut dot| {
            let return_pos = dot
                .dot_mut()
                .pop_callstack()
                .unwrap_or_else(|| *dot.position());

            let _ = dot.dot_mut().pop_callstack();
            dot.dot_mut().push_callstack(return_pos);
            dot
        }));
    }

    fn reset(&mut self) {
        self.queued.clear();
    }
}

pub fn default_native_library() -> Vec<Box<dyn NativeFunction>> {
    vec![
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
        Box::new(DupNative::new()),
        Box::new(MemNative::new()),
        Box::new(PushScopeNative::new()),
        Box::new(PopScopeNative::new()),
    ]
}

