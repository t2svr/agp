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

impl<DT> From<crossbeam_channel::SendError<DT>> for MemError {
    fn from(value: crossbeam_channel::SendError<DT>) -> Self {
        Self { info: value.to_string() }
    }
}

impl From<crossbeam_channel::RecvError> for MemError {
    fn from(value: crossbeam_channel::RecvError) -> Self {
        Self { info: value.to_string() }
    }
}

impl From<crossbeam_channel::TryRecvError> for MemError {
    fn from(value: crossbeam_channel::TryRecvError) -> Self {
        Self { info: value.to_string() }
    }
}

impl MemError {
    pub fn new(s: &str) -> Self {
        Self {
            info: String::from(s)
        }
    }
}