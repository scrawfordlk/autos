fn test() -> usize {
    unsafe {
        let p: *mut usize = wrapper(4);
        *p = 44;
        *ptr_add(p, 1) = 55;
        *ptr_add(p, 2) = 66;
        *ptr_add(p, 3) = 77;
        eval(p, 44) + eval(ptr_add(p, 1), 55) + eval(ptr_add(p, 2), 66) + eval(ptr_add(p, 3), 77) + 38
    }
}

fn eval(p: *mut usize, expected: usize) -> usize {
    (unsafe { *p } == expected) as usize
}

fn wrapper(usize_count: usize) -> *mut usize {
    unsafe {
        let p: *mut usize = malloc(usize_count * 8) as *mut usize; // hardcoded to not depend on generics
        if p as usize == 0 {
            exit(6)
        }
        p
    }
}

fn ptr_add(p: *mut usize, offset: usize) -> *mut usize {
    (p as usize + offset * 8) as *mut usize // hardcoded to not depend on generics
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn exit(code: usize) -> !;
}
