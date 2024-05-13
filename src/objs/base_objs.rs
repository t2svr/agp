use meme_derive::IObj;

use crate::core::{IObj, ObjCat, ObjType};

#[derive(IObj)]
#[obj_id_type(T)]
#[obj_data_type(V)]
#[obj_type(ObjCat::Rule)]
pub struct BaseObj<T, V>
where T: Clone + 'static, V: Clone + 'static {
    #[id]
    id: T,
    #[data]
    vec_data: Vec<V>
}

impl<T: Clone, V: Clone> BaseObj<T, V> {
    pub fn new(id: T) -> Self {
        Self{ id, vec_data: Vec::new() }
    }

    pub fn push_data(&mut self, val: V) {
        self.vec_data.push(val);
    }
}

#[derive(IObj)]
#[obj_id_type(i64)]
#[obj_data_type(f32)]
pub struct DeriveTestObj {
    #[id]
    id: i64,
    #[data]
    vec_data: Vec<f32>
}