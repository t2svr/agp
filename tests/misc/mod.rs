use meme;

#[test]
pub fn mem_hello() {
    assert_eq!(meme::lib_info::hello_meme(), "hello meme.")
}
