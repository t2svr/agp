// Copyright 2024 Junshuang Hu
use std::{fmt::Debug, hash::Hash, sync:: Mutex};

use crossbeam_channel::{unbounded, Receiver, Sender};
use krnl::scalar::Scalar;
use meme_derive::IObj;
use crate::{self as meme, core::{PObj, TypeGroup}, errors::MemError};

pub type ObjChannel<T, U = u32, OT = T, OU = U> = Channel<T, U, PObj<OT, OU>>;

#[derive(IObj, Debug)]
#[obj_type(TypeGroup::Com)]
pub struct Channel<T, U = u32, DT = T> 
where 
T: Eq + Hash + Clone + Debug + 'static, 
U: Scalar,
DT: Debug + 'static {
    #[tag]
    tag: T,

    #[amount]
    amount: U,

    r: Receiver<DT>,
    s: Sender<DT>,
}

impl<T, U, DT> Channel<T, U, DT> 
where 
T: Eq + Hash + Clone + Debug + 'static, 
U: Scalar,
DT: Debug + 'static {
    pub fn new_pair(tag_a: T, tag_b: T) -> (Self, Self) {
        let (s1, r1) = unbounded();
        let (s2, r2) = unbounded();
        (
            Self {
                tag: tag_a,
                amount: U::one(),
                r: r1,
                s: s2,
            },
            Self {
                tag: tag_b,
                amount: U::one(),
                r: r2,
                s: s1,
            }
        )
    }

    pub fn new_sr_pair(tag_s: T, tag_r: T) -> (SChannel<T, U, DT>, RChannel<T, U, DT>) {
        let (s, r) = unbounded();
        (
            SChannel {
                tag: tag_s,
                amount: U::one(),
                s
            },
            RChannel {
                tag: tag_r,
                amount: U::one(),
                r
            }
        )
    }

    pub fn new_clone(&self, tag: T) -> Self {
        Self { tag, amount: U::one(), r: self.r.clone(), s: self.s.clone() }
    }

    pub fn send(&self, d: DT) -> Result<(), MemError<DT>> {
        Ok(self.s.send(d)?)
    }

    pub fn receive(&self) -> Result<DT, MemError<DT>>  {
        Ok(self.r.recv()?)
    }

    pub fn try_receive(&self) -> Result<DT,  MemError<DT>>  {
        Ok(self.r.try_recv()?)
    }
}



#[derive(IObj, Debug)]
#[obj_type(TypeGroup::Com)]
pub struct SChannel<T, U = u32, DT = T> 
where 
T: Eq + Hash + Clone + Debug + 'static, 
U: Scalar,
DT: Debug + 'static {
    #[tag]
    tag: T,
    #[amount]
    amount: U,

    s: Sender<DT>,
}

impl<T, U, DT> SChannel<T, U, DT> 
where 
T: Eq + Hash + Clone + Debug + 'static, 
U: Scalar,
DT: Debug + 'static {
    pub fn send(&self, d: DT) -> Result<(),  MemError<DT>> {
        Ok(self.s.send(d)?)
    }
}

#[derive(IObj, Debug)]
#[obj_type(TypeGroup::Com)]
pub struct RChannel<T, U = u32, DT = T> 
where 
T: Eq + Hash + Clone + Debug + 'static, 
U: Scalar,
DT: Debug + 'static {
    #[tag]
    tag: T,
    #[amount]
    amount: U,

    r: Receiver<DT>,
}

impl<T, U, DT> RChannel<T, U, DT> 
where 
T: Eq + Hash + Clone + Debug + 'static, 
U: Scalar,
DT: Debug + 'static {
    pub fn receive(&self) -> Result<DT,  MemError<DT>>  {
        Ok(self.r.recv()?)
    }
}

#[derive(Debug)]
pub struct SendWrapper<OT, OU, CT = OT>
where 
OT: Send + Sync,
OU: Send + Sync,
CT: Send + Sync {
    pub obj: Mutex<Option<PObj<OT, OU>>>,
    pub channel_t: CT
}

impl<OT, OU, CT> SendWrapper<OT, OU, CT>
where 
OT: Send + Sync,
OU: Send + Sync,
CT: Send + Sync {
    pub fn new(obj: PObj<OT, OU>, channel_t: CT) -> Self {
        Self { obj: Mutex::new(Some(obj)), channel_t }
    }
}

#[derive(IObj, Debug)]
#[obj_type(TypeGroup::Com)]
pub struct SendMsg<T, U = u32, OU = U, OT = T, CT = T>
where 
T: Eq + Hash + Clone + Debug + Send + Sync + 'static, 
U: Scalar,
OU: Scalar,
OT: Debug + Send + Sync + 'static, 
CT: Debug + Send + Sync + 'static {
    #[tag]
    tag: T,
    #[amount]
    amount: U,

    pub send_msgs: Vec<SendWrapper<OT, OU, CT>>
}

impl<T, U, OU, OT, CT> SendMsg<T, U, OU, OT, CT>
where 
T: Eq + Hash + Clone + Debug + Send + Sync + 'static, 
U: Scalar,
OU: Scalar,
OT: Debug + Send + Sync + 'static, 
CT: Debug + Send + Sync + 'static {
    pub fn new(tag: T, send_msgs: Vec<SendWrapper<OT, OU, CT>>) -> Self {
        Self { tag, amount: U::one(), send_msgs }
    }
}