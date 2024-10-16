

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

// #[derive(IObj, IM2Obj)]
// #[data_type(ValueType)]
// #[obj_type(ObjCat::Membrane)]
// pub struct BaseMem<IdType, ValueType>
// where IdType: Clone + Eq + Hash + Display + 'static, ValueType: Clone + 'static {
//     #[id]
//     id: IdType, 
//     #[data]
//     vec_data: Vec<ValueType>,

//     objs: HashMap<IdType, PM2Obj<IdType, ValueType>>,
//     rules: HashMap<IdType, PM2Rule<IdType, ValueType>>,
//     sub_mem_handels: Option<HashMap<IdType, thread::JoinHandle<Result<bool, MemError>>>>,

//     op_queue: Vec<M2Operation<IdType, ValueType>>,
   
//     ready: bool,

//     /// clone this to other mem
//     msg_sender: Sender<M2Operation<IdType, ValueType>>,
//     msg_receiver: Receiver<M2Operation<IdType, ValueType>>,

//     outter_sender: Option<Sender<M2Operation<IdType, ValueType>>>,
//     inner_senders: Option<HashMap<IdType ,Sender<M2Operation<IdType, ValueType>>>>
// }

// impl<T, V> IM2Mem for BaseMem<T, V>
// where T:  Clone + Eq + Hash + Display + 'static,V:  Clone + 'static {
//     fn get_pref_objs(&self) -> &HashMap<Self::IdType, PM2Obj<Self::IdType, Self::ValueType>> { &self.objs }
//     fn get_pref_rules(&self) -> &HashMap<Self::IdType, PM2Rule<Self::IdType, Self::ValueType>> { &self.rules }
//     fn set_outter_sender(&mut self, s: crossbeam_channel::Sender<M2Operation<T, V>>) { self.outter_sender = Some(s); }

//     fn ready(&self) -> bool { self.ready }
    
//     fn add_obj(&mut self, op: PM2Obj<Self::IdType, Self::ValueType>) {
//         self.objs.insert(op.get_id(), op);
//     }
    
//     fn add_rule(&mut self,  rp: PM2Rule<Self::IdType, Self::ValueType>) {
//         self.rules.insert(rp.get_id(), rp);
//     }

//     fn add_mem(&mut self, mut mp: PM2Mem<Self::IdType, Self::ValueType>) {
//         let id = mp.get_id();
//         if let Ok(sender) = mp.init() {
//             self.inner_senders.as_mut().unwrap().insert(id.clone(), sender);
//             mp.set_outter_sender(self.msg_sender.clone());
            
//             if let Ok(handel) = thread::Builder::new()
//             .name(id.to_string())
//             .spawn(move || -> Result<bool, MemError> {
//                 mp.start()
//             }){
//                 self.sub_mem_handels.as_mut().unwrap().insert(id, handel);
//             } else {
//                 log!(
//                     target: log_target::Mem::Exceptions.into(), 
//                     Level::Error, 
//                     "Thread of mem {id} can't spawn."
//                 );
//             }
//         } else {
//             log!(
//                 target: log_target::Mem::Exceptions.into(), 
//                 Level::Error, 
//                 "Mem {id} failed to init."
//             );
//         }
//     }

//     fn drop_obj(&mut self, id: &Self::IdType) {
//        self.objs.remove(id);
//     }
//     fn drop_rule(&mut self, id: &Self::IdType) {
//         self.rules.remove(id);
//     }

//     fn init(&mut self) -> Result<crossbeam_channel::Sender<M2Operation<Self::IdType, Self::ValueType>>, MemError> {
    
//         self.ready = true;

//         Ok(self.msg_sender.clone())
//     }

//     #[inline]
//     fn actions_on(&mut self, mut operation: M2Operation<T, V>) -> bool {
//         match operation.op_type {
//             OperationType::ObjAdd => {
//                 if let M2MsgDataObj::Obj(o) = operation.data {
//                     self.add_obj(o);
//                 }
//             },
//             OperationType::ObjAddBatch => {
//                 if let M2MsgDataObj::Objs(mut objs) = operation.data {
//                     while let Some(o) = objs.pop() {
//                         self.add_obj(o);
//                     }
//                 }
//             },
//             OperationType::ObjRemove => {
//                 self.drop_obj(&operation.target_id);
//             },
//             OperationType::ObjOut => {
//                 if operation.target_id == self.id {
//                     if let M2MsgDataObj::Obj(o) = operation.data {
//                         self.add_obj(o);
//                     }
//                 } else if let Err(e) = self.outter_sender.as_ref().unwrap().send(operation) {
//                     log!(
//                         target: log_target::Mem::Exceptions.into(),
//                         Level::Error,
//                         "Mem {} failed to send message to its outter: {:?}",
//                         self.id, e
//                     );
//                 }
//             },
//             OperationType::ObjIn => {
//                 let inner_id = operation.target_id.clone();
//                 if self.sub_mem_handels.as_ref().unwrap().contains_key(&inner_id) {
//                     if let Some(sender) = self.inner_senders.as_ref().unwrap().get(&inner_id) {
//                         operation.op_type = OperationType::ObjAdd;
//                         if let Err(e) = sender.send(operation) {
//                             log!(
//                                 target: log_target::Mem::Exceptions.into(), 
//                                 Level::Error, "Mem {} failed to send message to its inner {}: {:?}", 
//                                 self.id, inner_id, e
//                             );
//                         }
//                     }
//                 }
//             },
//             OperationType::MemAdd => {
//                 if let M2MsgDataObj::Membrane(m) = operation.data {
//                     self.add_mem(m);
//                 }
//             },
//             OperationType::MemRemove => {
//                 let sub_mem_id = operation.target_id.clone();
//                 if let Some(handel) = self.sub_mem_handels.as_mut().unwrap().remove(&sub_mem_id) {
//                     if let Some(sender) = self.inner_senders.as_mut().unwrap().remove(&sub_mem_id) {
//                         operation.op_type = OperationType::Stop;
//                         if let Err(e) = sender.send(operation) {
//                             log!(
//                                 target: log_target::Mem::Exceptions.into(), 
//                                 Level::Error, 
//                                 "Mem {} failed to send message to its inner {}: {:?}", 
//                                 self.id, sub_mem_id, e
//                             );
//                         }
//                         let _ = handel.join().expect("Couldn't join on the associated thread");
//                     }
//                 }
              
//             },
//             OperationType::MemAttachOutter => {
//                 if let M2MsgDataObj::Sender(s) = operation.data {
//                     self.outter_sender = Some(s);
//                 }
//             },
//             OperationType::MemAttachInner => {
//                 if let M2MsgDataObj::Inners((is, smh)) = operation.data {
//                     for (id, s) in is {
//                         self.inner_senders.as_mut().unwrap().insert(id,s);
//                     }
//                     for (id, h) in smh {
//                         self.sub_mem_handels.as_mut().unwrap().insert(id,h);
//                     }
//                 }
//                 assert_eq!(self.inner_senders.as_ref().unwrap().len(), self.sub_mem_handels.as_ref().unwrap().len(),)
//             },
//             OperationType::RuleAdd => {
//                 if let M2MsgDataObj::Rule(r) = operation.data {
//                     self.add_rule(r);
//                 }
//             },
//             OperationType::RuleAddBatch => {
//                 if let M2MsgDataObj::Rules(mut rules) = operation.data {
//                     while let Some(r) = rules.pop() {
//                         self.add_rule(r);
//                     }
//                 }
//             },
//             OperationType::Stop => {
//                 if !self.op_queue.is_empty() {
//                     self.op_queue.push(operation);
//                     let last_pos = self.op_queue.len() - 1;
//                     self.op_queue.swap(0, last_pos); // 延迟Stop
//                     return true;
//                 }
//                 for s in self.inner_senders.as_ref().unwrap().values() {
//                     let _ = s.send( M2Operation::<Self::IdType, Self::ValueType> {
//                         op_type: OperationType::MemAttachOutter,
//                         target_id: self.id.clone(),
//                         data: M2MsgDataObj::Sender(self.outter_sender.as_ref().unwrap().clone())
//                     });
//                 }
//                 let _ = self.outter_sender.as_ref().unwrap().send( M2Operation::<Self::IdType, Self::ValueType> {
//                     op_type: OperationType::MemAttachInner,
//                     target_id: self.id.clone(),
//                     data: M2MsgDataObj::Inners((self.inner_senders.take().unwrap(), self.sub_mem_handels.take().unwrap()))
//                 });
//                 return false;
//             },
//         }
//         true
//     }

//     fn run(&mut self) -> bool { // todo: 按照不同的类型分开存放对象 规则执行时便无需重新统计
//         loop {
//             log!(
//                 target: log_target::Mem::Info.into(),
//                 Level::Info, 
//                 "------------------ 
//                 Mem {} processing {} operations", 
//                 self.id, self.op_queue.len()
//             );
//             let mut inst = Instant::now();
//             let loop_start_inst = Instant::now();

//             while let Some(msg) = self.op_queue.pop() {
//                 if !self.actions_on(msg) {
//                     return true;
//                 }
//             }

//             log!(
//                 target: log_target::Mem::Performance.into(), 
//                 Level::Info,
//                 "Operations processing took {} μs", 
//                 inst.elapsed().as_micros()
//             );
//             log!(
//                 target: log_target::Mem::Info.into(), 
//                 Level::Info, "Mem {} processing {} rules", 
//                 self.id, self.rules.len()
//             );


//             let env_data = DataObj::new(self.id.clone(), self.vec_data.clone());

//             let mut gene_obj_stats: HashMap<TypeId, Vec<T>> = HashMap::new();
//             let mut r_rules: Vec<&PM2Rule<T,V>> = Vec::new();

//             for r in self.rules.values() {
//                 for n in r.obj_needs().general.iter() {
//                     gene_obj_stats.entry(n.tid).or_insert_with(|| Vec::new());
//                 }
//                 r_rules.push(r);
//             }

//             log!(
//                 target: log_target::Mem::Info.into(), 
//                 Level::Info, 
//                 "Mem {} stating {} objects", 
//                 self.id, self.objs.len()
//             );
//             inst = Instant::now();

//             self.objs.values().for_each(|o| {
//                 if let Some(v) = gene_obj_stats.get_mut(&o.get_obj_type().tid) {
//                     v.push(o.get_id());
//                 }
//             });

//             log!(
//                 target: log_target::Mem::Performance.into(), 
//                 Level::Info, "Object stat took {} μs", 
//                 inst.elapsed().as_micros()
//             );
//             log!(
//                 target: log_target::Mem::Info.into(), 
//                 Level::Info, 
//                 "Mem {} checking rules", 
//                 self.id
//             );

//             let mut rng = thread_rng();
//             let mut will_run: Vec<(T, Offer<T, V>)> = Vec::new();
//             r_rules.shuffle(&mut rng);
//             for r in r_rules {
//                 log!(
//                     target: log_target::Mem::Info.into(), 
//                     Level::Info, 
//                     "Mem {} checking rule {}", 
//                     self.id, r.get_id()
//                 );
//                 inst = Instant::now();

//                 let mut ofr: Offer<T, V> = Offer::new(r.obj_needs().general.len());
//                 let mut will_remove: Vec<T> = Vec::new();
//                 let mut spi_set: HashSet<T> = HashSet::new();
//                 let mut spi_count_of: HashMap<TypeId, usize> = HashMap::new();
//                 if !r.obj_needs().specific.is_empty() {
//                     if r.obj_needs().specific.iter().any(|spc| {
//                         if let Some(o) = self.objs.get(&spc.oid) {
//                             if let Some(c) = spi_count_of.get_mut(&o.get_obj_type().tid) {
//                                 *c += 1;
//                             } else {
//                                 spi_count_of.insert(o.get_obj_type().tid, 1);
//                             }
//                             false
//                         } else { true }
//                     }) { continue; }
//                     for spc in r.obj_needs().specific.iter() {
//                         if spc.no_data {
//                             ofr.specific.push(DataObj::empty_data(spc.oid.clone()));
//                         } else {
//                             ofr.specific.push(DataObj::new(spc.oid.clone(), self.objs.get(&spc.oid).unwrap().get_copy_data_vec()));
//                         }
                        
//                         if spc.is_take {
//                             will_remove.push(spc.oid.clone());
//                         }
//                         spi_set.insert(spc.oid.clone());
//                     }
//                 }
                
//                 let mut satisfied = true;
//                 let mut will_remove_gener: Vec<Vec<usize>> = Vec::new();
//                 will_remove_gener.resize(r.obj_needs().general.len(), Vec::new());
//                 for (i, g) in r.obj_needs().general.iter().enumerate() {
//                     let objs_of_g = gene_obj_stats.get(&g.tid).unwrap();
//                     let spi_count_in_g = spi_count_of.get(&g.tid).unwrap_or(&0);
//                     if let Some(c) = g.count {
//                         if c > objs_of_g.len() - spi_count_in_g {
//                             satisfied = false;
//                             break;
//                         }
//                     }
//                     let max_need_count = g.count.unwrap_or(objs_of_g.len()) + spi_count_in_g;
//                     let mut selected_index = (0..objs_of_g.len()).collect::<Vec<_>>();
//                     if g.is_random {
//                         selected_index.shuffle(&mut rng);
//                     }
//                     for si in selected_index.iter().take(max_need_count) {
//                         if ofr.general[i].len() == max_need_count - spi_count_in_g {
//                             break;
//                         }
//                         if !spi_set.contains(&objs_of_g[*si]) {
//                             let o = self.objs.get(&objs_of_g[*si]).unwrap();
//                             if g.no_data {
//                                 ofr.general[i].push(DataObj::empty_data(o.get_id().clone()));
//                             } else {
//                                 ofr.general[i].push(DataObj::new(o.get_id().clone(), o.get_copy_data_vec()));
//                             }
                            
//                             if g.is_take {
//                                 will_remove.push(o.get_id().clone());
//                             }
//                         }
//                         will_remove_gener[i].push(*si);
//                     }
//                 }

//                 if satisfied {// apply changes and clone data
//                     for oid in will_remove {
//                         self.objs.remove(&oid);
//                     }
//                     for (i, g) in r.obj_needs().general.iter().enumerate() {
//                         if let Some(v) = gene_obj_stats.get_mut(&g.tid) {
//                             helpers::vec_batch_remove_inplace(v, &will_remove_gener[i]);
//                         }
//                     }
//                     will_run.push((r.get_id(), ofr));
//                 }

//                 log!(
//                     target: log_target::Mem::Performance.into(), 
//                     Level::Info, 
//                     "Rule {} took {} μs to check, satisfied: {}", 
//                     r.get_id(), inst.elapsed().as_micros(), satisfied
//                 );
//             }

//             log!(
//                 target: log_target::Mem::Info.into(),
//                 Level::Info, 
//                 "Mem {} runing {} rules",
//                 self.id, will_run.len()
//             );
//             let run_start_inst = Instant::now();

//             while let Some((rid, offer)) = will_run.pop() {
//                 inst = Instant::now();
//                 if let Some(r) =  self.rules.get_mut(&rid) {
//                     if let Some(mut op) = r.run(env_data.clone(), offer) {
//                         self.op_queue.append(&mut op);
//                     }
//                 }
//                 log!(
//                     target: log_target::Mem::Performance.into(), 
//                     Level::Info, "Rule {} took {} μs to run", 
//                     rid, inst.elapsed().as_micros()
//                 );
//             }

//             log!(
//                 target: log_target::Mem::Performance.into(), 
//                 Level::Info, 
//                 "Rules took {} μs to run", 
//                 run_start_inst.elapsed().as_micros()
//             );

//             if let Ok(msg) = self.msg_receiver.try_recv() {
//                 self.op_queue.push(msg);
//             }
//             log!(
//                 target: log_target::Mem::Performance.into(), 
//                 Level::Info, 
//                 "Mem {} took {} ms to do a loop
//                 ------------------", 
//                 self.id, loop_start_inst.elapsed().as_millis()
//             );
//         }//loop
//     }

   
// }

// impl<IdType: Clone + Eq + Hash + Display, ValueType: Clone> BaseMem<IdType, ValueType> {
//     pub fn new(outter_sender: Sender<M2Operation<IdType, ValueType>>, id: IdType) -> Self {
//         let (s, r) = crossbeam_channel::unbounded();
//         Self{
//             id,
//             vec_data: Vec::new(),
//             ready: false,

//             objs: HashMap::new(),
//             rules: HashMap::new(),
//             sub_mem_handels: Some(HashMap::new()),

//             op_queue: Vec::new(),
         
//             msg_sender: s,
//             msg_receiver: r,
           
//             inner_senders: Some(HashMap::new()),
//             outter_sender:  Some(outter_sender)
//         }
//     }

//     pub fn with_data(outter_sender: Sender<M2Operation<IdType, ValueType>>, id: IdType, vec_data: Vec<ValueType>) -> Self {
//         let (s, r) = crossbeam_channel::unbounded();
//         Self{
//             id,
//             vec_data,
//             ready: false,

//             objs: HashMap::new(),
//             rules: HashMap::new(),
//             sub_mem_handels: Some(HashMap::new()),

//             op_queue: Vec::new(),
         
//             msg_sender: s,
//             msg_receiver: r,
           
//             inner_senders: Some(HashMap::new()),
//             outter_sender:  Some(outter_sender)
//         }
//     }
// }