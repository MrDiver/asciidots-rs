use asciidots_core::primitives::{Directed, Oriented};

use crate::{
    operator::{DotValue, Operand},
    queues::Pairable,
};

pub trait IsTrue {
    fn is_true(&self) -> bool;
}

pub trait EngineDot: Oriented + Directed + Pairable + Operand<Self::Value> + IsTrue {
    type Value: DotValue;
}
