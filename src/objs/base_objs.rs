use std::{fmt::Display, hash::Hash, ops::Add};

use meme_derive::IObj;

use crate::{core::{DataObj, IObj, IRule, Needs, ObjCat, ObjType, Offer, Operation, Operations, PObj}, helpers::needs_map_builder};

#[derive(IObj)]
#[id_type(T)]
#[data_type(V)]
//#[obj_type(ObjCat::Normal)] //默认类型为ObjCat::Normal
pub struct ExampleObj<T, V>
where T: Clone + 'static, V: Clone + 'static {
    #[id]
    id: T,
    #[data]
    some_exposed_data: Vec<V>
}

impl<T: Clone, V: Clone> ExampleObj<T, V> {
    pub fn new(id: T) -> Self {
        Self{ id, some_exposed_data: Vec::new() }
    }

    pub fn data_push_val(&mut self, val: V) {
        self.some_exposed_data.push(val);
    }
}

#[derive(IObj)]
#[id_type(T)]
#[data_type(V)]
#[obj_type(ObjCat::Rule)]
pub struct ExampleRule<T, V>
where T: Clone + 'static, V: Clone + 'static {
    #[id]
    id: T,
    #[data]
    some_exposed_data: Vec<V>,
    needed_types: Needs<T>,
    some_private_data: i64
}

impl<T: Clone, V: Clone> ExampleRule<T, V> {
    pub fn new(id: T) -> Self {
        Self{ 
            id, 
            some_exposed_data: Vec::new(), 
            some_private_data: 0i64,
            needed_types: needs_map_builder()
                .randomly()
                //.sequentially()
                //.reads()
                .takes()
                .all::<ExampleObj<T, V>>()
                //.some::<ExampleObj<T, V>>(10)
                //.the(T::new(24601))
                .build()
        }
    }
}

impl<T, V> IRule for ExampleRule<T, V>
where
T: Clone + Hash + Display + Eq + Send + Default,
V: Clone + Send + Add<V, Output = V>
{
    fn obj_needs(&self) -> &crate::core::Needs<T> {
        &self.needed_types
    }

    /// 解释：  
    /// env: 当前规则所在环境的数据对象  
    /// offered_data: 根据 fn obj_needs(&self) 中返回的对象需求信息获取到的数据对象  
    /// 返回：  
    ///     - None 表示该规则不被触发或者没有对其他对象的影响  
    ///     - Some(x) x为产生的一系列影响，例如新增对象，移除对象，破坏膜等等  
     fn run(&mut self, env: DataObj<Self::IdType, Self::ValueType>, mut offered_data: Offer<Self::IdType, Self::ValueType>) -> Option<Vec<Operation<Self::IdType, Self::ValueType>>> {
        if offered_data.general.is_empty()
        && offered_data.specific.is_empty() {
            return None;
        }
        let mut new_obj: Vec<PObj<Self::IdType, Self::ValueType>> = Vec::new();

        // Todo: 在这里实现规则的逻辑 返回产生的操作
        let mut res: Operations<T, V> = Vec::new();
        while let Some(mut o) = offered_data.general[0].pop() {
            if let Some(top) = o.data.last() {
                o.data.push((*top).clone() + (*top).clone());
            }
            new_obj.push(Box::new(ExampleObj{ id: o.id, some_exposed_data: o.data }));
        }
        res.push(Operation::obj_add_batch(env.id.clone(), new_obj));
        self.some_private_data += 1;
        if self.some_private_data >= 10 {
            res.push(Operation::stop(env.id))
        }
        Some(res)
    }

    fn about_rule(&self) -> &'static str {
        "这是一个可选的方法，可以用于日志的输出中规则的说明"
    }
   
}