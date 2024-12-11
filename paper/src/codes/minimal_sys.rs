// define tagged objects
#[derive(IObj)]
struct TA {
    #[tag]
    tg: i32,
    //define unique properties
    p_1: f32,
    p_2: f32,
}
impl TA {
    pub fn new(tag: i32, p: [f32;2]) -> Self {
        Self { 
            tg: tag,
            p_1: p[1], p_2: p[2]
        } 
    }
}

// define untagged objects
struct A;
struct B;
struct C;

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
            cond: helpers::condition_empty(),
            eff: helpers::effect_empty
        }
    }
}

// program main function
fn main() {
    let mut m = BasicMem::<u32, i32>::new(1, false);
    m.init(
        vec![
            Box::new(MinimalTaggedObj::new(2)), 
        ],
        vec![
            Box::new(MinimalRule::new(3)),
        ]
    );
    m.start();
}
