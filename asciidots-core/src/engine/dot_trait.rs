use std::hash::Hash;

use crate::primitives::{Directed, Oriented};

use super::{
    operator::{DotValue, Operand},
    queues::Pairable,
};

pub trait IsTrue {
    fn is_true(&self) -> bool;
}

pub trait EngineDot: Oriented + Directed + Pairable + Operand<Self::Value> + IsTrue + Send + Sync {
    type Value: DotValue;
    type CallstackKey: Clone + Hash + Default;

    fn selected(&self) -> Self::Value;
    fn set_selected(&mut self, value: Self::Value);
    fn addr(&self) -> Self::Value;
    fn current_location(&self) -> Self::CallstackKey;
    fn callstack_key_from_addr(&self) -> Self::CallstackKey;
    fn pop_callstack(&mut self) -> Option<Self::CallstackKey>;
    fn push_callstack(&mut self, key: Self::CallstackKey);
}
