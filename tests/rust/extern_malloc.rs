unsafe extern "C" {
    fn malloc(size: usize) -> *mut u8;
}

fn wrapper(usize_count: usize) -> *mut usize {
    unsafe { malloc(usize_count * 8) as *mut usize } // TODO: do not hardcode usize size
}

fn main() -> usize {
    let ptr: *mut usize = wrapper(4);
    unsafe {
        *ptr = 42;
        *ptr
    }
}

// fn main() -> usize {
//     unsafe {
//         let ptr: *mut usize = wrapper(4);
//         *ptr = 21;
//         *ptr_add(ptr, 1) = 21;
//         *ptr_add(ptr, 2) = 10;
//         *ptr_add(ptr, 3) = 11;
//         *ptr + *ptr_add(ptr, 1) + *ptr_add(ptr, 2) + *ptr_add(ptr, 3)
//     }
// }
//
// fn ptr_add(ptr: *mut usize, offset: usize) -> *mut usize {
//     (ptr as usize + offset * 8) as *mut usize // TODO: do not hardcode usize size
// }
