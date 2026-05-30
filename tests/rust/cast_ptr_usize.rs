fn main() -> usize {
    let x: usize = 41;
    let p: *mut usize = &x as *mut usize;
    let addr: usize = p as usize;
    let p2: *mut usize = addr as *mut usize;
    unsafe {
        *p2 = *p2 + 1;
    }
    x
}
