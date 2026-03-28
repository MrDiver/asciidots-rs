pub trait ValueSelector<K, V> {
    fn select(&mut self, k: K);
    fn reset_selection(&mut self);
    fn get(&self) -> V;
    fn set(&mut self, value: V);
}

pub enum AddrValueSelector {
    Value,
    Addr,
}

pub struct AddrValue {
    addr: f64,
    value: f64,
    selected: AddrValueSelector,
}

impl AddrValue {
    pub fn addr(&self) -> f64 {
        self.addr
    }
}

impl Default for AddrValue {
    fn default() -> Self {
        Self {
            addr: Default::default(),
            value: Default::default(),
            selected: AddrValueSelector::Value,
        }
    }
}

impl ValueSelector<AddrValueSelector, f64> for AddrValue {
    fn select(&mut self, k: AddrValueSelector) {
        self.selected = k;
    }

    fn reset_selection(&mut self) {
        self.selected = AddrValueSelector::Value
    }

    fn get(&self) -> f64 {
        match self.selected {
            AddrValueSelector::Value => self.value,
            AddrValueSelector::Addr => self.addr,
        }
    }

    fn set(&mut self, value: f64) {
        match self.selected {
            AddrValueSelector::Value => self.value = value,
            AddrValueSelector::Addr => self.addr = value,
        }
    }
}
