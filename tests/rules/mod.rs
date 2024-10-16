use meme::{core::{IObj, IRuleStat, ITaggedStore}, helpers, objs::BasicObjStore, rules::{BasicCondition, BasicEffect, BasicRuleStore}};
use meme_derive::{IObj, IRule};

use crate::objs::{TestObjA, TestObjB};


#[derive(IObj, IRule)]
struct TestRuleA {
    #[tag]
    tag: u32,
    #[effect]
    eff: BasicEffect<i32>,
    #[condition]
    cond: BasicCondition<i32>
}

impl TestRuleA {
    pub fn new(tag: u32) -> Self {
        Self {
            tag,

            cond: helpers::condition_builder()
                .the_tagged(1)
                .rand_tagged::<TestObjA>(1)
                .build(),

            eff: helpers::effect_builder()
                .crate_obj(|_| vec![Box::new(TestObjA::new(-1, 9.9))])
                .update_obj(|req| {
                    let old = req.set.as_ref().unwrap()[0];
                    let mut new_inner = 0.0;
                    if let Some(o) = old.as_any().downcast_ref::<TestObjA>() {
                        new_inner = o.get_inner()
                    } 
                    Box::new(TestObjA::new(-1, new_inner))
                })
                .build(),
        }
    }
}

#[derive(IObj, IRule)]
pub struct TestRuleB {
    #[tag]
    t: u32,
    #[condition]
    cond: BasicCondition<i32>,
    #[effect]
    eff: BasicEffect<i32>
}

impl TestRuleB {
    pub fn new(tag: u32) -> Self {
        Self {
            t: tag,

            cond: helpers::condition_builder()
            .the_tagged(3)
            .rand_tagged::<TestObjB>(3)
            .build(),

            eff: helpers::effect_builder()
            .crate_obj(|_| vec![Box::new(TestObjB::new(-2))])
            .build(),
        }
    }
}

#[test]
pub fn basic_rule_store_test() {
    let a1 = Box::new(TestObjA::new(1, 1.1));
    let a2 = Box::new(TestObjA::new(2, 2.2));
    let b1 = Box::new(TestObjB::new(3));
    let b2 = Box::new(TestObjB::new(4));
    let mut ost = BasicObjStore::new();
    ost.add_or_update(a1.obj_tag(), a1);
    ost.add_or_update(a2.obj_tag(), a2);
    ost.add_or_update(b1.obj_tag(), b1);
    ost.add_or_update(b2.obj_tag(), b2);

    let ra = Box::new(TestRuleA::new(1));
    let rb = Box::new(TestRuleB::new(2));
    let mut rst = BasicRuleStore::new();
    assert!(rst.add_or_update(ra.tag, ra).is_none());
    assert!(rst.add_or_update(rb.t, rb).is_none());
    assert!(rst.contains(&1));
    assert!(!rst.contains(&3));
    assert!(rst.get(&1).is_some_and(|r| r.obj_tag() == 1));
    let check_res = rst.check_on_tagged(&ost);
    assert_eq!(rst.index_of(&1), Some(0));
    assert!(check_res.conflict_executable.is_none());
    assert!(check_res.parallel_executable.is_some_and(|v| v[0] == 0 && v.len() == 1));

    assert!(rst.remove(&1).is_some_and(|r| r.obj_tag() == 1));
    let check_res_new = rst.check_on_tagged(&ost);
    assert!(check_res_new.conflict_executable.is_none() && check_res_new.parallel_executable.is_none());
}