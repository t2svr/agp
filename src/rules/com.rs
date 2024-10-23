use std::{fmt::Debug, hash::Hash};

use crate::{self as meme, core::{PObj, TypeGroup}, objs::com::{ObjChannel, SendMsg}};
use krnl::scalar::Scalar;
use meme::{helpers, rules::{BasicCondition, BasicEffect}};
use meme_derive::{IObj, IRule};


/// todo: 改进为无锁实现
#[derive(IObj, Debug, IRule)]
#[obj_type(TypeGroup::Com)]
pub struct SendReceiveRule<T, OT, U = u32, OU = u32>
where 
T: Clone + Hash + Eq + Debug +  'static,
OT: Clone + Hash + Eq + Send + Sync + Debug +'static,
U: Scalar,
OU: Scalar {
    #[tag]
    tag: T,
    #[amount]
    amount: U,
    #[effect]
    eff: BasicEffect<OT, OU>,
    #[condition]
    cond: BasicCondition<OT, OU>
}

impl<T, OT, U, OU> SendReceiveRule<T, OT, U, OU>
where 
T: Clone + Hash + Eq + Debug +  'static,
OT: Clone + Hash + Eq + Send + Sync + Debug +'static,
U: Scalar,
OU: Scalar
{
    pub fn new(tag: T, known_channels: Vec<OT>) -> Self {
     
        Self {
            tag, amount: U::one(), 

            cond: helpers::condition_builder()
            .some_tagged(known_channels)// `n`
            .rand_tagged::<SendMsg<OT, OU>>(1)//`m`
            .build(),

            eff: helpers::effect_builder()
            .remove_objs(|req| { // 复杂度 `O(mn)` ，有互斥锁
                let mo: &PObj<OT, OU> = req.the_rand_tagged(0, 0).unwrap();
                let cho = req.set.as_ref().unwrap();
                let mut res = Vec::new();
                let mut done = true;
                if let Some(msg) = mo.as_any().downcast_ref::<SendMsg<OT, OU>>() {
                    msg.send_msgs.iter().for_each(|w| {
                        if let Ok(mut wo) = w.obj.lock() { // 如果膜的对象不被外部直接修改，则改锁总是成功
                            if wo.is_some() {
                                if let Some(co) = cho.iter().find(|o| *o.obj_tag() == w.channel_t) {
                                    if let Some(ch) = co.as_any().downcast_ref::<ObjChannel<OT, OU>>() {
                                        if let Err(e) = ch.send(wo.take().unwrap()) {
                                            *wo = e.data;
                                        } else { return; }
                                    }
                                }
                            }
                        }
                        done = false;
                    });
                }
                if done {
                    res.push(mo.obj_tag().clone());
                }
                res
            })
            .crate_objs(|req| { // 复杂度 `O(n)`
                let cho = req.set.as_ref().unwrap();
                cho.iter().filter_map(|o| {
                    o.as_any().downcast_ref::<ObjChannel<OT, OU>>().and_then(|c| {
                        c.try_receive().ok()
                    })
                }).collect::<Vec<_>>()
            })
            .build(),
        }
    }
}