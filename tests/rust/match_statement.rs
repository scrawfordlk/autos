fn test() -> usize {
    let mut x: usize = 21;
    let y: usize = 1;

    match x {
        100 => return 2,
        4 => x = 3,
        y => x = x + y,
    }

    x
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
