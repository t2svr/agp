use std::{collections::HashMap, thread};

use uuid::Uuid;

use crate::errors::MemError;

pub enum ObjType {
    Normal, Rule, Membrane
}

pub enum MsgDataObj<T: Clone> {
    Obj(Box<dyn IObj<T> + Send>),
    Rule(Box<dyn IRule<T> + Send>),
    Membrane(Box<dyn IMem<T> + Send>),
    Sender(crossbeam_channel::Sender<Operation<T>>),
    Inners((HashMap<Uuid, crossbeam_channel::Sender<Operation<T>>>, HashMap<Uuid, thread::JoinHandle<Result<bool, MemError>>>)),
    None
}

pub enum OperationType {
    /// 在当前膜内 添加 一个id为target_id的对象p_obj  
    ObjAdd,
    /// 在当前膜内 移除 一个id为target_id的对象p_obj  
    ObjRemove,
    /// 向 膜外 id为target_id的对象传递对象p_obj, 可以跨越多层  
    ObjOut, 
    /// 向 膜内 id为target_id的对象传递对象p_obj, 可以跨越多层  
    ObjIn,

    RuleAdd,

    /// 在 膜内 添加一个id为target_id的膜p_mem
    MemAdd,
    /// 不解释
    MemRemove,
    MemAttachOutter,
    MemAttachInner,

    Stop
}

pub struct Operation<T: Clone> {
    pub op_type: OperationType,
    pub target_id: Uuid,
    pub data: MsgDataObj<T>
}

pub trait IObj<T: Clone> {
    fn get_id(self: &Self) -> Uuid;
    fn get_obj_type(self: &Self) -> ObjType;
    fn get_copy_data_vec(self: &Self) -> Vec<T>;
    fn get_ref_data_vec(self: &Self) -> &Vec<T>;
}

pub trait IRule<T: Clone>: IObj<T> {
    /// 重载这个函数
    fn about_rule(self: &Self) -> &'static str {
        "This is a mem rule"
    }
    
    /// 重载这个函数
    fn run(self: &Self, _pref_objs: &HashMap<Uuid, Box<dyn IObj<T> + Send>>) -> Option<Vec<Operation<T>>>;
}

pub trait IMem<T: Clone + 'static>: IObj<T> {
    fn get_pref_objs(&self) -> &HashMap<Uuid, Box<dyn IObj<T> + Send>>;
    fn get_pref_rules(&self) -> &HashMap<Uuid, Box<dyn IRule<T> + Send>>;
    fn set_outter_sender(&mut self, s: crossbeam_channel::Sender<Operation<T>>);

    fn add_obj(&mut self, op: Box::<dyn IObj<T> + Send>);
    fn add_rule(&mut self, rp: Box::<dyn IRule<T> + Send>);
    fn add_mem(&mut self, op: Box::<dyn IMem<T> + Send>);

    fn drop_obj(&mut self, id: &Uuid);
    fn drop_rule(&mut self, id: &Uuid);

    fn init(&mut self) -> Result<crossbeam_channel::Sender<Operation<T>>, MemError>;
    fn ready(&self) -> bool;

    fn start(&mut self) -> Result<bool, MemError> {
        if self.ready() {
            Ok(self.run())
        } else {
            Err(MemError::from_str("Mem start failed."))
        }
    }

    /// 膜的主循环
    fn run(&mut self) -> bool;

}

