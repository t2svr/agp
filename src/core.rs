// Copyright 2024 Junshuang Hu
use ahash::{AHashMap, AHashSet};
use krnl::scalar::Scalar;
use rand::seq::IteratorRandom;
use rand::seq::SliceRandom;

use crate::errors::MemError;
use crate::helpers;
use crate::lib_info::log_target;
use crate::rules::BasicCondition;
use crate::rules::BasicEffect;

use std::any::Any;

use std::any::TypeId;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;
use log::{log, Level};

pub type IntoSRStr = dyn Into<&'static str>;
pub type PObj<T, U = u32> = Box<dyn IObj<Tag = T, Unit = U> + Send + Sync>;
pub type ArcObj<T, U = u32> = Arc<dyn IObj<Tag = T, Unit = U> + Send + Sync>;
pub type PRule<T, OT = T, U = u32, OU = U, E = BasicEffect<OT, OU>, C = BasicCondition<OT, OU>> 
            = Box<dyn IRule<Tag = T, ObjTag = OT, Unit = U, ObjUnit = OU, Effect = E, Condition = C> + Send + Sync>;
pub type UntaggedPresences<U> = Vec<UntaggedPresence<U>>;
pub type TaggedPresences<T> = Vec<TaggedPresence<T>>;

pub type ObjsCrateFn<T, U> = fn(&mut RequestedObj<T, U>) -> Vec<PObj<T, U>>;
pub type ObjCrateFn<T, U> = fn(&mut RequestedObj<T, U>) -> PObj<T, U>;
pub type ObjsRemoveFn<T, U> = fn(&mut RequestedObj<T, U>) -> Vec<T>;
pub type ObjRemoveFn<T, U> = fn(&mut RequestedObj<T, U>) -> T;
pub type Vvec<T> = Vec<Vec<T>>;
pub type Qvec<T> = VecDeque<Vec<T>>;
pub type DynamicRequest<OT> = (VecDeque<DynamicRequestItem<OT>>, VecDeque<DynamicRequestItem<Vec<OT>>>);

pub const DEFAULT_GROUP: TypeGroup = TypeGroup::Normal;

pub trait IObj: Debug {
    type Tag: Clone + Hash + Eq;
    type Unit: Scalar;
    fn obj_tag(&self) -> &Self::Tag;
    fn obj_type(&self) -> ObjType;
    fn obj_amount(&self) -> Self::Unit;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait ITaggedStore<Tag, Value> {
    fn contains(&self, t: &Tag) -> bool;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;

    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Value> where Value: 'a;
    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut Value> where Value: 'a;

    fn get(&self, t: &Tag) -> Option<&Value>;
    /// 批量获取  
    /// 按顺序获取 `ts` 中 `tag` 对应的 `value`，不存在的会返回 `None`
    fn get_batch(&self, ts: &[Tag]) -> Vec<Option<&Value>>;
    /// 批量获取  
    /// 按顺序获取 `ts` 中 `tag` 对应的 `value`，不存在的会跳过
    fn get_batch_skip(&self, ts: &[Tag]) -> Vec<&Value>;
    fn get_mut(&mut self, t: &Tag) -> Option<&mut Value>;

    /// 添加 `(t, v)`  
    /// 如果已存在 `(t, v_old)` 则替换并返回 `(t, v_old)`
    fn add_or_update(&mut self, t: Tag, v: Value) -> Option<Value>;

    fn remove(&mut self, t: &Tag) -> Option<Value>;
    /// 批量删除  
    /// 按顺序移除并返回 `ts` 中 `tag` 对应的 `value`，不存在的会返回 `None`
    fn remove_batch(&mut self, ts: &[Tag]) -> Vec<Option<Value>>;
    /// 批量删除  
    /// 按顺序移除并返回 `ts` 中 `tag` 对应的 `value`，不存在的会被跳过
    fn remove_batch_skip(&mut self, ts: &[Tag]) -> Vec<Value>;
}

pub trait IUntaggedStore<Ty, Unit: Scalar> {
    fn contains_u(&self, ty: &Ty) -> bool;
    fn len_u(&self) -> usize;
    fn is_empty_u(&self) -> bool;

    fn iter_u<'a>(&'a self) -> impl Iterator<Item = &'a Unit> where Unit: 'a;
    fn iter_mut_u<'a>(&'a mut self) -> impl Iterator<Item = &'a mut Unit> where Unit: 'a;

    fn get_u(&self, ty: &Ty) -> Option<Unit>;
    /// 增加 `ty` 对象的数量  
    /// 如果已有 `ty` 对象 将增加 `amount` 单位并返回 `true`  
    /// 如果不存在则插入 `amount` 单位的 `ty` 并返回 `false`
    fn increase(&mut self, ty: &Ty, amount: Unit) -> bool;
    /// 减少 `ty` 对象的数量  
    /// 如果已有足够的 `ty` 对象 将减少 `amount` 单位并返回 `true`  
    /// 如果不存在 `ty` 或者数量不足则返回 `false`
    fn decrease(&mut self, ty: &Ty, amount: Unit) -> bool;
    fn remove_u(&mut self, ty: &Ty)-> Option<Unit>;
}

/// todo: 保证高效实现下的一致性
pub trait IObjStat<Unit: Scalar> {
    fn pos_of(&self, ty: &ObjType) -> Option<usize>;
    fn type_count(&self) -> usize;
    fn tid_at(&self, pos: usize) -> Option<&TypeId>;

    fn amounts(&self) -> impl Iterator<Item = &Unit>;
    fn amount_of(&self, ty: &ObjType) -> Option<Unit>;
    fn amount_of_many(&self, tys: &[ObjType]) -> Vec<&Unit>;

    fn amounts_u(&self) -> impl Iterator<Item = &Unit>;
    fn amount_of_u(&self, ty: &ObjType) -> Option<Unit>;
    fn amount_of_many_u(&self, tys: &[ObjType]) -> Vec<&Unit>;

    // /// 该方法用于表示是否存在对象的更改  
    // /// 如果对象被更改（或可能更改）则返回 true，否之返回 false  
    // /// 使用 [`IObjStat::dismiss()`] 来确认已处理更改，使该方法返回 false
    // fn modified(&self) -> bool;
    // /// 该方法用于确认外部已经处理了对象更改  
    // /// 置 [`IObjStat::modified()`] 为 false
    // fn dismiss(&mut self);
}

/// Effect 由规则产生，但是由膜解释，故在此处类型不受限  
/// 相同的 Effect 在不同类型的膜中可以解释成不同的操作，这种泛化能力用于可以减少规则的数量  
/// 允许泛化的理由：  
///   - 只依赖规则、对象以及对象的区域，实际上可以表示所有膜系统，不需要定义不同膜的特性
///   - 但是如果只用规则和对象，会导致定义复杂，规则设计困难，规则对象数量过多等问题
///   - 在有膜特征的系统中，膜特征隐式表示了膜内的难以列举的因素，简化了规则，例如，在不同细胞蛋白质的表达受到很多因素的控制，  
///     如果不考虑不同的细胞特征，而是列出一个适用于所有膜的规则（这样的规则确实存在）会十分艰巨
///   - 但是这也不意味着我们需要为每一种膜设计规则，某些规则是相似的，因此通过本接口的 Effect 可以设计出可以在  
///     多种膜内使用的规则，这些规则既不需要考虑所有条件，来用于任何膜，也不只为了某种膜系统设计
/// 
/// 可以称这种可用于多种膜系统的规则为泛用型规则  
/// 提出泛用型规则的理由：
///   - 生物体中多种细胞协同运行，不同类型的膜系统可以结合成为一个更大的系统（可以称为组合膜系统）
///   - 组合膜系统中就可以使用泛用型规则，简化设计，优化性能
/// 
/// todo: 引入两个接口，提供在 Effect 中使用 Condition 在检查时选择的对象的 tag 的能力 -ok
pub trait IRuleEffect {
    type Effect: Clone;
    fn from_builder(effs: Option<Vec<Self::Effect>>) -> Self;
    fn effects(&self) -> &Option<Vec<Self::Effect>>;
}

/// 规则的条件是规则执行需要的对象，每个（对于tagged）或者一定数量的（对于untagged）对象只能用于一次  
/// 规则执行，这些对象能且仅能被执行的规则修改  
/// 只记录 Untagged 的需求量，tagged 对象需要即时计算  
pub trait ICondition<Tag: Clone + Hash + Eq, Unit: Scalar>: Clone {
    fn from_builder(uts: Option<UntaggedPresences<Unit>>, tgs: Option<TaggedPresences<Tag>>, skip_take: bool) -> Self;
    fn untagged(&self) -> &Option<UntaggedPresences<Unit>>;
    fn tagged(&self) -> &Option<TaggedPresences<Tag>>;
    fn skip_take(&self) -> bool;
}


// todo: 在GPU上并行化规则
pub trait IRule: IObj {
    type ObjTag: Clone + Hash + Eq;
    type ObjUnit: Scalar;
    type Condition: ICondition<Self::ObjTag, Self::ObjUnit>;
    type Effect: IRuleEffect;
    fn condition(&self) -> &Self::Condition;
    fn effect(&self) -> &Self::Effect; // todo: 令 Effect 只能修改 Condition 选中的对象 -ok
}

/// todo: 保证高效实现下的一致性
pub trait IRuleStat<T, OT, U, E, C>
where OT: Clone + Hash + Eq, U: Scalar, C: ICondition<OT, U> {
    fn pos_of(&self, t: &T) -> Option<usize>;
    fn tag_at(&self, pos: usize) -> Option<T>;
    fn effect_at(&self, pos: usize) -> Option<&E>;
    fn condition_at(&self, pos: usize) -> Option<&C>;
    fn conditions_count(&self) -> usize;

    fn conditions<'a>(&'a self) -> impl Iterator<Item = &'a C> where C: 'a;
    fn req_of_types(&self) -> &AHashMap<TypeId, U>;
    
    /// 默认的检查方式可以分离出能并行应用的规则子集（不保证最大）  
    /// 如果不需要提前知道无冲突并行子集（即不需要冲突避免）  
    /// 可以使用 [`IRuleStat::check_on_simple`]
    fn check_on<OS>(&mut self, os: &OS) -> ExecutableRules<OT> 
    where OS: ITaggedStore<OT, PObj<OT, U>> + IUntaggedStore<TypeId, U> + IObjStat<U> {
        let mut rng = rand::thread_rng();
        let mut released_amount = vec![U::zero(); os.type_count()];
        let mut used_tgs: AHashMap<OT, (usize, bool)> = AHashMap::new(); 
      
        let mut conflict_executable = VecDeque::<ExecutableInfo<OT>>::new();

        let mut first_tag_confli = AHashSet::new();

        let mut executable = self.conditions() 
            .enumerate()
            .filter_map(|(i, c)| {
                let mut tag_satisfied = true;
                let mut amount_satisfied = true;
                let mut choosed: AHashSet<OT> = AHashSet::new();
                let mut choosed_each = None;
                
                if let Some(uts) = c.untagged() {
                    for u in uts {
                        if  os.amount_of(&u.ty).is_none_or(|a| a < u.amount) {
                            amount_satisfied = false;
                            break;
                        }
                    }
                }

                let (mut tag_set, mut tag_rand) = (None, None);
                if amount_satisfied {
                    if let Some(tgs) = c.tagged() {
                        for t in tgs {
                            match &t.info {
                                TaggedPresenceInfo::OfTag(tg) => {
                                    if !os.contains(tg) {
                                        tag_satisfied = false;
                                        break;
                                    }
                                    if t.use_by == UseBy::Tag {
                                        tag_set.get_or_insert(Vec::new()).push(tg.clone());
                                    }
                                    choosed.insert(tg.clone());
                                },
                                TaggedPresenceInfo::RandTags((ty, c)) => {
                                    let choosed_new = os.iter()// todo: 从没重复的tag中选择 -ok
                                        .filter_map(|o| {
                                            if o.obj_type() == *ty && !choosed.contains(o.obj_tag()) {
                                                Some(o.obj_tag().clone())
                                            } else {
                                                None
                                            }
                                        }).choose_multiple(&mut rng, *c);
                                    if choosed_new.len() != *c {
                                        tag_satisfied = false;
                                        break;
                                    }
                                    for t in choosed_new.iter() {
                                        choosed.insert(t.clone());
                                    }
                                    if t.use_by == UseBy::Tag {
                                        tag_rand.get_or_insert(Vec::new()).push(choosed_new);
                                    } else if t.use_by != UseBy::None {
                                        choosed_each.get_or_insert(VecDeque::new()).push_back(choosed_new);
                                    }
                                }
                            };
                        }
                    }
                }

                if tag_satisfied && amount_satisfied {
                    let einfo = ExecutableInfo { 
                        rule_index: i, 
                        rand_tags: choosed_each,
                        requested_tag: RequestTyped::new_opt(tag_set, tag_rand),
                        skip_take: c.skip_take()
                    };
                    let mut tag_confli = false;
                    if c.tagged().is_some() {
                        for t in choosed {
                            if let Some((first_pos, conflicted)) = used_tgs.get_mut(&t) {
                                if !(*conflicted) {
                                    *conflicted = true;
                                    first_tag_confli.insert(*first_pos);
                                }
                                tag_confli = true;
                            } else {
                                used_tgs.insert(t, (i, false));
                            }
                        }
                    }
                    if tag_confli {
                        conflict_executable.push_back(einfo);
                        None
                    } else {
                        Some(Some(einfo))
                    }
                } else {
                    if let Some(uts) = c.untagged() { // 确认该规则无法执行，从IRuleStat统计中释放需要的untagged对象
                        for u in uts {
                            if let Some(ind) = os.pos_of(&u.ty) {
                                released_amount[ind] += u.amount;
                            }
                        }
                    }
                    None
                }
            }).collect::<Vec<_>>();

        let conflict_tys = os.amounts()// 计算存在竞争的untagged对象类型
            .zip(released_amount.iter())
            .enumerate()
            .filter_map(|(i, (a, r))| {
                let tid = os.tid_at(i).unwrap();
                if let Some(req) = self.req_of_types().get(tid) {
                    if *req > *a + *r { 
                        return Some(*tid);
                    }
                    return None;
                }
                None
            }).fold(AHashSet::new(), |mut acc, e| { acc.insert(e); acc});
       
        let parallel_executable = executable
            .iter_mut()
            .filter_map(|einfo| {
                if einfo.is_none() {
                    None
                } else {
                    let i = einfo.as_ref().map(|info| info.rule_index).unwrap();
                    if first_tag_confli.contains(&i) {
                        conflict_executable.push_back(einfo.take().unwrap());
                        None
                    } else {
                        if let Some(c) = self.condition_at(i) {
                            if let Some(uts) = c.untagged() {
                                for u in uts {
                                    if conflict_tys.contains(&u.ty.tid) {
                                        conflict_executable.push_back(einfo.take().unwrap());
                                        return None;
                                    }
                                }
                            }
                        }
                        einfo.take()
                    }
                }
            }).collect::<VecDeque<_>>();

        ExecutableRules {
            parallel_executable: if parallel_executable.is_empty() { None } else { Some(parallel_executable) },
            conflict_executable: if conflict_executable.is_empty() { None } else { Some(conflict_executable) },
        }
    }
    
    fn check_on_simple<OS>(&mut self, os: &OS) -> ExecutableRules<OT> 
    where OS: ITaggedStore<OT, PObj<OT, U>> + IUntaggedStore<TypeId, U> + IObjStat<U> {
        let mut rng = rand::thread_rng();
        let conflict_executable = self.conditions() 
            .enumerate()
            .filter_map(|(i, c)| {
                let mut tag_satisfied = true;
                let mut amount_satisfied = true;
                let mut choosed: AHashSet<OT> = AHashSet::new();
                let mut choosed_each = None;
                
                if let Some(uts) = c.untagged() {
                    for u in uts {
                        if  os.amount_of(&u.ty).is_none_or(|a| a < u.amount) {
                            amount_satisfied = false;
                            break;
                        }
                    }
                }

                let (mut tag_set, mut tag_rand) = (None, None);
                if amount_satisfied {
                    if let Some(tgs) = c.tagged() {
                        for t in tgs {
                            match &t.info {
                                TaggedPresenceInfo::OfTag(tg) => {
                                    if !os.contains(tg) {
                                        tag_satisfied = false;
                                        break;
                                    }
                                    if t.use_by == UseBy::Tag {
                                        tag_set.get_or_insert(Vec::new()).push(tg.clone());
                                    }
                                    choosed.insert(tg.clone());
                                },
                                TaggedPresenceInfo::RandTags((ty, c)) => {
                                    let choosed_new = os.iter()// todo: 从没重复的tag中选择 -ok
                                        .filter_map(|o| {
                                            if o.obj_type() == *ty && !choosed.contains(o.obj_tag()) {
                                                Some(o.obj_tag().clone())
                                            } else {
                                                None
                                            }
                                        }).choose_multiple(&mut rng, *c);
                                    if choosed_new.len() != *c {
                                        tag_satisfied = false;
                                        break;
                                    }
                                    for t in choosed_new.iter() {
                                        choosed.insert(t.clone());
                                    }
                                    if t.use_by == UseBy::Tag {
                                        tag_rand.get_or_insert(Vec::new()).push(choosed_new);
                                    } else if t.use_by != UseBy::None {
                                        choosed_each.get_or_insert(VecDeque::new()).push_back(choosed_new);
                                    }
                                }
                            };
                        }
                    }
                }

                if tag_satisfied && amount_satisfied {
                    let einfo = ExecutableInfo { 
                        rule_index: i, 
                        rand_tags: choosed_each,
                        requested_tag: RequestTyped::new_opt(tag_set, tag_rand),
                        skip_take: c.skip_take()
                    };
                    Some(einfo)
                } else {
                    None
                }
            }).collect::<VecDeque<_>>();

        ExecutableRules {
            parallel_executable: None,
            conflict_executable: if conflict_executable.is_empty() { None } else { Some(conflict_executable) },
        }
    }

    fn check_on_tagged<OS>(&mut self, os: &OS) -> ExecutableRules<OT> 
    where OS: ITaggedStore<OT, PObj<OT, U>> + IObjStat<U> {
        let mut rng = rand::thread_rng();
        let mut used_tgs: AHashMap<OT, (usize, bool)> = AHashMap::new(); 
      
        let mut conflict_executable = VecDeque::<ExecutableInfo<OT>>::new();

        let mut first_tag_confli = AHashSet::new();

        let mut executable = self.conditions() 
            .enumerate()
            .filter_map(|(i, c)| {
                let mut tag_satisfied = true;
                let mut choosed: AHashSet<OT> = AHashSet::new();
                let mut choosed_each = None;
           
                let (mut tag_set, mut tag_rand) = (None, None);
                
                if let Some(tgs) = c.tagged() {
                    for t in tgs {
                        match &t.info {
                            TaggedPresenceInfo::OfTag(tg) => {
                                if !os.contains(tg) {
                                    tag_satisfied = false;
                                    break;
                                }
                                if t.use_by == UseBy::Tag {
                                    tag_set.get_or_insert(Vec::new()).push(tg.clone());
                                }
                                choosed.insert(tg.clone());
                            },
                            TaggedPresenceInfo::RandTags((ty, c)) => {
                                let choosed_new = os.iter()// todo: 从没重复的tag中选择 -ok
                                    .filter_map(|o| {
                                        if o.obj_type() == *ty && !choosed.contains(o.obj_tag()) {
                                            Some(o.obj_tag().clone())
                                        } else {
                                            None
                                        }
                                    })
                                    .choose_multiple(&mut rng, *c);
                                if choosed_new.len() != *c {
                                    tag_satisfied = false;
                                    break;
                                }
                                for t in choosed_new.iter() {
                                    choosed.insert(t.clone());
                                }
                                if t.use_by == UseBy::Tag {
                                    tag_rand.get_or_insert(Vec::new()).push(choosed_new);
                                } else if t.use_by != UseBy::None {
                                    choosed_each.get_or_insert(VecDeque::new()).push_back(choosed_new);
                                }
                            }
                        };
                    }
                }
            
                if tag_satisfied  {
                    let einfo = ExecutableInfo { 
                        rule_index: i, 
                        rand_tags: choosed_each,
                        requested_tag: RequestTyped::new_opt(tag_set, tag_rand),
                        skip_take: c.skip_take()
                    };
                    let mut tag_confli = false;
                    if c.tagged().is_some() {
                        for t in choosed {
                            if let Some((first_pos, conflicted)) = used_tgs.get_mut(&t) {
                                if !(*conflicted) {
                                    *conflicted = true;
                                    first_tag_confli.insert(*first_pos);
                                }
                                tag_confli = true;
                            } else {
                                used_tgs.insert(t, (i, false));
                            }
                        }
                    }
                    if tag_confli {
                        conflict_executable.push_back(einfo);
                        None
                    } else {
                        Some(Some(einfo))
                    }
                } else {
                    None
                }
            }).collect::<Vec<_>>();

        let parallel_executable = executable
            .iter_mut()
            .filter_map(|einfo| {
                if einfo.is_none() {
                    None
                } else {
                    let i = einfo.as_ref().map(|info| info.rule_index).unwrap();
                    if first_tag_confli.contains(&i) {
                        conflict_executable.push_back(einfo.take().unwrap());
                        None
                    } else {
                        einfo.take()
                    }
                }
            }).collect::<VecDeque<_>>();

        ExecutableRules {
            parallel_executable: if parallel_executable.is_empty() { None } else { Some(parallel_executable) },
            conflict_executable: if conflict_executable.is_empty() { None } else { Some(conflict_executable) },
        }
    }

    fn check_on_untagged<OS>(&self, os: &OS) -> ExecutableRules<OT>
    where OS: IUntaggedStore<OT, U> + IObjStat<U> {
       
        let mut released_amount = vec![U::zero(); os.type_count()];
       
        let mut conflict_executable = VecDeque::<ExecutableInfo<OT>>::new();

        let mut executable = self.conditions() 
            .enumerate()
            .filter_map(|(i, c)| {
                let mut amount_satisfied = true;
           
                if let Some(uts) = c.untagged() {
                    for u in uts {
                        if  os.amount_of(&u.ty).is_none_or(|a| a < u.amount) {
                            amount_satisfied = false;
                            break;
                        }
                    }
                }

                if amount_satisfied {
                    let einfo = ExecutableInfo { 
                        rule_index: i, 
                        rand_tags: None,
                        requested_tag: None,
                        skip_take: c.skip_take()
                    };
                    Some(Some(einfo))
                } else {
                    if let Some(uts) = c.untagged() { // 确认该规则无法执行，从IRuleStat统计中释放需要的untagged对象
                        for u in uts {
                            if let Some(ind) = os.pos_of(&u.ty) {
                                released_amount[ind] += u.amount;
                            }
                        }
                    }
                    None
                }
            }).collect::<Vec<_>>();

        let conflict_tys = os.amounts()// 计算存在竞争的untagged对象类型
            .zip(released_amount.iter())
            .enumerate()
            .filter_map(|(i, (a, r))| {
                let tid = os.tid_at(i).unwrap();
                if let Some(req) = self.req_of_types().get(tid) {
                    if *req > *a + *r { 
                        return Some(*tid);
                    }
                    return None;
                }
                None
            }).fold(AHashSet::new(), |mut acc, e| { acc.insert(e); acc});
       
        let parallel_executable = executable
            .iter_mut()
            .filter_map(|einfo| {
                if einfo.is_none() {
                    None
                } else {
                    let i =einfo.as_ref().map(|info| info.rule_index).unwrap();
                   
                    if let Some(c) = self.condition_at(i) {
                        if let Some(uts) = c.untagged() {
                            for u in uts {
                                if conflict_tys.contains(&u.ty.tid) {
                                    conflict_executable.push_back(einfo.take().unwrap());
                                    return None;
                                }
                            }
                        }
                    }
                    einfo.take()
                    
                }
            }).collect::<VecDeque<_>>();

        ExecutableRules {
            parallel_executable: if parallel_executable.is_empty() { None } else { Some(parallel_executable) },
            conflict_executable: if conflict_executable.is_empty() { None } else { Some(conflict_executable) },
        }
    }
    /// 动态执行 `rule_indexes` 中的规则，如果 `rule_indexes` 为 [`None`] 则尝试执行所有规则
    /// todo: 在分配rand时出现问题 -ok， 原因：在迭代器上enumerate 然而 迭代器中Condition并非顺序
    fn dynamic_execute<OS, F>(&mut self, os: &mut OS, rules_info: Option<VecDeque<ExecutableInfo<OT>>>, mut handler: F)
    where OS: ITaggedStore<OT, PObj<OT, U>> + IUntaggedStore<TypeId, U> + IObjStat<U>, F: FnMut(&mut OS, Option<T>, Option<&E>, DynamicRequest<OT>) {
        let mut rng = rand::thread_rng();

        let mut rinfo = rules_info.unwrap_or({
            let mut tmp = (0..self.conditions_count())
            .map(|i| ExecutableInfo {
                    rule_index: i, 
                    rand_tags: None,
                    requested_tag: None,
                    skip_take: false
                })
            .collect::<VecDeque<_>>();
            tmp.make_contiguous().shuffle(&mut rng);
            tmp
        });
    
        let ite = rinfo.iter_mut().filter_map(|i| self.condition_at(i.rule_index).map(|c| (i, c)));
        
        ite.for_each(|(i, c)| {
            let mut choosed: AHashSet<OT> = AHashSet::new();
            let mut rand = VecDeque::new();
            let mut set = VecDeque::new();
           
            if let Some(uts) = c.untagged() {
                for u in uts {
                    if os.amount_of(&u.ty).is_none_or(|a| a < u.amount ) {
                        return;
                    }
                }
            }
            if let Some(tgs) = c.tagged() {
                for t in tgs {// todo: 如果已提供 不选择 -ok 需要增强
                    match &t.info {
                        TaggedPresenceInfo::OfTag(tg) => {
                            if !os.contains(tg) {
                                return;
                            }
                            if t.use_by != UseBy::None {
                                set.push_back(DynamicRequestItem::new(tg.clone(), t.use_by.clone()));
                            }
                            choosed.insert(tg.clone());
                        },
                        TaggedPresenceInfo::RandTags((ty, c)) => {
                            if t.use_by != UseBy::None {
                                if let Some(v) = i.rand_tags.as_mut().and_then(|rtgs| rtgs.pop_front()) {
                                    rand.push_back(DynamicRequestItem::new(v, t.use_by.clone()));
                                    continue;
                                }
                            }
                            let choosed_new = os.iter()
                            .filter_map(|o| {
                                if o.obj_type() == *ty && !choosed.contains(o.obj_tag()) {
                                    Some(o.obj_tag().clone())
                                } else {
                                    None
                                }
                            })
                            .choose_multiple(&mut rng, *c);

                            if choosed_new.len() != *c {
                                return;
                            }
                            choosed_new.iter().for_each(|o|{
                                choosed.insert(o.clone());
                            });
                            if t.use_by != UseBy::None {
                                rand.push_back(DynamicRequestItem::new(choosed_new, t.use_by.clone()));
                            }
                            
                        }
                    };
                }
            }
            //执行
            handler(os, self.tag_at(i.rule_index), self.effect_at(i.rule_index), (set, rand));
        });
    }
}

pub trait IMem: IObj { 
    fn start(&mut self) -> Result<EmuStatus, MemError<Self::Tag>> {
        if self.ready() { Ok(self.run()) }
        else { 
            log!(
                target: log_target::Mem::Exceptions.into(), 
                Level::Error, 
                "Trying to start mem (TypeID: {:?}) but its not ready.",
                self.obj_type().tid
            );
            Err(MemError::new("Mem start failed."))
        }
    }
    fn run(&mut self) -> EmuStatus {
        loop {
            let loop_state = self.evolve();
            if loop_state != EmuStatus::Continue {
                return loop_state;
            }
        }
    }

    fn ready(&self) -> bool;
    fn evolve(&mut self) -> EmuStatus;
}

pub trait EffectHandler<Effect> {
    fn handle_for(&mut self, e: Effect);
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum TypeGroup {
    #[default]
    Normal,
    Rule,
    Membrane,
    Com,
    Log
}

 // todo: 用闭包作为参数，膜执行闭包 -ok
 // todo: 随机地选择 tagged 对象 -ok
#[derive(Debug, Clone)]
pub enum OperationEffect<OT = u32, U = u32>
where 
OT: Send + Sync, U: Send + Sync {
    CreateObjs(ObjsCrateFn<OT, U>),
    CreateObj(ObjCrateFn<OT, U>), 
    RemoveObjs(ObjsRemoveFn<OT, U>),
    RemoveObj(ObjRemoveFn<OT, U>),
    IncreaseObjUntagged((ObjType, U)),
    DecreaseObjUntagged((ObjType, U)),
    RemoveObjUntagged(ObjType),
    DissolveMem,
    Pause,
    Stop
}

#[derive(Debug, PartialEq)]
pub enum EmuStatus {
    Pause,
    Continue,
    Stopped,
    EmuError
}

#[derive(Debug, Clone)] 
pub enum TaggedPresenceInfo<Tag> {
    OfTag(Tag),
    RandTags((ObjType, usize))
}

#[derive(Debug, Clone, PartialEq)]
pub enum UseBy {
    None, Tag, Ref, Take
}

#[derive(Debug, Clone)] // todo: 标记 take -ok
pub struct TaggedPresence<Tag> {
    pub info: TaggedPresenceInfo<Tag>,
    pub use_by: UseBy
}

impl<Tag> TaggedPresence<Tag> {
    pub fn of_tag(tag: Tag, use_by: UseBy) -> Self {
        Self {
            info: TaggedPresenceInfo::OfTag(tag),
            use_by
        }
    }

    pub fn rand_tags(tag_info: (ObjType, usize), use_by: UseBy) -> Self {
        Self {
            info: TaggedPresenceInfo::RandTags(tag_info),
            use_by
        }
    }
}

#[derive(Debug, Clone)]
pub struct UntaggedPresence<Unit: Scalar> {
    pub ty: ObjType,
    pub amount: Unit,
    pub take: bool
}

#[derive(Debug, Clone)]
pub struct ObjType {
    pub group: &'static TypeGroup,
    pub tid: TypeId
}

impl PartialEq for ObjType {
    fn eq(&self, other: &Self) -> bool {
        self.tid == other.tid
    }
}

impl ObjType {
    pub fn new<T: IObj + ?Sized + 'static>(group: &'static TypeGroup) ->Self {
        Self { group, tid: TypeId::of::<T>() }
    }

    pub fn default_group<T: IObj + ?Sized + 'static>() -> Self {
        Self { group: &crate::core::DEFAULT_GROUP, tid: TypeId::of::<T>() }
    }
}

#[derive(Debug)]
pub struct IndexMap<K, V>
where K: Hash + Eq + Clone, V: Send + Sync {
    map: AHashMap<K, usize>,
    data_v: Vec<(K, V)>
}

impl<K, V> Default for IndexMap<K, V>
where K: Hash + Eq + Clone, V: Send + Sync {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> IndexMap<K, V>
where K: Hash + Eq + Clone, V: Send + Sync {
    pub fn new() -> Self {
        Self {map: AHashMap::new(), data_v: Vec::new()}
    }

    pub fn keys<'a>(&'a self) -> impl Iterator<Item = &'a K> where K: 'a {
        self.data_v.iter().map(|d| &d.0)
    }

    pub fn vals<'a>(&'a self) -> impl Iterator<Item = &'a V> where V: 'a {
        self.data_v.iter().map(|d| &d.1)
    }

    pub fn vals_mut<'a>(&'a mut self) ->  impl Iterator<Item = &'a mut V> where V: 'a  {
        self.data_v.iter_mut().map(|d| &mut d.1)
    }

    pub fn index_of(&self, key: &K) -> Option<usize> {
        self.map.get(key).copied()
    }

    pub fn at(&self, pos: usize) -> Option<&V> {
        self.data_v.get(pos).map(|v| &v.1)
    }

    pub fn containes(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        if let Some((_, v)) = self.map.get(key).and_then(|i| self.data_v.get(*i) ) {
            Some(v)
        } else {
            None
        }
    }

    pub fn get_key(&self, pos: usize) -> Option<&K> {
        self.data_v.get(pos).map(|d| &d.0)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if let Some((_, v)) = self.map.get(key).and_then(|i| self.data_v.get_mut(*i) ) {
            Some(v)
        } else {
            None
        }
    }

    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        if let Some(p) = self.map.get(&k) {
            self.data_v.push((k, v));
            Some( self.data_v.swap_remove(*p).1)
        } else {
            let pos = self.data_v.len();
            self.map.insert(k.clone(), pos);
            self.data_v.push((k, v));
            None
        }
    }

    pub fn remove(&mut self, key: &K) ->  Option<V> {
        if let Some(pos) = self.map.remove(key) {
            let ret = self.data_v.remove(pos).1;
            for i in pos..self.data_v.len() {
                let k = &self.data_v[i].0;
                if let Some(old_p) = self.map.get_mut(k) {
                    *old_p -= 1;
                }
            }
            return Some(ret);
        }
        None
    }

    pub fn remove_batch(&mut self, keys: &[K]) -> Vec<Option<(K, V)>> {
        let indexes = keys.iter().map(|k| self.map.get(k).cloned().unwrap_or(self.data_v.len())).collect::<Vec<_>>();
        helpers::vec_batch_remove(&mut self.data_v, &indexes)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[derive(Debug)]
pub struct ExecutableInfo<T> {
    pub rule_index: usize,
    pub rand_tags: Option<Qvec<T>>,
    pub requested_tag: Option<RequestTyped<T>>,
    pub skip_take: bool
}

#[derive(Debug)]
pub struct ExecutableRules<T> {
    pub parallel_executable: Option<VecDeque<ExecutableInfo<T>>>,
    pub conflict_executable: Option<VecDeque<ExecutableInfo<T>>>,
}

#[derive(Debug, Default)]
pub struct RequestTyped<T> {
   pub set: Option<Vec<T>>,
   pub rand: Option<Vvec<T>>
}

impl<T> RequestTyped<T> {
    pub fn new() -> Self {
        Self {
            set: None, rand: None
        }
    }

    pub fn new_opt(set: Option<Vec<T>>, rand: Option<Vvec<T>>) -> Option<Self> {
        if set.is_none() && rand.is_none() {
            None
        } else {
            Some(Self {
                set, rand
            })
        }
    }

    pub fn set_at(&self, pos: usize) -> Option<&T> {
        self.set.as_ref().and_then(|s| s.get(pos))
    }

    pub fn rand_at(&self, pos: usize) -> Option<&Vec<T>> {
        self.rand.as_ref().and_then(|s| s.get(pos))
    }

    pub fn set_at_mut(&mut self, pos: usize) -> Option<&mut T> {
        self.set.as_mut().and_then(|s| s.get_mut(pos))
    }

    pub fn rand_at_mut(&mut self, pos: usize) -> Option<&mut Vec<T>> {
        self.rand.as_mut().and_then(|s| s.get_mut(pos))
    }
}

pub struct DynamicRequestItem<T> {
    pub tag: T,
    pub method: UseBy
}

impl<T> DynamicRequestItem<T> {
    pub fn new(tag: T, method: UseBy) -> Self {
        Self {
            tag, method
        }
    }
} 

/// todo: 提供不获取对象引用的选项，在只需要对象tag时节省时间
/// todo: 提供获得对象所有权的选项，以实现无锁通信规则
#[derive(Debug)]
pub struct RequestedObj<'a, T, U = u32> {
    pub refr: Option<RequestTyped<&'a PObj<T, U>>>,
    pub take: Option<RequestTyped<PObj<T, U>>>,
    pub tag: Option<RequestTyped<T>>,
}

impl<'a, T, U> RequestedObj<'a, T, U> {
    pub fn new(
        refr: Option<RequestTyped<&'a PObj<T, U>>>,
        take: Option<RequestTyped<PObj<T, U>>>,
        tag: Option<RequestTyped<T>>,
        ) -> Self {
        Self {
            refr, take, tag
        }
    }

    pub fn set_ref_all(&self) -> Option<&Vec<&PObj<T, U>>> {
        self.refr.as_ref().and_then(|r| r.set.as_ref() )
    }
    
    pub fn set_tag(&self, pos: usize) -> Option<&T> {
        self.tag.as_ref().and_then(|t| t.set_at(pos))
    }

    pub fn rand_tags(&self, pos: usize) -> Option<&Vec<T>> {
        self.tag.as_ref().and_then(|t| t.rand_at(pos))
    }

    pub fn set_ref(&self, pos: usize) -> Option<&PObj<T, U>> {
        self.refr.as_ref().and_then(|r| r.set_at(pos).copied())
    }

    pub fn rand_refs(&self, pos: usize) -> Option<&Vec<&PObj<T, U>>> {
        self.refr.as_ref().and_then(|r| r.rand_at(pos))
    }

}