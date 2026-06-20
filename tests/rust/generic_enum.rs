enum Option<T> {
    Some(T),
    None,
}

fn test() -> usize {
    let opt: Option<usize> = Option::<usize>::Some(10);
    42
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
    fn malloc(size: usize) -> *mut u8;
}
