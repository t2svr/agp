use std::{any::TypeId, hash::Hash};

use ahash::AHashMap;
use krnl::scalar::Scalar;

use crate::core::{ICondition, IRuleEffect, IRuleStat, ITaggedStore, IndexMap, OperationEffect, PRule, TaggedPresences, UntaggedPresences};

pub mod com;

// pub struct BasicEffect<T, U>
// where T: Clone + Hash + Eq, U: Scalar {
//     pub opty: OperationType,
//     pub action_obj_gen: Option<>
// }

#[derive(Debug)]
pub struct BasicEffect<T = u32, U = u32>
where T: Send + Sync, U: Send + Sync {
    effects: Option<Vec<OperationEffect<T, U>>>,
}

impl<T, U> BasicEffect<T, U>
where T: Send + Sync, U: Send + Sync {
    pub fn new(ops: Option<Vec<OperationEffect<T, U>>>) -> Self {
        Self { effects: ops }
    }
}

impl<T, U> IRuleEffect for BasicEffect<T, U>
where T: Send + Sync + Clone, U: Send + Sync + Clone {
    type Effect = OperationEffect<T, U>;
    
    fn effects(&self) -> &Option<Vec<Self::Effect>> {
        &self.effects
    }
    
    fn from_builder(effs: Option<Vec<Self::Effect>>) -> Self {
        Self { effects: effs }
    }
}

#[derive(Debug, Clone)]
pub struct BasicCondition<T = u32, U = u32>
where T: Clone + Hash + Eq, U: Scalar {
    untagged_cond: Option<UntaggedPresences<U>>,
    tagged_cond: Option<TaggedPresences<T>>
}

impl<T, U> ICondition<T, U> for BasicCondition<T, U>
where T: Clone + Hash + Eq, U: Scalar {
        
    fn from_builder(uts: Option<UntaggedPresences<U>>, tgs: Option<TaggedPresences<T>>) -> Self {
        Self {
            untagged_cond: uts,
            tagged_cond: tgs,
        }
    }
    
    fn untagged(&self) -> &Option<UntaggedPresences<U>> {
        &self.untagged_cond
    }
    
    fn tagged(&self) -> &Option<TaggedPresences<T>> {
        &self.tagged_cond
    }

}

#[derive(Debug, Default)]
pub struct BasicRuleStore<T, OT = T, U = u32, OU = U, E = BasicEffect<OT, U>, C = BasicCondition<OT, U>>
where 
T: Hash + Eq + Clone + Send + Sync, 
OT: Eq + Hash + Clone , 
U: Scalar, 
OU: Scalar,
E: IRuleEffect, 
C: ICondition<OT, OU> {
    inner: IndexMap<T, PRule<T, OT, U, OU, E, C>>,
    stat: Vec<C>,
    amount: AHashMap<TypeId, OU>
}

impl<T, OT, U, OU, E, C> BasicRuleStore<T, OT, U, OU, E, C>
where 
T: Hash + Eq + Clone + Send + Sync, 
OT: Eq + Hash + Clone, 
U: Scalar, 
OU: Scalar,
E: IRuleEffect, 
C: ICondition<OT, OU> {
    pub fn new() -> Self {
        Self {
            inner: IndexMap::new(),
            stat: Vec::new(),
            amount: AHashMap::new()
        }
    }

    pub fn rules(&self) -> impl Iterator<Item = &PRule<T, OT, U, OU, E, C>> {
        self.inner.vals()
    }
}

impl<T, OT, U, OU, E, C> ITaggedStore<T, PRule<T, OT, U, OU, E, C>> for BasicRuleStore<T, OT, U, OU, E, C>
where 
T: Hash + Eq + Clone + Send + Sync, 
OT: Eq + Hash + Clone, 
U: Scalar, 
OU: Scalar,
E: IRuleEffect, 
C: ICondition<OT, OU> {

    fn contains(&self, t: &T) -> bool {
        self.inner.containes(t)
    }

    fn get(&self, t: &T) -> Option<&PRule<T, OT, U, OU, E, C>> {
        self.inner.get(t)
    }

    fn get_mut(&mut self, t: &T) -> Option<&mut PRule<T, OT, U, OU, E, C>> {
        self.inner.get_mut(t)
    }

    fn remove(&mut self, t: &T) -> Option<PRule<T, OT, U, OU, E, C>> {
        let old_ind = self.index_of(t);
        if let Some(old) = self.inner.remove(t) {
            let old_c = self.stat.remove(old_ind.unwrap());
            if let Some(o_req) = old_c.untagged() {
                for o in o_req {
                    let a = self.amount.get_mut(&o.ty.tid).unwrap();
                    if *a > o.amount {
                        *a -= o.amount;
                    } else {
                        self.amount.remove(&o.ty.tid);
                    }
                }
            }
            Some(old)
        } else {
            None
        }
    }

    fn add_or_update(&mut self, t: T, v: PRule<T, OT, U, OU, E, C>) -> Option<PRule<T, OT, U, OU, E, C>> {
        let cond = v.condition().clone();
        if let Some(o_req) = cond.untagged() {
            for o in o_req {
                if let Some(a) = self.amount.get_mut(&o.ty.tid) {
                    *a += o.amount;
                } else {
                    self.amount.insert(o.ty.tid, o.amount);
                }
            }
        }
        if let Some(old) =  self.inner.insert(t.clone(), v) {
            let ind = self.index_of(&t).unwrap();
            if let Some(o_req) = old.condition().untagged() {
                for o in o_req {
                    let a = self.amount.get_mut(&o.ty.tid).unwrap();
                    if *a > o.amount {
                        *a -= o.amount;
                    } else {
                        self.amount.remove(&o.ty.tid);
                    }
                }
            }
            self.stat[ind] = cond;
            Some(old)
        } else {
            self.stat.push(cond);
            None
        }
    }
    
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a PRule<T, OT, U, OU, E, C>> where PRule<T, OT, U, OU, E, C>: 'a {
        self.inner.vals()
    }
    
    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut PRule<T, OT, U, OU, E, C>> where PRule<T, OT, U, OU, E, C>: 'a {
        self.inner.vals_mut()
    }
    
    fn get_batch(&self, ts: &[T]) -> Vec<&PRule<T, OT, U, OU, E, C>> {
        ts.iter().filter_map(|t| self.inner.get(t)).collect()
    }
    
    fn remove_batch(&mut self, ts: &[T]) -> Vec<PRule<T, OT, U, OU, E, C>> {
        ts.iter().filter_map(|t| self.inner.remove(t)).collect()
    }
  
}

impl<T, OT, U, OU, E, C> IRuleStat<T, OT, OU, E, C> for BasicRuleStore<T, OT, U, OU, E, C>
where 
T: Hash + Eq + Clone + Send + Sync, 
OT: Eq + Hash + Clone, 
U: Scalar, 
OU: Scalar,
E: IRuleEffect, 
C: ICondition<OT, OU> {
    fn index_of(&self, t: &T) -> Option<usize> {
        self.inner.index_of(t)
    }

    fn conditions(&self) -> &Vec<C> {
        &self.stat
    }
    
    fn req_of_types(&self) -> &AHashMap<TypeId, OU> {
        &self.amount
    }
    
    fn effect_of(&self, ind: usize) -> Option<&E> {
        self.inner.at(ind).map(|r| r.effect())
    }

    fn condition_of(&self, ind: usize) -> Option<&C> {
        self.inner.at(ind).map(|r| r.condition())
    }
}
