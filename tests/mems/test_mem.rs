// Copyright 2024 Junshuang Hu
use std::thread;

use meme::{core::IMem, helpers, mems::basic::BasicMem, objs::com::{ObjChannel, SendMsg, SendWrapper}, rules::{com::SendReceiveRule, BasicCondition, BasicEffect}, tagged};
use meme_derive::*;
use crate::{objs::{TestObjA, TestObjB}, rules::{TestRuleA, TestRuleB, TestRuleC, TestRuleD}};

#[derive(IObj, Debug)]
pub struct StopObj {
    #[tag]
    tag: i32
}

#[derive(IObj, IRule, Debug, Clone)]
pub struct TestRuleStop {
    #[tag]
    tag: u32,
    #[effect]
    eff: BasicEffect<i32>,
    #[condition]
    cond: BasicCondition<i32>
}

impl TestRuleStop {
    pub fn new(tag: u32) -> Self {
        Self {
            tag,

            cond: helpers::condition_builder()
                .some_untagged::<StopObj>(1)
                .build(),

            eff: helpers::effect_builder()
                .stop_mem()
                .build(),
        }
    }
}

#[derive(IObj, IRule, Debug)]
pub struct TestComRule {
    #[tag]
    tag: u32,
    #[effect]
    eff: BasicEffect<i32>,
    #[condition]
    cond: BasicCondition<i32>
}

impl TestComRule {
    pub fn new(tag: u32, to_ch: i32) -> Self {
        Self {
            tag,

            cond: helpers::condition_builder()
                .some_untagged::<TestObjB>(1)
                .the_tagged(to_ch).by_ref()
                .build(),

            eff: helpers::effect_builder()
                .crate_obj(|req| {
                    let co = req.set_ref(0).unwrap();
                    let ct: &i32 = co.obj_tag();
                    let v = vec![
                        SendWrapper::new(Box::new(TestObjA::new(helpers::IdGen::next_i32_id(), 555.555)), ct.clone())
                    ];
                    let pobj = Box::new(SendMsg::<i32>::new(helpers::IdGen::next_i32_id(), v));
                    pobj
                })
                //.crate_obj(|_| Box::new(StopObj { tag: helpers::IdGen::next_i32_id() }))
                .increase_untagged::<StopObj>(1)
                .build(),
        }
    }
}

#[test]
pub fn basics() {
    let mut m = BasicMem::<u32, i32>::new(100, false);
    let mut ids = Vec::new();
    ids.resize_with(4, || helpers::IdGen::next_i32_id());
    let (ca, cb) = ObjChannel::<i32>::new_pair(ids[2], ids[3]);
    m.init(
        vec![
            tagged!(TestObjA::new(ids[0], 1.1)), 
            tagged!(TestObjA::new(ids[1], 2.2)),
            tagged!(ca)
        ],
        Default::default(),
        vec![
            tagged!(TestRuleA::new(0, ids[0])),
            tagged!(TestRuleB::new(1)),
            tagged!(TestRuleStop::new(2)),
            tagged!(SendReceiveRule::new(3, vec![ids[2]])),
            tagged!(TestComRule::new(4, ids[2])),
            tagged!(TestRuleC::new(5)),
            tagged!(TestRuleD::new(6))
        ]
    );

    let t_handle = thread::spawn( move || m.start() );
    let got_obj = cb.receive();
    let res = t_handle.join().unwrap().ok();
    assert!(res.is_some());
    assert!(got_obj.is_ok());
    let got_obj_gene = got_obj.unwrap();
    let got_a = got_obj_gene.as_any().downcast_ref::<TestObjA>();
    assert!(got_a.is_some());
    assert_eq!(got_a.unwrap().get_inner(),  555.555);
}
