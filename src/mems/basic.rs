// Copyright 2024 Junshuang Hu
use crate::lib_info::log_target;
use crate::rules::BasicCondition;
use crate::rules::BasicEffect;
use crate as meme;
use crate::core::*;
use crate::meme_derive::*;
use crate::objs::BasicObjStore;
use crate::rules::BasicRuleStore;

use std::any::TypeId;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use log::{log, Level};

use krnl::scalar::Scalar;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;

pub type PBasicRule<RT, OT, U> = PRule<RT, OT, U, U, BasicEffect<OT, U>, BasicCondition<OT, U>>;

#[derive(Debug, Default)]
pub struct EPOut<T,U> {
    pub to_add: Vec<PObj<T,U>>,
    pub to_remove: Vec<T>,
    pub to_inc: Vec<(TypeId, U)>,
    pub to_dec: Vec<(TypeId, U)>
}

impl<T, U> EPOut<T, U> {
    pub fn new() -> Self {
        Self { to_add: Vec::new(), to_remove: Vec::new(), to_inc: Vec::new(), to_dec: Vec::new() }
    }

    pub fn is_empty(&self) -> bool {
        self.to_add.is_empty() && self.to_remove.is_empty() &&
        self.to_inc.is_empty() && self.to_dec.is_empty()
    }
}

#[derive(IObj, Debug)]
pub struct BasicMem<T, OT = T, RT = T, U = u32>
where 
T: Clone + Hash + Eq + Debug + 'static, 
OT: Clone + Hash + Eq + Send + Sync + Debug + 'static, 
RT: Clone + Hash + Eq + Send + Sync + Debug + 'static,
U: Scalar {
    #[tag]
    tag: T,

    ready: bool,
    no_parallel: bool,

    objs: BasicObjStore<OT, U>,
    rules: BasicRuleStore<RT, OT, U>,

}

impl<T, OT, RT, U> BasicMem<T, OT, RT, U>
where 
T: Clone + Hash + Eq + Debug + 'static, 
OT: Clone + Hash + Eq + Send + Sync + Debug + 'static, 
RT: Clone + Hash + Eq + Send + Sync + Debug + 'static,
U: Scalar
{
    pub fn new(tag: T, no_parallel: bool) -> Self {
        Self {
            tag,
            ready: false,
            no_parallel,
            objs: BasicObjStore::new(),
            rules:  BasicRuleStore::new()
        }
    }

    pub fn init(&mut self, mut tagged: Vec<PObj<OT, U>>, mut untagged: Vec<(TypeId, U)>, mut rules: Vec<PBasicRule<RT, OT, U>>) {
        while let Some(o) = tagged.pop() {
            self.objs.add_or_update(o.obj_tag().clone(), o);
        }
        while let Some(r) = rules.pop() {
            self.rules.add_or_update(r.obj_tag().clone(), r);
        }
        while let Some((ty, amount)) = untagged.pop() {
            self.objs.increase(&ty, amount);
        }
        self.ready = true;
    }

    pub fn effect_proc(
        es: &Vec<OperationEffect<OT, U>>, mut req: RequestedObj<'_, OT, U>, stop_mux: &Arc<Mutex<bool>>,
        out: &mut EPOut<OT, U>) {
        for e in es {
            match e {
                OperationEffect::CreateObj(f) => {
                    out.to_add.push(f(&mut req));
                },
                OperationEffect::CreateObjs(f) => {
                    let mut new_o = f(&mut req);
                    while let Some(o) = new_o.pop() {
                        out.to_add.push(o);
                    }
                },
                OperationEffect::RemoveObjs(f) => {
                    let mut v = f(&mut req);
                    while let Some(t) = v.pop() {
                        out.to_remove.push(t);
                    }
                },
                OperationEffect::IncreaseObjUntagged((t, u)) => {
                    out.to_inc.push((t.tid, *u));
                },
                OperationEffect::DecreaseObjUntagged((t, u)) => {
                    out.to_dec.push((t.tid, *u));
                },
                OperationEffect::RemoveObjUntagged(_) => {

                }
                OperationEffect::Stop => {
                    *stop_mux.lock().unwrap() = true;
                },
                _ => {}
            }
        }
    }

    pub fn apply_influences(ep_out: &mut EPOut<OT, U>, os: &mut BasicObjStore<OT, U>) {
        while let Some(t) = ep_out.to_remove.pop() {
            os.remove(&t);
        }
        while let Some(o) = ep_out.to_add.pop() {
            os.add_or_update(o.obj_tag().clone(), o);
        }
        while let Some((ty, a)) = ep_out.to_inc.pop() {
            os.increase(&ty, a);
        }
        while let Some((ty, a)) = ep_out.to_dec.pop() {
            os.decrease(&ty, a);
        }
    }
}

impl<T, OT, RT, U> IMem for BasicMem<T, OT, RT, U>
where 
T: Clone + Hash + Eq + Send + Sync + Debug + 'static, 
OT: Clone + Hash + Eq + Send + Sync + Debug + 'static, 
RT: Clone + Hash + Eq + Send + Sync + Debug + 'static,
U: Scalar
{
    
    fn ready(&self) -> bool {
        self.ready
    }
    
    fn evolve(&mut self) -> EmuStatus {
        let stop = Arc::new(Mutex::new(false));
    
        let time_loop = Instant::now();
        let mut time = Instant::now();
        log!(
            target: log_target::Mem::Info.into(), 
            Level::Info, 
            "Mem {:?} : Checking {} rules with {} objects.",
            self.tag, self.rules.len(), self.objs.len()
        );
        let executable = if self.no_parallel {
            self.rules.check_on_simple(&self.objs)
        } else {
            self.rules.check_on(&self.objs)
        }; // todo: 可选检查方式 -ok

        if executable.conflict_executable.is_none()
        && executable.parallel_executable.is_none() { //膜内规则无法执行，故只能依靠外部改变更改膜内对象或规则，因此为了节省计算资源暂停该膜
            return EmuStatus::Pause;
        }
        log!(
            target: log_target::Mem::Performance.into(), 
            Level::Info, 
            "Mem {:?} : took {} μs to check rules.",
            self.tag, time.elapsed().as_micros()
        );
        time = Instant::now();
        log!(
            target: log_target::Mem::Info.into(), 
            Level::Info, 
            "Mem {:?} : {:?} rules will run parallelly and {:?} rules will run sequentially.",
            self.tag,
            executable.parallel_executable.as_ref().map(|e| e.len()), 
            executable.conflict_executable.as_ref().map(|e| e.len())
        );

        //这些规则可以无冲突应用，它们的需求都能被同时满足
        //并行地应用规则，用Map Reduce模式获得结果
        let mut updates = Vec::new();
        if let Some(mut pe) = executable.parallel_executable { 
            while let Some(e) = pe.pop_front() {
                let c = self.rules.condition_at(e.rule_index);
                if c.is_none() {
                    updates.push(((None, None, e), EPOut::new()));
                    continue;
                }
                let c = c.unwrap();
                let opt_tp = c.tagged().as_ref();
                if c.skip_take() {
                    updates.push(((None, opt_tp, e), EPOut::new()));
                    continue;
                }
                let mut take = None;
                if let Some(tp) = opt_tp {
                    let mut rand_taken = None;
                    let mut set_taken= None;
                    let mut i = 0;
                    tp
                    .iter()
                    .filter(|p| p.use_by == UseBy::Take)
                    .for_each(|p| {
                        match &p.info {
                            TaggedPresenceInfo::OfTag(t) => {
                                if let Some(o) = self.objs.remove(t) {
                                    set_taken.get_or_insert(Vec::new()).push(o);
                                } else {
                                    log!(
                                        target: log_target::Mem::Exceptions.into(), 
                                        Level::Error, 
                                        "In mem {:?} : Trying to get *set* obj by *take* {:?} for rule {:?} but failed.",
                                        self.tag, t, self.rules.tag_at(e.rule_index)
                                    );
                                }
                            },
                            TaggedPresenceInfo::RandTags(_) => {
                                if let Some(rtgs) = e.rand_tags.as_ref().and_then(|rtgs| rtgs.get(i)) {
                                    i += 1;
                                    let os = self.objs.remove_batch_skip(rtgs);
                                    if os.len() != rtgs.len() {
                                        log!(
                                            target: log_target::Mem::Exceptions.into(), 
                                            Level::Error, 
                                            "In mem {:?} : Trying to get *rand* obj {:?} by *take* for rule {:?} but missing objs ( got {} but should be {} ).",
                                            self.tag, rtgs, self.rules.tag_at(e.rule_index), os.len(), rtgs.len()
                                        );
                                    }
                                    rand_taken.get_or_insert(Vec::new()).push(os);
                                }
                            },
                        }
                    });
                    take = RequestTyped::new_opt(set_taken, rand_taken);
                }
                updates.push(((take, opt_tp, e), EPOut::new()));
            }//while let

            //并行执行
            updates.par_iter_mut()
            .filter_map(|(a, b)| {
                self.rules.effect_at(a.2.rule_index).and_then(|eff| eff.effects().as_ref()).map(|es| (a, b, es))
            })
            .for_each(|((take, tp, e), proc_out, es)| {
                let (mut refr_set, mut refr_rand) = (None, None);
                if let Some(tps) = tp {
                    tps.iter()
                    .filter(|p| p.use_by == UseBy::Ref)
                    .for_each(|p| {
                        match &p.info {
                            TaggedPresenceInfo::OfTag(t) => {
                                if let Some(o) = self.objs.get(t) {
                                    refr_set.get_or_insert(Vec::new()).push(o);
                                } else {
                                    log!(
                                        target: log_target::Mem::Exceptions.into(), 
                                        Level::Error, 
                                        "In mem {:?} : Trying to get *set* obj by *ref* {:?} for rule {:?} but failed.",
                                        self.tag, t, self.rules.tag_at(e.rule_index)
                                    );
                                }
                            },
                            TaggedPresenceInfo::RandTags(_) => {
                                if let Some(tgs) = e.rand_tags.as_mut().and_then(|rtgs| rtgs.pop_front()) {
                                    let rv = self.objs.get_batch_skip(&tgs);
                                    if rv.len() != tgs.len() {
                                        log!(
                                            target: log_target::Mem::Exceptions.into(), 
                                            Level::Error, 
                                            "In mem {:?} : Trying to get *rand* obj {:?} by *ref* for rule {:?} but missing objs ( got {} but should be {} ).",
                                            self.tag, tgs, self.rules.tag_at(e.rule_index), rv.len(), tgs.len()
                                        );
                                    }
                                    refr_rand.get_or_insert(Vec::new()).push(rv); 
                                }
                            },
                        }
                    });
                }
                //todo: 收集对象 -ok
                let refr = RequestTyped::new_opt(refr_set, refr_rand);
                let r = RequestedObj::new(refr, take.take(), e.requested_tag.take());
                Self::effect_proc(es, r, &stop, proc_out);
            });

            // 应用更改
            updates.iter_mut().for_each(|(_, epo)| {
                Self::apply_influences( epo, &mut self.objs);
            });
        }

        //这些规则单独可以应用，但是同时应用可能会冲突
        if let Some(mut ce) = executable.conflict_executable { // todo: 动态应用 -ok 
            let mut rng = thread_rng();
            ce.make_contiguous().shuffle(&mut rng);
            let mut proc_out = EPOut::new();
            self.rules.dynamic_execute(
                &mut self.objs, Some(ce),
                |os, rule_tag, e, mut req| {
                    if let Some(es) = e.and_then(|e| e.effects().as_ref()) {
                        let (mut refr_set, mut refr_rand) = (None, None);
                        let (mut tag_set, mut tag_rand) = (None, None);
                        let (mut take_set, mut take_rand) = (None, None);
                        //todo: 收集对象
                        for s in req.0.iter() {
                            if s.method == UseBy::Take {
                                if let Some(o) = os.remove(&s.tag) {
                                    take_set.get_or_insert(Vec::new()).push(o);
                                } else {
                                    log!(
                                        target: log_target::Mem::Exceptions.into(), 
                                        Level::Error, 
                                        "In mem {:?} : Trying to get *set* obj by *take* {:?} for rule {:?} but failed.",
                                        self.tag, s.tag, rule_tag
                                    );
                                }
                            }
                        }
                        for r in req.1.iter() {
                            if r.method == UseBy::Take {
                                let v = os.remove_batch_skip(&r.tag);
                                if v.len() != r.tag.len() {
                                    log!(
                                        target: log_target::Mem::Exceptions.into(), 
                                        Level::Error, 
                                        "In mem {:?} : Trying to get *rand* obj {:?} by *take* for rule {:?} but missing objs ( got {} but should be {} ).",
                                        self.tag, r.tag, rule_tag, v.len(),  r.tag.len()
                                    );
                                }
                                take_rand.get_or_insert(Vec::new()).push(v);
                            }
                        }
                        while let Some(s) = req.0.pop_front() {
                            match s.method {
                                UseBy::Tag => {
                                    tag_set.get_or_insert(Vec::new()).push(s.tag);
                                },
                                UseBy::Ref => {
                                    if let Some(ro) = os.get(&s.tag) {
                                        refr_set.get_or_insert(Vec::new()).push(ro);
                                    } else {
                                        log!(
                                            target: log_target::Mem::Exceptions.into(), 
                                            Level::Error, 
                                            "In mem {:?} : Trying to get *set* obj by *ref* {:?} for rule {:?} but failed.",
                                            self.tag, s.tag, rule_tag
                                        );
                                    }
                                },
                                _ => {}
                            };
                        }
                        while let Some(r) = req.1.pop_front() {
                            match r.method {
                                UseBy::Tag => {
                                    tag_rand.get_or_insert(Vec::new()).push(r.tag);
                                },
                                UseBy::Ref => {
                                    let ro = os.get_batch_skip(&r.tag);
                                    if ro.len() != r.tag.len() {
                                        log!(
                                            target: log_target::Mem::Exceptions.into(), 
                                            Level::Error, 
                                            "In mem {:?} : Trying to get *rand* obj {:?} by *ref* for rule {:?} but missing objs ( got {} but should be {} ).",
                                            self.tag, r.tag,  rule_tag, ro.len(),  r.tag.len()
                                        );
                                    }
                                    refr_rand.get_or_insert(Vec::new()).push(ro);
                                },
                                _ => {}
                            };
                        }
                        let take = RequestTyped::new_opt(take_set, take_rand);
                        let refr = RequestTyped::new_opt(refr_set, refr_rand);
                        let tag = RequestTyped::new_opt(tag_set, tag_rand);
                        let r = RequestedObj::new(refr, take, tag);
                        Self::effect_proc(es, r, &stop, &mut proc_out);
                        Self::apply_influences(&mut proc_out, os);
                    }
                }
            );
        }
        log!(
            target: log_target::Mem::Performance.into(), 
            Level::Info, 
            "Mem {:?} : took {} μs to apply rules.",
            self.tag, time.elapsed().as_micros()
        );
        log!(
            target: log_target::Mem::Performance.into(), 
            Level::Info, 
            "Mem {:?} : took {} μs to do a loop.",
            self.tag, time_loop.elapsed().as_micros()
        );

        if stop.is_poisoned()
        || *stop.lock().unwrap() {
            return EmuStatus::Stopped;
        }
        EmuStatus::Continue
    }
  
}