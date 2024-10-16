use std::{any::TypeId, collections::hash_map::Values, hash::Hash};

use ahash::AHashMap;
use krnl::scalar::Scalar;

use crate::core::{IObjStat, ITaggedStore, IndexMap, PObj};

pub mod com;
// todo: 分类储存obj
pub struct BasicObjStore<T = u32, U = u32>
where T: Clone + Hash + Eq, U: Scalar {
    instances: AHashMap<T, PObj<T, U>>,
    amount: IndexMap<TypeId, U>,
    modified: bool
}

impl<T, U> Default for BasicObjStore<T, U>
where T: Clone + Hash + Eq, U: Scalar {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, U> BasicObjStore<T, U> 
where T: Clone + Hash + Eq, U: Scalar {
    pub fn new() -> Self {
        Self { instances: AHashMap::new(), amount: IndexMap::new(), modified: false }
    }

    pub fn objs(&self) -> Values<T, PObj<T, U>> {
        self.instances.values()
    }
}

impl<T, U> ITaggedStore<T, PObj<T, U>> for BasicObjStore<T, U>
where T: Clone + Hash + Eq, U: Scalar {

    fn contains(&self, t: &T) -> bool {
        self.instances.contains_key(t)
    }

    fn get(&self, t: &T) -> Option<&PObj<T, U>> {
        self.instances.get(t)
    }

    fn get_mut(&mut self, t: &T) -> Option<&mut PObj<T, U>> {
        self.modified = true;
        self.instances.get_mut(t)
    }

    fn remove(&mut self, t: &T) -> Option<PObj<T, U>> {
        if let Some(tag_o) = self.instances.remove(t) {
            self.modified = true;
            if let Some(am) = self.amount.get_mut(&tag_o.obj_type().tid) {
                *am -= tag_o.obj_amount();
            }
            return Some(tag_o);
        }
        None
    }

    /// 不失败
    fn add_or_update(&mut self, t: T, v: PObj<T, U>) -> Option<PObj<T, U>> {
        self.modified = true;
        if let Some(am) = self.amount.get_mut(&v.obj_type().tid) {
            *am += v.obj_amount();
        } else {
            self.amount.insert(v.obj_type().tid, v.obj_amount());
        }
        if let Some(old) = self.instances.insert(t, v) {
            if let Some(am) = self.amount.get_mut(&old.obj_type().tid) {
                *am -= old.obj_amount();
            }
            return Some(old);
        }
        None
    }
    
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a PObj<T, U>> where PObj<T, U>: 'a {
        self.instances.values()
    }
    
}

impl<T, U> IObjStat<U> for BasicObjStore<T, U>
where T: Clone + Hash + Eq, U: Scalar  {
    fn amount(&self) -> &Vec<U> {
        self.amount.vals()
    }

    fn get_tid(&self, pos: usize) -> Option<&TypeId> {
        self.amount.get_key(pos)
    }

    fn index_of(&self, ty: &crate::core::ObjType) -> Option<usize> {
        self.amount.index_of(&ty.tid)
    }

    fn amount_of(&self, ty: &crate::core::ObjType) -> Option<U> {
        self.amount.get(&ty.tid).cloned()
    }

    fn amount_of_many(&self, tys: &[crate::core::ObjType]) -> Vec<&U> {
        tys.iter().filter_map(|ty| self.amount.get(&ty.tid)).collect()
    }
    
    fn modified(&self) -> bool {
        self.modified
    }
    
    fn dismiss(&mut self) {
        self.modified = false;
    }
}