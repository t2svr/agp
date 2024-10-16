// use meme_derive::{IObj, IRule};
// use crate::{self as meme, helpers};

// use super::{BasicCondition, BasicEffect};

// #[derive(IObj, IRule)]
// struct TestRuleA {
//     #[tag]
//     tag: u32,
//     #[effect]
//     eff: BasicEffect<i32>,
//     #[condition]
//     cond: BasicCondition<i32>
// }

// impl TestRuleA {
//     pub fn new(tag: u32) -> Self {
//         Self {
//             tag,

//             eff: helpers::effect_builder()
//             .build(),

//             cond: helpers::condition_builder()
//             .build(),
//         }
//     }
// }
