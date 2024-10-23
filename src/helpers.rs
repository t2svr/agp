use std::hash::Hash;
use std::sync::atomic::{self, Ordering};

use idgenerator::{IdGeneratorOptions, IdInstance};
use krnl::buffer::{Buffer, BufferBase, BufferRepr};
use krnl::scalar::Scalar;
use log::Level;
use log::log;

use crate::core::{ICondition, IObj, IRuleEffect, ObjCrateFn, ObjRemoveFn, ObjType, ObjsCrateFn, ObjsRemoveFn, OperationEffect, TaggedPresence, TaggedPresences, UntaggedPresence, UntaggedPresences};
use crate::gpu;
use crate::lib_info::log_target;

#[derive(Debug)]
pub struct EffectBuilder<E> {
    effs: Option<Vec<E>>,
}

impl<E> EffectBuilder<E> {
    pub fn new() -> Self {
        Self { effs: None }
    }
}

impl<E> Default for EffectBuilder<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, U> EffectBuilder<OperationEffect<T, U>>
where T: Send + Sync, U: Send + Sync {

    pub fn add_op(mut self, op: OperationEffect<T, U>) -> Self {
        let e = self.effs.get_or_insert(Vec::new());
        e.push(op);
        self
    }

    pub fn crate_objs(mut self, f: ObjsCrateFn<T, U>) -> Self {
        let e = self.effs.get_or_insert(Vec::new());
        e.push(OperationEffect::CreateObjs(f));
        self
    }

    pub fn crate_obj(mut self, f: ObjCrateFn<T, U>) -> Self {
        let e = self.effs.get_or_insert(Vec::new());
        e.push(OperationEffect::CreateObj(f));
        self
    }

    pub fn remove_obj(mut self, f: ObjRemoveFn<T, U>) -> Self {
        let e = self.effs.get_or_insert(Vec::new());
        e.push(OperationEffect::RemoveObj(f));
        self
    }

    pub fn remove_objs(mut self, f: ObjsRemoveFn<T, U>) -> Self {
        let e = self.effs.get_or_insert(Vec::new());
        e.push(OperationEffect::RemoveObjs(f));
        self
    }

    pub fn stop_mem(mut self) -> Self {
        let e = self.effs.get_or_insert(Vec::new());
        e.push(OperationEffect::Stop);
        self
    }

    pub fn build<RE: IRuleEffect<Effect = OperationEffect<T, U>>>(&mut self) -> RE {
        RE::from_builder(self.effs.take())
    }
}


// todo: tagged 和 untagged 的对象中，如果存在同类对象的处理方法 -ignored
// todo: tagged 对象的 amount 无法预测，需要即时判断 -ignored
#[derive(Debug)]
pub struct ConditionBuilder<T = u32, U = u32>
where T: Clone + Hash + Eq, U: Scalar{
    of_type: Option<UntaggedPresences<U>>,
    of_tag: Option<TaggedPresences<T>>
}

impl<T: Clone + Hash + Eq, U: Scalar> Default for ConditionBuilder<T, U> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + Hash + Eq, U: Scalar> ConditionBuilder<T, U> {
    pub fn new() -> Self {
        Self {
            of_type: None,
            of_tag: None
        }
    }
    
    pub fn some_untagged<Obj: IObj + ?Sized + 'static>(mut self, amount: U) -> Self {
        let oty = self.of_type.get_or_insert(Vec::new());
        oty.push(UntaggedPresence {
            ty: ObjType::new::<Obj>(&crate::core::DEFAULT_GROUP),
            amount,
        });
        self
    }

    /// 选取指定tag的对象
    pub fn the_tagged(mut self, tag: T) -> Self {
        let otg = self.of_tag.get_or_insert(Vec::new());
        otg.push(TaggedPresence::OfTag(tag));
        self
    }

     /// 选取指定tag的对象
     pub fn some_tagged(mut self, tags: Vec<T>) -> Self {
        let otg = self.of_tag.get_or_insert(Vec::new());
        for t in tags {
            otg.push(TaggedPresence::OfTag(t));
        }
        self
    }

    /// 随机选择tagged对象，随机选择会有开销（`O(n)`，`n` 为对象数量），如果选择失败的情况较多，   
    /// 可以使用[`ConditionBuilder::some_untagged`]来要求该类对象数量满足  
    /// 因为untagged会被优先检查，失败时提前返回，避免选择的开销，例如：  
    /// 
    /// 
    /// ```
    /// use meme_derive::IObj;
    /// use meme::rules::BasicCondition;
    /// 
    /// #[derive(IObj, Debug)]
    /// struct TestObj {
    ///     #[tag]
    ///     tag: i32
    /// }
    /// 
    /// let cond = meme::helpers::condition_builder()
    ///            .rand_tagged::<TestObj>(3)
    ///            .some_untagged::<TestObj>(3)
    ///            .build::<BasicCondition<i32>>();
    /// ```
    pub fn rand_tagged<Obj: IObj + 'static>(mut self, count: usize) -> Self {
        let otg = self.of_tag.get_or_insert(Vec::new());
        otg.push(TaggedPresence::RandTags((ObjType::new::<Obj>(&crate::core::DEFAULT_GROUP), count)));
        self
    }

    pub fn build<C: ICondition<T, U>>(&mut self) -> C {
        C::from_builder(self.of_type.take(), self.of_tag.take())
    }
    
}

/// 获取条件构造器
#[inline]
pub fn condition_builder<T, U>() -> ConditionBuilder<T, U> 
where T: Clone + Hash + Eq, U: Scalar {
    ConditionBuilder::new()
}

#[inline]
pub fn condition_empty<T, U, C>() -> C 
where T: Clone + Hash + Eq, U: Scalar, C: ICondition<T, U> {
    C::from_builder(None, None)
}

/// 获取影响构造器
#[inline]
pub fn effect_builder<E>() -> EffectBuilder<E> {
    EffectBuilder::new()
}

#[inline]
pub fn effect_empty<RE: IRuleEffect>() -> RE {
    RE::from_builder(None)
}


// todo: 全局tag生成器 -ok

static COUNTER_USIZE: atomic::AtomicUsize = atomic::AtomicUsize::new(0);
static COUNTER_U32: atomic::AtomicU32 = atomic::AtomicU32::new(0);
static COUNTER_I32: atomic::AtomicI32 = atomic::AtomicI32::new(0);

pub static ID_GEN: once_cell::sync::Lazy<IdGen> = once_cell::sync::Lazy::new(|| {
    let gen = IdGen {};
    gen.init_id_gen();
    gen
});

/// 注意，只能用于同一程序内的ID生成
pub struct IdGen {

}

impl IdGen {
    pub fn init_id_gen(&self) {
        let _ = IdInstance::init(IdGeneratorOptions::new().worker_id_bit_len(8).seq_bit_len(3).worker_id(0));
    }
    
    /// 唯一ID生成，只用于i64，较慢，随机均匀
    pub fn next_i64_id(&self) -> i64 {
        IdInstance::next_id()
    }

    /// 唯一ID生成，用于usize，快，顺序
    pub fn next_usize_id() -> usize { 
        COUNTER_USIZE.fetch_add(1, Ordering::Relaxed) 
    }
    /// 唯一ID生成，用于u32，快，顺序
    pub fn next_u32_id() -> u32 { 
        COUNTER_U32.fetch_add(1, Ordering::Relaxed) 
    }
    /// 唯一ID生成，用于i32，快，顺序
    pub fn next_i32_id() -> i32 { 
        COUNTER_I32.fetch_add(1, Ordering::Relaxed) 
    }
}


/// 创建GPU缓冲区(来自数据)
pub fn gpu_buffer_from<T: krnl::scalar::Scalar>(data: Vec<T>) -> Option<BufferBase<BufferRepr<T>>> {
    let size = data.len();
    let res = Buffer::from(data).into_device(gpu::DEVICE.clone());
    match res {
        Ok(buf) => {
            log!(
                target: log_target::GPU::Info.into(), 
                Level::Info, 
                "Created gpu buffer of size {}",
                buf.len()
            );
            Some(buf)
        },
        Err(e) => {
            log!(
                target: log_target::GPU::Exceptions.into(), 
                Level::Error, 
                "Failed to create gpu buffer for data of size {} : {:?}", 
                size, e
            );
            None
        },
    }
}

/// 创建GPU缓冲区(全零)
pub fn gpu_buffer_zeros<T: krnl::scalar::Scalar>(size: usize) -> Option<BufferBase<BufferRepr<T>>> {
    let res = Buffer::zeros(gpu::DEVICE.clone(), size);
    match res {
        Ok(buf) => {
            log!(
                target: log_target::GPU::Info.into(), 
                Level::Info, 
                "Created gpu buffer zeros of size {}",
                buf.len()
            );
            Some(buf)
        },
        Err(e) => {
            log!(
                target: log_target::GPU::Exceptions.into(), 
                Level::Error, 
                "Failed to create gpu buffer zeros for data of size {} : {:?}", 
                size, e
            );
            None
        },
    }
}

/// 批量移除Vec元素(就地unsafe)
#[inline]
#[allow(unused_assignments)]
pub fn vec_batch_remove_inplace<T>(v: &mut Vec<T>, indexes: &[usize]) {
    let mut indexes_sorted = indexes.to_owned();
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