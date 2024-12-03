Start:

- install rustup
- install rust-toolchains
- cargo install cargo-make
- cargo install clippy
- cargo install flamegraph


并行化所用的库：
CPU并行：Rayon
GPU并行：krnl

膜逻辑上有包含关系 实现时没有 内外层膜通信方式和同级膜通信方式实现上完全一致 
一个膜能被其他膜感知 完全依靠通信对象

已知缺陷： 
对于通信规则，目前使用锁来获取传输的对象，会带来额外开销  
可以改进为无锁实现，可能的方案为将通信能力转移到膜对象上  

每一个BasicMem对象占用内存：  

```
T *1  
bool *3  
Vec *3  
AHashMap *4  

设  
n：对象数量  
m：规则数量  
k：对象种类数量  
l：规则种类数量  
q: HashMap 每个Entry的维护开销  

其中：  
Vec<(TypeId, U)>        -- O(k)
Vec<(RT, PRule)>        -- O(m)
Vec<C>                  -- O(m)
AHashMap<TypeId, usize> -- O(l*q)
AHashMap<RT, usize>     -- O(m*q)
AHashMap<OT, PObj>      -- O(n*q)
AHashMap<TypeId, U>     -- O(k*q)
```

每个膜内的规则不会太多，对象种类也不会太多，因此主要开销来自：  
```
AHashMap<OT, PObj>      -- O(n*q)
```

Rust 的 AHashMap 足够高效，因此在规则和对象种类较少时也可以使用  
所以本库目前没有为了少对象的膜进行特定优化

todo:
使规则可以获得规则的引用
使规则可以增加规则

next milestone:
加速随机对象选择


