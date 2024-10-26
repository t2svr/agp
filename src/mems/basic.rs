use crate::rules::BasicCondition;
use crate::rules::BasicEffect;
use crate as meme;
use crate::core::*;
use crate::meme_derive::*;
use crate::objs::BasicObjStore;
use crate::rules::BasicRuleStore;

use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::Mutex;

use krnl::scalar::Scalar;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;

pub type PBasicRule<RT, OT, U> = PRule<RT, OT, U, U, BasicEffect<OT, U>, BasicCondition<OT, U>>;

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
    pub fn new(tag: T) -> Self {
        Self {
            tag,
            ready: false,
            objs: BasicObjStore::new(),
            rules:  BasicRuleStore::new()
        }
    }

    pub fn init(&mut self, mut objs: Vec<PObj<OT, U>>, mut rules: Vec<PBasicRule<RT, OT, U>>) {
        while let Some(o) = objs.pop() {
            self.objs.add_or_update(o.obj_tag().clone(), o);
        }
        while let Some(r) = rules.pop() {
            self.rules.add_or_update(r.obj_tag().clone(), r);
        }
        self.ready = true;
    }

    pub fn effect_proc(
        es: &Vec<OperationEffect<OT, U>>, req: RequestedObj<'_, OT, U>, stop_mux: &Arc<Mutex<bool>>,
        out_to_add: &mut Vec<PObj<OT, U>>, out_to_remove: &mut Vec<OT>) {
        for e in es {
            match e {
                OperationEffect::CreateObjs(f) => {
                    let mut new_o = f(&req);
                    while let Some(o) = new_o.pop() {
                        out_to_add.push(o);
                    }
                },
                OperationEffect::RemoveObjs(f) => {
                    let mut v = f(&req);
                    while let Some(t) = v.pop() {
                        out_to_remove.push(t);
                    }
                },
                OperationEffect::CreateObj(f) => {
                    out_to_add.push(f(&req));
                },
                OperationEffect::Stop => {
                    *stop_mux.lock().unwrap() = true;
                }
                _ => {}
            }
        }
    }
}

impl<T, OT, RT, U> IMem for BasicMem<T, OT, RT, U>
where 
T: Clone + Hash + Eq + Debug + 'static, 
OT: Clone + Hash + Eq + Send + Sync + Debug + 'static, 
RT: Clone + Hash + Eq + Send + Sync + Debug + 'static,
U: Scalar  {
    
    fn ready(&self) -> bool {
        self.ready
    }
    
    fn run(&mut self) -> EmuStatus {
        let stop = Arc::new(Mutex::new(false));
        loop {
            let executable = self.rules.check_on_simple(&self.objs);
            if executable.conflict_executable.is_none()
            && executable.parallel_executable.is_none() { //膜内规则无法执行，故只能依靠外部改变更改膜内对象或规则，因此为了节省计算资源暂停该膜
                return EmuStatus::Pause;
            }

            //这些规则可以无冲突应用，它们的需求都能被同时满足
            //并行地应用规则，用Map Reduce模式获得结果
            if let Some(pe) = executable.parallel_executable { 
                let (mut to_add, mut to_remove) = pe.par_iter()
                    .filter_map(|i| {
                        self.rules.effect_of(i.rule_index).and_then(|e| e.effects().as_ref()).map(|es| (i, es))
                    })
                    .map(|(i, es)| {
                        let (mut to_add_tmp, mut to_remove_tmp) = (Vec::new(), Vec::new());
                        let req_set = if let Some(tgs) = self.rules.condition_of(i.rule_index).and_then(|c| c.tagged().as_ref()) {
                            let mut res = Vec::new();
                            for tp in tgs {
                                if let TaggedPresenceInfo::OfTag(t) = &tp.info {
                                    res.push(self.objs.get(t).unwrap());
                                }
                            }
                            Some(res)
                        } else {
                            None
                        };

                        let req_rand = if let Some(r_tgs) = &i.rand_tags {
                            Some(
                                r_tgs.iter().map(|v| 
                                    v.iter().map(|t| 
                                        self.objs.get(t).unwrap()
                                    ).collect::<Vec<_>>()
                                ).collect::<Vec<Vec<_>>>()
                            )
                        } else {
                            None
                        };
                        let req = RequestedObj::new(req_set, req_rand);
                        Self::effect_proc(es, req, &stop, &mut to_add_tmp, &mut to_remove_tmp);
                        (to_add_tmp, to_remove_tmp)
                    })
                    .reduce(|| (Vec::new(), Vec::new()), |mut a, mut b| {
                        a.0.append(&mut b.0);
                        a.1.append(&mut b.1);
                        a
                    });

                while let Some(t) = to_remove.pop() {
                    self.objs.remove(&t);
                }
                while let Some(o) = to_add.pop() {
                    self.objs.add_or_update(o.obj_tag().clone(), o);
                }
            }

            //这些规则单独可以应用，但是同时应用可能会冲突
            if let Some(mut ce) = executable.conflict_executable { // todo: 动态应用 -ok 
                let mut rng = thread_rng();
                ce.shuffle(&mut rng);
                let (mut to_add_tmp, mut to_remove_tmp) = (Vec::new(), Vec::new());
                self.rules.dynamic_execute(
                    &mut self.objs, Some(ce),
                    |os, e, req| {
                       
                        if let Some(es) = e.and_then(|e| e.effects().as_ref()) {
                            let (set, rand) = (
                                if req.0.is_empty() { None } else {
                                    Some(
                                        req.0.iter().map(|t| os.get(t).unwrap()).collect::<Vec<_>>()
                                    )
                                },
                                if req.1.is_empty() { None } else {
                                    Some(
                                        req.1.iter().map(|v| {
                                            v.iter().map(|t| os.get(t).unwrap()).collect::<Vec<_>>()
                                        }).collect::<Vec<_>>()
                                    )
                                },
                            );
                            let r = RequestedObj::new(set, rand);
                            Self::effect_proc(es, r, &stop, &mut to_add_tmp, &mut to_remove_tmp);
                            while let Some(t) = to_remove_tmp.pop() {
                                os.remove(&t);
                            }
                            while let Some(o) = to_add_tmp.pop() {
                                os.add_or_update(o.obj_tag().clone(), o);
                            }
                        }
                    }
                );
            }

            if stop.is_poisoned()
            || *stop.lock().unwrap() {
                return EmuStatus::Stopped;
            }
        }
    }
  
}