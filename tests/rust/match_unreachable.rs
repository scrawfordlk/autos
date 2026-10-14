fn test() -> usize {
    match 4 {
        x => 42,
        x => 7,
        y => 7,
        z => 7,
        5 => 7,
        _ => 7,
    }
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
