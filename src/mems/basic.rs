use crate as meme;
use crate::core::*;
use crate::meme_derive::*;
use crate::objs::BasicObjStore;
use crate::rules::BasicRuleStore;

use std::hash::Hash;
use std::sync::Arc;
use std::sync::Mutex;

use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;

#[derive(IObj)]
pub struct BasicMem<T>
where T: Clone + Hash + Eq + Send + Sync + 'static {
    #[tag]
    tag: T,

    ready: bool,

    objs: BasicObjStore<T>,
    rules: BasicRuleStore<T>,

}

impl<T> BasicMem<T>
where T: Clone + Hash + Eq + Send + Sync + 'static {
    pub fn new(tag: T) -> Self {
        Self {
            tag,
            ready: false,
            objs: BasicObjStore::new(),
            rules:  BasicRuleStore::new()
        }
    }

    pub fn init(&mut self) {

        self.ready = true;
    }
}

impl<T> IMem for BasicMem<T>
where T: Clone + Hash + Eq + Send + Sync + 'static {
    
    fn ready(&self) -> bool {
        self.ready
    }
    
    fn run(&mut self) -> EmuStatus {
        let stop = Arc::new(Mutex::new(false));
        loop {
            let executable = self.rules.check_on_tagged(&self.objs);
            if executable.conflict_executable.is_none()
            && executable.parallel_executable.is_none() { //膜内规则无法执行，故只能依靠外部改变更改膜内对象或规则，因此为了节省计算资源暂停该膜
                return EmuStatus::Pause;
            }

            //这些规则可以无冲突应用，它们的需求都能被同时满足
            //并行地应用规则，用Map Reduce模式获得结果
            if let Some(pe) = executable.parallel_executable { 
                let (mut to_add, mut to_remove) = pe.par_iter()
                    .filter_map(|i| {
                        self.rules.effect_of(*i).and_then(|e| e.effects().as_ref()).map(|es| (i, es))
                    })
                    .map(|(i, es)| {
                        let (mut to_add, mut to_remove) = (Vec::new(), Vec::new());
                        let req_set = if let Some(tgs) = self.rules.condition_of(*i).and_then(|c| c.tagged().as_ref()) {
                            let mut res = Vec::new();
                            for tp in tgs {
                                if let TaggedPresence::OfTag(t) = tp {
                                    res.push(self.objs.get(t).unwrap());
                                }
                            }
                            Some(res)
                        } else {
                            None
                        };
                        let req_rand = executable.rand_tags_for.get(i)
                        .map(|r| 
                            r.iter().map(|v| 
                                v.iter().map(|t| 
                                    self.objs.get(t).unwrap()
                                ).collect::<Vec<_>>()
                            ).collect::<Vec<Vec<_>>>()
                        );
                       
                        let req = RequestedObj { set: req_set, rand: req_rand };
                        for e in es {
                            match e {
                                OperationEffect::CreateObj(f) => {
                                    let mut new_o = f(&req);
                                    while let Some(o) = new_o.pop() {
                                        to_add.push(o);
                                    }
                                },
                                OperationEffect::RemoveObj(t) => {
                                    to_remove.push(t.clone());
                                },
                                OperationEffect::UpdateObj(f) => {
                                    to_add.push(f(&req));
                                },
                                OperationEffect::Stop => {
                                    *stop.lock().unwrap() = true;
                                }
                                _ => {}
                            }
                        }
                        (to_add, to_remove)
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
                    self.objs.add_or_update(o.obj_tag(), o);
                }
                
            }

            //这些规则单独可以应用，但是同时应用可能会冲突
            if let Some(mut ce) = executable.conflict_executable { // todo: 动态应用 -ok
                let mut rng = thread_rng();
                ce.shuffle(&mut rng);
                self.rules.dynamic_execute(&mut self.objs, Some(&ce),
                |os, e, req| {
                    let r = RequestedObj {
                        set: if req.0.is_empty() { None } else {
                            Some(req.0.iter().filter_map(|t| os.get(t)).collect::<Vec<_>>()) 
                        },
                        rand: if req.1.is_empty() { None } else {
                            Some(
                                req.1.iter().map(|v| 
                                    v.iter().filter_map(|t| 
                                        os.get(t))
                                    .collect::<Vec<_>>()
                                ).collect::<Vec<_>>()
                            )
                        },
                    };

                    let (mut to_add, mut to_remove) = (Vec::new(), Vec::new());
                    if let Some(es) = e.and_then(|e| e.effects().as_ref()) {
                        for e in es {
                            match e {
                                OperationEffect::CreateObj(f) => {
                                    let mut new_o = f(&r);
                                    while let Some(o) = new_o.pop() {
                                        to_add.push(o);
                                    }
                                },
                                OperationEffect::RemoveObj(t) => {
                                    to_remove.push(t.clone());
                                },
                                OperationEffect::UpdateObj(f) => {
                                    to_add.push(f(&r));
                                },
                                OperationEffect::Stop => {
                                    *stop.lock().unwrap() = true;
                                }
                                _ => {}
                            }
                        }
                    }

                    while let Some(t) = to_remove.pop() {
                        os.remove(&t);
                    }
                    while let Some(o) = to_add.pop() {
                        os.add_or_update(o.obj_tag(), o);
                    }
                });
            }

            if stop.is_poisoned()
            || *stop.lock().unwrap() {
                return EmuStatus::Stopped;
            }
        }
    }
  
}