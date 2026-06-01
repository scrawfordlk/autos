fn test() -> usize {
    let mut x: usize = 1;
    mutate(&mut x);
    x
}

fn mutate(reference: &mut usize) {
    *reference = 42;
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
