fn test() -> usize {
    let mut i: usize = 0;
    while i < 100 {
        while i == 42 {
            return i;
        }

        i = i + 1;
    }
    return 0;
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
