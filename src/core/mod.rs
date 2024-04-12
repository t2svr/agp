use crate::errors::MemError;

use std::any::TypeId;
use std::{collections::HashMap, fmt::Display, thread};
use std::hash::Hash;

pub enum ObjType {
    Normal(TypeId),
    Rule(TypeId),
    Membrane(TypeId)
}

pub enum MsgDataObj<T: Clone, IdType: Clone + Eq + Hash + Display> {
    Obj(Box<dyn IObj<T, IdType> + Send>),
    Objs(Vec<Box<dyn IObj<T, IdType> + Send>>),
    Rule(Box<dyn IRule<T, IdType> + Send>),
    Rules(Vec<Box<dyn IRule<T, IdType> + Send>>),
    Membrane(Box<dyn IMem<T, IdType> + Send>),
    Sender(crossbeam_channel::Sender<Operation<T, IdType>>),
    Inners((HashMap<IdType, crossbeam_channel::Sender<Operation<T, IdType>>>, HashMap<IdType, thread::JoinHandle<Result<bool, MemError>>>)),
    None
}

pub enum OperationType {
    ObjAdd,
    ObjAddBatch,
    /// 在当前膜内 移除 一个id为target_id的对象p_obj  
    ObjRemove,
    /// 向 膜外 id为target_id的对象传递对象p_obj, 可以跨越多层  
    ObjOut, 
    /// 向 膜内 id为target_id的对象传递对象p_obj, 可以跨越多层  
    ObjIn,

    RuleAdd,
    RuleAddBatch,

    /// 在 膜内 添加一个id为target_id的膜p_mem
    MemAdd,
    /// 不解释
    MemRemove,
    MemAttachOutter,
    MemAttachInner,

    Stop
}

pub struct Operation<T: Clone,  IdType: Clone + Eq + Hash + Display> {
    pub op_type: OperationType,
    pub target_id: IdType,
    pub data: MsgDataObj<T, IdType>
}

pub trait IObj<T: Clone, IdType> {
    fn get_id(self: &Self) -> IdType;
    fn get_obj_type(self: &Self) -> ObjType;
    fn get_copy_data_vec(self: &Self) -> Vec<T>;
    fn get_ref_data_vec(self: &Self) -> &Vec<T>;
    
}

pub trait IRule<T: Clone,  IdType: Clone + Eq + Hash + Display>: IObj<T, IdType> {
    /// 规则的描述
    fn about_rule(self: &Self) -> &'static str {
        "This is a mem rule"
    }

    /// 约定：  
    /// pref_env_data为规则所在的膜对象  的数据vec
    /// pref_objs为这个膜对象中的对象（根据实现可以包含规则对象）  
    fn run(&mut self, pref_env_data: &Vec<T> , pref_objs: &HashMap<IdType, Box<dyn IObj<T, IdType> + Send>>) -> Option<Vec<Operation<T, IdType>>>;
}

pub trait IMem<T: Clone + 'static, IdType: Clone + Eq + Hash + Display> : IObj<T, IdType> {
    fn get_pref_objs(&self) -> &HashMap<IdType, Box<dyn IObj<T, IdType> + Send>>;
    fn get_pref_rules(&self) -> &HashMap<IdType, Box<dyn IRule<T, IdType> + Send>>;
    fn set_outter_sender(&mut self, s: crossbeam_channel::Sender<Operation<T, IdType>>);

    fn add_obj(&mut self, op: Box::<dyn IObj<T, IdType> + Send>);
    fn add_rule(&mut self, rp: Box::<dyn IRule<T, IdType> + Send>);
    fn add_mem(&mut self, op: Box::<dyn IMem<T, IdType> + Send>);

    fn drop_obj(&mut self, id: &IdType);
    fn drop_rule(&mut self, id: &IdType);

    fn init(&mut self) -> Result<crossbeam_channel::Sender<Operation<T, IdType>>, MemError>;
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

