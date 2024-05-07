use meme_derive::IObj;

use crate::core::{IObj, ObjT, ObjType};

#[derive(IObj)]
pub struct BaseObj<T, V>
where T: Clone + 'static, V: Clone + 'static {
    #[id(T)]
    id: T,
    #[data(V)]
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
pub struct DeriveTestObj {
    #[id(i64)]
    id: i64,
    #[data(f32)]
    vec_data: Vec<f32>
}