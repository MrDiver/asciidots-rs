use crate::state_machine::DotMachine;

pub trait NativeFunction {
    fn name(&self) -> &str;

    fn dot_enter(&mut self, dot: DotMachine);

    fn tick_into(&mut self, output: &mut Vec<DotMachine>);

    fn reset(&mut self);
}
