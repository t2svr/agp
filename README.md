1. Everything in MEME is obj ( reason rules are obj: in application it is uncertain which rules a mem will apply, take rules as objs helps save memory, cpu and makes design easier)
    - objs are in all kinds of types
    - there could be a certain amount of objs of a type
2. obj stored in ObjStore
    - ObjStore in memory
    - ObjStore in GPU memory
    - ObjStore in Disk
    - ObjStore in Distributed Database
3. Rule has Condition and Effect
4. Mem owns ObjStore (of rule, mem and other objs)
5. 



def:
1. obj
2. rule obj and mem obj
3. 

并行化所用的库：
CPU并行：Rayon
GPU并行：krnl

膜逻辑上有包含关系 实现时没有 内外层膜通信方式和同级膜通信方式实现上完全一致 
一个膜能被其他膜感知 完全依靠通信对象