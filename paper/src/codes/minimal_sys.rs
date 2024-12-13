use meme::core::*;
use meme_derive::*;
use meme::rules::{BasicCondition, BasicEffect};
use meme::mems::basic::BasicMem;
use meme::helpers;

// define tagged objects
#[derive(IObj)]
struct TA {
    #[tag]
    tag: i32,
    //define unique properties
    p: f32
}

// define untagged objects
struct A;struct B;struct C;

// define rules
#[derive(IObj, IRule)]
pub struct MinimalRule {
    #[tag]
    tg: u32,
    #[condition]
    cond: BasicCondition<i32>,
    #[effect]
    eff: BasicEffect<i32>
}
impl MinimalRule {
    pub fn new(tag: u32) -> Self {
        Self {
            tg: tag,
            //use helpers to construct condition and effect
            cond: helpers::condition_empty(),
            eff: helpers::effect_empty()
        }
    }
}

// program main function
fn main() {
    let mut m = BasicMem::<u32, i32>::new(1, false);
    m.init(
        vec![tagged!(TA{ tag:2, p: 1.1 })],
        vec![untagged!(A, 10), untagged!(B, 20)],
        vec![tagged!(MinimalRule::new(3))]
    );
    m.start();
}
