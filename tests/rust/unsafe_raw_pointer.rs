fn main() -> usize {
    let mut x: usize = 40;
    let p: *mut usize = (&mut x) as *mut usize;
    unsafe {
        *p = *p + 2;
    };
    x
}
