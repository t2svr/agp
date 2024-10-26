use ahash::{AHashMap, AHashSet};
use krnl::scalar::Scalar;
use rand::seq::IteratorRandom;
use rand::seq::SliceRandom;

use crate::errors::MemError;
use crate::helpers;
use crate::lib_info::log_target;

use std::any::Any;

use std::any::TypeId;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;
use log::{log, Level};

pub type IntoSRStr = dyn Into<&'static str>;
pub type PObj<T, U = u32> = Box<dyn IObj<Tag = T, Unit = U> + Send + Sync>;
pub type ArcObj<T, U = u32> = Arc<dyn IObj<Tag = T, Unit = U> + Send + Sync>;
pub type PRule<T, OT, U, OU, E, C> = Box<dyn IRule<Tag = T, ObjTag = OT, Unit = U, ObjUnit = OU, Effect = E, Condition = C> + Send + Sync>;
pub type UntaggedPresences<U> = Vec<UntaggedPresence<U>>;
pub type TaggedPresences<T> = Vec<TaggedPresence<T>>;

pub type ObjsCrateFn<T, U> = fn(&RequestedObj<T, U>) -> Vec<PObj<T, U>>;
pub type ObjCrateFn<T, U> = fn(&RequestedObj<T, U>) -> PObj<T, U>;
pub type ObjsRemoveFn<T, U> = fn(&RequestedObj<T, U>) -> Vec<T>;
pub type ObjRemoveFn<T, U> = fn(&RequestedObj<T, U>) -> T;
pub type Vvec<T> = Vec<Vec<T>>;

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
    fn get(&self, t: &Tag) -> Option<&Value>;
    fn get_batch(&self, ts: &[Tag]) -> Vec<&Value>;
    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a Value> where Value: 'a;
    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut Value> where Value: 'a;
    fn get_mut(&mut self, t: &Tag) -> Option<&mut Value>;
    fn remove(&mut self, t: &Tag) -> Option<Value>;
    fn remove_batch(&mut self, ts: &[Tag]) -> Vec<Value>;
    fn add_or_update(&mut self, t: Tag, v: Value) -> Option<Value>;
}

pub trait IUntaggedStore<T, Unit: Scalar> {
    fn contains(&self, ty: &T) -> bool;
    fn get(&self, ty: T) -> Option<&Unit>;
    fn increase(&mut self, ty: &T, amount: Unit) -> Option<Unit>;
    fn decrease(&mut self, ty: &T, amount: Unit) -> Option<Unit>;
    fn remove(&mut self, ty: &T)-> Option<Unit>;
}

/// todo: 保证高效实现下的一致性
pub trait IObjStat<Unit: Scalar> {
    fn index_of(&self, ty: &ObjType) -> Option<usize>;
    fn type_count(&self) -> usize;
    fn get_tid(&self, pos: usize) -> Option<&TypeId>;
    /// 该方法用于表示是否存在对象的更改  
    /// 如果对象被更改（或可能更改）则返回 true，否之返回 false  
    /// 使用 [`IObjStat::dismiss()`] 来确认已处理更改，使该方法返回 false
    fn modified(&self) -> bool;
    /// 该方法用于确认外部已经处理了对象更改  
    /// 置 [`IObjStat::modified()`] 为 false
    fn dismiss(&mut self);
    fn amounts(&self) -> impl Iterator<Item = &Unit>;
    fn amount_of(&self, ty: &ObjType) -> Option<Unit>;
    fn amount_of_many(&self, tys: &[ObjType]) -> Vec<&Unit>;
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
/// todo: 引入两个接口，提供在 Effect 中使用 Condition 在检查时选择的对象的 tag 的能力
pub trait IRuleEffect {
    type Effect: Clone;
    fn from_builder(effs: Option<Vec<Self::Effect>>) -> Self;
    fn effects(&self) -> &Option<Vec<Self::Effect>>;
}

/// 规则的条件是规则执行需要的对象，每个（对于tagged）或者一定数量的（对于untagged）对象只能用于一次  
/// 规则执行，这些对象能且仅能被执行的规则修改  
/// 只记录 Untagged 的需求量，tagged 对象需要即时计算  
pub trait ICondition<Tag: Clone + Hash + Eq, Unit: Scalar>: Clone {
    fn from_builder(uts: Option<UntaggedPresences<Unit>>, tgs: Option<TaggedPresences<Tag>>) -> Self;
    fn untagged(&self) -> &Option<UntaggedPresences<Unit>>;
    fn tagged(&self) -> &Option<TaggedPresences<Tag>>;
}


// todo: 在GPU上并行化规则
pub trait IRule: IObj {
    type ObjTag: Clone + Hash + Eq;
    type ObjUnit: Scalar;
    type Condition: ICondition<Self::ObjTag, Self::ObjUnit>;
    type Effect: IRuleEffect;
    fn condition(&self) -> &Self::Condition;
    fn effect(&self) -> &Self::Effect; // todo: 令 Effect 只能修改 Condition 选中的对象
}

/// todo: 保证高效实现下的一致性
pub trait IRuleStat<T, OT, U, E, C>
where OT: Clone + Hash + Eq, U: Scalar, C: ICondition<OT, U> {
    fn index_of(&self, t: &T) -> Option<usize>;
    fn conditions(&self) -> &Vec<C>;
    fn effect_of(&self, ind: usize) -> Option<&E>;
    fn condition_of(&self, ind: usize) -> Option<&C>;
    fn req_of_types(&self) -> &AHashMap<TypeId, U>;
    /// 默认的检查方式可以分离出能并行应用的规则子集（不保证最大）  
    /// 如果不需要提前知道无冲突并行子集（即不需要冲突避免）  
    /// 可以使用 [`IRuleStat::check_on_simple`]
    fn check_on<OS>(&mut self, os: &OS) -> ExecutableRules<OT> 
    where OS: ITaggedStore<OT, PObj<OT, U>> + IObjStat<U> {
        let mut rng = rand::thread_rng();
        let mut released_amount = vec![U::zero(); os.type_count()];
        let mut used_tgs: AHashMap<OT, (usize, Option<usize>, bool)> = AHashMap::new(); 
        let mut rand_tags = Vec::new();
      
        let mut conflict_executable = Vec::<ExecutableInfo<OT>>::new();

        let mut first_tag_confli = AHashSet::new();

        let executable = self.conditions() 
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let mut tag_satisfied = true;
                let mut amount_satisfied = true;
                let mut choosed: AHashSet<OT> = AHashSet::new();
                let mut choosed_each = Vec::new();
                
                if let Some(uts) = c.untagged() {
                    for u in uts {
                        if  os.amount_of(&u.ty).is_none_or(|a| a < u.amount) {
                            amount_satisfied = false;
                            break;
                        }
                    }
                }
                
                if amount_satisfied {
                    if let Some(tgs) = c.tagged() {
                        for t in tgs {
                            match &t.info {
                                TaggedPresenceInfo::OfTag(tg) => {
                                    if !os.contains(tg) {
                                        tag_satisfied = false;
                                        break;
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
                                    choosed_each.push(choosed_new);
                                }
                            };
                        }
                    }
                }

                if tag_satisfied && amount_satisfied {
                    let rand_tgs_ind = if !choosed_each.is_empty() {
                        rand_tags.push(Some(choosed_each));
                        Some(rand_tags.len() - 1)
                    } else { None };

                    let mut tag_confli = false;
                    if c.tagged().is_some() {
                        for t in choosed {
                            if let Some((first_pos, first_rand_tgs_ind, conflicted)) = used_tgs.get_mut(&t) {
                                if !(*conflicted) {
                                    *conflicted = true;
                                    if first_tag_confli.insert(*first_pos) {
                                        conflict_executable.push(
                                            ExecutableInfo { 
                                                rule_index: *first_pos, 
                                                rand_tags: first_rand_tgs_ind.and_then(|i| rand_tags[i].take())
                                            }
                                        );
                                    }
                                }
                                tag_confli = true;
                            } else {
                                used_tgs.insert(t, (i, rand_tgs_ind, false));
                            }
                        }
                    }
                    
                    if tag_confli {
                        conflict_executable.push(ExecutableInfo { 
                            rule_index: i, 
                            rand_tags: rand_tgs_ind.and_then(|i| rand_tags[i].take())
                        });
                        None
                    } else {
                        Some((i, rand_tgs_ind))
                    }
                } else {
                    if let Some(uts) = c.untagged() { // 确认该规则无法执行，从IRuleStat统计中释放需要的untagged对象
                        for u in uts {
                            if let Some(ind) = os.index_of(&u.ty) {
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
                let tid = os.get_tid(i).unwrap();
                if let Some(req) = self.req_of_types().get(tid) {
                    if *req > *a + *r { 
                        return Some(*tid);
                    }
                    return None;
                }
                None
            }).fold(AHashSet::new(), |mut acc, e| { acc.insert(e); acc});
       
        let parallel_executable = executable
            .iter()
            .filter_map(|i| 
                if first_tag_confli.contains(&i.0) {
                    None
                } else {
                    Some((i, &self.conditions()[i.0]))
                }
            )
            .filter_map(|((i, rand_tgs_i), c)| {
                let einfo = ExecutableInfo { 
                    rule_index: *i, 
                    rand_tags: rand_tgs_i.and_then(|i| rand_tags[i].take())
                };
                if let Some(uts) = c.untagged() {
                    for u in uts {
                        if conflict_tys.contains(&u.ty.tid) {
                            conflict_executable.push(einfo);
                            return None;
                        }
                    }
                }
                Some(einfo)
            }).collect::<Vec<_>>();

        ExecutableRules {
            parallel_executable: if parallel_executable.is_empty() { None } else { Some(parallel_executable) },
            conflict_executable: if conflict_executable.is_empty() { None } else { Some(conflict_executable) },
        }
    }
    
    fn check_on_simple<OS>(&mut self, os: &OS) -> ExecutableRules<OT> 
    where OS: ITaggedStore<OT, PObj<OT, U>> + IObjStat<U> {
        let mut rng = rand::thread_rng();
        let mut released_amount = vec![U::zero(); os.type_count()];

        let conflict_executable = self.conditions() 
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let mut tag_satisfied = true;
                let mut amount_satisfied = true;
                let mut choosed: AHashSet<OT> = AHashSet::new();
                let mut choosed_each = Vec::new();
                
                if let Some(uts) = c.untagged() {
                    for u in uts {
                        if  os.amount_of(&u.ty).is_none_or(|a| a < u.amount) {
                            amount_satisfied = false;
                            break;
                        }
                    }
                }
                
                if amount_satisfied {
                    if let Some(tgs) = c.tagged() {
                        for t in tgs {
                            match &t.info {
                                TaggedPresenceInfo::OfTag(tg) => {
                                    if !os.contains(tg) {
                                        tag_satisfied = false;
                                        break;
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
                                    choosed_each.push(choosed_new);
                                }
                            };
                        }
                    }
                }

                if tag_satisfied && amount_satisfied {
                    let rand_tags = if choosed_each.is_empty() { None } else { Some(choosed_each) };
                    Some(ExecutableInfo { rule_index: i, rand_tags })
                } else {
                    if let Some(uts) = c.untagged() { // 确认该规则无法执行，从IRuleStat统计中释放需要的untagged对象
                        for u in uts {
                            if let Some(ind) = os.index_of(&u.ty) {
                                released_amount[ind] += u.amount;
                            }
                        }
                    }
                    None
                }
            }).collect::<Vec<_>>();

        ExecutableRules {
            parallel_executable: None,
            conflict_executable: if conflict_executable.is_empty() { None } else { Some(conflict_executable) },
        }
    }

    fn check_on_tagged<OS>(&mut self, os: &OS) -> ExecutableRules<OT> 
    where OS: ITaggedStore<OT, PObj<OT, U>> + IObjStat<U> {
        let mut rng = rand::thread_rng();
        
        let mut used_tgs: AHashMap<OT, (usize, Option<usize>, bool)> = AHashMap::new(); 
        let mut rand_tags = Vec::new();
      
        let mut conflict_executable = Vec::<ExecutableInfo<OT>>::new();

        let mut first_tag_confli = AHashSet::new();

        let executable = self.conditions() 
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let mut tag_satisfied = true;
          
                let mut choosed: AHashSet<OT> = AHashSet::new();
                let mut choosed_each = Vec::new();
           
                if let Some(tgs) = c.tagged() {
                    for t in tgs {
                        match &t.info {
                            TaggedPresenceInfo::OfTag(tg) => {
                                if !os.contains(tg) {
                                    tag_satisfied = false;
                                    break;
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
                                choosed_each.push(choosed_new);
                            }
                        };
                    }
                }
                

                if tag_satisfied {
                    let rand_tgs_ind = if !choosed_each.is_empty() {
                        rand_tags.push(Some(choosed_each));
                        Some(rand_tags.len() - 1)
                    } else { None };

                    let mut tag_confli = false;
                    if c.tagged().is_some() {
                        for t in choosed {
                            if let Some((first_pos, first_rand_tgs_ind, conflicted)) = used_tgs.get_mut(&t) {
                                if !(*conflicted) {
                                    *conflicted = true;
                                    if first_tag_confli.insert(*first_pos) {
                                        conflict_executable.push(
                                            ExecutableInfo { 
                                                rule_index: *first_pos, 
                                                rand_tags: first_rand_tgs_ind.and_then(|i| rand_tags[i].take())
                                            }
                                        );
                                    }
                                }
                                tag_confli = true;
                            } else {
                                used_tgs.insert(t, (i, rand_tgs_ind, false));
                            }
                        }
                    }
                    
                    if tag_confli {
                        conflict_executable.push(ExecutableInfo { 
                            rule_index: i, 
                            rand_tags: rand_tgs_ind.and_then(|i| rand_tags[i].take())
                        });
                        None
                    } else {
                        Some((i, rand_tgs_ind))
                    }
                } else {
                    None
                }
            }).collect::<Vec<_>>();

        let parallel_executable = executable
            .iter()
            .filter_map(|i| 
                if first_tag_confli.contains(&i.0) {
                    None
                } else {
                    Some(i)
                }
            )
            .filter_map(|(i, rand_tgs_i)| {
                let einfo = ExecutableInfo { 
                    rule_index: *i, 
                    rand_tags: rand_tgs_i.and_then(|i| rand_tags[i].take())
                };
                Some(einfo)
            }).collect::<Vec<_>>();

        ExecutableRules {
            parallel_executable: if parallel_executable.is_empty() { None } else { Some(parallel_executable) },
            conflict_executable: if conflict_executable.is_empty() { None } else { Some(conflict_executable) },
        }
    }

    fn check_on_untagged<OS>(&self, os: &OS) -> ExecutableRules<OT>
    where OS: IUntaggedStore<OT, U> + IObjStat<U> {
       
        let mut released_amount = vec![U::zero(); os.type_count()];

        let mut conflict_executable = Vec::new();

        let executable = self.conditions() 
            .iter()
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
                    Some(i)
                } else {
                    if let Some(uts) = c.untagged() { // 确认该规则无法执行，从IRuleStat统计中释放需要的untagged对象
                        for u in uts {
                            if let Some(ind) = os.index_of(&u.ty) {
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
                let tid = os.get_tid(i).unwrap();
                if let Some(req) = self.req_of_types().get(tid) {
                    if *req > *a + *r { 
                        return Some(*tid);
                    }
                    return None;
                }
                None
            }).fold(AHashSet::new(), |mut acc, e| { acc.insert(e); acc});

        let parallel_executable = executable
            .iter()
            .map(|i| (i, &self.conditions()[*i]))
            .filter_map(|(i, c)| {
                if let Some(uts) = c.untagged() {
                    for u in uts {
                        if conflict_tys.contains(&u.ty.tid) {
                            conflict_executable.push(ExecutableInfo { rule_index: *i, rand_tags: None });
                            return None;
                        }
                    }
                }
                Some(ExecutableInfo { rule_index: *i, rand_tags: None })
            }).collect::<Vec<_>>();

        ExecutableRules {
            parallel_executable: if parallel_executable.is_empty() { None } else { Some(parallel_executable) },
            conflict_executable: if conflict_executable.is_empty() { None } else { Some(conflict_executable) },
        }
    }
    /// 动态执行 `rule_indexes` 中的规则，如果 `rule_indexes` 为 [`None`] 则尝试执行所有规则
    /// todo: 在分配rand时出现问题 -ok， 原因：在迭代器上enumerate 然而 迭代器中Condition并非顺序
    fn dynamic_execute<OS, F>(&mut self, os: &mut OS, rules_info: Option<Vec<ExecutableInfo<OT>>>, mut handler: F)
    where OS: ITaggedStore<OT, PObj<OT, U>> + IObjStat<U>, F: FnMut(&mut OS, Option<&E>, (Vec<OT>, Vvec<OT>)) {
        let mut rng = rand::thread_rng();

        let mut rinfo = rules_info.unwrap_or({
            let mut tmp = (0..self.conditions().len()).map(|i| ExecutableInfo { rule_index: i, rand_tags: None }).collect::<Vec<_>>();
            tmp.shuffle(&mut rng);
            tmp
        });
    
        let ite = rinfo.iter_mut().filter_map(|i| self.condition_of(i.rule_index).map(|c| (i, c)));
        
        ite.for_each(|(i, c)| {
            let mut choosed: AHashSet<OT> = AHashSet::new();
            let mut skip_rand_choose = false;
            let mut rand = if i.rand_tags.is_some() { 
                skip_rand_choose = true;
                i.rand_tags.take().unwrap()
             } else { 
                Vec::new() 
            };
            let mut set = Vec::new();
            if let Some(uts) = c.untagged() {
                for u in uts {
                    if os.amount_of(&u.ty).is_none_or(|a| a < u.amount ) {
                        return;
                    }
                }
            }
            if let Some(tgs) = c.tagged() {
                for t in tgs {
                    match &t.info {
                        TaggedPresenceInfo::OfTag(tg) => {
                            if !os.contains(tg) {
                                return;
                            }
                            set.push(tg.clone());
                            choosed.insert(tg.clone());
                        },
                        TaggedPresenceInfo::RandTags((ty, c)) => {// todo: 如果已提供 不选择 -ok
                            if skip_rand_choose {
                                continue;
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
                            rand.push(choosed_new);
                        }
                    };
                }
            }
            //执行
            handler(os, self.effect_of(i.rule_index), (set, rand));
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

    fn ready(&self) -> bool;
    fn run(&mut self) -> EmuStatus;
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
pub enum OperationEffect<T = u32, U = u32>
where T: Send + Sync, U: Send + Sync {
    CreateObjs(ObjsCrateFn<T, U>),
    CreateObj(ObjCrateFn<T, U>), 
    RemoveObjs(ObjsRemoveFn<T, U>),
    RemoveObj(ObjRemoveFn<T, U>),
    CreateObjUntagged(ObjType),
    RemoveObjUntagged((ObjType, U)),
    DissolveMem,
    Pause,
    Stop
}

#[derive(Debug)]
pub enum EmuStatus {
    Pause,
    Stopped,
    EmuError
}

#[derive(Debug, Clone)] 
pub enum TaggedPresenceInfo<Tag> {
    OfTag(Tag),
    RandTags((ObjType, usize))
}

#[derive(Debug, Clone)] // todo: 标记 take -ok
pub struct TaggedPresence<Tag> {
    pub info: TaggedPresenceInfo<Tag>,
    pub take: bool,
}

impl<Tag> TaggedPresence<Tag> {
    pub fn of_tag(tag: Tag, take: bool) -> Self {
        Self {
            info: TaggedPresenceInfo::OfTag(tag),
            take,
        }
    }

    pub fn rand_tags(tag_info: (ObjType, usize), take: bool) -> Self {
        Self {
            info: TaggedPresenceInfo::RandTags(tag_info),
            take,
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
    pub rand_tags: Option<Vvec<T>>
}

#[derive(Debug)]
pub struct ExecutableRules<T> {
    pub parallel_executable: Option<Vec<ExecutableInfo<T>>>,
    pub conflict_executable: Option<Vec<ExecutableInfo<T>>>,
}

/// todo: 提供不获取对象引用的选项，在只需要对象tag时节省时间
/// todo: 提供获得对象所有权的选项，以实现无锁通信规则
#[derive(Debug)]
pub struct RequestedObj<'a, T, U = u32> {
    pub set: Option<Vec<&'a PObj<T, U>>>,
    pub rand: Option<Vvec<&'a PObj<T, U>>>,

    pub set_taken:  Option<Vec<PObj<T, U>>>,
    pub rand_taken: Option<Vvec<PObj<T, U>>>,

    pub set_tags: Option<Vec<T>>,
    pub rand_tags: Option<Vvec<T>>,
}

impl<'a, T, U> RequestedObj<'a, T, U> {
    pub fn new(set: Option<Vec<&'a PObj<T, U>>>, rand: Option<Vvec<&'a PObj<T, U>>>,) -> Self {
        Self {
            set,
            rand,
            set_taken: None,
            rand_taken: None,
            set_tags: None,
            rand_tags: None,
        }
    }
    
    pub fn the_tagged(&self, i: usize) -> Option<&PObj<T, U>> {
        if let Some(s) = &self.set {
            if i >= s.len() {
                None
            } else {
                Some(s[i])
            }
        } else {
            None
        }
    }
    pub fn rand_tagged(&self, i: usize) -> Option<&Vec<&PObj<T, U>>> {
        if let Some(r) = &self.rand {
            if i >= r.len() {
                None
            } else {
                Some(&r[i])
            }
        } else {
            None
        }
    }

    pub fn the_rand_tagged(&self, i: usize, j: usize) -> Option<&PObj<T, U>> {
        if let Some(r) = &self.rand {
            if i >= r.len()
            || j >= r[i].len() {
                None
            } else {
                Some(r[i][j])
            }
        } else {
            None
        }
    }

}