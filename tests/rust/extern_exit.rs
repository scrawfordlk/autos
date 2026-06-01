fn test() -> usize {
    wrapper()
}

fn wrapper() -> ! {
    unsafe { exit(42) }
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
