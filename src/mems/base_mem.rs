use crate::errors::MemError;
use crate::core::*;
use crate::meme_derive::IObj;
use crate::lib_info;

use std::collections::HashMap;
use std::rc::Rc;
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

    fn run(&mut self) -> bool {
        loop {
            while let Some(mut msg) = self.op_queue.pop() {
                match msg.op_type {
                    OperationType::ObjAdd => {
                        if let MsgDataObj::Obj(o) = msg.data {
                            self.add_obj(o);
                            
                        }
                    },
                    OperationType::ObjAddBatch => {
                        if let MsgDataObj::Objs(mut objs) = msg.data {
                            while let Some(o) = objs.pop() {
                                self.add_obj(o);
                            }
                        }
                    },
                    OperationType::ObjRemove => {
                        self.drop_obj(&msg.target_id);
                    },
                    OperationType::ObjOut => {
                        if msg.target_id == self.id {
                            if let MsgDataObj::Obj(o) = msg.data {
                                self.add_obj(o);
                            }
                        } else if let Err(e) = self.outter_sender.as_ref().unwrap().send(msg) {
                            log!(target: lib_info::LOG_TARGET_MEM, Level::Error, "Mem {} failed to send message to its outter: {:?}", self.id, e);
                        }
                    },
                    OperationType::ObjIn => {
                        let inner_id = msg.target_id.clone();
                        if self.sub_mem_handels.as_ref().unwrap().contains_key(&inner_id) {
                            if let Some(sender) = self.inner_senders.as_ref().unwrap().get(&inner_id) {
                                msg.op_type = OperationType::ObjAdd;
                                if let Err(e) = sender.send(msg) {
                                    log!(target: lib_info::LOG_TARGET_MEM, Level::Error, "Mem {} failed to send message to its inner {}: {:?}", self.id, inner_id, e);
                                }
                            }
                        }
                    },
                    OperationType::MemAdd => {
                        if let MsgDataObj::Membrane(m) = msg.data {
                            self.add_mem(m);
                        }
                    },
                    OperationType::MemRemove => {
                        let sub_mem_id = msg.target_id.clone();
                        if let Some(handel) = self.sub_mem_handels.as_mut().unwrap().remove(&sub_mem_id) {
                            if let Some(sender) = self.inner_senders.as_mut().unwrap().remove(&sub_mem_id) {
                                msg.op_type = OperationType::Stop;
                                if let Err(e) = sender.send(msg) {
                                    log!(target: lib_info::LOG_TARGET_MEM, Level::Error, "Mem {} failed to send message to its inner {}: {:?}", self.id, sub_mem_id, e);
                                }
                                let _ = handel.join().expect("Couldn't join on the associated thread");
                            }
                        }
                      
                    },
                    OperationType::MemAttachOutter => {
                        if let MsgDataObj::Sender(s) = msg.data {
                            self.outter_sender = Some(s);
                        }
                    },
                    OperationType::MemAttachInner => {
                        if let MsgDataObj::Inners((is, smh)) = msg.data {
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
                        if let MsgDataObj::Rule(r) = msg.data {
                            self.add_rule(r);
                        }
                    },
                    OperationType::RuleAddBatch => {
                        if let MsgDataObj::Rules(mut rules) = msg.data {
                            while let Some(r) = rules.pop() {
                                self.add_rule(r);
                            }
                        }
                    },
                    OperationType::Stop => {
                        if !self.op_queue.is_empty() {
                            self.op_queue.push(msg);
                            let last_pos = self.op_queue.len() - 1;
                            self.op_queue.swap(0, last_pos);
                            continue;
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
                        return true;
                    },
                }
            }//while let

            for r in self.rules.values_mut() { //todo: 多线程化
                let needs = r.obj_needs();
                let mut offer = Offer::<T, V>::new(needs.general_count);
                let mut rand_picks: Vec<Vec<Rc<&PObj<T, V>>>> = Vec::new();
                let mut will_remove: Vec<T> = Vec::new();
                rand_picks.resize(needs.general_count, Vec::new());
                
                for (id, is_take) in needs.specific.iter() {
                    if let Some(o) = self.objs.get(id) {
                        offer.specific.push(DataObj { id: o.get_id(), data: o.get_copy_data_vec() });
                    }
                    if *is_take {
                        self.objs.remove(id);
                    }
                }

                let mut count = needs.general.iter().map(|n| n.count).collect::<Vec<_>>();
               for o in self.objs.values() {
                    if let Some(pos) = needs.pos_map.get(&o.get_obj_type().tid) {
                        if needs.general[*pos].is_random {
                            rand_picks[*pos].push(Rc::new(o));
                        } else {
                            if let Some(ref mut c) = count[*pos] {
                                if *c == 0 {
                                    continue;
                                }
                                *c -= 1;
                            }
                            if needs.general[*pos].is_take {
                                will_remove.push(o.get_id());
                            }
                            offer.general[*pos].push(DataObj { id: o.get_id(), data:o.get_copy_data_vec() });
                        }
                    }
                }

                let mut rng = thread_rng();
                for (i, ptrs)in rand_picks.iter_mut().enumerate() {
                    if ptrs.is_empty() { continue; }
                    ptrs.shuffle(&mut rng);
                    for p in ptrs.iter().take(needs.general[i].count.unwrap_or(ptrs.len())) {
                        if needs.general[i].is_take {
                            will_remove.push(p.get_id());
                        }
                        offer.general[i].push(DataObj { id: p.get_id(), data: p.get_copy_data_vec() });
                    }
                }
               
                for id in will_remove { //取决于Hasher
                    self.objs.remove(&id);
                }
              
                if let Some(mut op) = r.run(DataObj::new(self.id.clone(), self.vec_data.clone()), offer) {
                    self.op_queue.append(&mut op);
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