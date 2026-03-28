use crate::engine::dot_trait::EngineDot;

pub trait NativeFunction<D: EngineDot>: Send + Sync {
    fn name(&self) -> &str;

    fn dot_enter(&mut self, dot: D);

    fn tick_into(&mut self, output: &mut Vec<D>);

    fn reset(&mut self);
}
