use crate::{errors::MemError, helpers::NeedCount};
use crate::core::*;
use crate::meme_derive::IObj;

use std::collections::HashMap;
use std::thread;
use std::hash::Hash;
use crossbeam_channel::{Receiver, Sender};
use std::fmt::Display;

#[derive(IObj)]
#[obj_id_type(IdType)]
#[obj_data_type(ValueType)]
#[obj_type(ObjCat::Membrane)]
pub struct BaseMem<IdType, ValueType>
where IdType: Clone + Eq + Hash + Display + 'static, ValueType: Clone + 'static {
    #[id]
    id: IdType, 
    #[data]
    vec_data: Vec<ValueType>,

    objs: HashMap<IdType, Box<dyn IObj<IdType = IdType, ValueType = ValueType> + Send>>,
    rules: HashMap<IdType, Box<dyn IRule<IdType = IdType, ValueType = ValueType>  + Send>>,
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
    fn get_pref_objs(&self) -> &HashMap<Self::IdType, Box<(dyn IObj<IdType = T, ValueType = V> + Send + 'static)>> { &self.objs }
    fn get_pref_rules(&self) -> &HashMap<Self::IdType, Box<dyn IRule<IdType = T, ValueType = V> + Send>> { &self.rules }
    fn set_outter_sender(&mut self, s: crossbeam_channel::Sender<Operation<T, V>>) { self.outter_sender = Some(s); }

    fn ready(&self) -> bool { self.ready }
    
    fn add_obj(&mut self, op: Box<(dyn IObj<IdType = T, ValueType = V> + Send + 'static)>) {
        if let Some(old) = self.objs.get_mut(&op.get_id()) {
            *old = op;
        } else {
            self.objs.insert(op.get_id(), op);
        }
    }
    
    fn add_rule(&mut self,  rp: Box::<dyn IRule<IdType = Self::IdType, ValueType = Self::ValueType> + Send>) {
        self.rules.insert(rp.get_id(), rp);
    }

    fn add_mem(&mut self, mut mp: Box::<dyn IMem<IdType = Self::IdType, ValueType = Self::ValueType> + Send>) {
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
                println!("err: can't start mem thread for: {}", id);
            }
        } else {
            println!("err: can't init mem: {}", id);
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

        return Ok(self.msg_sender.clone());
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
                        } else{
                            if let Err(e) = self.outter_sender.as_ref().unwrap().send(msg) {
                                println!("{}", e);
                            }
                        }
                    },
                    OperationType::ObjIn => {
                        if self.sub_mem_handels.as_ref().unwrap().contains_key(&msg.target_id) {
                            if let Some(sender) = self.inner_senders.as_ref().unwrap().get(&msg.target_id) {
                                msg.op_type = OperationType::ObjAdd;
                                if let Err(e) = sender.send(msg) {
                                    println!("{}", e);
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
                        if let Some(handel) = self.sub_mem_handels.as_mut().unwrap().remove(&msg.target_id) {
                            if let Some(sender) = self.inner_senders.as_mut().unwrap().remove(&msg.target_id) {
                                msg.op_type = OperationType::Stop;
                                if let Err(e) = sender.send(msg) {
                                    println!("{}", e);
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

            for r in self.rules.values_mut() { //todo: 多线程化规则判断
                let needed_types = r.obj_type_needed();

                let mut obj_vec_clones: Vec<Vec<(Self::IdType, Vec<Self::ValueType>)>> = Vec::with_capacity(needed_types.len());
                let mut obj_count: Vec<usize> = Vec::with_capacity(needed_types.len());
                let mut obj_to_remove: Vec<T> = Vec::new();
                obj_vec_clones.resize(needed_types.len(), Vec::new());
                obj_count.resize(needed_types.len(), 0usize);

                for (_, (index, nc, _)) in needed_types {
                    if let NeedCount::Some(c) = nc {
                        obj_count[*index] = *c;
                    }
                }
                
                for (id, v) in self.objs.iter() {
                    if let Some((index, nc, is_take)) = needed_types.get(&v.get_obj_type().tid) {
                        if let NeedCount::Some(c) = nc {
                            if obj_count[*index] == 0 {
                                continue;
                            }
                            obj_count[*index] -= 1;
                        }
                        obj_vec_clones[*index].push((id.clone(), v.get_copy_data_vec()));
                        if *is_take {
                            obj_to_remove.push(id.clone());
                        }
                    }
                }

                for id in obj_to_remove {
                    self.objs.remove(&id);
                }
                
                if let Some(mut op) = r.run(self.vec_data.clone(), obj_vec_clones) {
                    self.op_queue.append(&mut op);
                }
            }

            if let Ok(msg) = self.msg_receiver.try_recv() {
                self.op_queue.push(msg);
            }
        }
    
        
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