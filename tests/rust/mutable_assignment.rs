fn test() -> usize {
    let mut x: usize = 1000;
    x = 21;
    x = 2 * x;
    x
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
