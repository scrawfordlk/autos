fn test() -> usize {
    pow(2, 5) + pow(3, 2) + pow(9, 0)
}

fn pow(base: usize, exp: usize) -> usize {
    if exp == 0 {
        return 1;
    }

    return base * pow(base, exp - 1);
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
