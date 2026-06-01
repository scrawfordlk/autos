fn test() -> usize {
    let mut x: usize = 41;
    let p: *mut usize = &mut x as *mut usize;
    let addr: usize = p as usize;
    let p2: *mut usize = addr as *mut usize;
    unsafe {
        *p2 = *p2 + 1;
    }
    x
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
