use crate::errors::MemError;

use std::any::TypeId;
use std::{collections::HashMap, fmt::Display, thread};
use std::hash::Hash;

#[derive(Debug)]
pub enum ObjT {
    Normal,
    Rule,
    Membrane
}

#[derive(Debug)]
pub struct ObjType {
    pub t: ObjT,
    pub tid: TypeId
}

impl ObjType {
    pub fn new<T: 'static>(t: ObjT) ->Self {
        Self {
            t, tid: TypeId::of::<T>()
        }
    }
}



pub enum MsgDataObj<ValueType: Clone, IdType: Clone + Eq + Hash + Display> {
    Obj(Box<dyn IObj<IdType = IdType, ValueType = ValueType> + Send>),
    Objs(Vec<Box<dyn IObj<IdType = IdType, ValueType = ValueType> + Send>>),
    Rule(Box<dyn IRule<IdType =  IdType, ValueType =ValueType> + Send>),
    Rules(Vec<Box<dyn IRule<IdType =  IdType, ValueType =ValueType> + Send>>),
    Membrane(Box<dyn IMem<IdType = IdType, ValueType = ValueType> + Send>),
    Sender(crossbeam_channel::Sender<Operation<IdType, ValueType>>),
    Inners((HashMap<IdType, crossbeam_channel::Sender<Operation<IdType, ValueType>>>, HashMap<IdType, thread::JoinHandle<Result<bool, MemError>>>)),
    RowData(Vec<ValueType>),
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
    MemRemove,
    MemAttachOutter,
    MemAttachInner,

    Stop
}

pub struct Operation<IdType: Clone + Eq + Hash + Display, ValueType: Clone> {
    pub op_type: OperationType,
    pub target_id: IdType,
    pub data: MsgDataObj<ValueType, IdType>
}

impl<ValueType: Clone,  IdType: Clone + Eq + Hash + Display> Operation<IdType, ValueType> {
    pub fn new( op_type: OperationType, target_id: IdType, data: MsgDataObj<ValueType, IdType>) -> Self {
        Self {
            op_type, target_id, data
        }
    }
}

pub trait IObj {
    type IdType: Clone;
    type ValueType: Clone;
    fn get_id(self: &Self) -> Self::IdType;
    fn get_obj_type(self: &Self) -> ObjType;
    fn get_copy_data_vec(self: &Self) -> Vec<Self::ValueType>;
    fn get_ref_data_vec(self: &Self) -> &Vec<Self::ValueType>;
    
}

pub trait IRule: IObj
where Self::IdType: Clone + Eq + Hash + Display, Self::ValueType: Clone 
{
    /// 规则的描述
    fn about_rule(self: &Self) -> &'static str {
        "This is a mem rule"
    }

    fn obj_type_needed(&self) -> &HashMap<TypeId, usize>;
    
    /// 约定：
    /// pref_env_data为规则所在的膜对象  的数据vec
    /// pref_objs为这个膜对象中的对象（根据实现可以包含规则对象）  
    fn run(&mut self, env_data: Vec<Self::ValueType>, objs_data: Vec<Vec<(Self::IdType, Vec<Self::ValueType>)>>) -> Option<Vec<Operation<Self::IdType, Self::ValueType>>>;
}

pub trait IMem : IObj
where Self::ValueType: Clone + 'static , Self::IdType: Clone + Eq + Hash + Display
{
    fn get_pref_objs(&self) -> &HashMap<Self::IdType, Box<dyn IObj<IdType = Self::IdType, ValueType = Self::ValueType> + Send>>;
    fn get_pref_rules(&self) -> &HashMap<Self::IdType, Box<dyn IRule<IdType = Self::IdType, ValueType = Self::ValueType> + Send>>;
    fn set_outter_sender(&mut self, s: crossbeam_channel::Sender<Operation<Self::IdType, Self::ValueType>>);

    fn add_obj(&mut self, op: Box::<dyn IObj<IdType = Self::IdType, ValueType = Self::ValueType> + Send>);
    fn add_rule(&mut self, rp: Box::<dyn IRule<IdType = Self::IdType, ValueType = Self::ValueType> + Send>);
    fn add_mem(&mut self, mp: Box::<dyn IMem<IdType = Self::IdType, ValueType = Self::ValueType> + Send>);

    fn drop_obj(&mut self, id: &Self::IdType);
    fn drop_rule(&mut self, id: &Self::IdType);

    fn init(&mut self) -> Result<crossbeam_channel::Sender<Operation<Self::IdType, Self::ValueType>>, MemError>;
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
