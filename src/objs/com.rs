// use std::{fmt::Display, hash::Hash, ops::Add};

// use meme_derive::{IM2Obj, IObj};

// use crate::{core::{DataObj, IObj, IM2Obj, IM2Rule, Needs, ObjCat, ObjType, Offer, M2Operation, M2Operations, PM2Obj}, helpers::needs_map_builder};

pub struct ComObj {

} 

// #[derive(IObj, IM2Obj)]
// //#[obj_type(ObjCat::Normal)] //默认类型为ObjCat::Normal
// #[data_type(V)]
// pub struct ExampleObj<T, V>
// where T: Clone + 'static, V: Clone + 'static {
//     #[id]
//     id: T,
//     #[data]
//     some_exposed_data: Vec<V>
// }

// impl<T: Clone, V: Clone> ExampleObj<T, V> {
//     pub fn new(id: T) -> Self {
//         Self{ id, some_exposed_data: Vec::new() }
//     }

//     pub fn data_push_val(&mut self, val: V) {
//         self.some_exposed_data.push(val);
//     }
// }

// #[derive(IObj, IM2Obj)]
// #[obj_type(ObjCat::Rule)]
// #[data_type(V)]
// pub struct ExampleRule<T, V>
// where T: Clone + 'static, V: Clone + 'static {
//     #[id]
//     id: T,
//     #[data]
//     some_exposed_data: Vec<V>,
//     needed_types: Needs<T>,
//     // 自定义的字段
//     iterations: u32,
//     iterations_stop: u32
// }

// impl<T: Clone, V: Clone> ExampleRule<T, V> {
//     pub fn new(id: T, iterations_stop: u32) -> Self {
//         Self{ 
//             id, iterations_stop, iterations: 0,
//             some_exposed_data: Vec::new(), 
//             needed_types: needs_map_builder()
//                 .randomly()
//                 //.sequentially()
//                 //.reads()
//                 .takes()
//                 .all::<ExampleObj<T, V>>()
//                 //.some::<ExampleObj<T, V>>(10)
//                 //.the(T::new(24601))
//                 .build()
//         }
//     }
// }

// impl<T, V> IM2Rule for ExampleRule<T, V>
// where
// T: Clone + Hash + Display + Eq + Send + Default,
// V: Clone + Send + Add<V, Output = V> + From<i32>
// {
//     fn obj_needs(&self) -> &crate::core::Needs<T> {
//         &self.needed_types
//     }

//     /// 解释：  
//     /// env: 当前规则所在环境的数据对象  
//     /// offered_data: 根据 fn obj_needs(&self) 中返回的对象需求信息获取到的数据对象  
//     /// 返回：  
//     ///     - None 表示该规则不被触发或者没有对其他对象的影响  
//     ///     - Some(x) x为产生的一系列影响，例如新增对象，移除对象，破坏膜等等   
//     /// 
//     /// 这个例子使用了非必要的复杂操作如泛化，u32 到 i32 转换等，以体现通用性，实际使用时应尽量避免
//      fn run(&mut self, env: DataObj<Self::IdType, Self::ValueType>, mut offered_data: Offer<Self::IdType, Self::ValueType>)
//       -> Option<Vec<M2Operation<Self::IdType, Self::ValueType>>> {
//         let mut new_obj: Vec<PM2Obj<Self::IdType, Self::ValueType>> = Vec::new();

//         // Todo: 在这里实现规则的逻辑 返回产生的操作
//         let mut res: M2Operations<T, V> = Vec::new();
//         while let Some(mut o) = offered_data.general[0].pop() {
//             if let Some(top) = o.data.last() {
//                 if self.iterations >= 10 {
//                     o.data.push((*top).clone());
//                 } else {
//                     o.data.push((*top).clone() + (*top).clone());
//                 }
//             }
//             new_obj.push(Box::new(ExampleObj{ id: o.id, some_exposed_data: o.data }));
//         }
//         res.push(M2Operation::obj_add_batch(env.id.clone(), new_obj));
//         self.iterations += 1;
//         if self.iterations >= self.iterations_stop {
//             self.some_exposed_data.push((self.iterations as i32).into());
//             res.push(M2Operation::stop(env.id))
//         }
//         Some(res)
//     }
// }