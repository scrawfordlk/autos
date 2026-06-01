fn test() -> usize {
    let mut x: usize = 41;
    {
        let mut_ref_x: &mut usize = &mut x;
        *mut_ref_x = *mut_ref_x + 1;
    }
    x
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
