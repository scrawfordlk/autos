fn test() -> usize {
    {
        return 42;
        return 0;
    }
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
