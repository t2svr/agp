use std::any::TypeId;
use std::collections::HashMap;

use krnl::buffer::{Buffer, BufferBase, BufferRepr};
use log::Level;
use log::log;

use crate::core::GeneralNeed;
use crate::core::Needs;
use crate::gpu;
use crate::lib_info;

pub struct NeedsBuilder<IdType>{
    general: Option<Vec<GeneralNeed>>,
    specific: Option<Vec<(IdType, bool)>>,
    is_random: bool,
    is_take: bool
}

impl<T> Default for NeedsBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> NeedsBuilder<T> {
    pub fn new() -> Self {
        Self {
            general: Some(Vec::new()),
            specific: Some(Vec::new()),
            is_random: false,
            is_take: false
        }
    }
    
    /// 选取count个类型为ObjT的对象
    pub fn some<ObjT: 'static>(mut self, count: usize) -> NeedsBuilder<T> {
        if let Some(ref mut n) = self.general {
           n.push(GeneralNeed { 
                tid: TypeId::of::<ObjT>(),
                count: Some(count), 
                is_take: self.is_take, 
                is_random: self.is_random 
            });
        }
        self
    }

    /// 选取所有类型为ObjT的对象
    pub fn all<ObjT: 'static>(mut self) -> NeedsBuilder<T> {
        if let Some(ref mut n) = self.general {
            n.push(GeneralNeed { 
                 tid: TypeId::of::<ObjT>(),
                 count: None, 
                 is_take: self.is_take, 
                 is_random: self.is_random 
             });
         }
        self
    }

    /// 选取指定id的对象
    pub fn the(mut self, id: T) -> NeedsBuilder<T> {
        if let Some(ref mut g) = self.specific {
            g.push((id, self.is_take));
        }
        self
    }

    /// 规则每次运行 获取的对象都随机排列 这会有额外的运算开销 复杂度取决于shuffle的算法
    pub fn randomly(mut self) -> NeedsBuilder<T> {
        self.is_random = true;
        self
    }

    ///  规则每次运行 获取的对象都按照默认的顺序 默认顺序取决于Hasher
    pub fn sequentially(mut self) -> NeedsBuilder<T> {
        self.is_random = false;
        self
    }

    /// 之后选取的对象会被释放
    pub fn takes(mut self) -> NeedsBuilder<T> {
        self.is_take = true;
        self
    }

    /// 之后选取的对象会被保留
    pub fn reads(mut self) -> NeedsBuilder<T> {
        self.is_take = false;
        self
    }

    pub fn build(&mut self) -> Needs<T>  {
        let mut pos_map = HashMap::<TypeId, usize>::new();
        let general =  self.general.take().unwrap();
        let specific =  self.specific.take().unwrap();
        for (i ,tid) in general.iter().map(|n| n.tid).enumerate() {
            pos_map.insert(tid, i);
        }
        Needs { 
            pos_map, 
            general_count: general.len(), specific_count: specific.len(),
            general, specific
        }
    }
    
}

/// 获取对象需求构造器
#[inline]
pub fn needs_map_builder<T>() -> NeedsBuilder<T> {
    NeedsBuilder::new()
}

/// 创建GPU缓冲区
#[inline]
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

/// 批量移除Vec元素(就地unsafe)
#[inline]
#[allow(unused_assignments)]
pub fn vec_batch_remove_inplace<T>(v: &mut Vec<T>, indexes: &Vec<usize>) {
    let mut indexes_sorted = indexes.clone();
    indexes_sorted.sort_unstable();
    let (mut cp_start_pos, mut cp_end_pos, mut cp_to_pos) = (0usize, 0usize, 0usize);
    let mut i = 0;
    while i < indexes_sorted.len() {
        assert!((0..v.len()).contains(&indexes_sorted[i]));
        cp_to_pos = indexes_sorted[i] - i;
        cp_start_pos = indexes_sorted[i] + 1;
        while i + 1 < indexes_sorted.len() 
        && cp_start_pos == indexes_sorted[i + 1] {
            cp_start_pos += 1;
            i += 1;
        }
        cp_end_pos = if i + 1 < indexes_sorted.len() {
            indexes_sorted[i + 1] - 1
        } else { 
             v.len() - 1 
        };
        if cp_start_pos >= v.len() {
            break;
        }
        unsafe {
            let dst = v.as_mut_ptr().add(cp_to_pos);
            let src = v.as_mut_ptr().add(cp_start_pos);
            std::ptr::copy(src, dst, cp_end_pos - cp_start_pos + 1);
        }
        i += 1;
    }
    v.truncate(v.len() - indexes.len());
}
/* 
/// 批量移除Vec元素(迭代器复制)
pub fn vec_batch_remove_iter<T: Clone>(v: &Vec<T>, indexes: &Vec<usize>) -> Vec<T> {
    let mut removed = Vec::<bool>::with_capacity(v.len());
    removed.resize(v.len(), false);
    for i in indexes {
        removed[*i] = true;
    }
    v.iter().enumerate().filter_map(|(i, v)| {
        if removed[i] {  None } 
        else { Some(v.clone()) }
    }).collect::<Vec<T>>()
}

/// 批量移除Vec元素(手动复制)
pub fn vec_batch_remove<T: Clone>(v: &Vec<T>, indexes: &Vec<usize>) -> Vec<T> {
    let mut removed = Vec::<bool>::with_capacity(v.len());
    removed.resize(v.len(), false);
    for i in indexes {
        removed[*i] = true;
    }
    let mut res: Vec<T> = Vec::new();
    for i in 0..v.len() {
        if !removed[i] {
            res.push(v[i].clone());
        } 
    }
    res
}
*/

/*
unsafe {
            // infallible
            let ret;
            {
                // the place we are taking from.
                let ptr = self.as_mut_ptr().add(index);
                // copy it out, unsafely having a copy of the value on
                // the stack and in the vector at the same time.
                ret = ptr::read(ptr);

                // Shift everything down to fill in that spot.
                ptr::copy(ptr.add(1), ptr, len - index - 1);
            }
            self.set_len(len - 1);
            ret
        }
         */

        
    // let mut is_skip = true;
    // while j < remove_tag.len() {
    //     while j < remove_tag.len()
    //     && remove_tag[j] == true {
    //         is_skip = false;
    //         j += 1;
    //     }
    //     if j == remove_tag.len() {
    //         break;
    //     }
    //     if !is_skip {
    //         v[i] = v[j].clone();
    //     }
    //     i += 1;
    //     j += 1;
    // }