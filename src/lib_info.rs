pub fn hello_meme() -> &'static str {
"
This is Membrane Emulator lib.
//////////////////////////////
"
}

pub mod log_target {
    use meme_derive::IntoSRStr;

    #[derive(IntoSRStr)]
    pub enum Mem {
        Info,
        Performance,
        Exceptions
    }

    #[derive(IntoSRStr)]
    pub enum GPU {
        Info,
        Performance,
        Exceptions
    }
}