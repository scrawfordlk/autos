fn pass<T>(parameter: T) -> T {
    // let reference: &T = other::<T>(&parameter);
    parameter
}

fn test() -> usize {
    pass::<u8>(42 as u8); // type mismatch would say "T", not the instance
    42
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
