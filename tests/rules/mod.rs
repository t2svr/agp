use meme::{core::{IObj, IRuleStat, ITaggedStore}, helpers, objs::BasicObjStore, rules::{BasicCondition, BasicEffect, BasicRuleStore}};
use meme_derive::{IObj, IRule};

use crate::objs::{TestObjA, TestObjB};

#[derive(IObj, IRule, Debug)]
pub struct TestRuleA {
    #[tag]
    tag: u32,
    #[effect]
    eff: BasicEffect<i32>,
    #[condition]
    cond: BasicCondition<i32>
}

impl TestRuleA {
    pub fn new(tag: u32, req_id: i32) -> Self {
        Self {
            tag,

            cond: helpers::condition_builder()
                .the_tagged(req_id)
                .build(),

            eff: helpers::effect_builder()
                .crate_obj(|req| {
                    let old = req.the_tagged(0).unwrap();
                    let mut new_inner = 10.0;
                    if let Some(o) = old.as_any().downcast_ref::<TestObjA>() {
                        new_inner += o.get_inner()
                    }
                    Box::new(TestObjA::new(helpers::IdGen::next_i32_id(), new_inner))
                })
                .build(),
        }
    }
}

#[derive(IObj, IRule, Debug)]
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
            .some_untagged::<TestObjA>(10)
            .rand_tagged::<TestObjA>(10)
            .build(),

            eff: helpers::effect_builder()
            .crate_obj(|_| Box::new(TestObjB::new(helpers::IdGen::next_i32_id())))
            .remove_objs(|req| {
                req.rand_tagged(0).unwrap().iter().map(|o| o.obj_tag().clone()).collect::<Vec<_>>()
            })
            .build(),
        }
    }
}

#[test]
pub fn basic_rule_store_test() {
    let mut ids = Vec::new();
    ids.resize_with(4, || helpers::IdGen::next_i32_id() );

    let a1 = Box::new(TestObjA::new(ids[0], 1.1));
    let a2 = Box::new(TestObjA::new(ids[1], 2.2));
    let b1 = Box::new(TestObjB::new(ids[2]));
    let b2 = Box::new(TestObjB::new(ids[3]));
    let mut ost = BasicObjStore::new();
    ost.add_or_update(a1.obj_tag().clone(), a1);
    ost.add_or_update(a2.obj_tag().clone(), a2);
    ost.add_or_update(b1.obj_tag().clone(), b1);
    ost.add_or_update(b2.obj_tag().clone(), b2);

    let ra = Box::new(TestRuleA::new(0, ids[0]));
    let rb = Box::new(TestRuleB::new(1));
    let mut rst = BasicRuleStore::new();
    assert!(rst.add_or_update(ra.tag, ra).is_none());
    assert!(rst.add_or_update(rb.t, rb).is_none());
    assert!(rst.contains(&1));
    assert!(!rst.contains(&3));
    assert!(rst.get(&0).is_some_and(|r| r.obj_tag() == &0));
    let check_res = rst.check_on_tagged(&ost);
    assert_eq!(rst.index_of(&0), Some(0));
    assert!(check_res.conflict_executable.is_none());
    assert!(check_res.parallel_executable.is_some_and(|v| v[0].rule_index == 0 && v.len() == 1));

    assert!(rst.remove(&0).is_some_and(|r| *r.obj_tag() == 0));
    let check_res_new = rst.check_on_tagged(&ost);
    assert!(check_res_new.conflict_executable.is_none() && check_res_new.parallel_executable.is_none());

    // todo: 测试规则的执行
}