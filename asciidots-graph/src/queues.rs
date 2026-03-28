use std::collections::VecDeque;

use rustc_hash::FxHashMap;

#[derive(Debug, Default)]
pub struct PairingQueue<T> {
    values: FxHashMap<u64, VecDeque<T>>,
}

pub trait Pairable {
    fn key(&self) -> u64;
}

impl<T: Pairable> PairingQueue<T> {
    pub fn push(&mut self, value: T) {
        self.values.entry(value.key()).or_default().push_back(value)
    }

    pub fn find_match(&mut self, value: &T) -> Option<T> {
        self.values.get_mut(&value.key())?.pop_front()
    }
}
