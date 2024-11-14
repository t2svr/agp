
use std::any::TypeId;

use meme::core::{IObjStat, ITaggedStore, TypeGroup};
use meme::objs::BasicObjStore;
use meme_derive::IObj;

#[derive(IObj, Debug)]
#[obj_type(TypeGroup::Normal)]
pub struct TestObjC {
    #[tag]
    tag: i32
}

impl TestObjC {
    pub fn new(tag: i32) -> Self {
        Self {tag}
    }
}

#[derive(IObj, Debug)]
#[obj_type(TypeGroup::Normal)]
pub struct TestObjA {
    #[tag]
    tag: i32,

    inner: f32
}

impl TestObjA {
    pub fn new(tag: i32, inner: f32) -> Self {
        Self { tag, inner }
    }

    pub fn set_inner(&mut self, v: f32) {
        self.inner = v;
    }

    pub fn get_inner(&self) -> f32 {
        self.inner
    }
}

#[derive(IObj, Debug)]
pub struct TestObjB {
    #[tag]
    tag: i32
}

impl TestObjB {
    pub fn new(tag: i32) -> Self {
        Self { tag }
    }
}

#[test]
pub fn basic_obj_store_test() {
    let pa1 = Box::new(TestObjA {tag: 1, inner: 4.0});
    let pa2 = Box::new(TestObjA {tag: 2, inner: 5.5});
    let pb1 = Box::new(TestObjB {tag: 3});
    let pb2 = Box::new(TestObjB {tag: 4});
    let pbr = Box::new(TestObjB {tag: 3});
    let mut st = BasicObjStore::new();
    assert!(st.add_or_update(pa1.tag, pa1).is_none());
    assert!(st.add_or_update(pa2.tag, pa2).is_none());
    assert!(st.add_or_update(pb1.tag, pb1).is_none());
    assert!(st.add_or_update(pb2.tag, pb2).is_none());
    assert_eq!(st.add_or_update(pbr.tag, pbr).unwrap().obj_tag(), &3);
    assert!(st.contains(&2));
    assert!(st.contains(&3));
    assert!(!st.contains(&6));
    assert!(st.get(&5).is_none());
    assert_eq!(st.get(&1).unwrap().obj_tag(), &1);
    assert_eq!(st.get(&3).unwrap().obj_amount(), 1);

    let mp = st.get_mut(&2).unwrap();
    let casted = mp.as_any_mut().downcast_mut::<TestObjA>().unwrap();
    casted.set_inner(9.99);

    let mp_new = st.get_mut(&2).unwrap();
    let casted_new = mp_new.as_any_mut().downcast_mut::<TestObjA>();
    assert!(casted_new.is_some_and(|o| o.inner == 9.99));

    let pat = st.get(&2).unwrap();
    let pbt = st.get(&3).unwrap();
    let tys = vec![pat.obj_type(), pbt.obj_type()];

    assert!(st.remove(&1).is_some());
    assert_eq!(*st.amounts().collect::<Vec<_>>(), vec![&1, &2]);
    assert_eq!(st.amount_of(&tys[0]), Some(1));
    assert_eq!(st.amount_of(&tys[1]), Some(2));
    
    assert_eq!(st.amount_of_many(&tys[..]), vec![&1, &2]);

    let mut ite = st.objs();
    ite.next();
    ite.next();
    ite.next();
    assert!(ite.next().is_none());

    assert_eq!(*st.tid_at(0).unwrap(), TypeId::of::<TestObjA>());

}