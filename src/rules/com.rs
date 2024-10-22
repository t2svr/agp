use std::{fmt::Debug, hash::Hash};

use crate::{self as meme, core::{PObj, TypeGroup}, objs::com::{ObjChannel, SendMsg}};
use krnl::scalar::Scalar;
use meme::{helpers, rules::{BasicCondition, BasicEffect}};
use meme_derive::{IObj, IRule};


#[derive(IObj, Debug, IRule)]
#[obj_type(TypeGroup::Com)]
pub struct SendRule<T, OT, U = u32, OU = u32>
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
    cond: BasicCondition<OT, OU>,

}

impl<T, OT, U, OU> SendRule<T, OT, U, OU>
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
            .some_tagged(known_channels)
            .rand_tagged::<SendMsg<OT, OU>>(1)
            .build(),

            eff: helpers::effect_builder()
            .remove_objs(|req| {
                let mo: &PObj<OT, OU> = req.the_rand_tagged(0, 0).unwrap();
                let cho = req.set.as_ref().unwrap();
                let mut res = Vec::new();
                let mut done = true;
                if let Some(msg) = mo.as_any().downcast_ref::<SendMsg<OT, OU>>() {
                    msg.send_msgs.iter().for_each(|w| {
                        if let Some(co) = cho.iter().find(|o| *o.obj_tag() == w.channel_t) {
                            if let Some(ch) = co.as_any().downcast_ref::<ObjChannel<OT, OU>>() {
                                if ch.send(w.obj.clone()).is_err() {
                                    done = false;
                                }
                            }
                        }
                    });
                }
                if done {
                    res.push(mo.obj_tag().clone());
                }
                res
            })
            .build(),
        }
    }
}