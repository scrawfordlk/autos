fn test() -> usize {
    let mut i: usize = 0;
    while i < 1000000 {
        let x: usize = i;
        let y: usize = i;
        let z: usize = x + y;
        i = i + 1;
    }
    42
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
