
mod misc;
mod mems;

#[test]
fn all() {
    misc::mem_hello();
    mems::test_mem::basics();
}
