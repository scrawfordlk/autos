fn main() -> usize {
    wrapper()
}

fn wrapper() -> ! {
    unsafe { exit(42) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
