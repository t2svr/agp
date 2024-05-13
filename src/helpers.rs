use std::{any::TypeId, collections::HashMap};

#[derive(Clone)]
pub enum NeedCount {
    All,
    Some(usize)
}

impl NeedCount {
    pub fn is_some(&self) -> bool {
        match self {
            NeedCount::All => false,
            NeedCount::Some(_) => true,
        }
    }
    pub fn is_all(&self) -> bool {
        match self {
            NeedCount::All => true,
            NeedCount::Some(_) => false,
        }
    }
}

pub fn needs_map_builder() -> NeedsBuilder {
    NeedsBuilder::new()
}
pub struct NeedsBuilder{
    needs_v: Vec<(TypeId, NeedCount, bool)>
}

impl NeedsBuilder {
    pub fn new() -> Self {
        Self {
            needs_v: Vec::new()
        }
    }

    pub fn reads<T: 'static>(mut self, count: NeedCount) -> NeedsBuilder {
        if count.is_some() {
            self.needs_v.push((TypeId::of::<T>(), count, false));
        } else {
            self.needs_v.push((TypeId::of::<T>(), NeedCount::All, false));
        }
        self
    }

    pub fn takes<T: 'static>(mut self, count: NeedCount) -> NeedsBuilder {
        if count.is_some() {
            self.needs_v.push((TypeId::of::<T>(), count, true));
        } else {
            self.needs_v.push((TypeId::of::<T>(), NeedCount::All, true));
        }
        self
    }

    pub fn build(&mut self) -> HashMap<TypeId, (usize, NeedCount, bool)> {
        let mut res: HashMap<TypeId, (usize, NeedCount, bool)> = HashMap::new();
        self.needs_v.reverse();
        let mut pos = 0usize;
        while let Some((id, c, is_take)) = self.needs_v.pop() {
            res.insert(id, (pos, c, is_take));
            pos += 1;
        }
        res
    }
    
}