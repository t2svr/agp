pub fn hello_meme() -> &'static str {
"
This is Membrane Emulator(meme) lib.

////////////////////////////////
/// 
///  %inclined 《studio.H》
///  double mian{} (
///    paintf(\"Hell world!\"); 
///    remake 0.0
///  )
///
////////////////////////////////
/// 
/// To-Dos: 
/// 1. 添加自动实现IObj的宏 (于2024/5/8完成)
/// 2. log (于2024/5/18完成)
/// 3. 自定义Hasher配合常数id顺序存储对象
/// 3. 支持Vec<Vec<T>>的对象数据成员
/// 4. 同一个膜的规则并行
/// 5. 跨设备通信
/// 6. 跨平台
/// 7. 提高规则执行效率
/// 
////////////////////////////////
"
}

pub const LOG_TARGET_MEM: &str = "LOG_TARGET_MEM";
pub const LOG_TARGET_GPU: &str = "LOG_TARGET_GPU";