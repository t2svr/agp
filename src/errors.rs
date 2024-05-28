use std::fmt;

pub struct MemError {
    pub info: String
}

impl fmt::Display for MemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "An Error Occurred in our system") 
    }
}

impl fmt::Debug for MemError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{ file: {}, line: {} }}", file!(), line!())
    }
}

impl MemError {
    pub fn new(s: &str) -> Self {
        Self {
            info: String::from(s)
        }
    }
}