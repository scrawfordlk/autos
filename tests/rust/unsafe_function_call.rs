fn test() -> usize {
    unsafe { f(41) }
}

unsafe fn f(x: usize) -> usize {
    unsafe { g(x) }
}

unsafe fn g(x: usize) -> usize {
    x + 1
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
