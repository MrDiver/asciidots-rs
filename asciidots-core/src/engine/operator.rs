use std::ops::{Add, Div, Mul, Rem, Sub};

use crate::primitives::{Orientation, Oriented};

use super::{
    dot_trait::EngineDot,
    queues::{Pairable, PairingQueue},
};

#[derive(Debug)]
pub struct OperatorQueue<T: Pairable> {
    horizontal: PairingQueue<T>,
    vertical: PairingQueue<T>,
}

#[derive(Debug)]
pub struct OperatorPair<T> {
    pub horizontal: T,
    pub vertical: T,
}

impl<T: Pairable> Default for OperatorQueue<T> {
    fn default() -> Self {
        Self {
            horizontal: PairingQueue::default(),
            vertical: PairingQueue::default(),
        }
    }
}

impl<T: Pairable> OperatorQueue<T> {
    pub fn len(&self) -> usize {
        self.horizontal.len() + self.vertical.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.horizontal.iter().chain(self.vertical.iter())
    }

    pub fn push_horizontal_match_vertical(&mut self, value: T) -> Option<OperatorPair<T>> {
        if let Some(matched) = self.vertical.find_match(&value) {
            Some(OperatorPair {
                horizontal: value,
                vertical: matched,
            })
        } else {
            self.horizontal.push(value);
            None
        }
    }

    pub fn push_vertical_match_horizontal(&mut self, value: T) -> Option<OperatorPair<T>> {
        if let Some(matched) = self.horizontal.find_match(&value) {
            Some(OperatorPair {
                horizontal: matched,
                vertical: value,
            })
        } else {
            self.vertical.push(value);
            None
        }
    }
}

pub trait DotValue:
    Sized
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Rem<Output = Self>
    + PartialEq
    + PartialOrd
    + Copy
{
    fn pow(self, other: Self) -> Self;
    fn bool_val(b: bool) -> Self;
    fn bitand(self, other: Self) -> Self;
    fn bitor(self, other: Self) -> Self;
    fn bitxor(self, other: Self) -> Self;
}

impl DotValue for f64 {
    fn pow(self, other: Self) -> Self {
        self.powf(other)
    }
    fn bool_val(b: bool) -> Self {
        if b { 1.0 } else { 0.0 }
    }
    fn bitand(self, other: Self) -> Self {
        (self as u64 & other as u64) as f64
    }
    fn bitor(self, other: Self) -> Self {
        (self as u64 | other as u64) as f64
    }
    fn bitxor(self, other: Self) -> Self {
        (self as u64 ^ other as u64) as f64
    }
}

pub trait Operand<V: DotValue>: Sized {
    fn apply_operator(self, other: Self, op: impl Fn(V, V) -> V) -> Self;
}

pub struct Operator<T, V: DotValue>
where
    T: Oriented + Pairable + Operand<V>,
{
    pub c: char,
    pub queue: OperatorQueue<T>,
    pub kind: Orientation,
    pub _marker: std::marker::PhantomData<V>,
}

impl<T, V> Operator<T, V>
where
    T: Oriented + Pairable + Operand<V>,
    V: DotValue,
{
    pub fn enter(&mut self, operand: T) -> Option<T> {
        let OperatorPair {
            horizontal,
            vertical,
        } = match operand.orientation() {
            Orientation::Horizontal => self.queue.push_horizontal_match_vertical(operand),
            Orientation::Vertical => self.queue.push_vertical_match_horizontal(operand),
        }?;
        let op_closure: fn(V, V) -> V = match self.c {
            '+' => V::add,
            '-' => V::sub,
            '*' => V::mul,
            '/' => V::div,
            '%' => V::rem,
            '^' => V::pow,
            '=' => |a, b| V::bool_val(a == b),
            '!' => |a, b| V::bool_val(a != b),
            '>' => |a, b| V::bool_val(a > b),
            '<' => |a, b| V::bool_val(a < b),
            'G' => |a, b| V::bool_val(a >= b),
            'L' => |a, b| V::bool_val(a <= b),
            '&' => V::bitand,
            '|' => V::bitor,
            'x' => V::bitxor,
            _ => |a, _| a,
        };
        Some(match self.kind {
            Orientation::Horizontal => horizontal.apply_operator(vertical, op_closure),
            Orientation::Vertical => vertical.apply_operator(horizontal, op_closure),
        })
    }
}

pub struct ConditionalOperator<T: EngineDot> {
    pub queue: OperatorQueue<T>,
    pub negated: bool,
}

impl<T: EngineDot> ConditionalOperator<T> {
    pub fn enter(&mut self, dot: T) -> Option<T> {
        let OperatorPair {
            mut horizontal,
            vertical,
        } = match dot.orientation() {
            Orientation::Horizontal => self.queue.push_horizontal_match_vertical(dot),
            Orientation::Vertical => self.queue.push_vertical_match_horizontal(dot),
        }?;
        let redirect_up = if self.negated {
            !vertical.is_true()
        } else {
            vertical.is_true()
        };
        if redirect_up {
            horizontal.set_direction(crate::primitives::Direction::Up);
        }
        Some(horizontal)
    }
}
