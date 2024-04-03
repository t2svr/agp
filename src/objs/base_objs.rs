use std::{collections::HashMap, ops::Add};

use uuid::Uuid;

use crate::core::{IObj, IRule, ObjType, Operation, OperationType, MsgDataObj};

pub struct BaseObj<T> {
    id: Uuid,
    vec_data: Vec<T>
}

impl<T: Clone> IObj<T> for BaseObj<T> {
    fn get_id(self: &Self) -> Uuid { self.id }
    fn get_obj_type(self: &Self) -> ObjType { ObjType::Normal }
    
    fn get_copy_data_vec(self: &Self) -> Vec<T> {
        self.vec_data.clone()
    }
    
    fn get_ref_data_vec(self: &Self) -> &Vec<T> {
        &self.vec_data
    }
}

impl<T> BaseObj<T> {
    pub fn new() -> Self {
        Self{ id: Uuid::new_v4(), vec_data: Vec::new() }
    }

    pub fn push_data(&mut self, val: T) {
        self.vec_data.push(val);
    }
}

pub struct TestRuleA<T> {
    id: Uuid,
    vec_data: Vec<T>
}
pub struct TestRuleB<T> {
    id: Uuid,
    vec_data: Vec<T>
}

impl<T: Clone > IObj<T> for TestRuleA<T> {
    fn get_id(self: &Self) -> Uuid { self.id }
    fn get_obj_type(self: &Self) -> ObjType { ObjType::Rule }
    
    
    fn get_copy_data_vec(self: &Self) -> Vec<T> {
        self.vec_data.clone()
    }
    
    fn get_ref_data_vec(self: &Self) -> &Vec<T> {
        &self.vec_data
    }
}

impl<T: Clone > IObj<T> for TestRuleB<T> {
    fn get_id(self: &Self) -> Uuid { self.id }
    fn get_obj_type(self: &Self) -> ObjType { ObjType::Rule }
    
    
    fn get_copy_data_vec(self: &Self) -> Vec<T> {
        self.vec_data.clone()
    }
    
    fn get_ref_data_vec(self: &Self) -> &Vec<T> {
        &self.vec_data
    }
}

impl IRule<i32> for TestRuleA<i32> {
    fn run(&self, pref_objs: &HashMap<Uuid, Box<dyn IObj<i32> + Send>>) -> Option<Vec<Operation<i32>>> {
        if pref_objs.is_empty() {
            let mut res: Vec<Operation<i32>> = Vec::new();
            let mut o = Box::new(BaseObj::<i32>::new());
            o.vec_data.push(100);
            let op = Operation::<i32> {
                op_type: OperationType::ObjAdd,
                target_id: o.id,
                data: MsgDataObj::Obj(o),
            };
            res.push(op);
            Some(res)
        } else {
            None
        }
    }
}

impl<T: Clone + Send + 'static + Add> IRule<T> for TestRuleB<T>
where T: Add<T, Output = T>
{
    fn run(&self, pref_objs: &HashMap<Uuid, Box<dyn IObj<T> + Send>>) -> Option<Vec<Operation<T>>> {
        if pref_objs.len() < 10 {
            let mut res: Vec<Operation<T>> = Vec::new();
            let mut obj = Box::new(BaseObj::<T>::new());

            if let Some(sum) = pref_objs.values().flat_map(|o| {
                o.get_ref_data_vec()
            }).map(|rt| (*rt).clone() ).reduce(|acc, e| {
                acc + e
            }){
                obj.vec_data.push(sum.clone());
            }
            
            let op = Operation::<T> {
                op_type: OperationType::ObjAdd,
                target_id: obj.id,
                data: MsgDataObj::Obj(obj),
            };
            res.push(op);
            Some(res)
        } else {
            None
        }
    }
}

impl<T> TestRuleA<T> {
    pub fn new() -> Self {
        Self{ id: Uuid::new_v4(), vec_data: Vec::new() }
    }
}
impl<T> TestRuleB<T> {
    pub fn new() -> Self {
        Self{ id: Uuid::new_v4(), vec_data: Vec::new() }
    }
}


