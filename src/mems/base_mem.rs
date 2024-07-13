use crate::errors::MemError;
use crate::core::*;
use crate::helpers;
use crate::meme_derive::IObj;
use crate::lib_info;

use std::any::TypeId;
use std::collections::HashMap;
use std::collections::HashSet;
use std::thread;
use std::hash::Hash;
use crossbeam_channel::{Receiver, Sender};
use rand::prelude::*;
use std::fmt::Display;

use log::Level;
use log::log;

#[derive(IObj)]
#[id_type(IdType)]
#[data_type(ValueType)]
#[obj_type(ObjCat::Membrane)]
pub struct BaseMem<IdType, ValueType>
where IdType: Clone + Eq + Hash + Display + 'static, ValueType: Clone + 'static {
    #[id]
    id: IdType, 
    #[data]
    vec_data: Vec<ValueType>,

    objs: HashMap<IdType, PObj<IdType, ValueType>>,
    rules: HashMap<IdType, PRule<IdType, ValueType>>,
    sub_mem_handels: Option<HashMap<IdType, thread::JoinHandle<Result<bool, MemError>>>>,

    op_queue: Vec<Operation<IdType, ValueType>>,
   
    ready: bool,

    /// clone this to other mem
    msg_sender: Sender<Operation<IdType, ValueType>>,
    msg_receiver: Receiver<Operation<IdType, ValueType>>,

    outter_sender: Option<Sender<Operation<IdType, ValueType>>>,
    inner_senders: Option<HashMap<IdType ,Sender<Operation<IdType, ValueType>>>>
}

impl<T, V> IMem for BaseMem<T, V>
where T:  Clone + Eq + Hash + Display + 'static,V:  Clone + 'static {
    fn get_pref_objs(&self) -> &HashMap<Self::IdType, PObj<Self::IdType, Self::ValueType>> { &self.objs }
    fn get_pref_rules(&self) -> &HashMap<Self::IdType, PRule<Self::IdType, Self::ValueType>> { &self.rules }
    fn set_outter_sender(&mut self, s: crossbeam_channel::Sender<Operation<T, V>>) { self.outter_sender = Some(s); }

    fn ready(&self) -> bool { self.ready }
    
    fn add_obj(&mut self, op: PObj<Self::IdType, Self::ValueType>) {
        self.objs.insert(op.get_id(), op);
    }
    
    fn add_rule(&mut self,  rp: PRule<Self::IdType, Self::ValueType>) {
        self.rules.insert(rp.get_id(), rp);
    }

    fn add_mem(&mut self, mut mp: PMem<Self::IdType, Self::ValueType>) {
        let id = mp.get_id();
        if let Ok(sender) = mp.init() {
            self.inner_senders.as_mut().unwrap().insert(id.clone(), sender);
            mp.set_outter_sender(self.msg_sender.clone());
            
            if let Ok(handel) = thread::Builder::new()
            .name(id.to_string())
            .spawn(move || -> Result<bool, MemError> {
                mp.start()
            }){
                self.sub_mem_handels.as_mut().unwrap().insert(id, handel);
            } else {
                log!(target: lib_info::LOG_TARGET_MEM, Level::Error, "Thread of mem {id} can't spawn.");
            }
        } else {
            log!(target: lib_info::LOG_TARGET_MEM, Level::Error, "Mem {id} failed to init.");
        }
    }

    fn drop_obj(&mut self, id: &Self::IdType) {
       self.objs.remove(id);
    }
    fn drop_rule(&mut self, id: &Self::IdType) {
        self.rules.remove(id);
    }

    fn init(&mut self) -> Result<crossbeam_channel::Sender<Operation<Self::IdType, Self::ValueType>>, MemError> {
    
        self.ready = true;

        Ok(self.msg_sender.clone())
    }

    #[inline]
    fn actions_on(&mut self, mut op: Operation<T, V>) -> bool {
        match op.op_type {
            OperationType::ObjAdd => {
                if let MsgDataObj::Obj(o) = op.data {
                    self.add_obj(o);
                }
            },
            OperationType::ObjAddBatch => {
                if let MsgDataObj::Objs(mut objs) = op.data {
                    while let Some(o) = objs.pop() {
                        self.add_obj(o);
                    }
                }
            },
            OperationType::ObjRemove => {
                self.drop_obj(&op.target_id);
            },
            OperationType::ObjOut => {
                if op.target_id == self.id {
                    if let MsgDataObj::Obj(o) = op.data {
                        self.add_obj(o);
                    }
                } else if let Err(e) = self.outter_sender.as_ref().unwrap().send(op) {
                    log!(target: lib_info::LOG_TARGET_MEM, Level::Error, "Mem {} failed to send message to its outter: {:?}", self.id, e);
                }
            },
            OperationType::ObjIn => {
                let inner_id = op.target_id.clone();
                if self.sub_mem_handels.as_ref().unwrap().contains_key(&inner_id) {
                    if let Some(sender) = self.inner_senders.as_ref().unwrap().get(&inner_id) {
                        op.op_type = OperationType::ObjAdd;
                        if let Err(e) = sender.send(op) {
                            log!(target: lib_info::LOG_TARGET_MEM, Level::Error, "Mem {} failed to send message to its inner {}: {:?}", self.id, inner_id, e);
                        }
                    }
                }
            },
            OperationType::MemAdd => {
                if let MsgDataObj::Membrane(m) = op.data {
                    self.add_mem(m);
                }
            },
            OperationType::MemRemove => {
                let sub_mem_id = op.target_id.clone();
                if let Some(handel) = self.sub_mem_handels.as_mut().unwrap().remove(&sub_mem_id) {
                    if let Some(sender) = self.inner_senders.as_mut().unwrap().remove(&sub_mem_id) {
                        op.op_type = OperationType::Stop;
                        if let Err(e) = sender.send(op) {
                            log!(target: lib_info::LOG_TARGET_MEM, Level::Error, "Mem {} failed to send message to its inner {}: {:?}", self.id, sub_mem_id, e);
                        }
                        let _ = handel.join().expect("Couldn't join on the associated thread");
                    }
                }
              
            },
            OperationType::MemAttachOutter => {
                if let MsgDataObj::Sender(s) = op.data {
                    self.outter_sender = Some(s);
                }
            },
            OperationType::MemAttachInner => {
                if let MsgDataObj::Inners((is, smh)) = op.data {
                    for (id, s) in is {
                        self.inner_senders.as_mut().unwrap().insert(id,s);
                    }
                    for (id, h) in smh {
                        self.sub_mem_handels.as_mut().unwrap().insert(id,h);
                    }
                }
                assert_eq!(self.inner_senders.as_ref().unwrap().len(), self.sub_mem_handels.as_ref().unwrap().len(),)
            },
            OperationType::RuleAdd => {
                if let MsgDataObj::Rule(r) = op.data {
                    self.add_rule(r);
                }
            },
            OperationType::RuleAddBatch => {
                if let MsgDataObj::Rules(mut rules) = op.data {
                    while let Some(r) = rules.pop() {
                        self.add_rule(r);
                    }
                }
            },
            OperationType::Stop => {
                if !self.op_queue.is_empty() {
                    self.op_queue.push(op);
                    let last_pos = self.op_queue.len() - 1;
                    self.op_queue.swap(0, last_pos); // 延迟Stop
                    return true;
                }
                for s in self.inner_senders.as_ref().unwrap().values() {
                    let _ = s.send( Operation::<Self::IdType, Self::ValueType> {
                        op_type: OperationType::MemAttachOutter,
                        target_id: self.id.clone(),
                        data: MsgDataObj::Sender(self.outter_sender.as_ref().unwrap().clone())
                    });
                }
                let _ = self.outter_sender.as_ref().unwrap().send( Operation::<Self::IdType, Self::ValueType> {
                    op_type: OperationType::MemAttachInner,
                    target_id: self.id.clone(),
                    data: MsgDataObj::Inners((self.inner_senders.take().unwrap(), self.sub_mem_handels.take().unwrap()))
                });
                return false;
            },
        }
        return true;
    }

    fn run(&mut self) -> bool { // todo: 按照不同的类型分开存放对象 规则执行时便无需重新统计
        loop {
            while let Some(msg) = self.op_queue.pop() {
                if self.actions_on(msg) == false {
                    return true;
                }
            }

            let mut will_run: Vec<(T, Offer<T, V>)> = Vec::new();
            let env_data = DataObj::new(self.id.clone(), self.vec_data.clone());

            let mut gene_obj_stats: HashMap<TypeId, Vec<T>> = HashMap::new();
            let mut r_rules: Vec<&PRule<T,V>> = Vec::new();

            for r in self.rules.values() {
                for n in r.obj_needs().general.iter() {
                    if !gene_obj_stats.contains_key(&n.tid) {
                        gene_obj_stats.insert(n.tid, Vec::new());
                    }
                }
                r_rules.push(r);
            }

            self.objs.values().for_each(|o| {
                if let Some(v) = gene_obj_stats.get_mut(&o.get_obj_type().tid) {
                    v.push(o.get_id());
                }
            });

            let mut rng = thread_rng();
            r_rules.shuffle(&mut rng);
            for r in r_rules {
                let mut ofr: Offer<T, V> = Offer::new(r.obj_needs().general_count);
                let mut will_remove: Vec<T> = Vec::new();
                let mut spi_set: HashSet<T> = HashSet::new();
                let mut spi_count_of: HashMap<TypeId, usize> = HashMap::new();
                if !r.obj_needs().specific.is_empty() {
                    if r.obj_needs().specific.iter().any(|(id, _)| {
                        if let Some(o) = self.objs.get(id) {
                            if let Some(c) = spi_count_of.get_mut(&o.get_obj_type().tid) {
                                *c += 1;
                            } else {
                                spi_count_of.insert(o.get_obj_type().tid, 1);
                            }
                            false
                        } else { true }
                    }) { continue; }
                    for (oid, is_take) in r.obj_needs().specific.iter() {
                        ofr.specific.push(DataObj::new(oid.clone(), self.objs.get(oid).unwrap().get_copy_data_vec()));
                        if *is_take {
                            will_remove.push(oid.clone());
                        }
                        spi_set.insert(oid.clone());
                    }
                }
                
                let mut satisfied = true;
                let mut will_remove_gener: Vec<Vec<usize>> = Vec::new();
                will_remove_gener.resize(r.obj_needs().general_count, Vec::new());
                for (i, g) in r.obj_needs().general.iter().enumerate() {
                    let objs_of_g = gene_obj_stats.get(&g.tid).unwrap();
                    let spi_count_in_g = spi_count_of.get(&g.tid).unwrap_or(&0);
                    if let Some(c) = g.count {
                        if c > objs_of_g.len() - spi_count_in_g {
                            satisfied = false;
                            break;
                        }
                    }
                    let max_need_count = g.count.unwrap_or(objs_of_g.len()) + spi_count_in_g;
                    let mut selected_index = (0..objs_of_g.len()).collect::<Vec<_>>();
                    if g.is_random {
                        selected_index.shuffle(&mut rng);
                    }
                    for si in selected_index.iter().take(max_need_count) {
                        if ofr.general[i].len() == max_need_count - spi_count_in_g {
                            break;
                        }
                        if !spi_set.contains(&objs_of_g[*si]) {
                            let o = self.objs.get(&objs_of_g[*si]).unwrap();
                            ofr.general[i].push(DataObj::new(o.get_id().clone(), o.get_copy_data_vec()));
                            if g.is_take {
                                will_remove.push(o.get_id().clone());
                            }
                        }
                        will_remove_gener[i].push(*si);
                    }
                }

                if satisfied {// apply changes and clone data
                    for oid in will_remove {
                        self.objs.remove(&oid);
                    }
                    for (i, g) in r.obj_needs().general.iter().enumerate() {
                        if let Some(v) = gene_obj_stats.get_mut(&g.tid) {
                            helpers::vec_batch_remove_inplace(v, &will_remove_gener[i]);
                        }
                    }
                    will_run.push((r.get_id(), ofr));
                }
            }

            while let Some((rid, offer)) = will_run.pop() {
                if let Some(r) =  self.rules.get_mut(&rid) {
                    if let Some(mut op) = r.run(env_data.clone(), offer) {
                        self.op_queue.append(&mut op);
                    }
                }
            }

            if let Ok(msg) = self.msg_receiver.try_recv() {
                self.op_queue.push(msg);
            }
          
        }//loop
    }

   
}

impl<IdType: Clone + Eq + Hash + Display, ValueType: Clone> BaseMem<IdType, ValueType> {
    pub fn new(outter_sender: Sender<Operation<IdType, ValueType>>, id: IdType) -> Self {
        let (s, r) = crossbeam_channel::unbounded();
        Self{
            id,
            vec_data: Vec::new(),
            ready: false,

            objs: HashMap::new(),
            rules: HashMap::new(),
            sub_mem_handels: Some(HashMap::new()),

            op_queue: Vec::new(),
         
            msg_sender: s,
            msg_receiver: r,
           
            inner_senders: Some(HashMap::new()),
            outter_sender:  Some(outter_sender)
        }
    }

    pub fn with_data(outter_sender: Sender<Operation<IdType, ValueType>>, id: IdType, vec_data: Vec<ValueType>) -> Self {
        let (s, r) = crossbeam_channel::unbounded();
        Self{
            id,
            vec_data,
            ready: false,

            objs: HashMap::new(),
            rules: HashMap::new(),
            sub_mem_handels: Some(HashMap::new()),

            op_queue: Vec::new(),
         
            msg_sender: s,
            msg_receiver: r,
           
            inner_senders: Some(HashMap::new()),
            outter_sender:  Some(outter_sender)
        }
    }
}