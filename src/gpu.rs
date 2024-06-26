
use krnl::device::Device;
use once_cell::sync;

pub static DEVICE: sync::Lazy<Device> = sync::Lazy::new(|| {
    Device::builder().build().ok().unwrap_or(Device::host())
});

// #[module]
// pub mod kernels {
    
//     #[cfg(not(target_arch = "spirv"))]
//     use krnl::krnl_core;
//     use krnl_core::macros::kernel;

//     pub fn pix_rgb8_diff(x: [u8; 3], y: [u8; 3]) -> f32 {
//         (x[0] - y[0]) as f32 * (x[0] - y[0]) as f32 +
//         (x[1] - y[1]) as f32 * (x[1] - y[1]) as f32 +
//         (x[2] - y[2]) as f32 * (x[2] - y[2]) as f32
//     }

//     #[kernel]
//     pub fn cost_kernel(shape_height: u32, shape_width: u32
//         , #[global] row_nums: Slice<u32>
//         , #[global] matches_data_v: Slice<f32>
//         , #[global] l_pic: Slice<u8>
//         , #[global] r_pic: Slice<u8> 
//         , #[global] out: UnsafeSlice<f32>
//         ) {
//         use krnl_core::buffer::UnsafeIndex;
//         let pix_id = kernel.global_id() as u32;
//         let i: u32 = pix_id / shape_width;
//         let j: u32 = pix_id % shape_width;
//         let disp = matches_data_v[(pix_id*3 + 2) as usize];
//         let this_row = row_nums[i as usize] as usize - 1;
//         let mut pix_index_l = this_row * shape_width as usize;
//         pix_index_l = pix_index_l + j as usize;
//         let shift = {
//             let r_j =  j as f32 + disp;
//             if r_j < 0f32 {
//                 - (j as f32)
//             } else if r_j > ( shape_width - 1 ) as f32 {
//                 (shape_width - 1 - j) as f32
//             } else {
//                 disp
//             }
//         };
//         let pix_index_r = ( pix_index_l as f32 + shift ) as usize ;
//         let mut cost = pix_rgb8_diff(
//             [l_pic[pix_index_l*3], l_pic[pix_index_l*3 + 1], l_pic[pix_index_l*3 + 2]]
//             ,[r_pic[pix_index_r*3], r_pic[pix_index_r*3 + 1], r_pic[pix_index_r*3 + 2]]
//         );
//         unsafe { *out.unsafe_index_mut(pix_id as usize) = cost; }
//     }

// }



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