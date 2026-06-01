fn test() -> usize {
    let mut x: usize = 0;
    let mut seen: usize = {
        let ref_x: &usize = &x;
        x = 41;
        *ref_x
    };
    let mut_ref_x: &mut usize = &mut x;
    *mut_ref_x = seen + 1;
    x
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
