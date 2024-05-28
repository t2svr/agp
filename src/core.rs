use crate::errors::MemError;

use std::any::TypeId;
use std::{collections::HashMap, fmt::Display, thread};
use std::hash::Hash;

#[derive(Debug)]
pub enum ObjCat {
    Normal,
    Rule,
    Membrane
}

#[derive(Debug)]
pub struct ObjType {
    pub t: ObjCat,
    pub tid: TypeId
}

impl ObjType {
    pub fn new<T: 'static>(t: ObjCat) ->Self {
        Self {
            t, tid: TypeId::of::<T>()
        }
    }
}


pub type SenderMap<IdType, ValueType> = HashMap<IdType, crossbeam_channel::Sender<Operation<IdType, ValueType>>>;
pub type HandleMap<IdType> =  HashMap<IdType, thread::JoinHandle<Result<bool, MemError>>>;

pub enum MsgDataObj<IdType: Clone + Eq + Hash + Display, ValueType: Clone> {
    Obj(Box<dyn IObj<IdType = IdType, ValueType = ValueType> + Send>),
    Objs(Vec<Box<dyn IObj<IdType = IdType, ValueType = ValueType> + Send>>),
    Rule(Box<dyn IRule<IdType =  IdType, ValueType =ValueType> + Send>),
    Rules(Vec<Box<dyn IRule<IdType =  IdType, ValueType =ValueType> + Send>>),
    Membrane(Box<dyn IMem<IdType = IdType, ValueType = ValueType> + Send>),
    Sender(crossbeam_channel::Sender<Operation<IdType, ValueType>>),
    Inners((SenderMap<IdType, ValueType>, HandleMap<IdType>)),
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
    pub data: MsgDataObj<IdType, ValueType>
}

pub type PObj<IdType, ValueType> = Box<dyn IObj<IdType = IdType, ValueType = ValueType> + Send>;
pub type PRule<IdType, ValueType> = Box<dyn IRule<IdType = IdType, ValueType = ValueType> + Send>;

impl<T, V> Operation<T, V>
where T: Clone + Eq + Hash + Display, V: Clone
{
    
    pub fn obj_add_batch(target_id: T, objs: Vec<PObj<T, V>>) -> Self{
        Self {
            op_type: OperationType::ObjAddBatch,
            target_id,
            data: MsgDataObj::Objs(objs)
        }
    }
}


#[derive(Clone)]
pub enum NeedCount {
    All,
    Some(usize)
}

impl Default for NeedCount {
    fn default() -> Self {
        Self::All
    }
}

impl NeedCount {
    pub fn is_some(&self) -> bool {
        match self {
            NeedCount::All => false,
            NeedCount::Some(_) => true,
        }
    }
    pub fn is_all(&self) -> bool {
        match self {
            NeedCount::All => true,
            NeedCount::Some(_) => false,
        }
    }
}

pub type NeedsMap = HashMap<TypeId, (usize, NeedCount, bool)>;

//HashMap<TypeId, (usize, NeedCount, bool)>

impl<ValueType: Clone,  IdType: Clone + Eq + Hash + Display> Operation<IdType, ValueType> {
    pub fn new(op_type: OperationType, target_id: IdType, data: MsgDataObj<IdType, ValueType>) -> Self {
        Self {
            op_type, target_id, data
        }
    }
}

pub trait IObj {
    type IdType: Clone;
    type ValueType: Clone;
    fn get_id(&self) -> Self::IdType;
    fn get_obj_type(&self) -> ObjType;
    fn get_copy_data_vec(&self) -> Vec<Self::ValueType>;
    fn get_ref_data_vec(&self) -> &Vec<Self::ValueType>;
    
}

#[derive(Clone)]
pub struct DataObj<IdType, ValueType> {
    pub id: IdType,
    pub data: Vec<ValueType>
}

impl<IdType, ValueType> DataObj<IdType, ValueType> {
    pub fn new(id: IdType, data: Vec<ValueType>) -> Self {
        Self { id, data }
    }
}

pub type DataObjs<IdType, ValueType> = Vec<DataObj<IdType, ValueType>>;
pub type Operations<IdType, ValueType> = Vec<Operation<IdType, ValueType>>;

pub trait IRule: IObj
where Self::IdType: Clone + Eq + Hash + Display, Self::ValueType: Clone 
{
    /// 规则的描述
    fn about_rule(&self) -> &'static str {
        "This is a mem rule"
    }

    fn obj_type_needed(&self) -> &NeedsMap;
    
    fn run(&mut self, env: DataObj<Self::IdType, Self::ValueType>, objs_data: Vec<DataObjs<Self::IdType, Self::ValueType>>)
     -> Option<Operations<Self::IdType, Self::ValueType>>;
}

pub type OperationSender<IdType, ValueType> = crossbeam_channel::Sender<Operation<IdType, ValueType>>;

pub trait IMem : IObj
where Self::ValueType: Clone + 'static , Self::IdType: Clone + Eq + Hash + Display
{
    fn get_pref_objs(&self) -> &HashMap<Self::IdType, PObj<Self::IdType, Self::ValueType>>;
    fn get_pref_rules(&self) -> &HashMap<Self::IdType, PRule<Self::IdType, Self::ValueType>>;
    fn set_outter_sender(&mut self, s: crossbeam_channel::Sender<Operation<Self::IdType, Self::ValueType>>);

    fn add_obj(&mut self, op: Box::<dyn IObj<IdType = Self::IdType, ValueType = Self::ValueType> + Send>);
    fn add_rule(&mut self, rp: Box::<dyn IRule<IdType = Self::IdType, ValueType = Self::ValueType> + Send>);
    fn add_mem(&mut self, mp: Box::<dyn IMem<IdType = Self::IdType, ValueType = Self::ValueType> + Send>);

    fn drop_obj(&mut self, id: &Self::IdType);
    fn drop_rule(&mut self, id: &Self::IdType);

    fn init(&mut self) -> Result<OperationSender<Self::IdType, Self::ValueType>, MemError>;
    fn ready(&self) -> bool;

    fn start(&mut self) -> Result<bool, MemError> {
        if self.ready() {
            Ok(self.run())
        } else {
            Err(MemError::new("Mem start failed."))
        }
    }

    /// 膜的主循环
    fn run(&mut self) -> bool;

}

