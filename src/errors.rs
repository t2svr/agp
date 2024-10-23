use std::fmt;

pub struct MemError<T> {
    pub info: String,
    pub data: Option<T>
}

impl<T> fmt::Display for MemError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "An Error Occurred in our system") 
    }
}

impl<T> fmt::Debug for MemError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{ file: {}, line: {} }}", file!(), line!())
    }
}

impl<T> From<crossbeam_channel::SendError<T>> for MemError<T> {
    fn from(value: crossbeam_channel::SendError<T>) -> Self {
        Self { info: value.to_string(), data: Some(value.into_inner()) }
    }
}

impl<T> From<crossbeam_channel::RecvError> for MemError<T> {
    fn from(value: crossbeam_channel::RecvError) -> Self {
        Self { info: value.to_string(), data: None }
    }
}

impl<T> From<crossbeam_channel::TryRecvError> for MemError<T> {
    fn from(value: crossbeam_channel::TryRecvError) -> Self {
        Self { info: value.to_string(), data: None }
    }
}

impl<T> MemError<T> {
    pub fn new(s: &str) -> Self {
        Self {
            info: String::from(s),
            data: None
        }
    }
}