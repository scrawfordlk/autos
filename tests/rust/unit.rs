fn main() {
    let x: () = {};
    let _y: () = x;
    unsafe { exit(42) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
