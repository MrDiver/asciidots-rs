use std::hash::{Hash, Hasher};

pub trait Callstackable<T: Clone + Hash> {
    fn callstack(&self) -> &Callstack<T>;
    fn callstack_mut(&mut self) -> &mut Callstack<T>;
}

use rustc_hash::FxHasher;

#[derive(Debug, Default, Clone)]
pub struct CallFrame<T> {
    key: T,
    acc_hash: u64,
}

#[derive(Debug, Default, Clone)]
pub struct Callstack<T> {
    frames: Vec<CallFrame<T>>,
}

impl<T: Clone + Hash> Callstack<T> {
    pub fn push(&mut self, key: &T) {
        let key = key.clone();
        let mut hasher = FxHasher::default();
        let last_hash = self.current_hash();
        last_hash.hash(&mut hasher);
        key.hash(&mut hasher);
        let acc_hash = hasher.finish();
        self.frames.push(CallFrame { key, acc_hash });
    }

    pub fn pop(&mut self) -> Option<T> {
        let CallFrame { key, .. } = self.frames.pop()?;
        Some(key)
    }

    pub fn current_hash(&self) -> u64 {
        self.frames.last().map(|v| v.acc_hash).unwrap_or(0)
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn last(&self) -> Option<&T> {
        self.frames.last().map(|f| &f.key)
    }
}

impl<T: Clone + Hash> PartialEq for Callstack<T> {
    fn eq(&self, other: &Self) -> bool {
        self.current_hash() == other.current_hash()
    }
}
