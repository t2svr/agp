// Copyright 2024 Junshuang Hu
use std::{any::TypeId, collections::hash_map::Values, hash::Hash};

use ahash::AHashMap;
use krnl::scalar::Scalar;

use crate::core::{IObjStat, ITaggedStore, IUntaggedStore, IndexMap, PObj};

pub mod com;
// todo: 分类储存obj

#[derive(Debug, Default)]
pub struct BasicObjStore<T = u32, U = u32>
where T: Clone + Hash + Eq, U: Scalar {
    instances: AHashMap<T, PObj<T, U>>,
    amount: IndexMap<TypeId,(U, U)>,
    modified: bool
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
                am.0 -= tag_o.obj_amount();
            }
            return Some(tag_o);
        }
        None
    }

    fn add_or_update(&mut self, t: T, v: PObj<T, U>) -> Option<PObj<T, U>> {
        self.modified = true;
        if let Some(am) = self.amount.get_mut(&v.obj_type().tid) {
            am.0 += v.obj_amount();
        } else {
            self.amount.insert(v.obj_type().tid, (v.obj_amount(), U::zero()));
        }
        if let Some(old) = self.instances.insert(t, v) {
            if let Some(am) = self.amount.get_mut(&old.obj_type().tid) {
                am.0 -= old.obj_amount();
            }
            return Some(old);
        }
        None
    }
    
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a PObj<T, U>> where PObj<T, U>: 'a {
        self.instances.values()
    }
    
    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut PObj<T, U>> where PObj<T, U>: 'a {
        self.instances.values_mut()
    }
    
    fn get_batch(&self, ts: &[T]) -> Vec<Option<&PObj<T, U>>> {
        ts.iter().map(|t: &T| self.instances.get(t)).collect()
    }


    fn get_batch_skip(&self, ts: &[T]) -> Vec<&PObj<T, U>> {
        ts.iter().filter_map(|t| self.instances.get(t)).collect()
    }

    fn remove_batch(&mut self, ts: &[T]) -> Vec<Option<PObj<T, U>>> {
        ts.iter()
        .map(|t| self.instances.remove(t))
        .map(|o| {
            if let Some(ref po) = o {
                if let Some(am) = self.amount.get_mut(&po.obj_type().tid) {
                    am.0 -= po.obj_amount();
                }
            }
            o
        })
        .collect()
    }

    fn remove_batch_skip(&mut self, ts: &[T]) -> Vec<PObj<T, U>> {
        ts.iter()
        .filter_map(|t| self.instances.remove(t))
        .map(|o| {
            if let Some(am) = self.amount.get_mut(&o.obj_type().tid) {
                am.0 -= o.obj_amount();
            }
            o
        })
        .collect()
    }
    
    fn len(&self) -> usize {
        self.instances.len()
    }
    
    fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }

}

impl<T, U> IObjStat<U> for BasicObjStore<T, U>
where T: Clone + Hash + Eq, U: Scalar {
    fn amounts(&self) -> impl Iterator<Item = &U> {
        self.amount.vals().map(|v| &v.0)
    }

    fn tid_at(&self, pos: usize) -> Option<&TypeId> {
        self.amount.get_key(pos)
    }

    fn pos_of(&self, ty: &crate::core::ObjType) -> Option<usize> {
        self.amount.index_of(&ty.tid)
    }

    fn amount_of(&self, ty: &crate::core::ObjType) -> Option<U> {
        self.amount.get(&ty.tid).cloned().map(|v| v.0)
    }

    fn amount_of_many(&self, tys: &[crate::core::ObjType]) -> Vec<&U> {
        tys.iter().filter_map(|ty| self.amount.get(&ty.tid).map(|v| &v.0)).collect()
    }

    fn type_count(&self) -> usize {
        self.amount.len()
    }
}

impl<T, U> IUntaggedStore<TypeId, U> for  BasicObjStore<T, U> // todo: amount分开tagged 和untagged
where T: Clone + Hash + Eq, U: Scalar {
    fn contains_u(&self, ty: &TypeId) -> bool {
        self.amount.containes(ty)
    }

    fn len_u(&self) -> usize {
        self.amount.len()
    }

    fn is_empty_u(&self) -> bool {
        self.amount.is_empty()
    }

    fn iter_u<'a>(&'a self) -> impl Iterator<Item = &'a U> where U: 'a {
        self.amount.vals().map(|v| &v.1)
    }

    fn iter_mut_u<'a>(&'a mut self) -> impl Iterator<Item = &'a mut U> where U: 'a {
        self.amount.vals_mut().map(|v| &mut v.1)
    }

    fn get_u(&self, ty: &TypeId) -> Option<U> {
        self.amount.get(ty).cloned().map(|v| v.1)
    }

    fn increase(&mut self, ty: &TypeId, amount: U) -> bool {
        if let Some(a) = self.amount.get_mut(ty) {
            a.1 += amount;
            a.0 += amount;
            true
        } else {
            self.amount.insert(ty.clone(), (amount, amount));
            false
        }
    }

    fn decrease(&mut self, ty: &TypeId, amount: U) -> bool {
        if let Some(a) = self.amount.get_mut(ty) {
            a.1 -= amount;
            a.0 -= amount;
            true
        } else {
            false
        }
    }

    fn remove_u(&mut self, ty: &TypeId)-> Option<U> {
        if self.amount.containes(ty) {
            let a = self.amount.get(ty).cloned().unwrap();
            if a.0 != a.1 {
                if let Some(u) = self.amount.get_mut(ty) {
                    u.0 -= u.1;
                    u.1 = U::zero();
                }
                None
            } else {
                self.amount.remove(ty).map(|u| u.0)
            }
        } else {
            None
        }
       
    }
}