use smallvec::SmallVec;

use rustc_hash::FxHashMap;

pub trait Pairable {
    fn key(&self) -> u64;
}

#[derive(Debug)]
pub struct PairingQueue<T> {
    values: FxHashMap<u64, SmallVec<[T; 4]>>,
}

impl<T> Default for PairingQueue<T> {
    fn default() -> Self {
        Self {
            values: FxHashMap::default(),
        }
    }
}

impl<T: Pairable> PairingQueue<T> {
    pub fn push(&mut self, value: T) {
        self.values.entry(value.key()).or_default().push(value);
    }

    pub fn find_match(&mut self, value: &T) -> Option<T> {
        let bucket = self.values.get_mut(&value.key())?;
        if bucket.is_empty() {
            return None;
        }
        Some(bucket.remove(0))
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.values.values().flat_map(|q| q.iter())
    }

    pub fn len(&self) -> usize {
        self.values.values().map(|q| q.len()).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
