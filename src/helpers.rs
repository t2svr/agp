use std::any::TypeId;

use krnl::buffer::{Buffer, BufferBase, BufferRepr};
use log::Level;
use log::log;

use crate::core::NeedCount;
use crate::core::NeedsMap;
use crate::gpu;
use crate::lib_info;


pub fn needs_map_builder() -> NeedsBuilder {
    NeedsBuilder::new()
}
pub struct NeedsBuilder{
    needs_v: Vec<(TypeId, NeedCount, bool)>
}

impl Default for NeedsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl NeedsBuilder {
    pub fn new() -> Self {
        Self {
            needs_v: Vec::new()
        }
    }
    
    /// 表示该规则读取count个T对象
    pub fn reads<T: 'static>(mut self, count: NeedCount) -> NeedsBuilder {
        if count.is_some() {
            self.needs_v.push((TypeId::of::<T>(), count, false));
        } else {
            self.needs_v.push((TypeId::of::<T>(), NeedCount::All, false));
        }
        self
    }

    /// 表示该规则消耗count个T对象
    pub fn takes<T: 'static>(mut self, count: NeedCount) -> NeedsBuilder {
        if count.is_some() {
            self.needs_v.push((TypeId::of::<T>(), count, true));
        } else {
            self.needs_v.push((TypeId::of::<T>(), NeedCount::All, true));
        }
        self
    }

    pub fn build(&mut self) -> NeedsMap {
        let mut res = NeedsMap::new();
        self.needs_v.reverse();
        let mut pos = 0usize;
        while let Some((id, c, is_take)) = self.needs_v.pop() {
            res.insert(id, (pos, c, is_take));
            pos += 1;
        }
        res
    }
    
}

pub fn make_gpu_buffer<T: krnl::scalar::Scalar>(data: Vec<T>) -> Option<BufferBase<BufferRepr<T>>> {
    let res = Buffer::from(data).into_device(gpu::DEVICE.clone());
    match res {
        Ok(buf) => {
            Some(buf)
        },
        Err(e) => {
            log!(target: lib_info::LOG_TARGET_GPU, Level::Error, "Failed to create gpu buffer: {:?}", e);
            None
        },
    }
}