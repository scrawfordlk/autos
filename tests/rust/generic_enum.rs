enum Either<T> {
    Left(T),
    Right(T),
}

fn test() -> usize {
    let either: Either<usize> = Either::<usize>::Right(42);
    match either {
        Either::Left(value) => value,
        Either::Right(value) => value,
    }
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
    fn malloc(size: usize) -> *mut u8;
}
