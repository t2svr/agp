use std::{any::TypeId, collections::HashMap};


pub fn make_need_map(needs: Vec<TypeId>) -> HashMap<TypeId, usize> {
    let mut res: HashMap<TypeId, usize> = HashMap::new();
    for (i, id) in needs.iter().enumerate() {
        res.insert(*id, i);
    }
    res
}