
use std::{sync::Arc, thread};

use meme::{core::IMem, helpers, mems::basic::BasicMem, objs::com::{ObjChannel, SendMsg, SendWrapper}, rules::{com::SendRule, BasicCondition, BasicEffect}};
use meme_derive::*;
use crate::{objs::{TestObjA, TestObjB}, rules::{TestRuleA, TestRuleB}};

#[derive(IObj, Debug)]
pub struct StopObj {
    #[tag]
    tag: i32
}

#[derive(IObj, IRule, Debug)]
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
                .the_tagged(to_ch)
                .build(),

            eff: helpers::effect_builder()
                .crate_obj(|req| {
                    let co = req.the_tagged(0).unwrap();
                    let ct: &i32 = co.obj_tag();
                    let v = vec![
                        SendWrapper { obj: Arc::new(TestObjA::new(helpers::IdGen::next_i32_id(), 555.555)), channel_t: ct.clone() }
                    ];
                    Box::new(SendMsg::<i32>::new(helpers::IdGen::next_i32_id(), v))
                })
                .crate_obj(|_| Box::new(StopObj { tag: helpers::IdGen::next_i32_id() }))
                .build(),
        }
    }
}

#[test]
pub fn basics() {
    let mut m = BasicMem::<u32, i32>::new(100);
    let mut ids = Vec::new();
    ids.resize_with(4, || helpers::IdGen::next_i32_id());
    let (ca, cb) = ObjChannel::<i32>::new_pair(ids[2], ids[3]);
    m.init(
        vec![
            Box::new(TestObjA::new(ids[0], 1.1)), 
            Box::new(TestObjA::new(ids[1], 2.2)),
            Box::new(ca)
        ],
        vec![
            Box::new(TestRuleA::new(0, ids[0])),
            Box::new(TestRuleB::new(1)),
            Box::new(TestRuleStop::new(2)),
            Box::new(TestComRule::new(4, ids[2])),
            Box::new(SendRule::new(3, vec![ids[2]])),
        ]
    );

    let t_handle = thread::spawn( move || m.start() );
    let got_obj = cb.receive();
    let res = t_handle.join().unwrap().ok();
    assert!(res.is_some());
    assert!(got_obj.is_ok());
}
