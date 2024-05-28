use std::{fmt::Display, hash::Hash};

use meme_derive::IObj;

use crate::{core::{DataObj, IObj, IRule, NeedCount, NeedsMap, ObjCat, ObjType}, helpers::needs_map_builder};

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
    needed_types: NeedsMap,
    some_private_data: i64
}

impl<T: Clone, V: Clone> ExampleRule<T, V> {
    pub fn new(id: T) -> Self {
        Self{ 
            id, some_exposed_data: Vec::new(), some_private_data: 0i64,
            needed_types: needs_map_builder()
                .reads::<ExampleObj<T, V>>(NeedCount::All)// objs_data[0]
                //.takes::<ExampleObj<T, V>>(NeedCount::Some(10)) // objs_data[1]
                .build()
        }
    }
}

impl<T: Clone + Hash + Display + Eq + Send + Default, V: Clone + Send> IRule for ExampleRule<T, V> {
    fn obj_type_needed(&self) -> &crate::core::NeedsMap {
        &self.needed_types
    }

    /// 解释：  
    /// env: 当前规则所在为环境的信息， env.0 为环境的膜的id,  env.1 为环境的膜的数据  
    /// objs_data: 输入的各种对象的 id 和数据
    /// objs_data\[ i \] 为第i种输入的对象， i 的顺序为 needs_map_builder 中设定时的顺序， 本例中 objs_data\[ 0 \] 为 ExampleObj 对象  
    /// objs_data\[ i \]\[ j \] 为输入的第 j 个第 i 种对象， j 的顺序认作随机
     fn run(&mut self, env: DataObj<Self::IdType, Self::ValueType>, objs_data: Vec<crate::core::DataObjs<Self::IdType, Self::ValueType>>) -> Option<Vec<crate::core::Operation<Self::IdType, Self::ValueType>>> {
        if objs_data[0].is_empty() {
            return None;
        }
        let mut new_obj: Vec<crate::core::PObj<Self::IdType, Self::ValueType>> = Vec::new();

        // Todo: 在这里实现规则的逻辑 创建规则产生的操作
        if self.some_private_data <= 100 {
            self.some_private_data += 1;
        }
        new_obj.push(Box::new(ExampleObj::new(Default::default())));
        let res: Vec<crate::core::Operation<Self::IdType, Self::ValueType>> = vec![
            crate::core::Operation::obj_add_batch(env.id, new_obj)
        ];
        //

        Some(res)  
    }
    fn about_rule(&self) -> &'static str {
        "这是一个可选的方法，可以用于日志的输出"
    }
    
   
}