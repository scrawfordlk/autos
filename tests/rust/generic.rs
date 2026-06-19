fn end<T>(value: T) -> T {
    value
}

fn other<T>(value: T) -> T {
    deep::<T>(value)
}

fn pass<T>(parameter: T) -> usize {
    other::<T>(parameter);
    other::<usize>(42)
}

fn deep<T>(arg: T) -> T {
    return end::<T>(arg);
}

fn test() -> usize {
    pass::<u8>(1 as u8);
    pass::<u8>(2 as u8)
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
