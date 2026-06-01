fn test() -> usize {
    if 4 == 0 {
        return 1;
    }

    if 3 == 3 {
        return 42;
    }

    2
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
