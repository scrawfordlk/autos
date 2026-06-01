fn test() -> usize {
    unsafe {
        let p: *mut usize = wrapper(4);
        *p = 21;
        *ptr_add(p, 1) = 21;
        *ptr_add(p, 2) = 10;
        *ptr_add(p, 3) = 11;
        *p + *ptr_add(p, 1) + *ptr_add(p, 2) + *ptr_add(p, 3)
    }
}

fn wrapper(usize_count: usize) -> *mut usize {
    unsafe { malloc(usize_count * 8) as *mut usize } // TODO: do not hardcode usize size
}

fn ptr_add(p: *mut usize, offset: usize) -> *mut usize {
    (p as usize + offset * 8) as *mut usize // TODO: do not hardcode usize size
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn exit(code: usize) -> !;
}
