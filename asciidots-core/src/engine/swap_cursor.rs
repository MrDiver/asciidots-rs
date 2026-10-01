#[derive(Debug)]
pub struct SwapCursor<T> {
    idx: usize,
    vec: *mut Vec<T>,
}

impl<T> SwapCursor<T> {
    pub fn new(vec: *mut Vec<T>) -> SwapCursor<T> {
        Self { idx: 0, vec: vec }
    }

    pub fn next(&mut self) -> Option<Entry<'_, T>> {
        let len = unsafe { (*self.vec).len() };
        (self.idx < len).then(|| Entry(self))
    }
}

pub struct Entry<'a, T>(&'a mut SwapCursor<T>);

impl<'a, T> Entry<'a, T> {
    pub fn idx(&self) -> usize {
        self.0.idx
    }

    pub fn peek(&self) -> &T {
        unsafe { &(&(*self.0.vec))[self.0.idx] }
    }

    pub fn peek_mut(&mut self) -> &mut T {
        unsafe { &mut (&mut (*self.0.vec))[self.0.idx] }
    }

    pub fn next(self) -> &'a T {
        let i = self.0.idx;
        self.0.idx += 1;
        unsafe { &(&(*self.0.vec))[i] }
    }

    pub fn next_mut(self) -> &'a mut T {
        let i = self.0.idx;
        self.0.idx += 1;
        unsafe { &mut (&mut (*self.0.vec))[i] }
    }

    pub fn take(self) -> T {
        unsafe { (*self.0.vec).swap_remove(self.0.idx) }
    }

    pub fn skip(self) {
        self.0.idx += 1;
    }
}
