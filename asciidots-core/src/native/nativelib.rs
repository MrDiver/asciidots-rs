use crate::{
    engine::dot_trait::EngineDot,
    native::native_fun::NativeFunction,
    primitives::Orientation,
    state_machine::DotMachine,
};

macro_rules! define_unary_native {
    ($struct_name:ident, $native_name:literal, $operation:expr) => {
        pub struct $struct_name<D> {
            queued: Vec<D>,
        }

        impl<D> $struct_name<D> {
            pub fn new() -> Self {
                Self { queued: Vec::new() }
            }
        }

        impl<D: Default> Default for $struct_name<D> {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<D> NativeFunction<D> for $struct_name<D>
        where
            D: crate::engine::dot_trait::EngineDot<Value = f64>,
        {
            fn name(&self) -> &str {
                $native_name
            }

            fn dot_enter(&mut self, dot: D) {
                self.queued.push(dot);
            }

            fn tick_into(&mut self, output: &mut Vec<D>) {
                output.extend(self.queued.drain(..).map(|mut dot| {
                    let value = dot.selected();
                    dot.set_selected(($operation)(value));
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
struct LoopIterState<D> {
    template: D,
    current: f64,
    end: f64,
}

pub struct ForNative<D> {
    active: Vec<LoopIterState<D>>,
}

impl<D> ForNative<D> {
    pub fn new() -> Self {
        Self { active: Vec::new() }
    }
}

impl<D: Default> Default for ForNative<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D> NativeFunction<D> for ForNative<D>
where
    D: crate::engine::dot_trait::EngineDot<Value = f64> + Clone,
{
    fn name(&self) -> &str {
        "for"
    }

    fn dot_enter(&mut self, dot: D) {
        let end = dot.selected();
        self.active.push(LoopIterState {
            template: dot,
            current: 0.0,
            end,
        });
    }

    fn tick_into(&mut self, output: &mut Vec<D>) {
        for iter_state in &mut self.active {
            if iter_state.current < iter_state.end {
                let mut dot = iter_state.template.clone();
                dot.set_selected(iter_state.current);
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

pub struct DupIterState<D> {
    current: f64,
    end: f64,
    template: D,
}

pub struct DupNative<D> {
    active: Vec<DupIterState<D>>,
}

impl<D> DupNative<D> {
    pub fn new() -> Self {
        Self {
            active: Vec::default(),
        }
    }
}

impl<D: Default> Default for DupNative<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D> NativeFunction<D> for DupNative<D>
where
    D: EngineDot<Value = f64> + Clone,
{
    fn name(&self) -> &str {
        "dup"
    }

    fn dot_enter(&mut self, dot: D) {
        let end = dot.addr();
        self.active.push(DupIterState {
            current: 0.0,
            end,
            template: dot,
        });
    }

    fn tick_into(&mut self, output: &mut Vec<D>) {
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

pub struct MemNative<D> {
    queued: Vec<D>,
    current: f64,
}

impl<D> MemNative<D> {
    pub fn new() -> Self {
        Self {
            queued: Vec::default(),
            current: 0.0,
        }
    }
}

impl<D: Default> Default for MemNative<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D> NativeFunction<D> for MemNative<D>
where
    D: EngineDot<Value = f64> + Clone,
{
    fn name(&self) -> &str {
        "mem"
    }

    fn dot_enter(&mut self, dot: D) {
        match dot.orientation() {
            Orientation::Horizontal => self.queued.push(dot),
            Orientation::Vertical => self.current = dot.selected(),
        }
    }

    fn tick_into(&mut self, output: &mut Vec<D>) {
        output.extend(self.queued.drain(..).map(|mut dot| {
            dot.set_selected(self.current);
            dot
        }));
    }

    fn reset(&mut self) {
        self.queued.clear();
        self.current = 0.0;
    }
}

pub struct PushScopeNative<D> {
    queued: Vec<D>,
}

impl<D> PushScopeNative<D> {
    pub fn new() -> Self {
        Self { queued: Vec::new() }
    }
}

impl<D: Default> Default for PushScopeNative<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D> NativeFunction<D> for PushScopeNative<D>
where
    D: EngineDot + Clone,
{
    fn name(&self) -> &str {
        "push"
    }

    fn dot_enter(&mut self, dot: D) {
        self.queued.push(dot);
    }

    fn tick_into(&mut self, output: &mut Vec<D>) {
        output.extend(self.queued.drain(..).map(|mut dot| {
            let return_pos = dot
                .pop_callstack()
                .unwrap_or_else(|| dot.current_location());

            let scope_key = dot.callstack_key_from_addr();
            dot.push_callstack(scope_key);
            dot.push_callstack(return_pos);
            dot
        }));
    }

    fn reset(&mut self) {
        self.queued.clear();
    }
}

pub struct PopScopeNative<D> {
    queued: Vec<D>,
}

impl<D> PopScopeNative<D> {
    pub fn new() -> Self {
        Self { queued: Vec::new() }
    }
}

impl<D: Default> Default for PopScopeNative<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D> NativeFunction<D> for PopScopeNative<D>
where
    D: EngineDot + Clone,
{
    fn name(&self) -> &str {
        "pop"
    }

    fn dot_enter(&mut self, dot: D) {
        self.queued.push(dot);
    }

    fn tick_into(&mut self, output: &mut Vec<D>) {
        output.extend(self.queued.drain(..).map(|mut dot| {
            let return_pos = dot
                .pop_callstack()
                .unwrap_or_else(|| dot.current_location());

            let _ = dot.pop_callstack();
            dot.push_callstack(return_pos);
            dot
        }));
    }

    fn reset(&mut self) {
        self.queued.clear();
    }
}

pub fn default_native_library() -> Vec<Box<dyn NativeFunction<DotMachine>>> {
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

