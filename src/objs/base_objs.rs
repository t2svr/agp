use std::any::Any;

use crate::core::{IObj, ObjType};

pub struct BaseObj<T, IdType> {
    id: IdType,
    vec_data: Vec<T>
}

impl<T: Clone + 'static, IdType: Clone + 'static> IObj<T, IdType> for BaseObj<T, IdType> {
    fn get_id(self: &Self) -> IdType { self.id.clone() }
    fn get_obj_type(self: &Self) -> ObjType { ObjType::Normal(self.type_id()) }
    
    fn get_copy_data_vec(self: &Self) -> Vec<T> {
        self.vec_data.clone()
    }
    
    fn get_ref_data_vec(self: &Self) -> &Vec<T> {
        &self.vec_data
    }
}

impl<T, IdType> BaseObj<T, IdType> {
    pub fn new(id: IdType) -> Self {
        Self{ id, vec_data: Vec::new() }
    }

    pub fn push_data(&mut self, val: T) {
        self.vec_data.push(val);
    }
}
