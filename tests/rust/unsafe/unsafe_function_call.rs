fn main() -> usize {
    unsafe { f(41) }
}

unsafe fn f(x: usize) -> usize {
    g(x)
}

unsafe fn g(x: usize) -> usize {
    x + 1
}
