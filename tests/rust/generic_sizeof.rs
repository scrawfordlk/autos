enum Composite {
    Small,
    Medium(usize),
    Large(usize, usize, usize),
}

fn compute_size<T>() -> usize {
    (size_of::<T>() == 16) as usize
        + (size_of::<Composite>() == 8 + 3 * 8) as usize
        + (size_of::<usize>() == 8) as usize
        + (size_of::<u8>() == 1) as usize
}

fn test() -> usize {
    compute_size::<&str>() + 38
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
