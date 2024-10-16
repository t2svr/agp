
use krnl::device::Device;
use log::{log, Level};
use once_cell::sync;

use crate::lib_info::log_target;

pub static DEVICE: sync::Lazy<Device> = sync::Lazy::new(|| {
    match Device::builder().build() {
        Ok(dev) => {
            log!(
                target: log_target::GPU::Info.into(), 
                Level::Info, 
                "GPU device crated: {:?},",
                dev.info()
            );
            dev
        },
        Err(e) => {
            log!(
                target: log_target::GPU::Exceptions.into(), 
                Level::Error, 
                "Can't build gpu device {:?} \n    return default",
                e
            );
            Device::default()
        }
    }

});


/*
Pass --debug-printf to krnlc to enable

#[kernel]
fn foo(x: f32) {
    use krnl_core::spirv_std; // spirv_std must be in scope
    use spirv_std::macros::debug_printfln;

    unsafe {
        debug_printfln!("Hello World!");
    }
}
 */