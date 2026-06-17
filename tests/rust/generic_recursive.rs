fn pass<T>(parameter: T, count: usize) -> T {
    if count == 0 {
        parameter
    } else {
        pass::<T>(parameter, pass::<usize>(count - 1, id::<usize>(0)))
    }
}

fn id<T>(x: T) -> T {
    x
}

fn test() -> usize {
    pass::<u8>(42 as u8, 10) as usize
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
